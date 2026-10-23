// SPDX-License-Identifier: GPL-3.0-or-later
//! Opening folders, web pages and the editor.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

/// The online user guide in the interface language.
pub(crate) fn help_url(german: bool) -> &'static str {
    if german {
        "https://github.com/typlet-app/typlet/blob/main/docs/user-guide.de.md"
    } else {
        "https://github.com/typlet-app/typlet/blob/main/docs/user-guide.md"
    }
}

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

/// Runs the editor with `args` and waits until it ends, at most `timeout`
/// (then it is stopped). Returns `true` if it ran and succeeded.
pub(crate) fn run_editor_and_wait(args: &[&str], timeout: Duration) -> bool {
    let Some(path) = editor_path() else {
        return false;
    };
    let mut child = match Command::new(&path).args(args).spawn() {
        Ok(child) => child,
        Err(error) => {
            log::warn!("cannot start the editor {}: {error}", path.display());
            return false;
        }
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if start.elapsed() < timeout => thread::sleep(Duration::from_millis(500)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
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
