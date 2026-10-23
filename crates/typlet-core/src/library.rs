// SPDX-License-Identifier: GPL-3.0-or-later
//! Loading and saving snippets on disk.
//!
//! A library is a folder. Every snippet is one JSON file named after its id;
//! sub-folders act as categories. This layout keeps the data readable,
//! makes backups trivial and lets users sync or share libraries with tools
//! they already have (OneDrive, Dropbox, Git) without merge conflicts between
//! unrelated snippets.
//!
//! Loading never fails as a whole because of one broken file: every problem
//! is reported per file and the remaining snippets stay usable.

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use crate::snippet::{Snippet, ValidationError};

/// File extension of snippet files.
pub const EXTENSION: &str = "json";

/// A snippet together with where it is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSnippet {
    /// The snippet.
    pub snippet: Snippet,
    /// Absolute path of its file.
    pub path: PathBuf,
    /// Category path relative to the library root (empty for the root).
    pub category: Vec<String>,
}

/// Why a single file could not be loaded or saved.
#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    /// Reading or writing failed.
    #[error("{path}: {source}")]
    Io {
        /// The affected path.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// The file is not valid snippet JSON.
    #[error("{path}: invalid snippet file: {source}")]
    Parse {
        /// The affected file.
        path: PathBuf,
        /// The underlying error.
        source: serde_json::Error,
    },
    /// The snippet breaks a validation rule.
    #[error("{path}: {source}")]
    Invalid {
        /// The affected file.
        path: PathBuf,
        /// The violated rule.
        source: ValidationError,
    },
}

/// A snippet library rooted at a folder.
#[derive(Debug, Clone)]
pub struct Library {
    root: PathBuf,
}

/// The result of loading a library.
#[derive(Debug, Default)]
pub struct LoadReport {
    /// Snippets that loaded successfully, sorted by category and label.
    pub snippets: Vec<LoadedSnippet>,
    /// Files that could not be loaded.
    pub errors: Vec<LibraryError>,
}

impl Library {
    /// Opens the library at `root`. The folder is created on first save.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The library folder.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Loads all snippets below the root. A missing root is an empty library.
    #[must_use]
    pub fn load(&self) -> LoadReport {
        let mut report = LoadReport::default();
        if self.root.is_dir() {
            self.load_dir(&self.root, &mut Vec::new(), &mut report);
        }
        report.snippets.sort_by(|a, b| {
            (&a.category, a.snippet.label.to_lowercase())
                .cmp(&(&b.category, b.snippet.label.to_lowercase()))
        });
        report
    }

    fn load_dir(&self, dir: &Path, category: &mut Vec<String>, report: &mut LoadReport) {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(source) => {
                report.errors.push(LibraryError::Io {
                    path: dir.to_path_buf(),
                    source,
                });
                return;
            }
        };

        let mut paths: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
        paths.sort();

        for path in paths {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.starts_with('.') {
                continue; // hidden files, editor swap files, temporary files
            }
            if path.is_dir() {
                category.push(name.to_owned());
                self.load_dir(&path, category, report);
                category.pop();
            } else if path.extension().and_then(|e| e.to_str()) == Some(EXTENSION) {
                match load_file(&path) {
                    Ok(snippet) => report.snippets.push(LoadedSnippet {
                        snippet,
                        path,
                        category: category.clone(),
                    }),
                    Err(error) => report.errors.push(error),
                }
            }
        }
    }

    /// Saves a snippet into `category` (relative to the root) atomically:
    /// the file is written next to its destination and then renamed, so a
    /// crash never leaves a half-written snippet behind.
    ///
    /// # Errors
    ///
    /// Fails if the snippet is invalid or the file cannot be written.
    pub fn save(&self, snippet: &Snippet, category: &[String]) -> Result<PathBuf, LibraryError> {
        let mut dir = self.root.clone();
        dir.extend(category);
        let path = dir.join(format!("{}.{EXTENSION}", snippet.id));

        snippet.validate().map_err(|source| LibraryError::Invalid {
            path: path.clone(),
            source,
        })?;

        let io_err = |source| LibraryError::Io {
            path: path.clone(),
            source,
        };
        fs::create_dir_all(&dir).map_err(io_err)?;

        let mut json =
            serde_json::to_string_pretty(snippet).map_err(|source| LibraryError::Parse {
                path: path.clone(),
                source,
            })?;
        json.push('\n');

        let mut temp = tempfile::Builder::new()
            .prefix(".typlet-")
            .suffix(".tmp")
            .tempfile_in(&dir)
            .map_err(io_err)?;
        temp.write_all(json.as_bytes()).map_err(io_err)?;
        temp.as_file().sync_all().map_err(io_err)?;
        temp.persist(&path).map_err(|e| io_err(e.error))?;
        Ok(path)
    }

    /// Deletes a snippet file.
    ///
    /// # Errors
    ///
    /// Fails if the file cannot be removed.
    pub fn delete(&self, loaded: &LoadedSnippet) -> Result<(), LibraryError> {
        fs::remove_file(&loaded.path).map_err(|source| LibraryError::Io {
            path: loaded.path.clone(),
            source,
        })
    }
}

/// Reads and validates one snippet file.
///
/// # Errors
///
/// Fails if the file cannot be read, parsed or validated.
pub fn load_file(path: &Path) -> Result<Snippet, LibraryError> {
    let text = fs::read_to_string(path).map_err(|source| LibraryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let snippet: Snippet = serde_json::from_str(&text).map_err(|source| LibraryError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    snippet.validate().map_err(|source| LibraryError::Invalid {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(snippet)
}
