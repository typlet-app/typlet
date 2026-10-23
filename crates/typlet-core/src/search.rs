// SPDX-License-Identifier: GPL-3.0-or-later
//! Searching snippets by trigger, name and text, as used by the quick search
//! window and the editor.
//!
//! Matching ignores case and accents (`Grüße` matches `grusse`), every word of
//! the query must appear somewhere, and results are ranked so that trigger
//! and name matches come before matches in the text.

use crate::document::Inline;
use crate::snippet::Snippet;

/// How well a word matched; higher is better.
const TRIGGER_EXACT: u32 = 1000;
const TRIGGER_PREFIX: u32 = 700;
const LABEL_WORD_PREFIX: u32 = 500;
const TRIGGER_CONTAINS: u32 = 400;
const LABEL_CONTAINS: u32 = 300;
const TEXT_WORD_PREFIX: u32 = 150;
const TEXT_CONTAINS: u32 = 100;

/// Pre-normalised text of one snippet, so repeated searches stay fast.
#[derive(Debug, Clone)]
struct Entry {
    trigger: String,
    label: String,
    text: String,
}

/// A search index over a list of snippets.
#[derive(Debug, Clone, Default)]
pub struct SearchIndex {
    entries: Vec<Entry>,
    label_order: Vec<usize>,
}

impl SearchIndex {
    /// Builds an index. Result indices refer to positions in `snippets`.
    #[must_use]
    pub fn new(snippets: &[Snippet]) -> Self {
        let entries: Vec<Entry> = snippets
            .iter()
            .map(|snippet| Entry {
                trigger: normalise(&snippet.trigger),
                label: normalise(&snippet.label),
                text: normalise(&document_text(snippet)),
            })
            .collect();
        let mut label_order: Vec<usize> = (0..snippets.len())
            .filter(|&i| snippets[i].enabled)
            .collect();
        label_order.sort_by(|&a, &b| {
            entries[a]
                .label
                .cmp(&entries[b].label)
                .then_with(|| entries[a].trigger.cmp(&entries[b].trigger))
        });
        Self {
            entries,
            label_order,
        }
    }

    /// Returns the indices of matching snippets, best match first.
    ///
    /// An empty query returns all enabled snippets sorted by name. Disabled
    /// snippets are never returned.
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<usize> {
        let words: Vec<String> = normalise(query)
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        if words.is_empty() {
            return self.label_order.clone();
        }
        let mut hits: Vec<(u32, usize)> = self
            .label_order
            .iter()
            .filter_map(|&i| {
                let entry = &self.entries[i];
                let mut total = 0;
                for word in &words {
                    total += score(entry, word)?;
                }
                Some((total, i))
            })
            .collect();
        // Stable sort keeps the alphabetical order among equal scores.
        hits.sort_by_key(|hit| std::cmp::Reverse(hit.0));
        hits.into_iter().map(|(_, i)| i).collect()
    }
}

fn score(entry: &Entry, word: &str) -> Option<u32> {
    if entry.trigger == word {
        return Some(TRIGGER_EXACT);
    }
    // Triggers often start with a symbol (`:sig`); ignore it for prefixes.
    let bare_trigger = entry
        .trigger
        .trim_start_matches(|c: char| !c.is_alphanumeric());
    if entry.trigger.starts_with(word) || bare_trigger.starts_with(word) {
        return Some(TRIGGER_PREFIX);
    }
    if has_word_prefix(&entry.label, word) {
        return Some(LABEL_WORD_PREFIX);
    }
    if entry.trigger.contains(word) {
        return Some(TRIGGER_CONTAINS);
    }
    if entry.label.contains(word) {
        return Some(LABEL_CONTAINS);
    }
    if has_word_prefix(&entry.text, word) {
        return Some(TEXT_WORD_PREFIX);
    }
    if entry.text.contains(word) {
        return Some(TEXT_CONTAINS);
    }
    None
}

fn has_word_prefix(haystack: &str, word: &str) -> bool {
    haystack
        .split(|c: char| !c.is_alphanumeric())
        .any(|part| part.starts_with(word))
}

/// The searchable text of a snippet: its text runs, without placeholders.
fn document_text(snippet: &Snippet) -> String {
    let mut out = String::new();
    for inline in snippet.content.inlines() {
        if let Inline::Text { text, .. } = inline {
            out.push_str(text);
            out.push(' ');
        }
    }
    out
}

/// Lower-cases and removes accents so that `Grüße` and `grusse` match.
#[must_use]
pub fn normalise(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            'ß' | 'ẞ' => out.push_str("ss"),
            'æ' | 'Æ' => out.push_str("ae"),
            'œ' | 'Œ' => out.push_str("oe"),
            'ø' | 'Ø' => out.push('o'),
            'ł' | 'Ł' => out.push('l'),
            c => {
                for lower in c.to_lowercase() {
                    out.push(strip_accent(lower));
                }
            }
        }
    }
    out
}

/// Maps common accented Latin letters to their base letter.
fn strip_accent(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
        'ç' | 'ć' | 'č' => 'c',
        'ď' | 'đ' => 'd',
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ė' | 'ę' | 'ě' => 'e',
        'ğ' => 'g',
        'ì' | 'í' | 'î' | 'ï' | 'ī' | 'į' | 'ı' => 'i',
        'ñ' | 'ń' | 'ň' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ō' | 'ő' => 'o',
        'ř' => 'r',
        'ś' | 'š' | 'ş' => 's',
        'ť' | 'ţ' => 't',
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' | 'ų' => 'u',
        'ý' | 'ÿ' => 'y',
        'ź' | 'ż' | 'ž' => 'z',
        c => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    fn snippet(trigger: &str, label: &str, text: &str) -> Snippet {
        Snippet::new(trigger, label, Document::from_plain_text(text))
    }

    fn library() -> Vec<Snippet> {
        vec![
            snippet(":mfg", "Grußformel", "Mit freundlichen Grüßen"),
            snippet(":adr", "Adresse", "Musterstraße 1, 12345 Musterstadt"),
            snippet(":sig", "Signatur", "Max Mustermann"),
            snippet(":rr", "Rückruf-Notiz", "Bitte um Rückruf"),
        ]
    }

    #[test]
    fn empty_query_lists_enabled_snippets_by_name() {
        let mut snippets = library();
        snippets[2].enabled = false;
        let index = SearchIndex::new(&snippets);
        assert_eq!(index.search("  "), vec![1, 0, 3]);
    }

    #[test]
    fn ignores_case_and_accents() {
        let index = SearchIndex::new(&library());
        assert_eq!(index.search("GRUSSE"), vec![0]);
        assert_eq!(index.search("ruckruf"), vec![3]);
        assert_eq!(normalise("Grüße Æon Ł"), "grusse aeon l");
    }

    #[test]
    fn trigger_and_name_rank_before_text() {
        let snippets = vec![
            snippet(":x", "Andere", "Signatur unten"),
            snippet(":sig", "Unterschrift", "Max"),
            snippet(":y", "Signatur lang", "Max"),
        ];
        let index = SearchIndex::new(&snippets);
        // `sig` is a trigger prefix for :sig, a name prefix for "Signatur
        // lang" and only text for :x.
        assert_eq!(index.search("sig"), vec![1, 2, 0]);
        assert_eq!(index.search(":sig"), vec![1]);
    }

    #[test]
    fn every_word_must_match() {
        let index = SearchIndex::new(&library());
        assert_eq!(index.search("muster 12345"), vec![1]);
        assert!(index.search("muster gruss").is_empty());
    }
}
