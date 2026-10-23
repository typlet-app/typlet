// SPDX-License-Identifier: GPL-3.0-or-later
//! Turns a matched trigger and its rendered text into concrete steps for the
//! platform layer: delete the trigger, insert the text, place the cursor.
//!
//! Keeping this logic here (instead of in each platform backend) means every
//! operating system behaves the same and the behaviour is unit-tested.

use crate::render::{InsertMethod, Rendered};

/// Content placed on the clipboard for pasting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardContent {
    /// Plain text with `\n` line endings. Always present.
    pub plain: String,
    /// HTML fragment, for formatted snippets.
    pub html: Option<String>,
    /// RTF document, for formatted snippets.
    pub rtf: Option<String>,
}

/// One step executed by the platform layer, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Press Backspace this many times.
    Backspace(usize),
    /// Type plain text. `\n` stands for a line break; the platform layer
    /// presses Enter or Shift+Enter depending on the target app.
    Type(String),
    /// Save the clipboard, put this content on it, press the paste shortcut
    /// and restore the previous clipboard.
    Paste(ClipboardContent),
    /// Press the Left arrow key this many times.
    MoveLeft(usize),
    /// Press the Right arrow key this many times.
    MoveRight(usize),
}

/// What was inserted by an expansion; needed to undo it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsertedText {
    /// The trigger that was replaced.
    pub trigger: String,
    /// Number of characters inserted (`\n` counts as one).
    pub chars: usize,
    /// Characters between the final cursor position and the end of the text.
    pub cursor_from_end: usize,
    /// Undo is only reliable for plain text; formatted pastes may insert
    /// list markers or other structure that is not a character.
    pub undoable: bool,
}

/// Plans the steps for an expansion.
///
/// Returns the actions plus a record of what will be inserted, for undo.
#[must_use]
pub fn plan_expansion(
    trigger: &str,
    rendered: &Rendered,
    method: InsertMethod,
    plain_only: bool,
) -> (Vec<Action>, InsertedText) {
    let mut actions = Vec::with_capacity(3);
    let trigger_chars = trigger.chars().count();
    if trigger_chars > 0 {
        actions.push(Action::Backspace(trigger_chars));
    }

    if !rendered.plain.is_empty() {
        match method {
            InsertMethod::Type => actions.push(Action::Type(rendered.plain.clone())),
            InsertMethod::Paste => {
                let rich = rendered.formatted && !plain_only;
                actions.push(Action::Paste(ClipboardContent {
                    plain: rendered.plain.clone(),
                    html: rich.then(|| rendered.html.clone()),
                    rtf: rich.then(|| rendered.rtf.clone()),
                }));
            }
        }
    }

    let cursor_from_end = rendered.cursor_from_end.unwrap_or(0);
    if cursor_from_end > 0 {
        actions.push(Action::MoveLeft(cursor_from_end));
    }

    let inserted = InsertedText {
        trigger: trigger.to_owned(),
        chars: rendered.plain.chars().count(),
        cursor_from_end,
        undoable: !rendered.formatted || plain_only,
    };
    (actions, inserted)
}

/// Plans the steps that undo an expansion after the user pressed Backspace.
///
/// `backspace_reached_app` tells whether the user's Backspace was delivered to
/// the target application (it already deleted one character) or was
/// suppressed by the platform layer.
///
/// Returns `None` if the expansion cannot be undone reliably.
#[must_use]
pub fn plan_undo(inserted: &InsertedText, backspace_reached_app: bool) -> Option<Vec<Action>> {
    if !inserted.undoable {
        return None;
    }
    // With the cursor inside the text, the user's Backspace deleted a character
    // in the middle; there is no reliable way to restore it.
    if backspace_reached_app && inserted.cursor_from_end > 0 {
        return None;
    }

    let mut actions = Vec::with_capacity(3);
    if inserted.cursor_from_end > 0 {
        actions.push(Action::MoveRight(inserted.cursor_from_end));
    }
    let to_delete = inserted.chars - usize::from(backspace_reached_app && inserted.chars > 0);
    if to_delete > 0 {
        actions.push(Action::Backspace(to_delete));
    }
    actions.push(Action::Type(inserted.trigger.clone()));
    Some(actions)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered(plain: &str, formatted: bool, cursor: Option<usize>) -> Rendered {
        Rendered {
            plain: plain.into(),
            html: format!("<p>{plain}</p>"),
            rtf: format!("{{\\rtf1 {plain}}}"),
            formatted,
            cursor_from_end: cursor,
        }
    }

    #[test]
    fn typed_expansion() {
        let (actions, inserted) = plan_expansion(
            ":sig",
            &rendered("Max, 23.10.2026", false, None),
            InsertMethod::Type,
            false,
        );
        assert_eq!(
            actions,
            vec![Action::Backspace(4), Action::Type("Max, 23.10.2026".into())]
        );
        assert_eq!(inserted.chars, 15);
        assert!(inserted.undoable);
    }

    #[test]
    fn pasted_formatted_expansion_with_cursor() {
        let (actions, inserted) = plan_expansion(
            ":mfg",
            &rendered("Grüße\nMax", true, Some(3)),
            InsertMethod::Paste,
            false,
        );
        assert_eq!(actions.len(), 3);
        assert!(matches!(&actions[1], Action::Paste(c) if c.html.is_some() && c.rtf.is_some()));
        assert_eq!(actions[2], Action::MoveLeft(3));
        assert!(!inserted.undoable);
    }

    #[test]
    fn plain_only_apps_get_no_rich_flavours() {
        let (actions, inserted) = plan_expansion(
            ":mfg",
            &rendered("Grüße", true, None),
            InsertMethod::Paste,
            true,
        );
        assert!(matches!(&actions[1], Action::Paste(c) if c.html.is_none() && c.rtf.is_none()));
        assert!(inserted.undoable);
    }

    #[test]
    fn undo_after_backspace_reached_app() {
        let inserted = InsertedText {
            trigger: ":sig".into(),
            chars: 15,
            cursor_from_end: 0,
            undoable: true,
        };
        assert_eq!(
            plan_undo(&inserted, true),
            Some(vec![Action::Backspace(14), Action::Type(":sig".into())])
        );
        assert_eq!(
            plan_undo(&inserted, false),
            Some(vec![Action::Backspace(15), Action::Type(":sig".into())])
        );
    }

    #[test]
    fn undo_with_cursor_inside_text() {
        let inserted = InsertedText {
            trigger: ":cc".into(),
            chars: 20,
            cursor_from_end: 8,
            undoable: true,
        };
        assert_eq!(plan_undo(&inserted, true), None);
        assert_eq!(
            plan_undo(&inserted, false),
            Some(vec![
                Action::MoveRight(8),
                Action::Backspace(20),
                Action::Type(":cc".into())
            ])
        );
    }

    #[test]
    fn formatted_pastes_are_not_undone() {
        let inserted = InsertedText {
            trigger: ":mfg".into(),
            chars: 10,
            cursor_from_end: 0,
            undoable: false,
        };
        assert_eq!(plan_undo(&inserted, false), None);
    }

    #[test]
    fn empty_expansion_only_deletes_trigger() {
        let (actions, _) =
            plan_expansion(":x", &rendered("", false, None), InsertMethod::Type, false);
        assert_eq!(actions, vec![Action::Backspace(2)]);
    }
}
