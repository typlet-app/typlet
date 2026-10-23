// SPDX-License-Identifier: GPL-3.0-or-later
//! RTF emitter.
//!
//! Some desktop applications (classic Office, WordPad, LibreOffice) prefer RTF
//! over HTML when pasting. Like the HTML flavour, the RTF output carries no
//! fonts or colours so the target's style applies.

use std::fmt::Write as _;

use super::{ResolvedBlock, Run, is_safe_link};
use crate::document::Marks;

/// Emits a complete RTF document.
pub(super) fn emit(blocks: &[ResolvedBlock]) -> String {
    let mut out = String::from(r"{\rtf1\ansi\deff0\uc1{\fonttbl{\f0\fnil;}}");
    for (index, block) in blocks.iter().enumerate() {
        if index > 0 {
            out.push_str(r"\par ");
        }
        match block {
            ResolvedBlock::Paragraph(runs) => emit_runs(&mut out, runs),
            ResolvedBlock::BulletList(items) => {
                for (item_index, item) in items.iter().enumerate() {
                    if item_index > 0 {
                        out.push_str(r"\par ");
                    }
                    escape_into(&mut out, "•");
                    out.push_str(r"\tab ");
                    emit_runs(&mut out, item);
                }
            }
            ResolvedBlock::OrderedList(items) => {
                for (item_index, item) in items.iter().enumerate() {
                    if item_index > 0 {
                        out.push_str(r"\par ");
                    }
                    let _ = write!(out, r"{}.\tab ", item_index + 1);
                    emit_runs(&mut out, item);
                }
            }
        }
    }
    out.push('}');
    out
}

fn emit_runs(out: &mut String, runs: &[Run]) {
    for run in runs {
        match run {
            Run::Text { text, marks } => emit_text(out, text, marks),
            Run::LineBreak => out.push_str(r"\line "),
            Run::Cursor => {}
        }
    }
}

fn emit_text(out: &mut String, text: &str, marks: &Marks) {
    let link = marks.link.as_deref().filter(|url| is_safe_link(url));
    if let Some(url) = link {
        out.push_str(r#"{\field{\*\fldinst{HYPERLINK ""#);
        escape_into(out, &url.replace('"', "%22"));
        out.push_str(r#""}}{\fldrslt{"#);
    }

    let styled = marks.bold || marks.italic || marks.underline;
    if styled {
        out.push('{');
        if marks.bold {
            out.push_str(r"\b");
        }
        if marks.italic {
            out.push_str(r"\i");
        }
        if marks.underline {
            out.push_str(r"\ul");
        }
        out.push(' ');
    }
    escape_into(out, text);
    if styled {
        out.push('}');
    }

    if link.is_some() {
        out.push_str("}}}");
    }
}

/// Escapes text for RTF. Non-ASCII characters become `\uN?` escapes with
/// signed 16-bit code units (surrogate pairs for characters outside the BMP).
fn escape_into(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '\\' => out.push_str(r"\\"),
            '{' => out.push_str(r"\{"),
            '}' => out.push_str(r"\}"),
            '\t' => out.push_str(r"\tab "),
            c if c.is_ascii() => out.push(c),
            c => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    // RTF requires the code unit as a signed 16-bit number;
                    // the reinterpretation is the documented encoding.
                    #[allow(clippy::cast_possible_wrap)]
                    let signed = *unit as i16;
                    let _ = write!(out, r"\u{signed}?");
                }
            }
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
    fn document_frame_and_paragraphs() {
        let rtf = emit(&[
            ResolvedBlock::Paragraph(vec![styled("eins", Marks::default())]),
            ResolvedBlock::Paragraph(vec![styled("zwei", Marks::default())]),
        ]);
        assert_eq!(
            rtf,
            r"{\rtf1\ansi\deff0\uc1{\fonttbl{\f0\fnil;}}eins\par zwei}"
        );
    }

    #[test]
    fn special_characters_are_escaped() {
        let rtf = emit(&[ResolvedBlock::Paragraph(vec![styled(
            r"a\b{c}ä€😀",
            Marks::default(),
        )])]);
        assert!(rtf.contains(r"a\\b\{c\}"));
        assert!(rtf.contains(&unicode_escape(228)), "ä is U+00E4");
        assert!(rtf.contains(&unicode_escape(8364)), "€ is U+20AC");
        assert!(
            rtf.contains(&(unicode_escape(-10179) + &unicode_escape(-8704))),
            "😀 is a surrogate pair"
        );
    }

    #[test]
    fn bullets_use_a_unicode_escape() {
        let rtf = emit(&[ResolvedBlock::BulletList(vec![vec![styled(
            "Punkt",
            Marks::default(),
        )]])]);
        assert!(rtf.contains(&(unicode_escape(8226) + r"\tab Punkt")));
    }

    /// Builds `\uN?` without writing the escape literally in source.
    fn unicode_escape(code: i32) -> String {
        format!("{}u{code}?", '\\')
    }

    #[test]
    fn marks_and_links() {
        let rtf = emit(&[ResolvedBlock::Paragraph(vec![styled(
            "Hier",
            Marks {
                bold: true,
                underline: true,
                link: Some("https://example.org".into()),
                ..Marks::default()
            },
        )])]);
        assert!(rtf.contains(
            r#"{\field{\*\fldinst{HYPERLINK "https://example.org"}}{\fldrslt{{\b\ul Hier}}}}"#
        ));
    }
}
