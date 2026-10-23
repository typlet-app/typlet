// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows virtual-key codes and their meaning for Typlet.
//!
//! Kept free of Windows APIs so the mapping is unit-tested on every platform.
#![cfg_attr(not(windows), allow(dead_code))] // only the tests use it elsewhere

use typlet_core::Key;

pub(crate) const BACK: u16 = 0x08;
pub(crate) const TAB: u16 = 0x09;
pub(crate) const RETURN: u16 = 0x0D;
pub(crate) const SHIFT: u16 = 0x10;
pub(crate) const CONTROL: u16 = 0x11;
pub(crate) const MENU: u16 = 0x12;
pub(crate) const CAPITAL: u16 = 0x14;
pub(crate) const ESCAPE: u16 = 0x1B;
pub(crate) const SPACE: u16 = 0x20;
pub(crate) const PRIOR: u16 = 0x21;
pub(crate) const NEXT: u16 = 0x22;
pub(crate) const END: u16 = 0x23;
pub(crate) const HOME: u16 = 0x24;
pub(crate) const LEFT: u16 = 0x25;
pub(crate) const UP: u16 = 0x26;
pub(crate) const RIGHT: u16 = 0x27;
pub(crate) const DOWN: u16 = 0x28;
pub(crate) const INSERT: u16 = 0x2D;
pub(crate) const DELETE: u16 = 0x2E;
pub(crate) const KEY_C: u16 = 0x43;
pub(crate) const KEY_V: u16 = 0x56;
pub(crate) const LWIN: u16 = 0x5B;
pub(crate) const RWIN: u16 = 0x5C;
pub(crate) const F1: u16 = 0x70;
pub(crate) const F24: u16 = 0x87;
pub(crate) const NUMLOCK: u16 = 0x90;
pub(crate) const SCROLL: u16 = 0x91;
pub(crate) const LSHIFT: u16 = 0xA0;
pub(crate) const RSHIFT: u16 = 0xA1;
pub(crate) const LCONTROL: u16 = 0xA2;
pub(crate) const RCONTROL: u16 = 0xA3;
pub(crate) const LMENU: u16 = 0xA4;
pub(crate) const RMENU: u16 = 0xA5;

/// Modifier and lock keys never produce text and never reset matching.
pub(crate) fn is_modifier_or_lock(vk: u16) -> bool {
    matches!(
        vk,
        SHIFT | CONTROL | MENU | CAPITAL | LWIN | RWIN | NUMLOCK | SCROLL | LSHIFT..=RMENU
    )
}

/// Keys that move the cursor or the focus.
pub(crate) fn resets_position(vk: u16) -> bool {
    matches!(
        vk,
        TAB | RETURN
            | ESCAPE
            | PRIOR
            | NEXT
            | END
            | HOME
            | LEFT
            | UP
            | RIGHT
            | DOWN
            | INSERT
            | DELETE
            | F1..=F24
    )
}

/// Keys that need the "extended key" flag when simulated, otherwise Windows
/// may treat them as numeric keypad keys.
pub(crate) fn is_extended(vk: u16) -> bool {
    matches!(
        vk,
        PRIOR..=DOWN | INSERT | DELETE | RCONTROL | RMENU | LWIN | RWIN
    )
}

/// Maps a virtual key to a hotkey key.
pub(crate) fn to_key(vk: u16) -> Option<Key> {
    Some(match vk {
        0x41..=0x5A => Key::Letter(char::from(u8::try_from(vk).ok()?)),
        0x30..=0x39 => Key::Digit(u8::try_from(vk - 0x30).ok()?),
        F1..=F24 => Key::Function(u8::try_from(vk - F1 + 1).ok()?),
        SPACE => Key::Space,
        RETURN => Key::Enter,
        TAB => Key::Tab,
        ESCAPE => Key::Escape,
        INSERT => Key::Insert,
        DELETE => Key::Delete,
        HOME => Key::Home,
        END => Key::End,
        PRIOR => Key::PageUp,
        NEXT => Key::PageDown,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_hotkey_keys() {
        assert_eq!(to_key(0x41), Some(Key::Letter('A')));
        assert_eq!(to_key(0x53), Some(Key::Letter('S')));
        assert_eq!(to_key(0x30), Some(Key::Digit(0)));
        assert_eq!(to_key(F1), Some(Key::Function(1)));
        assert_eq!(to_key(F24), Some(Key::Function(24)));
        assert_eq!(to_key(SPACE), Some(Key::Space));
        assert_eq!(to_key(SHIFT), None);
    }

    #[test]
    fn classifies_keys() {
        assert!(is_modifier_or_lock(LSHIFT));
        assert!(is_modifier_or_lock(RMENU));
        assert!(!is_modifier_or_lock(0x41));
        assert!(resets_position(LEFT));
        assert!(resets_position(F1 + 4));
        assert!(!resets_position(BACK));
        assert!(is_extended(DELETE));
        assert!(!is_extended(0x41));
    }
}
