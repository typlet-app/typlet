// SPDX-License-Identifier: GPL-3.0-or-later
//! An immutable, loaded copy of the snippet library.
//!
//! The watcher builds a new snapshot whenever files change and hands the same
//! `Arc` to the engine and the user interface.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use typlet_core::{Conflict, Library, SearchIndex, Snippet, SnippetId, find_conflicts};

/// The library as loaded at one point in time.
#[derive(Debug, Default)]
pub(crate) struct Snapshot {
    /// The library folder.
    pub(crate) root: PathBuf,
    /// All valid snippets, including disabled ones.
    pub(crate) snippets: Vec<Snippet>,
    /// Problems for diagnostics: broken files and trigger conflicts.
    pub(crate) issues: Vec<String>,
    /// Number of files that could not be loaded.
    pub(crate) broken_files: usize,
    /// Search over `snippets`.
    pub(crate) search: SearchIndex,
    by_id: HashMap<SnippetId, usize>,
}

impl Snapshot {
    /// Loads the library below `root`. Never fails: broken files become
    /// issues, a missing folder is an empty library.
    pub(crate) fn load(root: &Path) -> Self {
        let report = Library::new(root).load();
        let broken_files = report.errors.len();
        let mut issues: Vec<String> = report.errors.iter().map(ToString::to_string).collect();
        let snippets: Vec<Snippet> = report.snippets.into_iter().map(|s| s.snippet).collect();
        issues.extend(describe_conflicts(&snippets));
        Self::from_parts(root.to_path_buf(), snippets, issues, broken_files)
    }

    /// Builds a snapshot from already loaded snippets.
    pub(crate) fn from_parts(
        root: PathBuf,
        snippets: Vec<Snippet>,
        issues: Vec<String>,
        broken_files: usize,
    ) -> Self {
        let by_id = snippets
            .iter()
            .enumerate()
            .map(|(i, snippet)| (snippet.id, i))
            .collect();
        let search = SearchIndex::new(&snippets);
        Self {
            root,
            snippets,
            issues,
            broken_files,
            search,
            by_id,
        }
    }

    /// Finds a snippet by id.
    pub(crate) fn get(&self, id: SnippetId) -> Option<&Snippet> {
        self.by_id.get(&id).map(|&i| &self.snippets[i])
    }

    /// Returns `true` if an enabled snippet already uses `trigger`.
    pub(crate) fn trigger_taken(&self, trigger: &str) -> bool {
        self.snippets
            .iter()
            .any(|snippet| snippet.enabled && snippet.trigger == trigger)
    }

    /// Number of enabled snippets.
    pub(crate) fn enabled_count(&self) -> usize {
        self.snippets.iter().filter(|s| s.enabled).count()
    }
}

fn describe_conflicts(snippets: &[Snippet]) -> Vec<String> {
    let label = |id: SnippetId| {
        snippets.iter().find(|s| s.id == id).map_or_else(
            || id.to_string(),
            |s| format!("{} ({})", s.trigger, s.label),
        )
    };
    find_conflicts(snippets)
        .into_iter()
        .map(|conflict| match conflict {
            Conflict::Duplicate { trigger, snippets } => format!(
                "trigger {trigger} is used twice: {} and {}",
                label(snippets[0]),
                label(snippets[1])
            ),
            Conflict::Shadowed { by, shadowed } => format!(
                "{} can never be typed because {} expands first",
                label(shadowed),
                label(by)
            ),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use typlet_core::Document;

    #[test]
    fn loads_snippets_and_reports_conflicts() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::new(dir.path());
        let short = Snippet::new(":c", "Short", Document::from_plain_text("c"));
        let long = Snippet::new(":cc", "Long", Document::from_plain_text("cc"));
        library.save(&short, &[]).unwrap();
        library.save(&long, &[]).unwrap();
        std::fs::write(dir.path().join("broken.json"), "{").unwrap();

        let snapshot = Snapshot::load(dir.path());
        assert_eq!(snapshot.snippets.len(), 2);
        assert_eq!(snapshot.broken_files, 1);
        assert_eq!(snapshot.issues.len(), 2);
        assert!(snapshot.issues[1].contains(":cc (Long) can never be typed"));
        assert_eq!(
            snapshot.get(short.id).map(|s| s.label.as_str()),
            Some("Short")
        );
        assert!(snapshot.trigger_taken(":cc"));
        assert!(!snapshot.trigger_taken(":x"));
    }

    #[test]
    fn missing_folder_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let snapshot = Snapshot::load(&dir.path().join("missing"));
        assert!(snapshot.snippets.is_empty());
        assert_eq!(snapshot.enabled_count(), 0);
    }
}
