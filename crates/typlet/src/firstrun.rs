// SPDX-License-Identifier: GPL-3.0-or-later
//! Example snippets created on the first start, so the set-up assistant can
//! show Typlet working right away.

use std::fs;
use std::io;
use std::path::Path;

use typlet_core::{
    Block, Document, FieldDef, FieldKind, Inline, Library, LibraryError, Marks, Snippet,
};

/// The example the set-up assistant asks the user to type.
pub(crate) struct TestSnippet {
    /// What to type.
    pub(crate) trigger: &'static str,
    /// The text that appears.
    pub(crate) expected: &'static str,
}

/// The test example for a language.
pub(crate) fn test_snippet(german: bool) -> TestSnippet {
    if german {
        TestSnippet {
            trigger: ":mfg",
            expected: "Mit freundlichen Grüßen",
        }
    } else {
        TestSnippet {
            trigger: ":br",
            expected: "Best regards",
        }
    }
}

/// Returns `true` if the library folder has no snippet files yet.
pub(crate) fn library_is_empty(root: &Path) -> bool {
    match fs::read_dir(root) {
        Ok(mut entries) => !entries.any(|entry| entry.is_ok()),
        Err(error) => error.kind() == io::ErrorKind::NotFound,
    }
}

/// Writes the example snippets into an empty library.
pub(crate) fn create_examples(root: &Path, german: bool) -> Result<usize, LibraryError> {
    let library = Library::new(root);
    let examples = if german {
        german_examples()
    } else {
        english_examples()
    };
    for snippet in &examples {
        library.save(snippet, &[])?;
    }
    Ok(examples.len())
}

fn text(text: &str) -> Inline {
    Inline::text(text)
}

fn bold(text: &str) -> Inline {
    Inline::styled(
        text,
        Marks {
            bold: true,
            ..Marks::default()
        },
    )
}

fn paragraph(content: Vec<Inline>) -> Block {
    Block::Paragraph { content }
}

fn field(name: &str) -> Inline {
    Inline::Field { name: name.into() }
}

fn date(format: &str) -> Inline {
    Inline::DateTime {
        format: format.into(),
        offset_days: 0,
    }
}

fn snippet(trigger: &str, label: &str, blocks: Vec<Block>) -> Snippet {
    Snippet::new(trigger, label, Document { blocks })
}

fn german_examples() -> Vec<Snippet> {
    let test = test_snippet(true);
    let mut callback = snippet(
        ":rr",
        "Rückruf-Notiz (Formular)",
        vec![
            paragraph(vec![
                bold("Rückruf"),
                text(" vom "),
                date("%d.%m.%Y, %H:%M"),
            ]),
            Block::BulletList {
                items: vec![
                    vec![text("Anrufer: "), field("anrufer")],
                    vec![text("Anliegen: "), field("anliegen")],
                    vec![text("Rückruf erwünscht: "), field("rueckruf")],
                ],
            },
        ],
    );
    callback.fields = vec![
        FieldDef {
            name: "anrufer".into(),
            label: "Wer hat angerufen?".into(),
            kind: FieldKind::Text,
            default: None,
        },
        FieldDef {
            name: "anliegen".into(),
            label: "Anliegen".into(),
            kind: FieldKind::Choice {
                options: vec!["Angebot".into(), "Reklamation".into(), "Termin".into()],
            },
            default: Some("Angebot".into()),
        },
        FieldDef {
            name: "rueckruf".into(),
            label: "Rückruf erwünscht?".into(),
            kind: FieldKind::YesNo,
            default: Some("Ja".into()),
        },
    ];
    vec![
        snippet(
            test.trigger,
            "Grußformel",
            vec![paragraph(vec![text(test.expected)])],
        ),
        snippet(
            ":datum",
            "Heutiges Datum",
            vec![paragraph(vec![date("%d.%m.%Y")])],
        ),
        snippet(
            ":adr",
            "Meine Adresse (bitte anpassen)",
            vec![
                paragraph(vec![text("Max Mustermann")]),
                paragraph(vec![text("Musterstraße 1")]),
                paragraph(vec![text("12345 Musterstadt")]),
            ],
        ),
        snippet(
            ":sig",
            "E-Mail-Signatur (formatiert)",
            vec![
                paragraph(vec![text("Viele Grüße")]),
                paragraph(vec![
                    bold("Max Mustermann"),
                    Inline::LineBreak,
                    text("Musterfirma GmbH"),
                ]),
            ],
        ),
        callback,
    ]
}

fn english_examples() -> Vec<Snippet> {
    let test = test_snippet(false);
    let mut callback = snippet(
        ":call",
        "Call note (form)",
        vec![
            paragraph(vec![bold("Call"), text(" on "), date("%B %-d, %Y, %H:%M")]),
            Block::BulletList {
                items: vec![
                    vec![text("Caller: "), field("caller")],
                    vec![text("Topic: "), field("topic")],
                    vec![text("Call back: "), field("callback")],
                ],
            },
        ],
    );
    callback.fields = vec![
        FieldDef {
            name: "caller".into(),
            label: "Who called?".into(),
            kind: FieldKind::Text,
            default: None,
        },
        FieldDef {
            name: "topic".into(),
            label: "Topic".into(),
            kind: FieldKind::Choice {
                options: vec!["Quote".into(), "Complaint".into(), "Appointment".into()],
            },
            default: Some("Quote".into()),
        },
        FieldDef {
            name: "callback".into(),
            label: "Call back?".into(),
            kind: FieldKind::YesNo,
            default: Some("Yes".into()),
        },
    ];
    vec![
        snippet(
            test.trigger,
            "Closing",
            vec![paragraph(vec![text(test.expected)])],
        ),
        snippet(
            ":date",
            "Today's date",
            vec![paragraph(vec![date("%B %-d, %Y")])],
        ),
        snippet(
            ":addr",
            "My address (please edit)",
            vec![
                paragraph(vec![text("Jane Doe")]),
                paragraph(vec![text("1 Example Street")]),
                paragraph(vec![text("Springfield")]),
            ],
        ),
        snippet(
            ":sig",
            "Email signature (formatted)",
            vec![
                paragraph(vec![text("Kind regards")]),
                paragraph(vec![
                    bold("Jane Doe"),
                    Inline::LineBreak,
                    text("Example Ltd"),
                ]),
            ],
        ),
        callback,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn examples_are_valid_and_conflict_free() {
        for german in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            assert!(library_is_empty(dir.path()));
            assert_eq!(create_examples(dir.path(), german).unwrap(), 5);
            assert!(!library_is_empty(dir.path()));

            let report = Library::new(dir.path()).load();
            assert!(report.errors.is_empty());
            let snippets: Vec<Snippet> = report.snippets.into_iter().map(|s| s.snippet).collect();
            assert!(typlet_core::find_conflicts(&snippets).is_empty());
            let test = test_snippet(german);
            assert!(snippets.iter().any(|s| s.trigger == test.trigger));
        }
    }

    #[test]
    fn missing_folder_counts_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(library_is_empty(&dir.path().join("missing")));
    }
}
