// SPDX-License-Identifier: GPL-3.0-or-later
//! Snippets: a trigger, a label, a rich-text body and optional form fields.

use std::collections::HashSet;
use std::fmt;

use chrono::format::{Item, StrftimeItems};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::document::{Document, Inline};
use crate::hotkey::Hotkey;

/// Current version of the snippet file format.
pub const FORMAT_VERSION: u32 = 1;

/// Maximum length of a trigger in characters.
pub const MAX_TRIGGER_CHARS: usize = 64;

/// Stable identifier of a snippet. Also used as its file name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SnippetId(pub Uuid);

impl SnippetId {
    /// Creates a new, time-ordered identifier (UUID v7).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for SnippetId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SnippetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// How the expanded text is inserted into the target application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsertMode {
    /// Let Typlet decide: type short plain text, paste long or formatted text.
    #[default]
    Auto,
    /// Always simulate typing (plain text only).
    Type,
    /// Always paste via the clipboard (restoring the previous clipboard afterwards).
    Paste,
}

/// The kind of value a form field collects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldKind {
    /// A single line of text.
    Text,
    /// Multiple lines of text.
    MultilineText,
    /// One of a fixed list of options.
    Choice {
        /// The selectable options, in display order.
        options: Vec<String>,
    },
    /// Yes or no.
    YesNo,
}

/// A form field that is filled in before the snippet is inserted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDef {
    /// Internal name, referenced by [`Inline::Field`].
    pub name: String,
    /// Label shown in the form.
    pub label: String,
    /// The kind of input.
    pub kind: FieldKind,
    /// Pre-filled value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

/// A text snippet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snippet {
    /// File format version. Written as [`FORMAT_VERSION`].
    #[serde(default = "default_version")]
    pub version: u32,
    /// Stable identifier.
    pub id: SnippetId,
    /// What the user types to expand the snippet, for example `:sig`.
    pub trigger: String,
    /// Human-readable name, used in search and in the editor.
    pub label: String,
    /// The text that replaces the trigger.
    pub content: Document,
    /// Form fields asked for before insertion, in display order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldDef>,
    /// Disabled snippets are kept but never expand.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Per-snippet override of the insertion method.
    #[serde(default, skip_serializing_if = "is_auto")]
    pub insert_mode: InsertMode,
    /// Optional global keyboard shortcut that inserts the snippet directly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<Hotkey>,
}

fn default_version() -> u32 {
    FORMAT_VERSION
}

fn default_true() -> bool {
    true
}

#[allow(clippy::trivially_copy_pass_by_ref)] // signature required by serde
fn is_auto(mode: &InsertMode) -> bool {
    *mode == InsertMode::Auto
}

/// Reasons why a snippet is invalid.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    /// The file was written by a newer version of Typlet.
    #[error("snippet format version {0} is newer than supported version {FORMAT_VERSION}")]
    UnsupportedVersion(u32),
    /// The trigger is empty.
    #[error("the trigger is empty")]
    EmptyTrigger,
    /// The trigger contains whitespace or control characters.
    #[error("the trigger may not contain spaces or control characters")]
    InvalidTriggerCharacter,
    /// The trigger is too long.
    #[error("the trigger is longer than {MAX_TRIGGER_CHARS} characters")]
    TriggerTooLong,
    /// More than one cursor placeholder.
    #[error("the snippet contains more than one cursor position")]
    MultipleCursors,
    /// A text run contains a newline character.
    #[error("text runs may not contain newlines; use line breaks instead")]
    NewlineInText,
    /// A date/time placeholder has an invalid format pattern.
    #[error("invalid date/time format `{0}`")]
    InvalidDateFormat(String),
    /// A field placeholder references a field that is not defined.
    #[error("the field `{0}` is used but not defined")]
    UndefinedField(String),
    /// Two fields share the same name.
    #[error("the field name `{0}` is used twice")]
    DuplicateField(String),
    /// A choice field has no options.
    #[error("the choice field `{0}` has no options")]
    EmptyChoice(String),
    /// The snippet includes itself.
    #[error("the snippet includes itself")]
    IncludesItself,
}

impl Snippet {
    /// Creates a new, enabled snippet with a fresh identifier.
    #[must_use]
    pub fn new(trigger: impl Into<String>, label: impl Into<String>, content: Document) -> Self {
        Self {
            version: FORMAT_VERSION,
            id: SnippetId::new(),
            trigger: trigger.into(),
            label: label.into(),
            content,
            fields: Vec::new(),
            enabled: true,
            insert_mode: InsertMode::Auto,
            hotkey: None,
        }
    }

    /// Checks all rules a snippet must satisfy before it can be used.
    ///
    /// # Errors
    ///
    /// Returns the first rule that is violated.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.version > FORMAT_VERSION {
            return Err(ValidationError::UnsupportedVersion(self.version));
        }
        validate_trigger(&self.trigger)?;

        let mut field_names = HashSet::new();
        for field in &self.fields {
            if !field_names.insert(field.name.as_str()) {
                return Err(ValidationError::DuplicateField(field.name.clone()));
            }
            if let FieldKind::Choice { options } = &field.kind
                && options.is_empty()
            {
                return Err(ValidationError::EmptyChoice(field.name.clone()));
            }
        }

        let mut cursors = 0;
        for inline in self.content.inlines() {
            match inline {
                Inline::Text { text, .. } if text.contains(['\n', '\r']) => {
                    return Err(ValidationError::NewlineInText);
                }
                Inline::Cursor => {
                    cursors += 1;
                    if cursors > 1 {
                        return Err(ValidationError::MultipleCursors);
                    }
                }
                Inline::DateTime { format, .. } if !is_valid_date_format(format) => {
                    return Err(ValidationError::InvalidDateFormat(format.clone()));
                }
                Inline::Field { name } if !field_names.contains(name.as_str()) => {
                    return Err(ValidationError::UndefinedField(name.clone()));
                }
                Inline::Snippet { id } if *id == self.id => {
                    return Err(ValidationError::IncludesItself);
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Checks whether a string is usable as a trigger.
///
/// # Errors
///
/// Returns why the trigger is not usable.
pub fn validate_trigger(trigger: &str) -> Result<(), ValidationError> {
    if trigger.is_empty() {
        return Err(ValidationError::EmptyTrigger);
    }
    if trigger.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(ValidationError::InvalidTriggerCharacter);
    }
    if trigger.chars().count() > MAX_TRIGGER_CHARS {
        return Err(ValidationError::TriggerTooLong);
    }
    Ok(())
}

/// Returns `true` if `format` is a valid `strftime` pattern.
#[must_use]
pub fn is_valid_date_format(format: &str) -> bool {
    !StrftimeItems::new(format).any(|item| matches!(item, Item::Error))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snippet_with(content: Vec<Inline>) -> Snippet {
        Snippet::new(
            ":t",
            "Test",
            Document {
                blocks: vec![crate::document::Block::Paragraph { content }],
            },
        )
    }

    #[test]
    fn valid_snippet_passes() {
        let snippet = snippet_with(vec![
            Inline::text("Finn Ole Bach, "),
            Inline::DateTime {
                format: "%d.%m.%Y".into(),
                offset_days: 0,
            },
        ]);
        assert_eq!(snippet.validate(), Ok(()));
    }

    #[test]
    fn trigger_rules() {
        assert_eq!(validate_trigger(""), Err(ValidationError::EmptyTrigger));
        assert_eq!(
            validate_trigger(":a b"),
            Err(ValidationError::InvalidTriggerCharacter)
        );
        assert_eq!(
            validate_trigger(&"x".repeat(MAX_TRIGGER_CHARS + 1)),
            Err(ValidationError::TriggerTooLong)
        );
        assert_eq!(validate_trigger(":äöü"), Ok(()));
    }

    #[test]
    fn only_one_cursor_allowed() {
        let snippet = snippet_with(vec![Inline::Cursor, Inline::Cursor]);
        assert_eq!(snippet.validate(), Err(ValidationError::MultipleCursors));
    }

    #[test]
    fn invalid_date_format_is_rejected() {
        let snippet = snippet_with(vec![Inline::DateTime {
            format: "%Q".into(),
            offset_days: 0,
        }]);
        assert_eq!(
            snippet.validate(),
            Err(ValidationError::InvalidDateFormat("%Q".into()))
        );
    }

    #[test]
    fn fields_must_be_defined_unique_and_non_empty() {
        let mut snippet = snippet_with(vec![Inline::Field {
            name: "grund".into(),
        }]);
        assert_eq!(
            snippet.validate(),
            Err(ValidationError::UndefinedField("grund".into()))
        );

        let field = FieldDef {
            name: "grund".into(),
            label: "Grund".into(),
            kind: FieldKind::Text,
            default: None,
        };
        snippet.fields = vec![field.clone()];
        assert_eq!(snippet.validate(), Ok(()));

        snippet.fields.push(field);
        assert_eq!(
            snippet.validate(),
            Err(ValidationError::DuplicateField("grund".into()))
        );

        snippet.fields = vec![FieldDef {
            name: "grund".into(),
            label: "Grund".into(),
            kind: FieldKind::Choice { options: vec![] },
            default: None,
        }];
        assert_eq!(
            snippet.validate(),
            Err(ValidationError::EmptyChoice("grund".into()))
        );
    }

    #[test]
    fn newer_format_versions_are_rejected() {
        let mut snippet = snippet_with(vec![]);
        snippet.version = FORMAT_VERSION + 1;
        assert_eq!(
            snippet.validate(),
            Err(ValidationError::UnsupportedVersion(FORMAT_VERSION + 1))
        );
    }

    #[test]
    fn defaults_are_applied_when_deserialising() {
        let json = r#"{"id":"0192f1a0-0000-7000-8000-000000000000","trigger":":x","label":"X","content":{"blocks":[]}}"#;
        let snippet: Snippet = serde_json::from_str(json).unwrap();
        assert_eq!(snippet.version, FORMAT_VERSION);
        assert!(snippet.enabled);
        assert_eq!(snippet.insert_mode, InsertMode::Auto);
        assert!(snippet.fields.is_empty());
    }
}
