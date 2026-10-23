// SPDX-License-Identifier: GPL-3.0-or-later
//! # Typlet editor
//!
//! A Tauri window with a React + Tiptap interface (`web/`) for creating and
//! editing snippets, changing settings and importing from espanso. It works
//! only on files: the background program notices every change and reloads.
//!
//! See `docs/architecture.md` and ADR 0003.

// No console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod os;
mod updates;

use std::process::ExitCode;
use std::sync::Mutex;

use tauri::{Emitter, Manager, WebviewWindowBuilder, WindowEvent};
use typlet_platform::paths::Paths;

use commands::{EditorState, StartArgs};

#[allow(clippy::print_stderr)] // the only way to report a failed start
fn main() -> ExitCode {
    let Some(paths) = Paths::resolve() else {
        eprintln!("typlet-editor: cannot determine the home folder");
        return ExitCode::FAILURE;
    };
    let start = StartArgs::parse(std::env::args().skip(1));
    if start.background_only {
        // Started at login from an AppImage, whose only entry point is the
        // editor: start the background program and stay invisible.
        os::ensure_background(&paths);
        return ExitCode::SUCCESS;
    }
    let context = tauri::generate_context!();
    if start.check_updates {
        return updates::run_headless_check(paths, context);
    }

    let result = tauri::Builder::default()
        // A second start focuses the open window and passes its arguments,
        // for example `--open <id>` from the background program.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let request = StartArgs::parse(args.into_iter().skip(1));
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
            if let Some(id) = request.open_snippet {
                let _ = app.emit("open-snippet", id);
            } else if request.open_settings {
                let _ = app.emit("open-settings", ());
            } else if request.open_update {
                let _ = app.emit("open-update", ());
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup({
            let paths = paths.clone();
            move |app| {
                // The window is created here, not from the configuration,
                // so `--check-updates` can run without one.
                if let Some(config) = app.config().app.windows.first() {
                    WebviewWindowBuilder::from_config(app.handle(), config)?.build()?;
                }
                // The editor is useless without the background program.
                os::ensure_background(&paths);
                Ok(())
            }
        })
        // While the editor has the focus, the background program does not
        // expand triggers (they are being typed into the editor).
        .on_window_event({
            let lock = paths.editor_focus_lock.clone();
            let held = Mutex::new(None);
            move |_window, event| {
                if let WindowEvent::Focused(focused) = event
                    && let Ok(mut held) = held.lock()
                {
                    *held = if *focused {
                        typlet_platform::instance::acquire(&lock).ok().flatten()
                    } else {
                        None
                    };
                }
            }
        })
        .manage(EditorState { paths, start })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::load_library,
            commands::save_snippet,
            commands::delete_snippet,
            commands::validate_snippet,
            commands::preview,
            commands::load_settings,
            commands::save_settings,
            commands::parse_hotkey,
            commands::import_espanso,
            commands::open_folder,
            commands::start_background,
            commands::open_link,
            updates::update_status,
            updates::check_for_update,
            updates::install_update,
        ])
        .run(context);

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("typlet-editor: {error}");
            ExitCode::FAILURE
        }
    }
}
