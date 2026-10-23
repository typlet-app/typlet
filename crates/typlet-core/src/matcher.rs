// SPDX-License-Identifier: GPL-3.0-or-later
//! Detects typed triggers in the stream of keystrokes.
//!
//! Typlet expands **instantly**: as soon as the last character of a trigger is
//! typed, the trigger is replaced. Rules:
//!
//! * The **longest** matching trigger wins (`:fob` beats `fob`).
//! * A **backspace right after an expansion** undoes it.
//! * The platform layer sends [`Input::Reset`] whenever the typing position may
//!   have changed (mouse click, arrow keys, focus change, shortcuts). The
//!   matcher never guesses across such events.
//!
//! Privacy: the matcher keeps only the last few characters (at most the length
//! of the longest trigger) and nothing else.

use std::collections::{HashMap, VecDeque};

use crate::snippet::{Snippet, SnippetId};

/// A keystroke event relevant for matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// A character was typed.
    Char(char),
    /// Backspace was pressed.
    Backspace,
    /// The typing position may have changed; forget everything.
    Reset,
}

/// A trigger that was completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Expansion {
    /// The snippet to insert.
    pub snippet: SnippetId,
    /// How many characters of typed trigger must be deleted before inserting.
    pub trigger_chars: usize,
}

/// What the platform layer should do after an input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing to do.
    Nothing,
    /// Delete the typed trigger and insert the snippet.
    Expand(Expansion),
    /// The user pressed backspace right after this expansion: remove the
    /// inserted text and restore the trigger. The backspace itself must be
    /// swallowed (or compensated) by the platform layer.
    Undo(Expansion),
}

/// Problems between triggers that make a snippet unreachable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Conflict {
    /// Two snippets have the same trigger.
    Duplicate {
        /// The shared trigger.
        trigger: String,
        /// The snippets using it.
        snippets: [SnippetId; 2],
    },
    /// One trigger fires while another one is still being typed, so the
    /// second one can never be completed (for example `:c` and `:cc`).
    Shadowed {
        /// The snippet that fires first.
        by: SnippetId,
        /// The snippet that can never fire.
        shadowed: SnippetId,
    },
}

/// Matches typed characters against the triggers of enabled snippets.
#[derive(Debug, Default)]
pub struct Matcher {
    triggers: HashMap<String, SnippetId>,
    max_chars: usize,
    buffer: VecDeque<char>,
    last_expansion: Option<Expansion>,
}

impl Matcher {
    /// Creates a matcher for the enabled snippets in `snippets`.
    ///
    /// If several snippets share a trigger, the first one wins; use
    /// [`find_conflicts`] to report such problems to the user.
    pub fn new<'a>(snippets: impl IntoIterator<Item = &'a Snippet>) -> Self {
        let mut triggers = HashMap::new();
        for snippet in snippets.into_iter().filter(|s| s.enabled) {
            triggers
                .entry(snippet.trigger.clone())
                .or_insert(snippet.id);
        }
        let max_chars = triggers
            .keys()
            .map(|t| t.chars().count())
            .max()
            .unwrap_or(0);
        Self {
            triggers,
            max_chars,
            buffer: VecDeque::with_capacity(max_chars + 1),
            last_expansion: None,
        }
    }

    /// Feeds one input event and returns what to do.
    pub fn feed(&mut self, input: Input) -> Outcome {
        match input {
            Input::Reset => {
                self.buffer.clear();
                self.last_expansion = None;
                Outcome::Nothing
            }
            Input::Backspace => {
                if let Some(expansion) = self.last_expansion.take() {
                    return Outcome::Undo(expansion);
                }
                self.buffer.pop_back();
                Outcome::Nothing
            }
            Input::Char(c) => {
                self.last_expansion = None;
                if self.max_chars == 0 {
                    return Outcome::Nothing;
                }
                self.buffer.push_back(c);
                while self.buffer.len() > self.max_chars {
                    self.buffer.pop_front();
                }
                match self.longest_match() {
                    Some(expansion) => {
                        self.buffer.clear();
                        self.last_expansion = Some(expansion);
                        Outcome::Expand(expansion)
                    }
                    None => Outcome::Nothing,
                }
            }
        }
    }

    fn longest_match(&self) -> Option<Expansion> {
        let len = self.buffer.len();
        (1..=len).rev().find_map(|chars| {
            let suffix: String = self.buffer.iter().skip(len - chars).collect();
            self.triggers.get(&suffix).map(|&snippet| Expansion {
                snippet,
                trigger_chars: chars,
            })
        })
    }
}

/// Finds triggers that collide in instant-expansion mode.
#[must_use]
pub fn find_conflicts(snippets: &[Snippet]) -> Vec<Conflict> {
    let enabled: Vec<&Snippet> = snippets.iter().filter(|s| s.enabled).collect();
    let mut conflicts = Vec::new();

    for (i, a) in enabled.iter().enumerate() {
        for b in &enabled[i + 1..] {
            if a.trigger == b.trigger {
                conflicts.push(Conflict::Duplicate {
                    trigger: a.trigger.clone(),
                    snippets: [a.id, b.id],
                });
            }
        }
    }

    for a in &enabled {
        for b in &enabled {
            if a.id == b.id || a.trigger == b.trigger {
                continue;
            }
            // `a` fires while typing `b` if `a` occurs in `b` without its last character.
            let mut chars = b.trigger.chars();
            chars.next_back();
            if chars.as_str().contains(a.trigger.as_str()) {
                conflicts.push(Conflict::Shadowed {
                    by: a.id,
                    shadowed: b.id,
                });
            }
        }
    }
    conflicts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    fn snippet(trigger: &str) -> Snippet {
        Snippet::new(trigger, trigger, Document::default())
    }

    fn type_str(matcher: &mut Matcher, text: &str) -> Vec<Outcome> {
        text.chars()
            .map(|c| matcher.feed(Input::Char(c)))
            .filter(|o| *o != Outcome::Nothing)
            .collect()
    }

    #[test]
    fn expands_on_last_character() {
        let fob = snippet(":fob");
        let mut matcher = Matcher::new([&fob]);
        assert_eq!(
            type_str(&mut matcher, "Hallo :fob"),
            vec![Outcome::Expand(Expansion {
                snippet: fob.id,
                trigger_chars: 4
            })]
        );
    }

    #[test]
    fn longest_trigger_wins() {
        let short = snippet("fob");
        let long = snippet(":fob");
        let mut matcher = Matcher::new([&short, &long]);
        assert_eq!(
            type_str(&mut matcher, ":fob"),
            vec![Outcome::Expand(Expansion {
                snippet: long.id,
                trigger_chars: 4
            })]
        );
    }

    #[test]
    fn backspace_after_expansion_undoes_it() {
        let cc = snippet(":cc");
        let mut matcher = Matcher::new([&cc]);
        type_str(&mut matcher, ":cc");
        assert_eq!(
            matcher.feed(Input::Backspace),
            Outcome::Undo(Expansion {
                snippet: cc.id,
                trigger_chars: 3
            })
        );
        // A second backspace is an ordinary one.
        assert_eq!(matcher.feed(Input::Backspace), Outcome::Nothing);
    }

    #[test]
    fn backspace_while_typing_edits_the_buffer() {
        let cc = snippet(":cc");
        let mut matcher = Matcher::new([&cc]);
        type_str(&mut matcher, ":cx");
        matcher.feed(Input::Backspace);
        assert_eq!(type_str(&mut matcher, "c").len(), 1);
    }

    #[test]
    fn reset_forgets_partial_triggers() {
        let cc = snippet(":cc");
        let mut matcher = Matcher::new([&cc]);
        type_str(&mut matcher, ":c");
        matcher.feed(Input::Reset);
        assert!(type_str(&mut matcher, "c").is_empty());
    }

    #[test]
    fn disabled_snippets_never_fire() {
        let mut off = snippet(":off");
        off.enabled = false;
        let mut matcher = Matcher::new([&off]);
        assert!(type_str(&mut matcher, ":off").is_empty());
    }

    #[test]
    fn empty_matcher_is_inert() {
        let mut matcher = Matcher::new([]);
        assert!(type_str(&mut matcher, "anything").is_empty());
    }

    #[test]
    fn unicode_triggers() {
        let gruss = snippet(":grüß");
        let mut matcher = Matcher::new([&gruss]);
        assert_eq!(type_str(&mut matcher, ":grüß").len(), 1);
    }

    #[test]
    fn conflicts_are_detected() {
        let c = snippet(":c");
        let cc = snippet(":cc");
        let x1 = snippet(":x");
        let x2 = snippet(":x");
        let fob = snippet(":fob");
        let conflicts = find_conflicts(&[c.clone(), cc.clone(), x1.clone(), x2.clone(), fob]);
        assert!(conflicts.contains(&Conflict::Shadowed {
            by: c.id,
            shadowed: cc.id
        }));
        assert!(conflicts.contains(&Conflict::Duplicate {
            trigger: ":x".into(),
            snippets: [x1.id, x2.id]
        }));
        assert_eq!(conflicts.len(), 2);
    }

    #[test]
    fn suffix_overlap_is_not_a_conflict() {
        // Typing ":fob" ends with "fob", but the longest match wins, so both work.
        let conflicts = find_conflicts(&[snippet("fob"), snippet(":fob")]);
        assert!(conflicts.is_empty());
    }
}
