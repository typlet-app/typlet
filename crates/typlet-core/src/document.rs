// SPDX-License-Identifier: GPL-3.0-or-later
//! Rich-text model of a snippet body.
//!
//! A [`Document`] is a list of [`Block`]s (paragraphs and lists). Blocks hold
//! [`Inline`] items: formatted text, line breaks and placeholders that are
//! filled in at expansion time (date, cursor, clipboard, form fields).
//!
//! The model is deliberately small. Everything here must be representable in
//! plain text, HTML and RTF, because the target application decides which of
//! those formats it accepts.

use serde::{Deserialize, Serialize};

/// The body of a snippet.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Document {
    /// Top-level blocks in reading order.
    #[serde(default)]
    pub blocks: Vec<Block>,
}

/// A top-level block of a [`Document`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    /// A paragraph of inline content.
    Paragraph {
        /// Inline items of the paragraph.
        #[serde(default)]
        content: Vec<Inline>,
    },
    /// A bulleted list; each item is a run of inline content.
    BulletList {
        /// List items.
        items: Vec<Vec<Inline>>,
    },
    /// A numbered list; each item is a run of inline content.
    OrderedList {
        /// List items.
        items: Vec<Vec<Inline>>,
    },
}

/// An inline item inside a block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Inline {
    /// A run of text with optional formatting.
    Text {
        /// The text itself. May not contain newlines; use [`Inline::LineBreak`].
        text: String,
        /// Formatting applied to the whole run.
        #[serde(default, skip_serializing_if = "Marks::is_plain")]
        marks: Marks,
    },
    /// A line break inside a block (Shift+Enter in the editor).
    LineBreak,
    /// The current date and/or time, formatted with a `strftime` pattern
    /// (for example `%d.%m.%Y`), optionally shifted by a number of days.
    DateTime {
        /// `strftime`-style format pattern.
        format: String,
        /// Days to add (positive) or subtract (negative) from today.
        #[serde(default, skip_serializing_if = "is_zero")]
        offset_days: i64,
    },
    /// Where the text cursor is placed after insertion. At most one per document.
    Cursor,
    /// The current clipboard text.
    Clipboard,
    /// The value of a form field defined in the snippet's `fields`.
    Field {
        /// Name of the referenced field.
        name: String,
    },
}

/// Formatting marks for a text run.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Marks {
    /// Bold text.
    #[serde(skip_serializing_if = "is_false")]
    pub bold: bool,
    /// Italic text.
    #[serde(skip_serializing_if = "is_false")]
    pub italic: bool,
    /// Underlined text.
    #[serde(skip_serializing_if = "is_false")]
    pub underline: bool,
    /// Hyperlink target. Only `http`, `https` and `mailto` links are rendered as links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

impl Marks {
    /// Returns `true` if no formatting is applied.
    #[must_use]
    pub fn is_plain(&self) -> bool {
        !self.bold && !self.italic && !self.underline && self.link.is_none()
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)] // signature required by serde
fn is_false(value: &bool) -> bool {
    !*value
}

#[allow(clippy::trivially_copy_pass_by_ref)] // signature required by serde
fn is_zero(value: &i64) -> bool {
    *value == 0
}

impl Document {
    /// Builds a document from plain text. Every line becomes a paragraph.
    #[must_use]
    pub fn from_plain_text(text: &str) -> Self {
        let blocks = text
            .split('\n')
            .map(|line| Block::Paragraph {
                content: if line.is_empty() {
                    Vec::new()
                } else {
                    vec![Inline::text(line)]
                },
            })
            .collect();
        Self { blocks }
    }

    /// Iterates over all inline items of all blocks, in reading order.
    pub fn inlines(&self) -> impl Iterator<Item = &Inline> {
        self.blocks
            .iter()
            .flat_map(|block| -> Box<dyn Iterator<Item = &Inline> + '_> {
                match block {
                    Block::Paragraph { content } => Box::new(content.iter()),
                    Block::BulletList { items } | Block::OrderedList { items } => {
                        Box::new(items.iter().flatten())
                    }
                }
            })
    }

    /// Returns `true` if the document uses any formatting that plain text cannot
    /// express (marks or lists). Such documents are inserted via the clipboard
    /// with HTML and RTF flavours.
    #[must_use]
    pub fn is_formatted(&self) -> bool {
        self.blocks
            .iter()
            .any(|block| !matches!(block, Block::Paragraph { .. }))
            || self
                .inlines()
                .any(|inline| matches!(inline, Inline::Text { marks, .. } if !marks.is_plain()))
    }
}

impl Inline {
    /// Creates an unformatted text run.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            marks: Marks::default(),
        }
    }

    /// Creates a formatted text run.
    #[must_use]
    pub fn styled(text: impl Into<String>, marks: Marks) -> Self {
        Self::Text {
            text: text.into(),
            marks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_becomes_paragraphs() {
        let doc = Document::from_plain_text("Hallo\n\nWelt");
        assert_eq!(doc.blocks.len(), 3);
        assert_eq!(doc.blocks[1], Block::Paragraph { content: vec![] });
        assert!(!doc.is_formatted());
    }

    #[test]
    fn marks_and_lists_count_as_formatted() {
        let bold = Document {
            blocks: vec![Block::Paragraph {
                content: vec![Inline::styled(
                    "Achtung",
                    Marks {
                        bold: true,
                        ..Marks::default()
                    },
                )],
            }],
        };
        assert!(bold.is_formatted());

        let list = Document {
            blocks: vec![Block::BulletList {
                items: vec![vec![Inline::text("Punkt")]],
            }],
        };
        assert!(list.is_formatted());
    }

    #[test]
    fn json_is_compact_and_round_trips() {
        let doc = Document {
            blocks: vec![Block::Paragraph {
                content: vec![
                    Inline::text("Datum: "),
                    Inline::DateTime {
                        format: "%d.%m.%Y".into(),
                        offset_days: 0,
                    },
                    Inline::Cursor,
                ],
            }],
        };
        let json = serde_json::to_string(&doc).unwrap();
        assert_eq!(
            json,
            r#"{"blocks":[{"type":"paragraph","content":[{"type":"text","text":"Datum: "},{"type":"date_time","format":"%d.%m.%Y"},{"type":"cursor"}]}]}"#
        );
        let back: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(back, doc);
    }
}
