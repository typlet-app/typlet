// SPDX-License-Identifier: GPL-3.0-or-later
//! HTML emitter.
//!
//! The output carries structure and emphasis only: no fonts, sizes or colours.
//! Pasted text therefore adopts the style of the surrounding document, which is
//! what users expect in Outlook, Word or Gmail. Margins are zeroed so pasted
//! paragraphs do not gain extra spacing.

use super::{ResolvedBlock, Run, is_safe_link};
use crate::document::Marks;

const PARAGRAPH_OPEN: &str = r#"<p style="margin:0">"#;
const UL_OPEN: &str = r#"<ul style="margin:0">"#;
const OL_OPEN: &str = r#"<ol style="margin:0">"#;

/// Emits an HTML fragment.
pub(super) fn emit(blocks: &[ResolvedBlock]) -> String {
    let mut out = String::new();
    for block in blocks {
        match block {
            ResolvedBlock::Paragraph(runs) => {
                out.push_str(PARAGRAPH_OPEN);
                if has_visible_content(runs) {
                    emit_runs(&mut out, runs);
                } else {
                    // An empty paragraph would collapse; keep the blank line.
                    out.push_str("<br>");
                }
                out.push_str("</p>");
            }
            ResolvedBlock::BulletList(items) => emit_list(&mut out, UL_OPEN, "</ul>", items),
            ResolvedBlock::OrderedList(items) => emit_list(&mut out, OL_OPEN, "</ol>", items),
        }
    }
    out
}

fn emit_list(out: &mut String, open: &str, close: &str, items: &[Vec<Run>]) {
    out.push_str(open);
    for item in items {
        out.push_str("<li>");
        emit_runs(out, item);
        out.push_str("</li>");
    }
    out.push_str(close);
}

fn has_visible_content(runs: &[Run]) -> bool {
    runs.iter().any(|run| !matches!(run, Run::Cursor))
}

fn emit_runs(out: &mut String, runs: &[Run]) {
    for run in runs {
        match run {
            Run::Text { text, marks } => emit_text(out, text, marks),
            Run::LineBreak => out.push_str("<br>"),
            Run::Cursor => {}
        }
    }
}

fn emit_text(out: &mut String, text: &str, marks: &Marks) {
    let link = marks.link.as_deref().filter(|url| is_safe_link(url));
    if let Some(url) = link {
        out.push_str("<a href=\"");
        escape_into(out, url);
        out.push_str("\">");
    }
    if marks.bold {
        out.push_str("<strong>");
    }
    if marks.italic {
        out.push_str("<em>");
    }
    if marks.underline {
        out.push_str("<u>");
    }
    escape_into(out, text);
    if marks.underline {
        out.push_str("</u>");
    }
    if marks.italic {
        out.push_str("</em>");
    }
    if marks.bold {
        out.push_str("</strong>");
    }
    if link.is_some() {
        out.push_str("</a>");
    }
}

/// Escapes text for use in HTML content and double-quoted attributes.
fn escape_into(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn styled(text: &str, marks: Marks) -> Run {
        Run::Text {
            text: text.into(),
            marks,
        }
    }

    #[test]
    fn marks_are_nested_and_text_escaped() {
        let html = emit(&[ResolvedBlock::Paragraph(vec![styled(
            "<Fett & kursiv>",
            Marks {
                bold: true,
                italic: true,
                ..Marks::default()
            },
        )])]);
        assert_eq!(
            html,
            r#"<p style="margin:0"><strong><em>&lt;Fett &amp; kursiv&gt;</em></strong></p>"#
        );
    }

    #[test]
    fn safe_links_are_rendered_unsafe_links_are_dropped() {
        let safe = emit(&[ResolvedBlock::Paragraph(vec![styled(
            "Website",
            Marks {
                link: Some("https://example.org/?a=1&b=\"2\"".into()),
                ..Marks::default()
            },
        )])]);
        assert!(
            safe.contains(r#"<a href="https://example.org/?a=1&amp;b=&quot;2&quot;">Website</a>"#)
        );

        let unsafe_link = emit(&[ResolvedBlock::Paragraph(vec![styled(
            "Klick",
            Marks {
                link: Some("javascript:alert(1)".into()),
                ..Marks::default()
            },
        )])]);
        assert!(!unsafe_link.contains("<a "));
        assert!(unsafe_link.contains("Klick"));
    }

    #[test]
    fn empty_paragraphs_keep_their_line() {
        let html = emit(&[ResolvedBlock::Paragraph(vec![])]);
        assert_eq!(html, r#"<p style="margin:0"><br></p>"#);
    }

    #[test]
    fn lists_render_as_list_items() {
        let html = emit(&[ResolvedBlock::BulletList(vec![
            vec![styled("eins", Marks::default())],
            vec![styled("zwei", Marks::default())],
        ])]);
        assert_eq!(
            html,
            r#"<ul style="margin:0"><li>eins</li><li>zwei</li></ul>"#
        );
    }
}
