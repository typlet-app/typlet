// SPDX-License-Identifier: GPL-3.0-or-later
//! What is known about new versions of Typlet.
//!
//! The editor checks for updates and writes the result to a small file in
//! the data folder; the background program reads it and offers the update.
//! The background program itself never connects to the network
//! ([ADR 0015](https://github.com/typlet-app/typlet/blob/main/docs/adr/0015-automatic-updates.md)).

use std::cmp::Ordering;
use std::fs;
use std::io;
use std::path::Path;

use chrono::{NaiveDateTime, TimeDelta};
use serde::{Deserialize, Serialize};

use crate::fsutil::write_atomic;

/// How often the editor looks for a new version.
pub const CHECK_INTERVAL: TimeDelta = TimeDelta::hours(24);

/// A newer version that can be installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvailableUpdate {
    /// The new version, for example `0.2.0`.
    pub version: String,
    /// Release notes, if the release has any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Publication date as reported by the release.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// The update file (`update.json` in the data folder).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateState {
    /// When the editor last checked successfully (local time).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked_at: Option<NaiveDateTime>,
    /// The newest version found at that time, if it was newer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available: Option<AvailableUpdate>,
    /// The version the background program has already announced, so it does
    /// so only once.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub announced: Option<String>,
}

impl UpdateState {
    /// Reads the file. A missing or damaged file counts as "never checked":
    /// it is only a cache.
    #[must_use]
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
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

    /// `true` if the last check is older than [`CHECK_INTERVAL`] (or there
    /// was none, or the clock went backwards).
    #[must_use]
    pub fn is_due(&self, now: NaiveDateTime) -> bool {
        self.checked_at.is_none_or(|checked| {
            let age = now.signed_duration_since(checked);
            age >= CHECK_INTERVAL || age < TimeDelta::zero()
        })
    }

    /// The update to offer to a program running `current`, if it is newer.
    #[must_use]
    pub fn offer(&self, current: &str) -> Option<&AvailableUpdate> {
        self.available
            .as_ref()
            .filter(|update| is_newer(&update.version, current))
    }

    /// The update to announce: offered and not announced before.
    #[must_use]
    pub fn to_announce(&self, current: &str) -> Option<&AvailableUpdate> {
        self.offer(current)
            .filter(|update| self.announced.as_deref() != Some(update.version.as_str()))
    }
}

/// `true` if `candidate` is a newer version than `current`
/// (`major.minor.patch`, optionally with a pre-release such as `-rc.1`,
/// which comes before the release). Unreadable versions are never newer.
#[must_use]
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (Version::parse(candidate), Version::parse(current)) {
        (Some(candidate), Some(current)) => candidate.cmp(&current) == Ordering::Greater,
        _ => false,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Version<'a> {
    numbers: [u64; 3],
    pre: Option<&'a str>,
}

impl<'a> Version<'a> {
    fn parse(text: &'a str) -> Option<Self> {
        let text = text.trim().trim_start_matches('v');
        let text = text.split('+').next().unwrap_or(text); // build metadata
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (text, None),
        };
        let mut parts = core.split('.').map(|part| part.parse::<u64>().ok());
        let numbers = [parts.next()??, parts.next()??, parts.next()??];
        parts.next().is_none().then_some(Self { numbers, pre })
    }
}

impl Ord for Version<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.numbers
            .cmp(&other.numbers)
            .then_with(|| match (self.pre, other.pre) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(a), Some(b)) => compare_pre(a, b),
            })
    }
}

impl PartialOrd for Version<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Compares pre-release labels the semver way: dot-separated parts, numbers
/// numerically and before words.
fn compare_pre(a: &str, b: &str) -> Ordering {
    let mut a = a.split('.');
    let mut b = b.split('.');
    loop {
        match (a.next(), b.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let order = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(x), Ok(y)) => x.cmp(&y),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => x.cmp(y),
                };
                if order != Ordering::Equal {
                    return order;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(hour: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 10, 23)
            .and_then(|d| d.and_hms_opt(hour, 0, 0))
            .unwrap()
    }

    fn update(version: &str) -> AvailableUpdate {
        AvailableUpdate {
            version: version.into(),
            notes: None,
            date: None,
        }
    }

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("1.0.0", "1.0.0-rc.2"));
        assert!(is_newer("1.0.0-rc.10", "1.0.0-rc.2"));
        assert!(is_newer("1.0.0-rc.1", "1.0.0-beta.9"));
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("garbage", "0.1.0"));
        assert!(!is_newer("0.2", "0.1.0"));
        assert!(!is_newer("0.2.0.1", "0.1.0"));
    }

    #[test]
    fn checks_once_a_day() {
        let mut state = UpdateState::default();
        assert!(state.is_due(at(8)));
        state.checked_at = Some(at(8));
        assert!(!state.is_due(at(20)));
        assert!(state.is_due(at(8) + TimeDelta::hours(24)));
        // A clock that went backwards must not stop checking for good.
        assert!(state.is_due(at(7)));
    }

    #[test]
    fn offers_and_announces_newer_versions_once() {
        let mut state = UpdateState {
            available: Some(update("0.2.0")),
            ..UpdateState::default()
        };
        assert_eq!(state.offer("0.1.0"), Some(&update("0.2.0")));
        assert_eq!(state.to_announce("0.1.0"), Some(&update("0.2.0")));
        state.announced = Some("0.2.0".into());
        assert_eq!(state.to_announce("0.1.0"), None);
        assert_eq!(state.offer("0.1.0"), Some(&update("0.2.0")));
        // After installing it, the same file offers nothing.
        assert_eq!(state.offer("0.2.0"), None);
    }

    #[test]
    fn missing_or_broken_files_mean_never_checked() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update.json");
        assert_eq!(UpdateState::load(&path), UpdateState::default());
        fs::write(&path, "{ broken").unwrap();
        assert_eq!(UpdateState::load(&path), UpdateState::default());

        let state = UpdateState {
            checked_at: Some(at(9)),
            available: Some(AvailableUpdate {
                version: "0.2.0".into(),
                notes: Some("Faster".into()),
                date: Some("2026-10-30T10:00:00Z".into()),
            }),
            announced: None,
        };
        state.save(&path).unwrap();
        assert_eq!(UpdateState::load(&path), state);
    }
}
