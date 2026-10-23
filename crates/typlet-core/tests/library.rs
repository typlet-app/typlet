// SPDX-License-Identifier: GPL-3.0-or-later
//! Library storage: round trips, categories and resilience against broken files.

#![allow(clippy::unwrap_used)] // test helpers may panic on fixture errors

use std::fs;

use typlet_core::document::{Block, Document, Inline};
use typlet_core::{Library, LibraryError, Matcher, Snippet};

fn sample(trigger: &str, label: &str) -> Snippet {
    Snippet::new(
        trigger,
        label,
        Document {
            blocks: vec![Block::Paragraph {
                content: vec![Inline::text(label)],
            }],
        },
    )
}

#[test]
fn saved_snippets_load_back_identically() {
    let dir = tempfile::tempdir().unwrap();
    let library = Library::new(dir.path().join("snippets"));

    let a = sample(":a", "Alpha");
    let b = sample(":b", "Beta");
    library.save(&a, &[]).unwrap();
    library.save(&b, &["Arbeit".to_owned()]).unwrap();

    let report = library.load();
    assert!(report.errors.is_empty());
    assert_eq!(report.snippets.len(), 2);
    assert_eq!(report.snippets[0].snippet, a);
    assert!(report.snippets[0].category.is_empty());
    assert_eq!(report.snippets[1].snippet, b);
    assert_eq!(report.snippets[1].category, vec!["Arbeit".to_owned()]);
}

#[test]
fn saving_again_overwrites_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let library = Library::new(dir.path());

    let mut snippet = sample(":a", "Alpha");
    let path = library.save(&snippet, &[]).unwrap();
    snippet.label = "Alpha 2".into();
    assert_eq!(library.save(&snippet, &[]).unwrap(), path);

    let report = library.load();
    assert_eq!(report.snippets.len(), 1);
    assert_eq!(report.snippets[0].snippet.label, "Alpha 2");
}

#[test]
fn invalid_snippets_are_not_saved() {
    let dir = tempfile::tempdir().unwrap();
    let library = Library::new(dir.path());
    let broken = sample("", "No trigger");
    assert!(matches!(
        library.save(&broken, &[]),
        Err(LibraryError::Invalid { .. })
    ));
    assert!(library.load().snippets.is_empty());
}

#[test]
fn broken_files_are_reported_but_do_not_block_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let library = Library::new(dir.path());
    library.save(&sample(":ok", "Okay"), &[]).unwrap();

    fs::write(dir.path().join("broken.json"), "{ not json").unwrap();
    fs::write(dir.path().join("notes.txt"), "ignored").unwrap();
    fs::write(dir.path().join(".hidden.json"), "{ ignored }").unwrap();

    let report = library.load();
    assert_eq!(report.snippets.len(), 1);
    assert_eq!(report.errors.len(), 1);
    assert!(matches!(report.errors[0], LibraryError::Parse { .. }));
}

#[test]
fn missing_library_folder_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    let report = Library::new(dir.path().join("does-not-exist")).load();
    assert!(report.snippets.is_empty());
    assert!(report.errors.is_empty());
}

#[test]
fn deleting_removes_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let library = Library::new(dir.path());
    library.save(&sample(":a", "Alpha"), &[]).unwrap();
    let loaded = library.load().snippets.remove(0);
    library.delete(&loaded).unwrap();
    assert!(library.load().snippets.is_empty());
}

#[test]
fn loaded_library_feeds_the_matcher() {
    let dir = tempfile::tempdir().unwrap();
    let library = Library::new(dir.path());
    library.save(&sample(":a", "Alpha"), &[]).unwrap();

    let report = library.load();
    let mut matcher = Matcher::new(report.snippets.iter().map(|s| &s.snippet));
    matcher.feed(typlet_core::Input::Char(':'));
    assert!(matches!(
        matcher.feed(typlet_core::Input::Char('a')),
        typlet_core::Outcome::Expand(_)
    ));
}
