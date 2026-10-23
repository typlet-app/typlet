// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows raw keyboard input: which key was pressed and which modifiers are
//! down.
//!
//! Raw input only reports key presses; it cannot change or withhold them.
//! Messages arrive shortly after the key press, so Typlet tracks the
//! modifier keys from the raw events themselves instead of asking Windows
//! for the current state, which may already have changed.
//!
//! Kept free of Windows APIs so it is unit-tested on every platform.
#![cfg_attr(not(windows), allow(dead_code))] // only the tests use it elsewhere

use crate::{Modifiers, vk};

/// `RAWKEYBOARD.Flags`: the key was released.
pub(crate) const RI_KEY_BREAK: u16 = 0x1;
/// `RAWKEYBOARD.Flags`: the scan code has the `E0` prefix (right-hand
/// modifiers, navigation keys).
pub(crate) const RI_KEY_E0: u16 = 0x2;

/// Scan code of the right Shift key.
const SCAN_RIGHT_SHIFT: u16 = 0x36;
/// `VKey` of keyboard overruns and other events without a key.
const VK_NONE: u16 = 0xFF;

/// The modifier keys whose state is tracked, left and right separately.
const TRACKED: [u16; 8] = [
    vk::LSHIFT,
    vk::RSHIFT,
    vk::LCONTROL,
    vk::RCONTROL,
    vk::LMENU,
    vk::RMENU,
    vk::LWIN,
    vk::RWIN,
];

/// One `RAWKEYBOARD` event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RawKey {
    /// Scan code.
    pub(crate) make_code: u16,
    /// `RI_KEY_*` flags.
    pub(crate) flags: u16,
    /// Virtual key; modifiers are reported without left or right.
    pub(crate) vkey: u16,
}

/// A key press or release with a left- or right-specific virtual key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyPress {
    pub(crate) vk: u16,
    pub(crate) down: bool,
}

/// What a key press means for Typlet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyAction {
    /// Modifier or lock key: nothing to do.
    Ignore,
    /// The typing position may have changed, or this is a shortcut.
    Reset,
    /// Backspace.
    Backspace,
    /// A key that may type a character.
    Translate,
}

/// Decides what a key press means. `is_hotkey` is `true` if the key with
/// these modifiers is one of Typlet's shortcuts (Windows delivers those
/// separately, and they type nothing).
pub(crate) fn classify(vk_code: u16, mods: Modifiers, is_hotkey: bool) -> KeyAction {
    if vk::is_modifier_or_lock(vk_code) {
        KeyAction::Ignore
    } else if is_hotkey || mods.ctrl || mods.alt || mods.meta || vk::resets_position(vk_code) {
        KeyAction::Reset
    } else if vk_code == vk::BACK {
        KeyAction::Backspace
    } else {
        KeyAction::Translate
    }
}

/// Modifier keys that are down and the Caps Lock state, as reported by raw
/// input.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct KeyState {
    down: [bool; TRACKED.len()],
    caps_lock: bool,
    caps_down: bool,
}

impl KeyState {
    /// Starts with all modifiers released and the given Caps Lock state.
    pub(crate) fn new(caps_lock: bool) -> Self {
        Self {
            caps_lock,
            ..Self::default()
        }
    }

    /// Applies a raw event. Returns `None` for events that are not real key
    /// presses: overruns and the "fake" Shift that Windows adds around
    /// navigation keys while Num Lock is on.
    pub(crate) fn update(&mut self, raw: RawKey) -> Option<KeyPress> {
        if raw.vkey == 0 || raw.vkey == VK_NONE {
            return None;
        }
        let e0 = raw.flags & RI_KEY_E0 != 0;
        let down = raw.flags & RI_KEY_BREAK == 0;
        let vk_code = match raw.vkey {
            vk::SHIFT if e0 => return None,
            vk::SHIFT if raw.make_code == SCAN_RIGHT_SHIFT => vk::RSHIFT,
            vk::SHIFT => vk::LSHIFT,
            vk::CONTROL if e0 => vk::RCONTROL,
            vk::CONTROL => vk::LCONTROL,
            vk::MENU if e0 => vk::RMENU,
            vk::MENU => vk::LMENU,
            other => other,
        };
        if let Some(slot) = TRACKED.iter().position(|key| *key == vk_code) {
            self.down[slot] = down;
        }
        if vk_code == vk::CAPITAL {
            // Auto-repeat sends more presses while the key is held.
            if down && !self.caps_down {
                self.caps_lock = !self.caps_lock;
            }
            self.caps_down = down;
        }
        Some(KeyPress { vk: vk_code, down })
    }

    fn is_down(&self, vk_code: u16) -> bool {
        TRACKED
            .iter()
            .position(|key| *key == vk_code)
            .is_some_and(|slot| self.down[slot])
    }

    fn any_down(&self, keys: [u16; 2]) -> bool {
        keys.iter().any(|key| self.is_down(*key))
    }

    /// The modifiers for shortcut detection. AltGr arrives as Ctrl + right
    /// Alt; it types characters (`@`, `€`), so it counts as no modifier.
    pub(crate) fn modifiers(&self) -> Modifiers {
        let altgr = self.is_down(vk::RMENU);
        Modifiers {
            ctrl: self.any_down([vk::LCONTROL, vk::RCONTROL]) && !altgr,
            alt: self.any_down([vk::LMENU, vk::RMENU]) && !altgr,
            shift: self.any_down([vk::LSHIFT, vk::RSHIFT]),
            meta: self.any_down([vk::LWIN, vk::RWIN]),
        }
    }

    /// The keyboard state for `ToUnicodeEx`: Shift, Ctrl, Alt (left, right
    /// and either) and Caps Lock.
    pub(crate) fn keyboard_state(&self) -> [u8; 256] {
        const DOWN: u8 = 0x80;
        let mut state = [0u8; 256];
        for key in TRACKED {
            if self.is_down(key) {
                state[usize::from(key)] = DOWN;
            }
        }
        for (either, keys) in [
            (vk::SHIFT, [vk::LSHIFT, vk::RSHIFT]),
            (vk::CONTROL, [vk::LCONTROL, vk::RCONTROL]),
            (vk::MENU, [vk::LMENU, vk::RMENU]),
        ] {
            if self.any_down(keys) {
                state[usize::from(either)] = DOWN;
            }
        }
        if self.caps_lock {
            state[usize::from(vk::CAPITAL)] = 0x01;
        }
        state
    }

    /// Forgets modifiers that `still_down` reports as released. Raw input
    /// misses releases while another desktop is shown (lock screen, user
    /// account control); without this, Ctrl could stay "held" for good.
    pub(crate) fn forget_released(&mut self, still_down: impl Fn(u16) -> bool) {
        for (slot, key) in TRACKED.iter().enumerate() {
            if self.down[slot] && !still_down(*key) {
                self.down[slot] = false;
            }
        }
    }

    /// `true` if any modifier is held.
    pub(crate) fn any_modifier_down(&self) -> bool {
        self.down.iter().any(|down| *down)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_A: u16 = 0x41;

    fn press(make_code: u16, vkey: u16, e0: bool) -> RawKey {
        RawKey {
            make_code,
            flags: if e0 { RI_KEY_E0 } else { 0 },
            vkey,
        }
    }

    fn release(make_code: u16, vkey: u16, e0: bool) -> RawKey {
        RawKey {
            flags: press(make_code, vkey, e0).flags | RI_KEY_BREAK,
            ..press(make_code, vkey, e0)
        }
    }

    #[test]
    fn tells_left_and_right_modifiers_apart() {
        let mut keys = KeyState::default();
        assert_eq!(
            keys.update(press(0x36, vk::SHIFT, false)),
            Some(KeyPress {
                vk: vk::RSHIFT,
                down: true
            })
        );
        assert_eq!(
            keys.update(press(0x1D, vk::CONTROL, true)).map(|k| k.vk),
            Some(vk::RCONTROL)
        );
        assert_eq!(
            keys.update(press(0x38, vk::MENU, false)).map(|k| k.vk),
            Some(vk::LMENU)
        );
        let state = keys.keyboard_state();
        assert_eq!(state[usize::from(vk::RSHIFT)], 0x80);
        assert_eq!(state[usize::from(vk::SHIFT)], 0x80);
        assert_eq!(state[usize::from(vk::LSHIFT)], 0);
        assert_eq!(state[usize::from(vk::CONTROL)], 0x80);
        assert_eq!(state[usize::from(vk::MENU)], 0x80);

        keys.update(release(0x36, vk::SHIFT, false));
        keys.update(release(0x1D, vk::CONTROL, true));
        keys.update(release(0x38, vk::MENU, false));
        assert!(!keys.any_modifier_down());
    }

    #[test]
    fn ignores_fake_shift_and_overruns() {
        let mut keys = KeyState::default();
        assert_eq!(keys.update(press(0x2A, vk::SHIFT, true)), None);
        assert_eq!(keys.update(press(0xFF, VK_NONE, false)), None);
        assert!(!keys.modifiers().shift);
    }

    #[test]
    fn altgr_types_characters_instead_of_shortcuts() {
        let mut keys = KeyState::default();
        // AltGr arrives as left Ctrl followed by right Alt.
        keys.update(press(0x1D, vk::CONTROL, false));
        keys.update(press(0x38, vk::MENU, true));
        assert_eq!(keys.modifiers(), Modifiers::default());
        let state = keys.keyboard_state();
        assert_eq!(state[usize::from(vk::CONTROL)], 0x80);
        assert_eq!(state[usize::from(vk::MENU)], 0x80);
        assert_eq!(
            classify(0x51, keys.modifiers(), false),
            KeyAction::Translate
        );
    }

    #[test]
    fn ctrl_and_win_make_shortcuts() {
        let mut keys = KeyState::default();
        keys.update(press(0x1D, vk::CONTROL, false));
        assert!(keys.modifiers().ctrl);
        assert_eq!(classify(KEY_A, keys.modifiers(), false), KeyAction::Reset);
        keys.update(release(0x1D, vk::CONTROL, false));
        keys.update(press(0x5B, vk::LWIN, true));
        assert!(keys.modifiers().meta);
        assert_eq!(classify(KEY_A, keys.modifiers(), false), KeyAction::Reset);
    }

    #[test]
    fn caps_lock_toggles_once_per_press() {
        let mut keys = KeyState::new(false);
        keys.update(press(0x3A, vk::CAPITAL, false));
        keys.update(press(0x3A, vk::CAPITAL, false)); // auto-repeat
        keys.update(release(0x3A, vk::CAPITAL, false));
        assert_eq!(keys.keyboard_state()[usize::from(vk::CAPITAL)], 0x01);
        keys.update(press(0x3A, vk::CAPITAL, false));
        keys.update(release(0x3A, vk::CAPITAL, false));
        assert_eq!(keys.keyboard_state()[usize::from(vk::CAPITAL)], 0);
        assert_eq!(
            KeyState::new(true).keyboard_state()[usize::from(vk::CAPITAL)],
            0x01
        );
    }

    #[test]
    fn forgets_releases_it_missed() {
        let mut keys = KeyState::default();
        keys.update(press(0x1D, vk::CONTROL, false));
        keys.update(press(0x2A, vk::SHIFT, false));
        // The lock screen swallowed the release of Ctrl; Shift is still down.
        keys.forget_released(|key| key == vk::LSHIFT);
        assert_eq!(
            keys.modifiers(),
            Modifiers {
                shift: true,
                ..Modifiers::default()
            }
        );
    }

    #[test]
    fn classifies_key_presses() {
        let none = Modifiers::default();
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        assert_eq!(classify(vk::LSHIFT, none, false), KeyAction::Ignore);
        assert_eq!(classify(vk::CAPITAL, none, false), KeyAction::Ignore);
        assert_eq!(classify(KEY_A, shift, false), KeyAction::Translate);
        assert_eq!(classify(vk::BACK, none, false), KeyAction::Backspace);
        assert_eq!(classify(vk::LEFT, none, false), KeyAction::Reset);
        assert_eq!(classify(vk::RETURN, shift, false), KeyAction::Reset);
        // A shortcut without Ctrl, Alt or Win (for example Shift+F2 or F9)
        // must not type anything.
        assert_eq!(classify(KEY_A, shift, true), KeyAction::Reset);
    }
}
