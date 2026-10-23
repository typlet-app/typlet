// SPDX-License-Identifier: GPL-3.0-or-later
//! User settings and per-application rules.
//!
//! Settings live in a single JSON file next to the library. Unknown fields are
//! ignored and missing fields take their defaults, so older and newer versions
//! of Typlet can share a settings file.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::fsutil::write_atomic;
use crate::hotkey::Hotkey;
use crate::snippet::InsertMode;

/// Current version of the settings format.
pub const SETTINGS_VERSION: u32 = 1;

/// Default time the clipboard is left in place after pasting, before the
/// previous content is restored.
pub const DEFAULT_RESTORE_DELAY_MS: u32 = 300;

/// Language of the user interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    /// Follow the operating system.
    #[default]
    System,
    /// German.
    De,
    /// English.
    En,
}

/// How a line break is typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NewlineMode {
    /// Press Enter.
    #[default]
    Enter,
    /// Press Shift+Enter (chat apps, where Enter sends the message).
    ShiftEnter,
}

/// A user-defined rule for one application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRule {
    /// Executable or application id, compared case-insensitively
    /// (for example `excel.exe`).
    pub app: String,
    /// `false` disables Typlet in this application.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Overrides the insertion method.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insert_mode: Option<InsertMode>,
    /// Overrides how line breaks are typed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub newline: Option<NewlineMode>,
    /// Paste only plain text, without HTML or RTF.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plain_only: Option<bool>,
    /// Milliseconds before the clipboard is restored after pasting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_delay_ms: Option<u32>,
}

fn default_true() -> bool {
    true
}

/// The rule that applies to an application after combining built-in
/// defaults and user rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveRule {
    /// Whether Typlet expands snippets in this application.
    pub enabled: bool,
    /// Insertion method override, if any.
    pub insert_mode: Option<InsertMode>,
    /// How line breaks are typed.
    pub newline: NewlineMode,
    /// Paste only plain text.
    pub plain_only: bool,
    /// Milliseconds before the clipboard is restored.
    pub restore_delay_ms: u32,
}

impl Default for EffectiveRule {
    fn default() -> Self {
        Self {
            enabled: true,
            insert_mode: None,
            newline: NewlineMode::Enter,
            plain_only: false,
            restore_delay_ms: DEFAULT_RESTORE_DELAY_MS,
        }
    }
}

/// All user settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)] // independent switches
pub struct Settings {
    /// Settings format version.
    pub version: u32,
    /// Interface language.
    pub language: Language,
    /// When `true`, no snippets are expanded.
    pub paused: bool,
    /// Start Typlet when the user logs in.
    pub start_with_system: bool,
    /// The setup assistant has been completed.
    pub first_run_complete: bool,
    /// Show desktop notifications for errors.
    pub notifications: bool,
    /// Shortcut for the quick search. `None` disables it.
    pub search_hotkey: Option<Hotkey>,
    /// Shortcut for "create snippet from selection". `None` disables it.
    pub selection_hotkey: Option<Hotkey>,
    /// Custom library folder. `None` uses the default location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_path: Option<PathBuf>,
    /// Per-application rules, applied after the built-in ones.
    pub app_rules: Vec<AppRule>,
    /// Look for a new version once a day (the editor does this; the
    /// background program never connects to the network).
    pub check_for_updates: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            language: Language::System,
            paused: false,
            start_with_system: true,
            first_run_complete: false,
            notifications: true,
            search_hotkey: Some(Hotkey::default_search()),
            selection_hotkey: Some(Hotkey::default_selection()),
            library_path: None,
            app_rules: Vec::new(),
            check_for_updates: true,
        }
    }
}

/// Why settings could not be loaded or saved.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// Reading or writing failed.
    #[error("{path}: {source}")]
    Io {
        /// The settings file.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// The file is not valid settings JSON.
    #[error("{path}: invalid settings file: {source}")]
    Parse {
        /// The settings file.
        path: PathBuf,
        /// The underlying error.
        source: serde_json::Error,
    },
}

impl Settings {
    /// Loads settings. A missing file yields the defaults.
    ///
    /// # Errors
    ///
    /// Fails if the file exists but cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self, SettingsError> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(source) => {
                return Err(SettingsError::Io {
                    path: path.to_path_buf(),
                    source,
                });
            }
        };
        serde_json::from_str(&text).map_err(|source| SettingsError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Saves settings atomically.
    ///
    /// # Errors
    ///
    /// Fails if the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), SettingsError> {
        let mut json =
            serde_json::to_string_pretty(self).map_err(|source| SettingsError::Parse {
                path: path.to_path_buf(),
                source,
            })?;
        json.push('\n');
        write_atomic(path, json.as_bytes()).map_err(|source| SettingsError::Io {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Returns the rule for an application, combining built-in defaults with
    /// the user's rules (user rules win). `None` means the application is
    /// unknown (for example on GNOME, where it cannot be detected).
    #[must_use]
    pub fn rule_for(&self, app: Option<&str>) -> EffectiveRule {
        let mut rule = EffectiveRule::default();
        let Some(app) = app.map(normalise_app) else {
            return rule;
        };
        for builtin in builtin_rules() {
            if builtin.app == app {
                apply(&mut rule, &builtin);
            }
        }
        for user in &self.app_rules {
            if normalise_app(&user.app) == app {
                apply(&mut rule, user);
            }
        }
        rule
    }
}

fn apply(rule: &mut EffectiveRule, overrides: &AppRule) {
    rule.enabled = overrides.enabled;
    if overrides.insert_mode.is_some() {
        rule.insert_mode = overrides.insert_mode;
    }
    if let Some(newline) = overrides.newline {
        rule.newline = newline;
    }
    if let Some(plain_only) = overrides.plain_only {
        rule.plain_only = plain_only;
    }
    if let Some(delay) = overrides.restore_delay_ms {
        rule.restore_delay_ms = delay;
    }
}

/// Lower-cases an app id and strips any directory part.
#[must_use]
pub fn normalise_app(app: &str) -> String {
    app.rsplit(['/', '\\'])
        .next()
        .unwrap_or(app)
        .trim()
        .to_lowercase()
}

/// Rules that make common applications work out of the box.
#[must_use]
pub fn builtin_rules() -> Vec<AppRule> {
    let rule = |app: &str| AppRule {
        app: app.to_owned(),
        enabled: true,
        insert_mode: None,
        newline: None,
        plain_only: None,
        restore_delay_ms: None,
    };
    let mut rules = Vec::new();

    // Excel turns pasted HTML into table cells.
    rules.push(AppRule {
        plain_only: Some(true),
        ..rule("excel.exe")
    });

    // In chat apps Enter sends the message.
    for app in [
        "ms-teams.exe",
        "teams.exe",
        "whatsapp.exe",
        "whatsapp.root.exe",
        "slack.exe",
        "discord.exe",
        "signal.exe",
        "telegram.exe",
    ] {
        rules.push(AppRule {
            newline: Some(NewlineMode::ShiftEnter),
            ..rule(app)
        });
    }

    // Browsers and remote desktops read the clipboard late.
    for app in [
        "chrome.exe",
        "msedge.exe",
        "firefox.exe",
        "brave.exe",
        "opera.exe",
    ] {
        rules.push(AppRule {
            restore_delay_ms: Some(600),
            ..rule(app)
        });
    }
    for app in ["mstsc.exe", "msrdc.exe", "wfica32.exe", "cdviewer.exe"] {
        rules.push(AppRule {
            restore_delay_ms: Some(1200),
            ..rule(app)
        });
    }
    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::load(&dir.path().join("settings.json")).unwrap();
        assert_eq!(settings, Settings::default());
        assert_eq!(settings.search_hotkey, Some(Hotkey::default_search()));
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let settings = Settings {
            paused: true,
            language: Language::De,
            app_rules: vec![AppRule {
                app: "Notepad.exe".into(),
                enabled: false,
                insert_mode: None,
                newline: None,
                plain_only: None,
                restore_delay_ms: None,
            }],
            ..Settings::default()
        };
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), settings);
    }

    #[test]
    fn partial_files_fill_in_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, r#"{"paused": true, "unknown_future_field": 1}"#).unwrap();
        let settings = Settings::load(&path).unwrap();
        assert!(settings.paused);
        assert!(settings.start_with_system);
    }

    #[test]
    fn broken_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, "{ nope").unwrap();
        assert!(matches!(
            Settings::load(&path),
            Err(SettingsError::Parse { .. })
        ));
    }

    #[test]
    fn builtin_and_user_rules_combine() {
        let mut settings = Settings::default();
        assert!(
            settings
                .rule_for(Some(r"C:\Program Files\Microsoft Office\EXCEL.EXE"))
                .plain_only
        );
        assert_eq!(
            settings.rule_for(Some("ms-teams.exe")).newline,
            NewlineMode::ShiftEnter
        );
        assert_eq!(settings.rule_for(None), EffectiveRule::default());

        settings.app_rules.push(AppRule {
            app: "excel.exe".into(),
            enabled: false,
            insert_mode: None,
            newline: None,
            plain_only: Some(false),
            restore_delay_ms: None,
        });
        let excel = settings.rule_for(Some("excel.exe"));
        assert!(!excel.enabled);
        assert!(!excel.plain_only);
    }
}
