// SPDX-License-Identifier: GPL-3.0-or-later
//! Plain-text emitter.

use super::{ResolvedBlock, Run};

/// Emits plain text and the cursor position (characters from the end).
pub(super) fn emit(blocks: &[ResolvedBlock]) -> (String, Option<usize>) {
    let mut out = String::new();
    let mut cursor_at: Option<usize> = None;

    for (index, block) in blocks.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        match block {
            ResolvedBlock::Paragraph(runs) => emit_runs(&mut out, runs, &mut cursor_at),
            ResolvedBlock::BulletList(items) => {
                for (item_index, item) in items.iter().enumerate() {
                    if item_index > 0 {
                        out.push('\n');
                    }
                    out.push_str("• ");
                    emit_runs(&mut out, item, &mut cursor_at);
                }
            }
            ResolvedBlock::OrderedList(items) => {
                for (item_index, item) in items.iter().enumerate() {
                    if item_index > 0 {
                        out.push('\n');
                    }
                    out.push_str(&format!("{}. ", item_index + 1));
                    emit_runs(&mut out, item, &mut cursor_at);
                }
            }
        }
    }

    let total = out.chars().count();
    (out, cursor_at.map(|position| total - position))
}

fn emit_runs(out: &mut String, runs: &[Run], cursor_at: &mut Option<usize>) {
    for run in runs {
        match run {
            Run::Text { text, .. } => out.push_str(text),
            Run::LineBreak => out.push('\n'),
            Run::Cursor => *cursor_at = Some(out.chars().count()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Marks;

    fn text(t: &str) -> Run {
        Run::Text {
            text: t.into(),
            marks: Marks::default(),
        }
    }

    #[test]
    fn lists_are_prefixed() {
        let (out, cursor) = emit(&[
            ResolvedBlock::Paragraph(vec![text("Liste:")]),
            ResolvedBlock::BulletList(vec![vec![text("eins")], vec![text("zwei")]]),
            ResolvedBlock::OrderedList(vec![vec![text("erst")], vec![text("dann")]]),
        ]);
        assert_eq!(out, "Liste:\n• eins\n• zwei\n1. erst\n2. dann");
        assert_eq!(cursor, None);
    }

    #[test]
    fn cursor_counts_unicode_characters() {
        let (out, cursor) = emit(&[ResolvedBlock::Paragraph(vec![
            text("Größe: "),
            Run::Cursor,
            text(" A ✓"),
        ])]);
        assert_eq!(out, "Größe:  A ✓");
        assert_eq!(cursor, Some(4));
    }
}
