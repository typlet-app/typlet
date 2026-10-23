// SPDX-License-Identifier: GPL-3.0-or-later
//! Checking for and installing new versions (ADR 0015).
//!
//! The editor is the only part of Typlet that connects to the network, and
//! only for this: it reads `latest.json` from the newest GitHub release and,
//! when the user agrees, downloads the installer. Updates are signed; the
//! updater refuses files that do not match the public key in
//! `tauri.conf.json`.
//!
//! The background program never connects to the network. Once a day it
//! starts `typlet-editor --check-updates`, which checks without a window,
//! records the result in `update.json` and exits.

use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime, State};
use tauri_plugin_updater::{Update, UpdaterExt};
use typlet_core::{AvailableUpdate, UpdateState};
use typlet_platform::paths::Paths;

use crate::commands::EditorState;
use crate::os::eprintln_safe;

const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// A headless check gives up after this long (slow or blocked network).
const HEADLESS_TIMEOUT: Duration = Duration::from_secs(120);

/// What the interface shows about updates.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct UpdateView {
    current: &'static str,
    available: Option<AvailableUpdate>,
    /// Last successful check, `YYYY-MM-DD HH:MM` local time.
    checked_at: Option<String>,
    /// Typlet can install the update itself (installed with a Typlet
    /// installer or AppImage); otherwise the interface links to the
    /// download page.
    can_install: bool,
}

/// Download progress, sent to the window as `update-progress`.
#[derive(Debug, Clone, Serialize)]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
}

/// `true` for installed builds; development builds cannot replace themselves.
fn can_install() -> bool {
    tauri::utils::platform::bundle_type().is_some()
}

fn view(state: &UpdateState, can_install: bool) -> UpdateView {
    UpdateView {
        current: CURRENT,
        available: state.offer(CURRENT).cloned(),
        checked_at: state
            .checked_at
            .map(|time| time.format("%Y-%m-%d %H:%M").to_string()),
        can_install,
    }
}

/// Records the result of a successful check in `update.json`.
fn record(paths: &Paths, update: Option<&Update>) {
    let mut state = UpdateState::load(&paths.update_state);
    state.checked_at = Some(chrono::Local::now().naive_local());
    state.available = update.map(|update| AvailableUpdate {
        version: update.version.clone(),
        notes: update.body.clone().filter(|body| !body.trim().is_empty()),
        date: update
            .raw_json
            .get("pub_date")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    });
    if let Err(error) = state.save(&paths.update_state) {
        eprintln_safe(&format!("cannot save the update state: {error}"));
    }
}

/// Looks for a newer version and records the result.
async fn check<R: Runtime>(app: &AppHandle<R>, paths: &Paths) -> Result<Option<Update>, String> {
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    record(paths, update.as_ref());
    Ok(update)
}

/// The last known state, without connecting to the network.
#[tauri::command]
pub(crate) fn update_status(state: State<'_, EditorState>) -> UpdateView {
    view(&UpdateState::load(&state.paths.update_state), can_install())
}

/// Checks now.
#[tauri::command]
pub(crate) async fn check_for_update(
    app: AppHandle,
    state: State<'_, EditorState>,
) -> Result<UpdateView, String> {
    check(&app, &state.paths).await?;
    Ok(update_status(state))
}

/// Downloads and installs the newest version, then restarts Typlet. On
/// Windows the installer takes over and this process ends during the call.
#[tauri::command]
pub(crate) async fn install_update(
    app: AppHandle,
    state: State<'_, EditorState>,
) -> Result<(), String> {
    if !can_install() {
        return Err("this copy of Typlet cannot update itself".into());
    }
    let Some(update) = check(&app, &state.paths).await? else {
        return Err("Typlet is already up to date".into());
    };
    let mut downloaded: u64 = 0;
    let progress = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += u64::try_from(chunk).unwrap_or(0);
                let _ = progress.emit("update-progress", Progress { downloaded, total });
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;

    // Linux: the new files are in place. The running background program is
    // the old version; ask it to quit, then restart the editor, which starts
    // the new background program.
    stop_background(&state.paths);
    app.restart()
}

/// Asks the background program to quit and waits up to five seconds.
fn stop_background(paths: &Paths) {
    if !typlet_platform::instance::is_running(&paths.lock_file) {
        return;
    }
    if let Err(error) = std::fs::write(&paths.quit_request, b"update\n") {
        eprintln_safe(&format!(
            "cannot ask the background program to quit: {error}"
        ));
        return;
    }
    let start = Instant::now();
    while typlet_platform::instance::is_running(&paths.lock_file)
        && start.elapsed() < Duration::from_secs(5)
    {
        thread::sleep(Duration::from_millis(100));
    }
}

/// `typlet-editor --check-updates`: checks without opening a window,
/// records the result and exits.
pub(crate) fn run_headless_check(paths: Paths, context: tauri::Context) -> ExitCode {
    let app = match tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
    {
        Ok(app) => app,
        Err(error) => {
            eprintln_safe(&format!("cannot check for updates: {error}"));
            return ExitCode::FAILURE;
        }
    };
    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        let code = match check(&handle, &paths).await {
            Ok(_) => 0,
            Err(error) => {
                eprintln_safe(&format!("update check failed: {error}"));
                1
            }
        };
        handle.exit(code);
    });
    // A blocked network must not leave an invisible process behind.
    let watchdog = app.handle().clone();
    thread::spawn(move || {
        thread::sleep(HEADLESS_TIMEOUT);
        watchdog.exit(1);
    });
    app.run(|_, _| {});
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_only_newer_versions() {
        let mut state = UpdateState::default();
        assert_eq!(view(&state, true).available, None);

        state.available = Some(AvailableUpdate {
            version: "999.0.0".into(),
            notes: None,
            date: None,
        });
        state.checked_at =
            chrono::NaiveDate::from_ymd_opt(2026, 10, 23).and_then(|d| d.and_hms_opt(9, 5, 0));
        let shown = view(&state, false);
        assert_eq!(
            shown.available.map(|a| a.version),
            Some("999.0.0".to_owned())
        );
        assert_eq!(shown.checked_at.as_deref(), Some("2026-10-23 09:05"));
        assert!(!shown.can_install);

        // The version that is running is not offered again.
        state.available = Some(AvailableUpdate {
            version: CURRENT.into(),
            notes: None,
            date: None,
        });
        assert_eq!(view(&state, true).available, None);
    }
}
