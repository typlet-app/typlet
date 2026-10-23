// SPDX-License-Identifier: GPL-3.0-or-later
//! Keeps the documentation honest: every example in `docs/examples` must load,
//! validate and render. If the file format changes, these docs must change too.

#![allow(clippy::unwrap_used)] // test helpers may panic on fixture errors

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::NaiveDate;
use typlet_core::{InsertMethod, Library, RenderContext, render};

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/examples")
}

fn context(fields: &HashMap<String, String>) -> RenderContext<'_> {
    RenderContext {
        now: NaiveDate::from_ymd_opt(2026, 10, 23)
            .unwrap()
            .and_hms_opt(9, 42, 0)
            .unwrap(),
        clipboard: Some("Zwischenablage"),
        fields,
    }
}

#[test]
fn all_documented_examples_load_without_errors() {
    let report = Library::new(examples_dir()).load();
    assert!(
        report.errors.is_empty(),
        "broken examples: {:?}",
        report.errors
    );
    assert_eq!(report.snippets.len(), 3);

    let email = report
        .snippets
        .iter()
        .find(|s| s.snippet.trigger == ":mfg")
        .unwrap();
    assert_eq!(email.category, vec!["email".to_owned()]);
}

#[test]
fn signature_with_date_renders_as_typed_text() {
    let report = Library::new(examples_dir()).load();
    let sig = &report
        .snippets
        .iter()
        .find(|s| s.snippet.trigger == ":sig")
        .unwrap()
        .snippet;

    let out = render(sig, &context(&HashMap::new())).unwrap();
    assert_eq!(out.plain, "Max Mustermann, 23.10.2026");
    assert_eq!(out.insert_method(sig.insert_mode), InsertMethod::Type);
}

#[test]
fn formatted_signature_is_pasted_with_all_flavours() {
    let report = Library::new(examples_dir()).load();
    let mfg = &report
        .snippets
        .iter()
        .find(|s| s.snippet.trigger == ":mfg")
        .unwrap()
        .snippet;

    let out = render(mfg, &context(&HashMap::new())).unwrap();
    assert!(out.formatted);
    assert_eq!(out.insert_method(mfg.insert_mode), InsertMethod::Paste);
    assert_eq!(mfg.hotkey, Some("Ctrl+Alt+M".parse().unwrap()));
    assert_eq!(
        out.plain,
        "Mit freundlichen Grüßen\n\nMax Mustermann\nMusterbetrieb GmbH\nwww.example.org"
    );
    assert!(out.html.contains("<strong>Max Mustermann</strong>"));
    assert!(
        out.html
            .contains(r#"<a href="https://www.example.org">www.example.org</a>"#)
    );
    assert!(out.rtf.contains(r"{\b Max Mustermann}"));
}

#[test]
fn callback_form_uses_entered_values_and_defaults() {
    let report = Library::new(examples_dir()).load();
    let rr = &report
        .snippets
        .iter()
        .find(|s| s.snippet.trigger == ":rr")
        .unwrap()
        .snippet;

    let mut values = HashMap::new();
    values.insert("caller".to_owned(), "Frau Beispiel".to_owned());
    values.insert("reason".to_owned(), "Termin".to_owned());

    let out = render(rr, &context(&values)).unwrap();
    assert_eq!(
        out.plain,
        "Rückruf vom 23.10.2026, 09:42\n• Anrufer: Frau Beispiel\n• Anliegen: Termin\n• Rückruf erwünscht: Ja\nNotiz: "
    );
    assert_eq!(out.cursor_from_end, Some(0));
}
