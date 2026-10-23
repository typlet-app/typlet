// SPDX-License-Identifier: GPL-3.0-or-later
//! Where Typlet keeps its files.
//!
//! | | Windows | Linux |
//! |---|---|---|
//! | Settings | `%APPDATA%\Typlet\config\settings.json` | `~/.config/typlet/settings.json` |
//! | Library | `%APPDATA%\Typlet\data\snippets` | `~/.local/share/typlet/snippets` |
//! | Log | `%APPDATA%\Typlet\data\typlet.log` | `~/.local/share/typlet/typlet.log` |
//!
//! The library location can be changed in the settings.

use std::path::PathBuf;

use directories::ProjectDirs;
use typlet_core::Settings;

/// Resolved file locations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    /// Folder for configuration.
    pub config_dir: PathBuf,
    /// Folder for data (library, log, lock file).
    pub data_dir: PathBuf,
    /// The settings file.
    pub settings_file: PathBuf,
    /// The default library folder.
    pub default_library: PathBuf,
    /// The log file. It never contains typed text.
    pub log_file: PathBuf,
    /// Lock file that keeps a second background program from starting.
    pub lock_file: PathBuf,
    /// Held by the editor while its window has the keyboard focus, so the
    /// background program does not expand triggers typed into the editor.
    pub editor_focus_lock: PathBuf,
    /// What the editor found out about new versions (`update.json`).
    pub update_state: PathBuf,
    /// Usage statistics (`stats.json`): counts only, never text.
    pub stats: PathBuf,
    /// Created by the editor after installing an update on Linux: the
    /// background program quits so the new version can start. Lives next to
    /// the settings, whose folder the background program watches.
    pub quit_request: PathBuf,
}

impl Paths {
    /// Resolves the locations for the current user.
    ///
    /// Returns `None` if the home folder cannot be determined.
    #[must_use]
    pub fn resolve() -> Option<Self> {
        let dirs = ProjectDirs::from("", "", "Typlet")?;
        let config_dir = dirs.config_dir().to_path_buf();
        let data_dir = dirs.data_dir().to_path_buf();
        Some(Self {
            settings_file: config_dir.join("settings.json"),
            default_library: data_dir.join("snippets"),
            log_file: data_dir.join("typlet.log"),
            lock_file: data_dir.join("typlet.lock"),
            editor_focus_lock: data_dir.join("editor-focus.lock"),
            update_state: data_dir.join("update.json"),
            stats: data_dir.join("stats.json"),
            quit_request: config_dir.join("quit-request"),
            config_dir,
            data_dir,
        })
    }

    /// The library folder to use with these settings.
    #[must_use]
    pub fn library(&self, settings: &Settings) -> PathBuf {
        settings
            .library_path
            .clone()
            .unwrap_or_else(|| self.default_library.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_library_overrides_default() {
        let paths = Paths::resolve().expect("home folder");
        let mut settings = Settings::default();
        assert_eq!(paths.library(&settings), paths.default_library);
        settings.library_path = Some(PathBuf::from("/tmp/mine"));
        assert_eq!(paths.library(&settings), PathBuf::from("/tmp/mine"));
    }
}
