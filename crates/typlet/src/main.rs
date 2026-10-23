// SPDX-License-Identifier: GPL-3.0-or-later
//! # Typlet
//!
//! The background program: listens for triggers, expands snippets and shows
//! the tray icon, quick search, forms, the set-up assistant and diagnostics.
//!
//! See `docs/architecture.md` for how the parts fit together.

// No console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod engine;
mod firstrun;
mod logger;
mod os;
mod report;
mod snapshot;
mod ui;
mod watcher;

use std::process::ExitCode;

use typlet_platform::autostart::AUTOSTART_FLAG;
use typlet_platform::instance;
use typlet_platform::paths::Paths;

const HELP: &str = "\
Typlet – a text expander for everyone

Usage: typlet [OPTION]

Options:
  --setup        open the set-up assistant
  --search       open the quick search
  --autostart    started automatically at login
  --version      print the version
  --help         print this help
";

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
enum Request {
    Run(app::StartOptions),
    Version,
    Help,
    Unknown(String),
}

fn parse(args: impl IntoIterator<Item = String>) -> Request {
    let mut options = app::StartOptions::default();
    for arg in args {
        match arg.as_str() {
            "--setup" => options.setup = true,
            "--search" => options.search = true,
            AUTOSTART_FLAG => {}
            "--version" | "-V" => return Request::Version,
            "--help" | "-h" => return Request::Help,
            _ => return Request::Unknown(arg),
        }
    }
    Request::Run(options)
}

#[allow(clippy::print_stdout, clippy::print_stderr)] // command-line output
fn main() -> ExitCode {
    let options = match parse(std::env::args().skip(1)) {
        Request::Run(options) => options,
        Request::Version => {
            println!("typlet {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Request::Help => {
            print!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Request::Unknown(arg) => {
            eprintln!("unknown option {arg}\n\n{HELP}");
            return ExitCode::from(2);
        }
    };

    let Some(paths) = Paths::resolve() else {
        eprintln!("typlet: cannot determine the home folder");
        return ExitCode::FAILURE;
    };
    logger::init(&paths.log_file);
    log::info!("Typlet {} starting", env!("CARGO_PKG_VERSION"));

    let _guard = match instance::acquire(&paths.lock_file) {
        Ok(Some(guard)) => Some(guard),
        Ok(None) => {
            log::info!("already running");
            // Starting Typlet again usually means "let me manage my snippets".
            if !os::launch_editor(&[]) && app::show_already_running().is_err() {
                return ExitCode::FAILURE;
            }
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            log::warn!("cannot create the lock file: {error}");
            None
        }
    };

    match app::run(paths, options) {
        Ok(()) => {
            log::info!("Typlet stopped");
            ExitCode::SUCCESS
        }
        Err(error) => {
            log::error!("Typlet stopped with an error: {error}");
            eprintln!("typlet: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Request {
        parse(list.iter().map(|s| (*s).to_owned()))
    }

    #[test]
    fn parses_command_line() {
        assert_eq!(args(&[]), Request::Run(app::StartOptions::default()));
        assert_eq!(
            args(&["--autostart", "--setup"]),
            Request::Run(app::StartOptions {
                setup: true,
                search: false
            })
        );
        assert_eq!(args(&["--version"]), Request::Version);
        assert_eq!(args(&["--bogus"]), Request::Unknown("--bogus".into()));
    }
}
