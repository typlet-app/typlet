// SPDX-License-Identifier: GPL-3.0-or-later
//! Opening folders and web pages, and starting the background program.

use std::path::Path;
use std::process::Command;

use typlet_platform::paths::Paths;

#[cfg(windows)]
const BACKGROUND_FILE: &str = "typlet.exe";
#[cfg(not(windows))]
const BACKGROUND_FILE: &str = "typlet";

/// Opens a folder in the file manager.
pub(crate) fn open_folder(path: &Path) {
    let _ = std::fs::create_dir_all(path);
    #[cfg(windows)]
    let result = Command::new("explorer.exe").arg(path).spawn();
    #[cfg(not(windows))]
    let result = Command::new("xdg-open").arg(path).spawn();
    if let Err(error) = result {
        eprintln_safe(&format!("cannot open {}: {error}", path.display()));
    }
}

/// Opens a web page in the default browser.
pub(crate) fn open_url(url: &str) {
    #[cfg(windows)]
    let result = Command::new("explorer.exe").arg(url).spawn();
    #[cfg(not(windows))]
    let result = Command::new("xdg-open").arg(url).spawn();
    if let Err(error) = result {
        eprintln_safe(&format!("cannot open {url}: {error}"));
    }
}

/// Starts the background program next to this executable unless it is
/// already running. Returns `true` if it is running afterwards (or was).
pub(crate) fn ensure_background(paths: &Paths) -> bool {
    if typlet_platform::instance::is_running(&paths.lock_file) {
        return true;
    }
    let Some(exe) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(BACKGROUND_FILE)))
        .filter(|path| path.is_file())
    else {
        return false;
    };
    match Command::new(&exe).spawn() {
        Ok(_) => true,
        Err(error) => {
            eprintln_safe(&format!("cannot start {}: {error}", exe.display()));
            false
        }
    }
}

/// The editor has no log file; problems go to standard error in debug
/// builds, where a console exists.
#[allow(clippy::print_stderr)]
pub(crate) fn eprintln_safe(message: &str) {
    if cfg!(debug_assertions) {
        eprintln!("typlet-editor: {message}");
    }
}
