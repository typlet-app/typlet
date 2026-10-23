// SPDX-License-Identifier: GPL-3.0-or-later
//! Turns a snippet into the text that is inserted into the target application.
//!
//! Rendering happens in two steps:
//!
//! 1. **Resolve**: placeholders are replaced by their values (date, clipboard,
//!    form fields). The result is a simple tree of formatted text runs.
//! 2. **Emit**: the resolved tree is written as plain text, HTML and RTF.
//!
//! The platform layer puts all flavours on the clipboard at once, so the target
//! application can pick the richest format it understands. Plain-text-only
//! fields automatically receive the plain flavour.

mod cf_html;
mod html;
mod plain;
mod rtf;

use std::collections::HashMap;

use chrono::{NaiveDateTime, TimeDelta};

use crate::document::{Block, Inline, Marks};
use crate::snippet::{InsertMode, Snippet, is_valid_date_format};

pub use cf_html::wrap_cf_html;

/// Plain text up to this many characters is typed; longer text is pasted.
pub const TYPE_THRESHOLD_CHARS: usize = 200;

/// Values needed to fill in placeholders.
#[derive(Debug, Clone)]
pub struct RenderContext<'a> {
    /// The current local date and time.
    pub now: NaiveDateTime,
    /// The clipboard text at the moment of expansion, if any.
    pub clipboard: Option<&'a str>,
    /// Form values entered by the user, keyed by field name.
    pub fields: &'a HashMap<String, String>,
}

/// A rendered snippet in all output flavours.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// Plain text with `\n` line endings.
    pub plain: String,
    /// An HTML fragment without fonts or colours, so it adopts the target's style.
    pub html: String,
    /// A complete RTF document.
    pub rtf: String,
    /// `true` if the snippet uses formatting that plain text cannot express.
    pub formatted: bool,
    /// Where to place the cursor, counted in characters back from the end of
    /// the inserted text (`\n` counts as one). `None` leaves it at the end.
    pub cursor_from_end: Option<usize>,
}

/// How the rendered text should be inserted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertMethod {
    /// Simulate typing the plain text.
    Type,
    /// Paste via the clipboard (all flavours), then restore the clipboard.
    Paste,
}

impl Rendered {
    /// Chooses the insertion method for this text.
    ///
    /// In [`InsertMode::Auto`], text is pasted when it is formatted, spans
    /// several lines or is long. Typing line breaks is unreliable (some apps
    /// drop them, chat apps send the message), and typing long text is slow.
    #[must_use]
    pub fn insert_method(&self, mode: InsertMode) -> InsertMethod {
        match mode {
            InsertMode::Type => InsertMethod::Type,
            InsertMode::Paste => InsertMethod::Paste,
            InsertMode::Auto => {
                if self.formatted
                    || self.plain.contains('\n')
                    || self.plain.chars().count() > TYPE_THRESHOLD_CHARS
                {
                    InsertMethod::Paste
                } else {
                    InsertMethod::Type
                }
            }
        }
    }
}

/// Why a snippet could not be rendered.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RenderError {
    /// Shifting the date left the supported calendar range.
    #[error("the date offset is out of range")]
    DateOutOfRange,
    /// A date/time placeholder has an invalid format pattern.
    #[error("invalid date/time format `{0}`")]
    InvalidDateFormat(String),
    /// A field has neither an entered value nor a default.
    #[error("no value for field `{0}`")]
    MissingField(String),
}

/// Renders a snippet.
///
/// # Errors
///
/// Fails if a placeholder cannot be filled in (see [`RenderError`]).
pub fn render(snippet: &Snippet, ctx: &RenderContext<'_>) -> Result<Rendered, RenderError> {
    let resolved = resolve(snippet, ctx)?;
    let (plain, cursor_from_end) = plain::emit(&resolved);
    Ok(Rendered {
        plain,
        html: html::emit(&resolved),
        rtf: rtf::emit(&resolved),
        formatted: snippet.content.is_formatted(),
        cursor_from_end,
    })
}

/// A block whose placeholders have been replaced by text.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ResolvedBlock {
    Paragraph(Vec<Run>),
    BulletList(Vec<Vec<Run>>),
    OrderedList(Vec<Vec<Run>>),
}

/// A resolved inline item.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Run {
    Text { text: String, marks: Marks },
    LineBreak,
    Cursor,
}

fn resolve(snippet: &Snippet, ctx: &RenderContext<'_>) -> Result<Vec<ResolvedBlock>, RenderError> {
    let resolve_runs = |inlines: &[Inline]| -> Result<Vec<Run>, RenderError> {
        let mut runs = Vec::with_capacity(inlines.len());
        for inline in inlines {
            match inline {
                Inline::Text { text, marks } => push_text(&mut runs, text, marks),
                Inline::LineBreak => runs.push(Run::LineBreak),
                Inline::Cursor => runs.push(Run::Cursor),
                Inline::Clipboard => {
                    push_text(
                        &mut runs,
                        ctx.clipboard.unwrap_or_default(),
                        &Marks::default(),
                    );
                }
                Inline::DateTime {
                    format,
                    offset_days,
                } => {
                    let text = format_date(ctx.now, format, *offset_days)?;
                    push_text(&mut runs, &text, &Marks::default());
                }
                Inline::Field { name } => {
                    let value = ctx
                        .fields
                        .get(name)
                        .map(String::as_str)
                        .or_else(|| {
                            snippet
                                .fields
                                .iter()
                                .find(|field| &field.name == name)
                                .and_then(|field| field.default.as_deref())
                        })
                        .ok_or_else(|| RenderError::MissingField(name.clone()))?;
                    push_text(&mut runs, value, &Marks::default());
                }
            }
        }
        Ok(runs)
    };

    snippet
        .content
        .blocks
        .iter()
        .map(|block| {
            Ok(match block {
                Block::Paragraph { content } => ResolvedBlock::Paragraph(resolve_runs(content)?),
                Block::BulletList { items } => ResolvedBlock::BulletList(
                    items
                        .iter()
                        .map(|item| resolve_runs(item))
                        .collect::<Result<_, _>>()?,
                ),
                Block::OrderedList { items } => ResolvedBlock::OrderedList(
                    items
                        .iter()
                        .map(|item| resolve_runs(item))
                        .collect::<Result<_, _>>()?,
                ),
            })
        })
        .collect()
}

/// Appends text, turning embedded newlines (from the clipboard or multi-line
/// fields) into line breaks so every emitter handles them consistently.
fn push_text(runs: &mut Vec<Run>, text: &str, marks: &Marks) {
    let normalised = text.replace("\r\n", "\n").replace('\r', "\n");
    for (index, line) in normalised.split('\n').enumerate() {
        if index > 0 {
            runs.push(Run::LineBreak);
        }
        if !line.is_empty() {
            runs.push(Run::Text {
                text: line.to_owned(),
                marks: marks.clone(),
            });
        }
    }
}

fn format_date(now: NaiveDateTime, format: &str, offset_days: i64) -> Result<String, RenderError> {
    if !is_valid_date_format(format) {
        return Err(RenderError::InvalidDateFormat(format.to_owned()));
    }
    let delta = TimeDelta::try_days(offset_days).ok_or(RenderError::DateOutOfRange)?;
    let date = now
        .checked_add_signed(delta)
        .ok_or(RenderError::DateOutOfRange)?;
    Ok(date.format(format).to_string())
}

/// Returns `true` for link targets that are safe to render as clickable links.
fn is_safe_link(url: &str) -> bool {
    let lower = url.trim_start().to_ascii_lowercase();
    ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use crate::snippet::{FieldDef, FieldKind};
    use chrono::NaiveDate;

    fn ctx_now() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 23)
            .unwrap()
            .and_hms_opt(9, 42, 0)
            .unwrap()
    }

    fn paragraph(content: Vec<Inline>) -> Snippet {
        Snippet::new(
            ":t",
            "Test",
            Document {
                blocks: vec![Block::Paragraph { content }],
            },
        )
    }

    fn render_with(
        snippet: &Snippet,
        clipboard: Option<&str>,
        fields: &HashMap<String, String>,
    ) -> Rendered {
        render(
            snippet,
            &RenderContext {
                now: ctx_now(),
                clipboard,
                fields,
            },
        )
        .unwrap()
    }

    #[test]
    fn date_with_offset() {
        let snippet = paragraph(vec![
            Inline::DateTime {
                format: "%d.%m.%Y".into(),
                offset_days: 0,
            },
            Inline::text(" / "),
            Inline::DateTime {
                format: "%d.%m.%Y".into(),
                offset_days: 14,
            },
        ]);
        let out = render_with(&snippet, None, &HashMap::new());
        assert_eq!(out.plain, "23.10.2026 / 06.11.2026");
    }

    #[test]
    fn clipboard_newlines_become_line_breaks() {
        let snippet = paragraph(vec![Inline::text("> "), Inline::Clipboard]);
        let out = render_with(&snippet, Some("eins\r\nzwei"), &HashMap::new());
        assert_eq!(out.plain, "> eins\nzwei");
        assert!(out.html.contains("eins<br>zwei"));
        assert!(out.rtf.contains("eins\\line zwei"));
    }

    #[test]
    fn fields_use_value_then_default_then_fail() {
        let mut snippet = paragraph(vec![Inline::Field {
            name: "grund".into(),
        }]);
        snippet.fields.push(FieldDef {
            name: "grund".into(),
            label: "Grund".into(),
            kind: FieldKind::Text,
            default: Some("Defekt".into()),
        });

        let mut values = HashMap::new();
        assert_eq!(render_with(&snippet, None, &values).plain, "Defekt");

        values.insert("grund".to_owned(), "Umbau".to_owned());
        assert_eq!(render_with(&snippet, None, &values).plain, "Umbau");

        snippet.fields[0].default = None;
        let err = render(
            &snippet,
            &RenderContext {
                now: ctx_now(),
                clipboard: None,
                fields: &HashMap::new(),
            },
        )
        .unwrap_err();
        assert_eq!(err, RenderError::MissingField("grund".into()));
    }

    #[test]
    fn cursor_position_counts_from_end() {
        let snippet = Snippet::new(
            ":cc",
            "CC",
            Document {
                blocks: vec![
                    Block::Paragraph {
                        content: vec![Inline::text("Verursacher: "), Inline::Cursor],
                    },
                    Block::Paragraph {
                        content: vec![Inline::text("Grund: ")],
                    },
                ],
            },
        );
        let out = render_with(&snippet, None, &HashMap::new());
        assert_eq!(out.plain, "Verursacher: \nGrund: ");
        assert_eq!(out.cursor_from_end, Some("\nGrund: ".chars().count()));
    }

    #[test]
    fn insert_method_rules() {
        let short = render_with(
            &paragraph(vec![Inline::text("kurz")]),
            None,
            &HashMap::new(),
        );
        assert_eq!(short.insert_method(InsertMode::Auto), InsertMethod::Type);
        assert_eq!(short.insert_method(InsertMode::Paste), InsertMethod::Paste);

        let multiline = render_with(
            &paragraph(vec![
                Inline::text("a"),
                Inline::LineBreak,
                Inline::text("b"),
            ]),
            None,
            &HashMap::new(),
        );
        assert_eq!(
            multiline.insert_method(InsertMode::Auto),
            InsertMethod::Paste
        );
        assert_eq!(
            multiline.insert_method(InsertMode::Type),
            InsertMethod::Type
        );

        let long = render_with(
            &paragraph(vec![Inline::text("x".repeat(TYPE_THRESHOLD_CHARS + 1))]),
            None,
            &HashMap::new(),
        );
        assert_eq!(long.insert_method(InsertMode::Auto), InsertMethod::Paste);
    }

    #[test]
    fn invalid_date_format_does_not_panic() {
        let snippet = paragraph(vec![Inline::DateTime {
            format: "%Q".into(),
            offset_days: 0,
        }]);
        let err = render(
            &snippet,
            &RenderContext {
                now: ctx_now(),
                clipboard: None,
                fields: &HashMap::new(),
            },
        )
        .unwrap_err();
        assert_eq!(err, RenderError::InvalidDateFormat("%Q".into()));
    }

    #[test]
    fn huge_offsets_are_reported() {
        let snippet = paragraph(vec![Inline::DateTime {
            format: "%Y".into(),
            offset_days: i64::MAX,
        }]);
        let err = render(
            &snippet,
            &RenderContext {
                now: ctx_now(),
                clipboard: None,
                fields: &HashMap::new(),
            },
        )
        .unwrap_err();
        assert_eq!(err, RenderError::DateOutOfRange);
    }

    #[test]
    fn link_safety() {
        assert!(is_safe_link("https://typlet.app"));
        assert!(is_safe_link("mailto:hallo@example.org"));
        assert!(!is_safe_link("javascript:alert(1)"));
        assert!(!is_safe_link("file:///etc/passwd"));
    }
}
