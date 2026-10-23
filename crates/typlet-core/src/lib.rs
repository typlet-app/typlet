// SPDX-License-Identifier: GPL-3.0-or-later
//! # typlet-core
//!
//! The platform-independent core of Typlet, a text expander for everyone.
//!
//! This crate owns everything that does not need the operating system:
//!
//! * [`document`]: the rich-text model of a snippet body
//! * [`snippet`]: snippets and their validation rules
//! * [`mod@render`]: plain text, HTML, RTF and `CF_HTML` output
//! * [`matcher`]: trigger detection in the keystroke stream
//! * [`library`]: loading and saving snippets on disk
//!
//! Platform crates feed keystrokes into a [`matcher::Matcher`], render the
//! matched [`snippet::Snippet`] with [`render::render`] and insert the result.

#![forbid(unsafe_code)]

pub mod document;
pub mod library;
pub mod matcher;
pub mod render;
pub mod snippet;

pub use document::{Block, Document, Inline, Marks};
pub use library::{Library, LibraryError, LoadReport, LoadedSnippet};
pub use matcher::{Conflict, Expansion, Input, Matcher, Outcome, find_conflicts};
pub use render::{InsertMethod, RenderContext, RenderError, Rendered, render, wrap_cf_html};
pub use snippet::{FieldDef, FieldKind, InsertMode, Snippet, SnippetId, ValidationError};
