// SPDX-License-Identifier: GPL-3.0-or-later
//! # typlet-import
//!
//! Imports snippets from other text expanders into the Typlet format.
//!
//! Currently supported: **espanso** match files (`match/*.yml`). Every
//! espanso trigger becomes one Typlet snippet. Plain text, HTML and Markdown
//! replacements, dates, the clipboard, the cursor position (`$|$`), forms
//! and nested matches are converted; shell scripts, regular expressions and
//! images are skipped with a reason the editor can show.
//!
//! This crate reads files but never writes them; the caller decides where
//! the imported snippets are saved.

#![forbid(unsafe_code)]

mod espanso;
mod rich;

pub use espanso::{import_dir, parse_file};

use std::collections::HashMap;

use serde::Serialize;
use typlet_core::{Snippet, SnippetId};

/// One snippet that was converted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Imported {
    /// The converted snippet with a fresh identifier.
    pub snippet: Snippet,
    /// The file it came from, relative to the imported folder.
    pub source: String,
    /// Suggested category, for example `["espanso", "base"]`.
    pub category: Vec<String>,
    /// Things that could not be carried over exactly.
    pub warnings: Vec<Warning>,
}

/// Something that was imported with a small difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Warning {
    /// espanso's `word: true` (expand only at word boundaries) is not
    /// supported; the trigger expands instantly.
    WordBoundary,
    /// espanso's `propagate_case` is not supported.
    PropagateCase,
    /// HTML or Markdown used formatting Typlet does not have; it was dropped.
    FormattingSimplified,
    /// A date offset was not a whole number of days and was rounded.
    OffsetRounded,
}

/// A match that could not be imported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Skipped {
    /// The file it came from.
    pub source: String,
    /// Its trigger, if it had one.
    pub trigger: String,
    /// Why it was skipped.
    pub reason: SkipReason,
}

/// Why a match was skipped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum SkipReason {
    /// Regular-expression triggers are not supported.
    Regex,
    /// Image snippets are not supported yet.
    Image,
    /// The match has no text to insert.
    NoText,
    /// The match uses a variable type Typlet does not support (for example
    /// `shell` or `script`).
    UnsupportedVariable(String),
    /// The trigger is not usable in Typlet (for example it contains spaces).
    InvalidTrigger(String),
    /// The match inserts another match (`type: match`) that does not exist
    /// or could not be imported; the value is that match's trigger.
    MissingMatch(String),
    /// The converted snippet breaks a Typlet rule.
    Invalid(String),
    /// The whole file could not be read.
    Unreadable(String),
}

/// The result of an import.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ImportReport {
    /// Converted snippets.
    pub imported: Vec<Imported>,
    /// Matches that were left out.
    pub skipped: Vec<Skipped>,
    /// Number of files read.
    pub files: usize,
    /// Included matches that still have to be linked: placeholder id →
    /// trigger of the included match.
    #[serde(skip)]
    includes: HashMap<SnippetId, String>,
}
