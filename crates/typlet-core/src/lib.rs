// SPDX-License-Identifier: GPL-3.0-or-later
//! # typlet-core
//!
//! The platform-independent core of Typlet, a text expander for everyone.
//!
//! This crate owns everything that does not need the operating system:
//!
//! * [`document`]: the rich-text model of a snippet body
//! * [`snippet`]: snippets and their validation rules
//! * [`nesting`]: snippets inside snippets
//! * [`mod@render`]: plain text, HTML, RTF and `CF_HTML` output
//! * [`matcher`]: trigger detection in the keystroke stream
//! * [`plan`]: the concrete steps that replace a trigger with its text
//! * [`library`]: loading and saving snippets on disk
//! * [`search`]: finding snippets by trigger, name and text
//! * [`settings`]: user settings and per-application rules
//! * [`hotkey`]: keyboard shortcuts
//! * [`update`]: what is known about new versions
//! * [`links`]: Typlet's web addresses (user guide, releases, donations)
//! * [`stats`]: local usage statistics, streaks and milestones
//!
//! Platform crates feed keystrokes into a [`matcher::Matcher`], render the
//! matched [`snippet::Snippet`] with [`render::render`], turn the result into
//! [`plan::Action`]s and execute them.

#![forbid(unsafe_code)]

pub mod document;
pub mod fsutil;
pub mod hotkey;
pub mod library;
pub mod links;
pub mod matcher;
pub mod nesting;
pub mod plan;
pub mod render;
pub mod search;
pub mod settings;
pub mod snippet;
pub mod stats;
pub mod update;

pub use document::{Block, Document, Inline, Marks};
pub use hotkey::{Hotkey, HotkeyError, Key};
pub use library::{Library, LibraryError, LoadReport, LoadedSnippet};
pub use matcher::{Conflict, Expansion, Input, Matcher, Outcome, find_conflicts};
pub use nesting::NestingError;
pub use plan::{Action, ClipboardContent, InsertedText, plan_expansion, plan_undo};
pub use render::{
    DateLanguage, InsertMethod, RenderContext, RenderError, Rendered, render, wrap_cf_html,
};
pub use search::SearchIndex;
pub use settings::{AppRule, EffectiveRule, Language, NewlineMode, Settings, SettingsError};
pub use snippet::{FieldDef, FieldKind, InsertMode, Snippet, SnippetId, ValidationError};
pub use update::{AvailableUpdate, UpdateState};
