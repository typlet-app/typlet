// SPDX-License-Identifier: GPL-3.0-or-later
//! The statistics page: what the background program counted in
//! `stats.json` (numbers only, never text), prepared for display.

use chrono::NaiveDate;
use serde::Serialize;
use tauri::State;
use typlet_core::Snippet;
use typlet_core::stats::{Milestone, Stats};

use crate::commands::{EditorState, library_snippets, settings};

/// Days shown in the activity chart: 26 weeks.
const CHART_DAYS: u64 = 26 * 7;

/// How many of the most used snippets are listed.
const TOP: usize = 5;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct StatsView {
    /// Counting is switched on in the settings.
    enabled: bool,
    today: String,
    since: Option<String>,
    insertions: u64,
    characters_saved: u64,
    minutes_saved: u64,
    streak: u32,
    best_streak: u32,
    /// Insertions per day for the chart, oldest first.
    days: Vec<DayView>,
    top: Vec<TopView>,
    milestones: Vec<MilestoneView>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct DayView {
    date: String,
    count: u32,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct TopView {
    id: String,
    /// `None` if the snippet has been deleted since.
    trigger: Option<String>,
    label: Option<String>,
    count: u64,
    last_used: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct MilestoneView {
    milestone: Milestone,
    reached: bool,
}

fn view(stats: &Stats, snippets: &[Snippet], today: NaiveDate, enabled: bool) -> StatsView {
    StatsView {
        enabled,
        today: today.to_string(),
        since: stats.since.map(|day| day.to_string()),
        insertions: stats.insertions,
        characters_saved: stats.characters_saved,
        minutes_saved: stats.minutes_saved(),
        streak: stats.streak(today),
        best_streak: stats.best_streak,
        days: stats
            .recent_days(today, CHART_DAYS)
            .into_iter()
            .map(|(day, count)| DayView {
                date: day.to_string(),
                count,
            })
            .collect(),
        top: stats
            .top(TOP)
            .into_iter()
            .map(|(id, used)| {
                let snippet = snippets.iter().find(|s| s.id == id);
                TopView {
                    id: id.to_string(),
                    trigger: snippet.map(|s| s.trigger.clone()),
                    label: snippet.map(|s| s.label.clone()),
                    count: used.count,
                    last_used: used.last_used.to_string(),
                }
            })
            .collect(),
        milestones: Milestone::all()
            .map(|milestone| MilestoneView {
                milestone,
                reached: stats.reached(milestone),
            })
            .collect(),
    }
}

/// The statistics for the statistics page.
#[tauri::command]
pub(crate) fn load_stats(state: State<'_, EditorState>) -> StatsView {
    let paths = &state.paths;
    view(
        &Stats::load(&paths.stats),
        &library_snippets(paths),
        chrono::Local::now().date_naive(),
        settings(paths).statistics,
    )
}

/// Deletes the statistics. The background program reads the file before
/// every change, so it starts counting from zero.
#[tauri::command]
pub(crate) fn reset_stats(state: State<'_, EditorState>) -> Result<(), String> {
    match std::fs::remove_file(&state.paths.stats) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.to_string()),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use typlet_core::Document;

    #[test]
    fn shows_totals_streaks_and_names() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 23).unwrap();
        let sig = Snippet::new(":sig", "Signature", Document::from_plain_text("Anna"));
        let gone = Snippet::new(":old", "Deleted", Document::from_plain_text("x"));
        let mut stats = Stats::default();
        stats.record(sig.id, 400, today.pred_opt().unwrap());
        stats.record(sig.id, 400, today);
        stats.record(gone.id, 10, today);

        let view = view(&stats, std::slice::from_ref(&sig), today, true);
        assert_eq!(view.insertions, 3);
        assert_eq!(view.minutes_saved, 4);
        assert_eq!(view.streak, 2);
        assert_eq!(view.since.as_deref(), Some("2026-10-22"));
        assert_eq!(view.days.len(), 182);
        assert_eq!(view.days.last().map(|d| d.count), Some(2));
        assert_eq!(view.top[0].trigger.as_deref(), Some(":sig"));
        assert_eq!(view.top[1].trigger, None);
        assert!(view.milestones.iter().all(|m| !m.reached));

        let json = serde_json::to_string(&view.milestones[0]).unwrap();
        assert_eq!(
            json,
            r#"{"milestone":{"kind":"insertions","value":10},"reached":false}"#
        );
    }
}
