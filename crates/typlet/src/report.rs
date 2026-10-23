// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostics report users can attach to bug reports.
//!
//! The report describes the system and what happened, never what was typed
//! or inserted.

use std::fmt::Write;

use typlet_platform::Status;

use crate::engine::{Attempt, AttemptResult};
use crate::snapshot::Snapshot;

/// Everything the report is built from.
pub(crate) struct ReportInput<'a> {
    pub(crate) status: &'a Status,
    pub(crate) engine_error: Option<&'a str>,
    pub(crate) paused: bool,
    pub(crate) library: &'a Snapshot,
    pub(crate) history: &'a [Attempt],
}

/// A short English description of an attempt, for the report.
pub(crate) fn describe(result: &AttemptResult) -> String {
    match result {
        AttemptResult::Typed => "typed".into(),
        AttemptResult::Pasted => "pasted as plain text".into(),
        AttemptResult::PastedFormatted => "pasted with formatting".into(),
        AttemptResult::FormOpened => "form opened".into(),
        AttemptResult::Undone => "undone with Backspace".into(),
        AttemptResult::SkippedPassword => "skipped: password field".into(),
        AttemptResult::SkippedApp => "skipped: turned off for this app".into(),
        AttemptResult::Failed(error) => format!("failed: {error}"),
    }
}

/// Builds the plain-text report.
pub(crate) fn build(input: &ReportInput<'_>) -> String {
    let mut out = String::new();
    let yes_no = |value: bool| if value { "yes" } else { "no" };
    let _ = writeln!(
        out,
        "Typlet {} on {} {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let _ = writeln!(
        out,
        "keyboard input: {} | inserting: {} | app detection: {} | paused: {}",
        yes_no(input.status.keyboard_access),
        yes_no(input.status.injection_access),
        yes_no(input.status.focus_detection),
        yes_no(input.paused)
    );
    if let Some(error) = input.engine_error {
        let _ = writeln!(out, "engine: {error}");
    }
    for problem in &input.status.problems {
        let _ = writeln!(out, "problem: {problem}");
    }
    let _ = writeln!(
        out,
        "snippets: {} enabled of {}, {} broken files",
        input.library.enabled_count(),
        input.library.snippets.len(),
        input.library.broken_files
    );
    for issue in &input.library.issues {
        let _ = writeln!(out, "library: {issue}");
    }
    if input.history.is_empty() {
        let _ = writeln!(out, "recent: none");
    }
    for attempt in input.history {
        let _ = writeln!(
            out,
            "recent: {} {} {}",
            attempt.at.format("%H:%M:%S"),
            attempt.app.as_deref().unwrap_or("(unknown app)"),
            describe(&attempt.result)
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_lists_status_library_and_history() {
        let status = Status {
            keyboard_access: true,
            injection_access: false,
            focus_detection: true,
            problems: vec!["no uinput".into()],
        };
        let library =
            Snapshot::from_parts("lib".into(), Vec::new(), vec!["broken.json: bad".into()], 1);
        let history = [Attempt {
            at: chrono::NaiveDate::from_ymd_opt(2026, 10, 23)
                .and_then(|d| d.and_hms_opt(8, 5, 0))
                .unwrap(),
            app: Some("chrome.exe".into()),
            result: AttemptResult::Failed("blocked".into()),
        }];
        let report = build(&ReportInput {
            status: &status,
            engine_error: None,
            paused: false,
            library: &library,
            history: &history,
        });
        assert!(report.contains("keyboard input: yes | inserting: no"));
        assert!(report.contains("problem: no uinput"));
        assert!(report.contains("snippets: 0 enabled of 0, 1 broken files"));
        assert!(report.contains("library: broken.json: bad"));
        assert!(report.contains("recent: 08:05:00 chrome.exe failed: blocked"));
    }
}
