// SPDX-License-Identifier: GPL-3.0-or-later
//! Keyboard shortcuts such as `Ctrl+Shift+Space`.
//!
//! Hotkeys are stored as readable strings. Parsing accepts English and German
//! key names (`Strg`, `Umschalt`, `Leertaste`, …) so users can type them the
//! way they know them.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A non-modifier key that can be part of a hotkey.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// A letter `A`–`Z` (always stored upper-case).
    Letter(char),
    /// A digit `0`–`9` on the main keyboard.
    Digit(u8),
    /// A function key `F1`–`F24`.
    Function(u8),
    /// The space bar.
    Space,
    /// Enter / Return.
    Enter,
    /// Tab.
    Tab,
    /// Escape.
    Escape,
    /// Insert.
    Insert,
    /// Delete.
    Delete,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
}

/// A key together with its modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::struct_excessive_bools)] // modifiers are independent flags
pub struct Hotkey {
    /// Control (`Strg`).
    pub ctrl: bool,
    /// Alt.
    pub alt: bool,
    /// Shift (`Umschalt`).
    pub shift: bool,
    /// Windows / Super / Command key.
    pub meta: bool,
    /// The main key.
    pub key: Key,
}

/// Why a hotkey string could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HotkeyError {
    /// The string is empty.
    #[error("the shortcut is empty")]
    Empty,
    /// A part is not a known key or modifier.
    #[error("unknown key `{0}`")]
    UnknownKey(String),
    /// No main key, only modifiers.
    #[error("the shortcut needs a key besides the modifiers")]
    MissingKey,
    /// More than one main key.
    #[error("the shortcut may contain only one key besides the modifiers")]
    TooManyKeys,
    /// Without Ctrl, Alt or the Windows key the shortcut would collide with typing.
    #[error("the shortcut needs Ctrl, Alt or the Windows key")]
    NeedsModifier,
}

impl Hotkey {
    /// Creates a hotkey with Ctrl and Shift.
    #[must_use]
    pub const fn ctrl_shift(key: Key) -> Self {
        Self {
            ctrl: true,
            alt: false,
            shift: true,
            meta: false,
            key,
        }
    }

    /// The default shortcut for the quick search: `Ctrl+Shift+Space`.
    #[must_use]
    pub const fn default_search() -> Self {
        Self::ctrl_shift(Key::Space)
    }

    /// The default shortcut for "create snippet from selection": `Ctrl+Alt+S`.
    #[must_use]
    pub const fn default_selection() -> Self {
        Self {
            ctrl: true,
            alt: true,
            shift: false,
            meta: false,
            key: Key::Letter('S'),
        }
    }
}

impl FromStr for Hotkey {
    type Err = HotkeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(HotkeyError::Empty);
        }
        let mut hotkey = Hotkey {
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
            key: Key::Space,
        };
        let mut key = None;

        for part in s.split('+').map(str::trim) {
            let lower = part.to_lowercase();
            match lower.as_str() {
                "ctrl" | "control" | "strg" => hotkey.ctrl = true,
                "alt" => hotkey.alt = true,
                "shift" | "umschalt" => hotkey.shift = true,
                "meta" | "super" | "win" | "windows" | "cmd" | "command" => hotkey.meta = true,
                _ => {
                    if key.is_some() {
                        return Err(HotkeyError::TooManyKeys);
                    }
                    key = Some(
                        parse_key(&lower)
                            .ok_or_else(|| HotkeyError::UnknownKey(part.to_owned()))?,
                    );
                }
            }
        }

        hotkey.key = key.ok_or(HotkeyError::MissingKey)?;
        let function_key = matches!(hotkey.key, Key::Function(_));
        if !function_key && !(hotkey.ctrl || hotkey.alt || hotkey.meta) {
            return Err(HotkeyError::NeedsModifier);
        }
        Ok(hotkey)
    }
}

fn parse_key(lower: &str) -> Option<Key> {
    let mut chars = lower.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if c.is_ascii_alphabetic() {
            return Some(Key::Letter(c.to_ascii_uppercase()));
        }
        if let Some(d) = c.to_digit(10) {
            #[allow(clippy::cast_possible_truncation)] // a digit always fits
            return Some(Key::Digit(d as u8));
        }
    }
    if let Some(n) = lower.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
        return (1..=24).contains(&n).then_some(Key::Function(n));
    }
    Some(match lower {
        "space" | "leertaste" | "leer" => Key::Space,
        "enter" | "return" | "eingabe" => Key::Enter,
        "tab" | "tabulator" => Key::Tab,
        "esc" | "escape" => Key::Escape,
        "insert" | "ins" | "einfg" | "einfügen" => Key::Insert,
        "delete" | "del" | "entf" => Key::Delete,
        "home" | "pos1" => Key::Home,
        "end" | "ende" => Key::End,
        "pageup" | "pgup" | "bildauf" => Key::PageUp,
        "pagedown" | "pgdn" | "bildab" => Key::PageDown,
        _ => return None,
    })
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Letter(c) => write!(f, "{c}"),
            Key::Digit(d) => write!(f, "{d}"),
            Key::Function(n) => write!(f, "F{n}"),
            Key::Space => f.write_str("Space"),
            Key::Enter => f.write_str("Enter"),
            Key::Tab => f.write_str("Tab"),
            Key::Escape => f.write_str("Esc"),
            Key::Insert => f.write_str("Insert"),
            Key::Delete => f.write_str("Delete"),
            Key::Home => f.write_str("Home"),
            Key::End => f.write_str("End"),
            Key::PageUp => f.write_str("PageUp"),
            Key::PageDown => f.write_str("PageDown"),
        }
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ctrl {
            f.write_str("Ctrl+")?;
        }
        if self.alt {
            f.write_str("Alt+")?;
        }
        if self.shift {
            f.write_str("Shift+")?;
        }
        if self.meta {
            f.write_str("Meta+")?;
        }
        write!(f, "{}", self.key)
    }
}

impl Hotkey {
    /// The hotkey with German key names as printed on German keyboards
    /// (`Strg+Umschalt+Leertaste`). [`Display`](fmt::Display) gives the
    /// English names that are also stored in files.
    #[must_use]
    pub fn german_label(&self) -> String {
        let mut out = String::new();
        for (held, name) in [
            (self.ctrl, "Strg+"),
            (self.alt, "Alt+"),
            (self.shift, "Umschalt+"),
            (self.meta, "Win+"),
        ] {
            if held {
                out.push_str(name);
            }
        }
        let key = match self.key {
            Key::Space => "Leertaste".to_owned(),
            Key::Enter => "Eingabe".to_owned(),
            Key::Delete => "Entf".to_owned(),
            Key::Insert => "Einfg".to_owned(),
            Key::Home => "Pos1".to_owned(),
            Key::End => "Ende".to_owned(),
            Key::PageUp => "Bild auf".to_owned(),
            Key::PageDown => "Bild ab".to_owned(),
            other => other.to_string(),
        };
        out.push_str(&key);
        out
    }
}

impl Serialize for Hotkey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Hotkey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn german_labels_use_keyboard_names() {
        assert_eq!(
            Hotkey::default_search().german_label(),
            "Strg+Umschalt+Leertaste"
        );
        assert_eq!(Hotkey::default_selection().german_label(), "Strg+Alt+S");
    }

    #[test]
    fn parses_english_and_german_names() {
        assert_eq!("Ctrl+Shift+Space".parse(), Ok(Hotkey::default_search()));
        assert_eq!(
            "strg + umschalt + leertaste".parse(),
            Ok(Hotkey::default_search())
        );
        assert_eq!("Ctrl+Alt+s".parse(), Ok(Hotkey::default_selection()));
    }

    #[test]
    fn display_round_trips() {
        for text in [
            "Ctrl+Shift+Space",
            "Alt+F4",
            "Ctrl+Meta+7",
            "F9",
            "Ctrl+PageDown",
        ] {
            let hotkey: Hotkey = text.parse().unwrap();
            assert_eq!(hotkey.to_string(), text);
        }
    }

    #[test]
    fn rejects_unsafe_or_broken_shortcuts() {
        assert_eq!("".parse::<Hotkey>(), Err(HotkeyError::Empty));
        assert_eq!("Shift+A".parse::<Hotkey>(), Err(HotkeyError::NeedsModifier));
        assert_eq!("Ctrl+Shift".parse::<Hotkey>(), Err(HotkeyError::MissingKey));
        assert_eq!("Ctrl+A+B".parse::<Hotkey>(), Err(HotkeyError::TooManyKeys));
        assert_eq!(
            "Ctrl+Banana".parse::<Hotkey>(),
            Err(HotkeyError::UnknownKey("Banana".into()))
        );
        assert_eq!(
            "Ctrl+F25".parse::<Hotkey>(),
            Err(HotkeyError::UnknownKey("F25".into()))
        );
    }

    #[test]
    fn serialises_as_string() {
        let json = serde_json::to_string(&Hotkey::default_search()).unwrap();
        assert_eq!(json, "\"Ctrl+Shift+Space\"");
        let back: Hotkey = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Hotkey::default_search());
    }
}
