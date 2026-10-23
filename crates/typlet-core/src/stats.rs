// SPDX-License-Identifier: GPL-3.0-or-later
//! Local usage statistics: how often snippets were inserted and how much
//! typing that saved, with streaks and milestones to make it fun.
//!
//! Only counts, dates and snippet ids are stored, never any text, and the
//! file never leaves the computer
//! ([ADR 0017](https://github.com/typlet-app/typlet/blob/main/docs/adr/0017-local-statistics.md)).
//! The background program records insertions; the editor shows them.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use chrono::{Days, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::fsutil::write_atomic;
use crate::snippet::SnippetId;

/// Version of the statistics file.
pub const FORMAT_VERSION: u32 = 1;

/// Typing speed used to estimate the time saved: 200 characters (about 40
/// words) per minute, an average typist.
pub const CHARS_PER_MINUTE: u64 = 200;

/// How many days of daily counts are kept, for streaks and the activity
/// chart. Totals are kept forever.
pub const KEEP_DAYS: u64 = 371;

/// Insertion counts that are celebrated.
pub const INSERTION_MILESTONES: [u64; 7] = [10, 50, 100, 500, 1_000, 5_000, 10_000];

/// Hours of typing saved that are celebrated.
pub const HOUR_MILESTONES: [u64; 5] = [1, 5, 10, 50, 100];

/// Streaks (days in a row with at least one insertion) that are celebrated.
pub const STREAK_MILESTONES: [u32; 5] = [3, 7, 30, 100, 365];

/// The statistics file (`stats.json` in the data folder).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    /// Format version.
    pub version: u32,
    /// The first day anything was counted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<NaiveDate>,
    /// Snippets inserted in total.
    pub insertions: u64,
    /// Characters the user did not have to type: inserted characters minus
    /// the typed trigger.
    pub characters_saved: u64,
    /// The longest streak so far, in days.
    pub best_streak: u32,
    /// Insertions per day, for the last [`KEEP_DAYS`] days.
    pub days: BTreeMap<NaiveDate, u32>,
    /// Use per snippet.
    pub snippets: BTreeMap<SnippetId, SnippetUse>,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            since: None,
            insertions: 0,
            characters_saved: 0,
            best_streak: 0,
            days: BTreeMap::new(),
            snippets: BTreeMap::new(),
        }
    }
}

/// How often one snippet was used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetUse {
    /// Number of insertions.
    pub count: u64,
    /// The last day it was inserted.
    pub last_used: NaiveDate,
}

/// Something worth celebrating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Milestone {
    /// This many snippets inserted.
    Insertions(u64),
    /// This many hours of typing saved.
    HoursSaved(u64),
    /// This many days in a row with at least one insertion.
    Streak(u32),
}

impl Milestone {
    /// Every milestone, in the order they are shown.
    pub fn all() -> impl Iterator<Item = Self> {
        INSERTION_MILESTONES
            .into_iter()
            .map(Self::Insertions)
            .chain(HOUR_MILESTONES.into_iter().map(Self::HoursSaved))
            .chain(STREAK_MILESTONES.into_iter().map(Self::Streak))
    }
}

impl Stats {
    /// Reads the file. A missing file means nothing was counted yet.
    ///
    /// A damaged file is renamed to `stats.json.damaged`, so it can be
    /// inspected, and counting starts again.
    #[must_use]
    pub fn load(path: &Path) -> Self {
        let Ok(text) = fs::read_to_string(path) else {
            return Self::default();
        };
        if let Ok(stats) = serde_json::from_str::<Self>(&text) {
            return stats;
        }
        let mut damaged = path.as_os_str().to_owned();
        damaged.push(".damaged");
        let _ = fs::rename(path, damaged);
        Self::default()
    }

    /// Writes the file atomically.
    ///
    /// # Errors
    ///
    /// Fails if the file cannot be written.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let mut json = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        json.push('\n');
        write_atomic(path, json.as_bytes())
    }

    /// Counts one insertion of `snippet` that saved `saved` characters of
    /// typing on `today`. Returns the milestones reached by it.
    pub fn record(&mut self, snippet: SnippetId, saved: u64, today: NaiveDate) -> Vec<Milestone> {
        let before = self.clone();
        self.since = Some(self.since.map_or(today, |since| since.min(today)));
        self.insertions += 1;
        self.characters_saved += saved;
        *self.days.entry(today).or_insert(0) += 1;
        let used = self.snippets.entry(snippet).or_insert(SnippetUse {
            count: 0,
            last_used: today,
        });
        used.count += 1;
        used.last_used = used.last_used.max(today);
        self.best_streak = self.best_streak.max(self.streak(today));
        self.prune(today);
        Milestone::all()
            .filter(|&m| self.reached(m) && !before.reached(m))
            .collect()
    }

    /// Takes back an insertion the user undid right away.
    pub fn unrecord(&mut self, snippet: SnippetId, saved: u64, today: NaiveDate) {
        self.insertions = self.insertions.saturating_sub(1);
        self.characters_saved = self.characters_saved.saturating_sub(saved);
        if let Some(count) = self.days.get_mut(&today) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.days.remove(&today);
            }
        }
        if let Some(used) = self.snippets.get_mut(&snippet) {
            used.count = used.count.saturating_sub(1);
            if used.count == 0 {
                self.snippets.remove(&snippet);
            }
        }
    }

    /// Days in a row with at least one insertion, ending today, or
    /// yesterday if nothing was inserted today yet (the streak is still
    /// alive until the day is over).
    #[must_use]
    pub fn streak(&self, today: NaiveDate) -> u32 {
        let active = |day: NaiveDate| self.days.get(&day).is_some_and(|&count| count > 0);
        let mut day = if active(today) {
            today
        } else if let Some(yesterday) = today.pred_opt().filter(|&d| active(d)) {
            yesterday
        } else {
            return 0;
        };
        let mut streak = 0;
        while active(day) {
            streak += 1;
            match day.pred_opt() {
                Some(previous) => day = previous,
                None => break,
            }
        }
        streak
    }

    /// Estimated minutes of typing saved, at [`CHARS_PER_MINUTE`].
    #[must_use]
    pub fn minutes_saved(&self) -> u64 {
        self.characters_saved / CHARS_PER_MINUTE
    }

    /// `true` if the milestone has been reached.
    #[must_use]
    pub fn reached(&self, milestone: Milestone) -> bool {
        match milestone {
            Milestone::Insertions(count) => self.insertions >= count,
            Milestone::HoursSaved(hours) => self.minutes_saved() >= hours * 60,
            Milestone::Streak(days) => self.best_streak >= days,
        }
    }

    /// The most used snippets, most used first (ties: most recent first).
    #[must_use]
    pub fn top(&self, count: usize) -> Vec<(SnippetId, SnippetUse)> {
        let mut all: Vec<(SnippetId, SnippetUse)> = self
            .snippets
            .iter()
            .map(|(id, used)| (*id, *used))
            .collect();
        all.sort_by(|a, b| {
            b.1.count
                .cmp(&a.1.count)
                .then(b.1.last_used.cmp(&a.1.last_used))
        });
        all.truncate(count);
        all
    }

    /// Insertions per day for the `count` days ending `today`, oldest first.
    #[must_use]
    pub fn recent_days(&self, today: NaiveDate, count: u64) -> Vec<(NaiveDate, u32)> {
        (0..count)
            .rev()
            .filter_map(|ago| today.checked_sub_days(Days::new(ago)))
            .map(|day| (day, self.days.get(&day).copied().unwrap_or(0)))
            .collect()
    }

    /// Forgets daily counts older than [`KEEP_DAYS`].
    fn prune(&mut self, today: NaiveDate) {
        if let Some(oldest) = today.checked_sub_days(Days::new(KEEP_DAYS)) {
            self.days.retain(|day, _| *day > oldest);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn records_and_takes_back_insertions() {
        let mut stats = Stats::default();
        let sig = SnippetId::new();
        let mfg = SnippetId::new();
        stats.record(sig, 120, day("2026-10-22"));
        stats.record(sig, 120, day("2026-10-23"));
        stats.record(mfg, 20, day("2026-10-23"));
        assert_eq!(stats.since, Some(day("2026-10-22")));
        assert_eq!(stats.insertions, 3);
        assert_eq!(stats.characters_saved, 260);
        assert_eq!(stats.minutes_saved(), 1);
        assert_eq!(stats.days[&day("2026-10-23")], 2);
        assert_eq!(stats.top(1)[0].0, sig);
        assert_eq!(stats.top(5).len(), 2);

        stats.unrecord(mfg, 20, day("2026-10-23"));
        assert_eq!(stats.insertions, 2);
        assert_eq!(stats.characters_saved, 240);
        assert!(!stats.snippets.contains_key(&mfg));
        assert_eq!(stats.days[&day("2026-10-23")], 1);
    }

    #[test]
    fn streaks_end_after_a_day_without_insertions() {
        let mut stats = Stats::default();
        let id = SnippetId::new();
        for d in ["2026-10-19", "2026-10-20", "2026-10-21"] {
            stats.record(id, 1, day(d));
        }
        assert_eq!(stats.streak(day("2026-10-21")), 3);
        // Still alive the next day until the day is over.
        assert_eq!(stats.streak(day("2026-10-22")), 3);
        assert_eq!(stats.streak(day("2026-10-23")), 0);
        stats.record(id, 1, day("2026-10-23"));
        assert_eq!(stats.streak(day("2026-10-23")), 1);
        assert_eq!(stats.best_streak, 3);
    }

    #[test]
    fn milestones_are_reported_once() {
        let mut stats = Stats::default();
        let id = SnippetId::new();
        let today = day("2026-10-23");
        for _ in 0..9 {
            assert!(stats.record(id, 0, today).is_empty());
        }
        assert_eq!(stats.record(id, 0, today), [Milestone::Insertions(10)]);
        assert!(stats.record(id, 0, today).is_empty());
        // One hour of typing at once.
        assert_eq!(
            stats.record(id, 60 * CHARS_PER_MINUTE, today),
            [Milestone::HoursSaved(1)]
        );

        let mut streak = Stats::default();
        let reached: Vec<Milestone> = ["2026-10-21", "2026-10-22", "2026-10-23"]
            .into_iter()
            .flat_map(|d| streak.record(id, 0, day(d)))
            .collect();
        assert_eq!(reached, [Milestone::Streak(3)]);
        assert!(streak.reached(Milestone::Streak(3)));
        assert!(!streak.reached(Milestone::Streak(7)));
    }

    #[test]
    fn old_days_are_forgotten_but_totals_stay() {
        let mut stats = Stats::default();
        let id = SnippetId::new();
        stats.record(id, 5, day("2025-01-01"));
        stats.record(id, 5, day("2026-10-23"));
        assert_eq!(stats.days.len(), 1);
        assert_eq!(stats.insertions, 2);
        let recent = stats.recent_days(day("2026-10-23"), 3);
        assert_eq!(
            recent,
            [
                (day("2026-10-21"), 0),
                (day("2026-10-22"), 0),
                (day("2026-10-23"), 1)
            ]
        );
    }

    #[test]
    fn files_round_trip_and_damaged_files_are_set_aside() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stats.json");
        assert_eq!(Stats::load(&path), Stats::default());

        let mut stats = Stats::default();
        stats.record(SnippetId::new(), 42, day("2026-10-23"));
        stats.save(&path).unwrap();
        assert_eq!(Stats::load(&path), stats);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"2026-10-23\": 1"));

        std::fs::write(&path, "{ broken").unwrap();
        assert_eq!(Stats::load(&path), Stats::default());
        assert!(dir.path().join("stats.json.damaged").exists());
        assert!(!path.exists());
    }
}
