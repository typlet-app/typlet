// SPDX-License-Identifier: GPL-3.0-or-later
//! Opening folders, web pages and the editor.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The online user guide.
pub(crate) const HELP_URL: &str =
    "https://github.com/typlet-app/typlet/blob/main/docs/user-guide.md";

#[cfg(windows)]
const EDITOR_FILE: &str = "typlet-editor.exe";
#[cfg(not(windows))]
const EDITOR_FILE: &str = "typlet-editor";

/// The editor program, if it is installed next to this program.
pub(crate) fn editor_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let path = exe.parent()?.join(EDITOR_FILE);
    path.is_file().then_some(path)
}

/// Starts the editor with `args`. Returns `false` if it is not installed or
/// could not be started.
pub(crate) fn launch_editor(args: &[&str]) -> bool {
    let Some(path) = editor_path() else {
        return false;
    };
    match Command::new(&path).args(args).spawn() {
        Ok(_) => true,
        Err(error) => {
            log::warn!("cannot start the editor {}: {error}", path.display());
            false
        }
    }
}

/// Opens a folder in the file manager.
pub(crate) fn open_folder(path: &Path) {
    let _ = std::fs::create_dir_all(path);
    open(path.as_os_str());
}

/// Opens a web page in the default browser.
pub(crate) fn open_url(url: &str) {
    open(std::ffi::OsStr::new(url));
}

fn open(target: &std::ffi::OsStr) {
    #[cfg(windows)]
    let result = Command::new("explorer.exe").arg(target).spawn();
    #[cfg(not(windows))]
    let result = Command::new("xdg-open").arg(target).spawn();
    if let Err(error) = result {
        log::warn!("cannot open {}: {error}", target.to_string_lossy());
    }
}
