// SPDX-License-Identifier: GPL-3.0-or-later
//! Snippets inside snippets.
//!
//! A snippet can insert another snippet with an [`Inline::Snippet`]
//! placeholder that names it by id, so renaming a trigger never breaks it
//! ([ADR 0016](https://github.com/typlet-app/typlet/blob/main/docs/adr/0016-snippets-inside-snippets.md)).
//! The included snippet brings its form fields along; fields with the same
//! name are asked once and share their value.

use std::collections::{HashMap, HashSet};

use crate::document::{Document, Inline};
use crate::snippet::{FieldDef, Snippet, SnippetId};

/// How deep snippets may be nested. Deeper chains are almost certainly a
/// mistake and would make the text hard to predict.
pub const MAX_DEPTH: usize = 5;

/// Why the snippets a snippet includes cannot be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NestingError {
    /// An included snippet does not exist (any more).
    #[error("an included snippet no longer exists")]
    Missing(SnippetId),
    /// Snippets include each other in a loop; the value is the trigger of
    /// the snippet that closes the loop.
    #[error("the snippet `{0}` includes itself, directly or through others")]
    Loop(String),
    /// More than [`MAX_DEPTH`] levels of snippets inside snippets.
    #[error("snippets are nested more than {MAX_DEPTH} levels deep")]
    TooDeep,
}

/// The ids of the snippets a document includes, in reading order.
pub fn included_ids(document: &Document) -> impl Iterator<Item = SnippetId> + '_ {
    document.inlines().filter_map(|inline| match inline {
        Inline::Snippet { id } => Some(*id),
        _ => None,
    })
}

/// Points included snippets to other ids, for example when an import finds
/// that an included snippet already exists under another id. Ids that are
/// not in `ids` stay as they are.
pub fn relink(document: &mut Document, ids: &HashMap<SnippetId, SnippetId>) {
    for inline in document.inlines_mut() {
        if let Inline::Snippet { id } = inline
            && let Some(new) = ids.get(id)
        {
            *id = *new;
        }
    }
}

/// Finds a snippet by id.
#[must_use]
pub fn find(snippets: &[Snippet], id: SnippetId) -> Option<&Snippet> {
    snippets.iter().find(|snippet| snippet.id == id)
}

/// Checks that every snippet `snippet` includes exists, and that there are
/// no loops and no chains deeper than [`MAX_DEPTH`].
///
/// # Errors
///
/// Returns the first problem found.
pub fn check(snippet: &Snippet, snippets: &[Snippet]) -> Result<(), NestingError> {
    fn walk(
        snippet: &Snippet,
        snippets: &[Snippet],
        stack: &mut Vec<SnippetId>,
    ) -> Result<(), NestingError> {
        for id in included_ids(&snippet.content) {
            if stack.contains(&id) || id == snippet.id {
                return Err(NestingError::Loop(snippet.trigger.clone()));
            }
            // The outer snippet is level 0; `stack.len()` is the level of `id`.
            if stack.len() > MAX_DEPTH {
                return Err(NestingError::TooDeep);
            }
            let included = find(snippets, id).ok_or(NestingError::Missing(id))?;
            stack.push(id);
            walk(included, snippets, stack)?;
            stack.pop();
        }
        Ok(())
    }
    walk(snippet, snippets, &mut vec![snippet.id])
}

/// The form fields to ask for: the snippet's own, then those of every
/// included snippet, each name once. Unknown or looping includes are
/// skipped here; rendering reports them.
#[must_use]
pub fn form_fields<'a>(snippet: &'a Snippet, snippets: &'a [Snippet]) -> Vec<&'a FieldDef> {
    let mut fields = Vec::new();
    let mut names = HashSet::new();
    let mut seen = HashSet::new();
    collect_fields(snippet, snippets, &mut fields, &mut names, &mut seen, 0);
    fields
}

fn collect_fields<'a>(
    snippet: &'a Snippet,
    snippets: &'a [Snippet],
    fields: &mut Vec<&'a FieldDef>,
    names: &mut HashSet<&'a str>,
    seen: &mut HashSet<SnippetId>,
    depth: usize,
) {
    if depth > MAX_DEPTH || !seen.insert(snippet.id) {
        return;
    }
    for field in &snippet.fields {
        if names.insert(field.name.as_str()) {
            fields.push(field);
        }
    }
    for id in included_ids(&snippet.content) {
        if let Some(included) = find(snippets, id) {
            collect_fields(included, snippets, fields, names, seen, depth + 1);
        }
    }
}

/// `true` if the snippet or anything it includes inserts the clipboard, so
/// the clipboard has to be read before rendering.
#[must_use]
pub fn uses_clipboard(snippet: &Snippet, snippets: &[Snippet]) -> bool {
    fn walk(document: &Document, snippets: &[Snippet], depth: usize) -> bool {
        document.uses_clipboard()
            || (depth < MAX_DEPTH
                && included_ids(document).any(|id| {
                    find(snippets, id).is_some_and(|s| walk(&s.content, snippets, depth + 1))
                }))
    }
    walk(&snippet.content, snippets, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Block;
    use crate::snippet::FieldKind;

    fn snippet(trigger: &str, content: Vec<Inline>) -> Snippet {
        Snippet::new(
            trigger,
            trigger,
            Document {
                blocks: vec![Block::Paragraph { content }],
            },
        )
    }

    fn include(other: &Snippet) -> Inline {
        Inline::Snippet { id: other.id }
    }

    fn field(name: &str, label: &str) -> FieldDef {
        FieldDef {
            name: name.into(),
            label: label.into(),
            kind: FieldKind::Text,
            default: None,
        }
    }

    #[test]
    fn finds_missing_includes_loops_and_deep_chains() {
        let greeting = snippet(":gruss", vec![Inline::text("Grüße")]);
        let mail = snippet(":mail", vec![Inline::text("Hallo"), include(&greeting)]);
        let library = vec![greeting.clone(), mail.clone()];
        assert_eq!(check(&mail, &library), Ok(()));
        assert_eq!(
            check(&mail, &library[1..]),
            Err(NestingError::Missing(greeting.id))
        );

        let mut a = snippet(":a", Vec::new());
        let mut b = snippet(":b", vec![include(&a)]);
        a.content = snippet(":a", vec![include(&b)]).content;
        b.content = snippet(":b", vec![include(&a)]).content;
        let library = vec![a.clone(), b.clone()];
        assert!(matches!(check(&a, &library), Err(NestingError::Loop(_))));

        let mut itself = snippet(":self", Vec::new());
        itself.content = snippet(":self", vec![include(&itself)]).content;
        assert!(matches!(
            check(&itself, std::slice::from_ref(&itself)),
            Err(NestingError::Loop(_))
        ));

        let mut chain = vec![snippet(":c0", vec![Inline::text("end")])];
        for level in 1..=MAX_DEPTH + 1 {
            let previous = &chain[level - 1];
            chain.push(snippet(&format!(":c{level}"), vec![include(previous)]));
        }
        assert_eq!(check(&chain[MAX_DEPTH], &chain), Ok(()));
        assert_eq!(
            check(&chain[MAX_DEPTH + 1], &chain),
            Err(NestingError::TooDeep)
        );
    }

    #[test]
    fn included_fields_are_asked_once() {
        let mut signature = snippet(
            ":sig",
            vec![Inline::Field {
                name: "name".into(),
            }],
        );
        signature.fields = vec![field("name", "Your name"), field("phone", "Phone")];
        let mut letter = snippet(
            ":brief",
            vec![
                Inline::Field {
                    name: "name".into(),
                },
                include(&signature),
                include(&signature),
            ],
        );
        letter.fields = vec![field("name", "Name")];
        let library = vec![signature.clone(), letter.clone()];

        let names: Vec<(&str, &str)> = form_fields(&letter, &library)
            .into_iter()
            .map(|f| (f.name.as_str(), f.label.as_str()))
            .collect();
        assert_eq!(names, vec![("name", "Name"), ("phone", "Phone")]);
    }

    #[test]
    fn relinks_included_snippets() {
        let old = snippet(":old", Vec::new());
        let new = snippet(":new", Vec::new());
        let other = snippet(":other", Vec::new());
        let mut document = Document {
            blocks: vec![Block::BulletList {
                items: vec![vec![include(&old)], vec![include(&other)]],
            }],
        };
        relink(&mut document, &HashMap::from([(old.id, new.id)]));
        assert_eq!(
            included_ids(&document).collect::<Vec<_>>(),
            vec![new.id, other.id]
        );
    }

    #[test]
    fn clipboard_use_is_inherited() {
        let paste = snippet(":clip", vec![Inline::Clipboard]);
        let outer = snippet(":outer", vec![include(&paste)]);
        assert!(uses_clipboard(&outer, &[paste.clone(), outer.clone()]));
        assert!(!uses_clipboard(&outer, &[]));
    }
}
