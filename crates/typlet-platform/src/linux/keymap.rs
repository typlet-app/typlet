// SPDX-License-Identifier: GPL-3.0-or-later
//! Keyboard layouts: detecting the user's layout, translating key codes into
//! characters (including dead keys via compose) and finding which key types a
//! given character.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use evdev::KeyCode;
use typlet_core::Key;
use xkbcommon::xkb::{self, Keycode, compose};

use super::devices::RawKey;
use crate::{Event, Modifiers, Shared};

/// An XKB layout selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Layout {
    pub(super) layout: String,
    pub(super) variant: String,
    pub(super) options: Option<String>,
}

impl fmt::Display for Layout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.variant.is_empty() {
            write!(f, "{}", self.layout)
        } else {
            write!(f, "{}({})", self.layout, self.variant)
        }
    }
}

impl Layout {
    fn new(layout: &str, variant: &str, options: Option<String>) -> Self {
        Self {
            layout: layout.trim().to_owned(),
            variant: variant.trim().to_owned(),
            options: options.filter(|o| !o.is_empty()),
        }
    }

    /// Detects the active layout: environment, GNOME, KDE, system default,
    /// then US as a last resort. Only the first configured layout is used.
    pub(super) fn detect() -> Self {
        Self::from_env()
            .or_else(Self::from_gnome)
            .or_else(Self::from_kde)
            .or_else(Self::from_system)
            .unwrap_or_else(|| Self::new("us", "", None))
    }

    fn from_env() -> Option<Self> {
        let layout = std::env::var("XKB_DEFAULT_LAYOUT").ok()?;
        let variant = std::env::var("XKB_DEFAULT_VARIANT").unwrap_or_default();
        let options = std::env::var("XKB_DEFAULT_OPTIONS").ok();
        Some(Self::new(first(&layout), first(&variant), options))
    }

    fn from_gnome() -> Option<Self> {
        let sources = gsettings("sources")?;
        let options = gsettings("xkb-options").map(|o| parse_gsettings_list(&o).join(","));
        let (layout, variant) = parse_gnome_sources(&sources)?;
        Some(Self::new(&layout, &variant, options))
    }

    fn from_kde() -> Option<Self> {
        let path = directories::BaseDirs::new()?.config_dir().join("kxkbrc");
        let text = fs::read_to_string(path).ok()?;
        let value = |key: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
                .map(str::to_owned)
        };
        let layout = value("LayoutList")?;
        let variant = value("VariantList").unwrap_or_default();
        let options = value("Options");
        Some(Self::new(first(&layout), first(&variant), options))
    }

    fn from_system() -> Option<Self> {
        let text = fs::read_to_string("/etc/default/keyboard").ok()?;
        parse_etc_default_keyboard(&text)
    }
}

fn first(list: &str) -> &str {
    list.split(',').next().unwrap_or_default()
}

fn gsettings(key: &str) -> Option<String> {
    let output = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.input-sources", key])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Parses `[('xkb', 'de+nodeadkeys'), ('xkb', 'us')]` into the first layout.
fn parse_gnome_sources(text: &str) -> Option<(String, String)> {
    let start = text.find("('xkb', '")? + "('xkb', '".len();
    let rest = &text[start..];
    let name = &rest[..rest.find('\'')?];
    let (layout, variant) = name.split_once('+').unwrap_or((name, ""));
    Some((layout.to_owned(), variant.to_owned()))
}

/// Parses `['compose:ralt', 'caps:escape']` (or `@as []`) into its items.
fn parse_gsettings_list(text: &str) -> Vec<String> {
    text.split('\'')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

fn parse_etc_default_keyboard(text: &str) -> Option<Layout> {
    let value = |key: &str| {
        text.lines().find_map(|line| {
            let rest = line.trim().strip_prefix(key)?.strip_prefix('=')?;
            Some(rest.trim_matches('"').to_owned())
        })
    };
    let layout = value("XKBLAYOUT").filter(|l| !l.is_empty())?;
    let variant = value("XKBVARIANT").unwrap_or_default();
    let options = value("XKBOPTIONS");
    Some(Layout::new(first(&layout), first(&variant), options))
}

fn compile(layout: &Layout) -> Option<xkb::Keymap> {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    xkb::Keymap::new_from_names(
        &context,
        "",
        "",
        layout.layout.as_str(),
        layout.variant.as_str(),
        layout.options.clone(),
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
}

fn mod_mask(keymap: &xkb::Keymap, name: &str) -> u32 {
    let index = keymap.mod_get_index(name);
    if index < 32 { 1 << index } else { 0 }
}

/// How to type one character: which key and which modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Stroke {
    pub(super) code: KeyCode,
    pub(super) shift: bool,
    pub(super) altgr: bool,
}

/// Characters that can be typed on the current layout.
#[derive(Debug, Default)]
pub(super) struct CharMap {
    strokes: HashMap<char, Stroke>,
}

impl CharMap {
    /// Builds the map for a layout. Returns `None` if the layout does not compile.
    pub(super) fn build(layout: &Layout) -> Option<Self> {
        let keymap = compile(layout)?;
        let shift = mod_mask(&keymap, xkb::MOD_NAME_SHIFT);
        let level3 = mod_mask(&keymap, xkb::MOD_NAME_ISO_LEVEL3_SHIFT);
        let allowed = shift | level3;

        let mut strokes = HashMap::new();
        for raw in keymap.min_keycode().raw()..=keymap.max_keycode().raw() {
            let Some(code) = raw.checked_sub(8).and_then(|c| u16::try_from(c).ok()) else {
                continue;
            };
            if code == 0 || code > 248 || is_keypad(code) {
                continue;
            }
            let keycode = Keycode::new(raw);
            for level in 0..keymap.num_levels_for_key(keycode, 0) {
                let [sym] = keymap.key_get_syms_by_level(keycode, 0, level) else {
                    continue;
                };
                let Some(c) = char::from_u32(xkb::keysym_to_utf32(*sym)) else {
                    continue;
                };
                if c == '\0' || c.is_control() {
                    continue;
                }
                let mut masks = [0u32; 8];
                let count = keymap.key_get_mods_for_level(keycode, 0, level, &mut masks);
                let best = masks[..count.min(masks.len())]
                    .iter()
                    .copied()
                    .filter(|mask| mask & !allowed == 0)
                    .min_by_key(|mask| mask.count_ones());
                if let Some(mask) = best {
                    strokes.entry(c).or_insert(Stroke {
                        code: KeyCode::new(code),
                        shift: mask & shift != 0,
                        altgr: mask & level3 != 0,
                    });
                }
            }
        }
        Some(Self { strokes })
    }

    pub(super) fn get(&self, c: char) -> Option<Stroke> {
        self.strokes.get(&c).copied()
    }

    pub(super) fn contains(&self, c: char) -> bool {
        self.strokes.contains_key(&c)
    }
}

fn is_keypad(code: u16) -> bool {
    matches!(code, 55 | 71..=83 | 96 | 98 | 117 | 118 | 121)
}

const MODIFIER_CODES: [KeyCode; 8] = [
    KeyCode::KEY_LEFTCTRL,
    KeyCode::KEY_RIGHTCTRL,
    KeyCode::KEY_LEFTSHIFT,
    KeyCode::KEY_RIGHTSHIFT,
    KeyCode::KEY_LEFTALT,
    KeyCode::KEY_RIGHTALT,
    KeyCode::KEY_LEFTMETA,
    KeyCode::KEY_RIGHTMETA,
];

fn modifier_slot(code: KeyCode) -> Option<usize> {
    MODIFIER_CODES.iter().position(|c| *c == code)
}

fn is_lock_key(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::KEY_CAPSLOCK | KeyCode::KEY_NUMLOCK | KeyCode::KEY_SCROLLLOCK
    )
}

fn is_pointer_button(code: u16) -> bool {
    (KeyCode::BTN_LEFT.0..=KeyCode::BTN_TASK.0).contains(&code) || code == KeyCode::BTN_TOUCH.0
}

/// Keys that move the cursor or the focus.
fn resets_position(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::KEY_ENTER
            | KeyCode::KEY_KPENTER
            | KeyCode::KEY_TAB
            | KeyCode::KEY_ESC
            | KeyCode::KEY_UP
            | KeyCode::KEY_DOWN
            | KeyCode::KEY_LEFT
            | KeyCode::KEY_RIGHT
            | KeyCode::KEY_HOME
            | KeyCode::KEY_END
            | KeyCode::KEY_PAGEUP
            | KeyCode::KEY_PAGEDOWN
            | KeyCode::KEY_INSERT
            | KeyCode::KEY_DELETE
    ) || function_number(code).is_some()
}

fn function_number(code: KeyCode) -> Option<u8> {
    let c = code.0;
    let n = match c {
        59..=68 => c - 58,    // F1–F10
        87 | 88 => c - 76,    // F11–F12
        183..=194 => c - 170, // F13–F24
        _ => return None,
    };
    u8::try_from(n).ok()
}

/// Turns raw key events into [`Event`]s. Runs on its own thread.
pub(super) struct Translator {
    keymap: xkb::Keymap,
    state: xkb::State,
    compose: Option<compose::State>,
    shared: Arc<Shared>,
    held_count: Arc<AtomicU8>,
    caps_lock: Arc<AtomicBool>,
    held: [bool; 8],
    right_alt_is_altgr: bool,
}

impl Translator {
    pub(super) fn new(
        layout: &Layout,
        shared: Arc<Shared>,
        held_count: Arc<AtomicU8>,
        caps_lock: Arc<AtomicBool>,
    ) -> Option<Self> {
        let keymap = compile(layout)?;
        let state = xkb::State::new(&keymap);
        let compose = compose_state();
        let right_alt = Keycode::new(u32::from(KeyCode::KEY_RIGHTALT.0) + 8);
        let right_alt_is_altgr = keymap
            .key_get_syms_by_level(right_alt, 0, 0)
            .iter()
            .any(|sym| matches!(sym.raw(), 0xfe03 | 0xff7e)); // ISO_Level3_Shift, Mode_switch
        Some(Self {
            keymap,
            state,
            compose,
            shared,
            held_count,
            caps_lock,
            held: [false; 8],
            right_alt_is_altgr,
        })
    }

    pub(super) fn handle(&mut self, raw: RawKey) {
        let RawKey { code, value } = raw;
        if is_pointer_button(code) {
            if value == 1 {
                self.reset();
            }
            return;
        }

        let key = KeyCode::new(code);
        let keycode = Keycode::new(u32::from(code) + 8);
        match value {
            0 => {
                self.state.update_key(keycode, xkb::KeyDirection::Up);
                self.set_held(key, false);
                return;
            }
            1 => {
                self.state.update_key(keycode, xkb::KeyDirection::Down);
                self.set_held(key, true);
            }
            2 => {}
            _ => return,
        }
        if modifier_slot(key).is_some() || is_lock_key(key) {
            return;
        }

        let mods = self.modifiers();
        if let Some(hotkey_key) = self.hotkey_key(key, keycode)
            && let Some(index) = self.shared.hotkey_index(mods, hotkey_key)
        {
            if value == 1 {
                self.shared.send(Event::Hotkey(index));
            }
            return;
        }
        if mods.ctrl || mods.alt || mods.meta {
            self.reset();
            return;
        }
        if key == KeyCode::KEY_BACKSPACE {
            if let Some(compose) = self.compose.as_mut() {
                compose.reset();
            }
            self.shared.send(Event::Backspace { suppressed: false });
            return;
        }
        if resets_position(key) {
            self.reset();
            return;
        }

        let sym = self.state.key_get_one_sym(keycode);
        if let Some(compose) = self.compose.as_mut()
            && compose.feed(sym) == compose::FeedResult::Accepted
        {
            match compose.status() {
                compose::Status::Composing => return,
                compose::Status::Composed => {
                    let text = compose.utf8().unwrap_or_default();
                    compose.reset();
                    self.emit(&text);
                    return;
                }
                compose::Status::Cancelled => {
                    compose.reset();
                    self.shared.send(Event::Reset);
                    return;
                }
                compose::Status::Nothing => {}
            }
        }
        let text = self.state.key_get_utf8(keycode);
        self.emit(&text);
    }

    fn emit(&self, text: &str) {
        for c in text.chars().filter(|c| !c.is_control()) {
            self.shared.send(Event::Char(c));
        }
    }

    fn reset(&mut self) {
        if let Some(compose) = self.compose.as_mut() {
            compose.reset();
        }
        self.shared.send(Event::Reset);
    }

    fn set_held(&mut self, key: KeyCode, down: bool) {
        if let Some(slot) = modifier_slot(key) {
            self.held[slot] = down;
            let count = self.held.iter().filter(|h| **h).count();
            self.held_count
                .store(u8::try_from(count).unwrap_or(u8::MAX), Ordering::Release);
        }
        let caps = self
            .state
            .mod_name_is_active(xkb::MOD_NAME_CAPS, xkb::STATE_MODS_LOCKED);
        self.caps_lock.store(caps, Ordering::Release);
    }

    fn modifiers(&self) -> Modifiers {
        let [lctrl, rctrl, lshift, rshift, lalt, ralt, lmeta, rmeta] = self.held;
        Modifiers {
            ctrl: lctrl || rctrl,
            alt: lalt || (ralt && !self.right_alt_is_altgr),
            shift: lshift || rshift,
            meta: lmeta || rmeta,
        }
    }

    /// The hotkey key for a key code. Letters follow the layout (so `Z` is
    /// where the user sees it); other keys use their position.
    fn hotkey_key(&self, key: KeyCode, keycode: Keycode) -> Option<Key> {
        if let Some(n) = function_number(key) {
            return Some(Key::Function(n));
        }
        let named = match key {
            KeyCode::KEY_SPACE => Some(Key::Space),
            KeyCode::KEY_ENTER | KeyCode::KEY_KPENTER => Some(Key::Enter),
            KeyCode::KEY_TAB => Some(Key::Tab),
            KeyCode::KEY_ESC => Some(Key::Escape),
            KeyCode::KEY_INSERT => Some(Key::Insert),
            KeyCode::KEY_DELETE => Some(Key::Delete),
            KeyCode::KEY_HOME => Some(Key::Home),
            KeyCode::KEY_END => Some(Key::End),
            KeyCode::KEY_PAGEUP => Some(Key::PageUp),
            KeyCode::KEY_PAGEDOWN => Some(Key::PageDown),
            _ => None,
        };
        if named.is_some() {
            return named;
        }
        if (KeyCode::KEY_1.0..=KeyCode::KEY_0.0).contains(&key.0) {
            let digit = (key.0 - KeyCode::KEY_1.0 + 1) % 10;
            return u8::try_from(digit).ok().map(Key::Digit);
        }
        let sym = *self.keymap.key_get_syms_by_level(keycode, 0, 0).first()?;
        let c = char::from_u32(xkb::keysym_to_utf32(sym))?;
        c.is_ascii_alphabetic()
            .then(|| Key::Letter(c.to_ascii_uppercase()))
    }
}

fn compose_state() -> Option<compose::State> {
    let locale: OsString = ["LC_ALL", "LC_CTYPE", "LANG"]
        .iter()
        .find_map(|var| std::env::var_os(var).filter(|v| !v.is_empty()))
        .unwrap_or_else(|| OsString::from("C"));
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let table =
        compose::Table::new_from_locale(&context, &locale, compose::COMPILE_NO_FLAGS).ok()?;
    Some(compose::State::new(&table, compose::STATE_NO_FLAGS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gnome_sources() {
        assert_eq!(
            parse_gnome_sources("[('xkb', 'de+nodeadkeys'), ('xkb', 'us')]\n"),
            Some(("de".into(), "nodeadkeys".into()))
        );
        assert_eq!(
            parse_gnome_sources("[('ibus', 'mozc-jp'), ('xkb', 'us')]"),
            Some(("us".into(), String::new()))
        );
        assert_eq!(parse_gnome_sources("@a(ss) []"), None);
    }

    #[test]
    fn parses_gsettings_lists() {
        assert_eq!(
            parse_gsettings_list("['compose:ralt', 'caps:escape']"),
            vec!["compose:ralt".to_owned(), "caps:escape".to_owned()]
        );
        assert!(parse_gsettings_list("@as []").is_empty());
    }

    #[test]
    fn parses_etc_default_keyboard() {
        let text = "XKBMODEL=\"pc105\"\nXKBLAYOUT=\"de,us\"\nXKBVARIANT=\"\"\nXKBOPTIONS=\"\"\n";
        assert_eq!(
            parse_etc_default_keyboard(text),
            Some(Layout::new("de", "", None))
        );
    }

    #[test]
    fn function_keys_are_numbered() {
        assert_eq!(function_number(KeyCode::KEY_F1), Some(1));
        assert_eq!(function_number(KeyCode::KEY_F12), Some(12));
        assert_eq!(function_number(KeyCode::KEY_F24), Some(24));
        assert_eq!(function_number(KeyCode::KEY_A), None);
    }

    #[test]
    fn german_layout_maps_special_characters() {
        let Some(map) = CharMap::build(&Layout::new("de", "", None)) else {
            // xkeyboard-config is not installed in this environment.
            return;
        };
        let at = map.get('@').expect("@ is AltGr+Q on German keyboards");
        assert_eq!(at.code, KeyCode::KEY_Q);
        assert!(at.altgr);
        let z = map.get('z').expect("z exists");
        assert_eq!(z.code, KeyCode::KEY_Y, "German keyboards swap Y and Z");
        assert!(map.get('Z').is_some_and(|s| s.shift));
        assert!(map.contains('ä'));
        assert!(!map.contains('•'));
    }
}
