// SPDX-License-Identifier: GPL-3.0-or-later
//! Simulated key presses through a uinput virtual keyboard.

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, EventType, InputEvent, KeyCode};
use typlet_core::NewlineMode;

use super::devices::VIRTUAL_DEVICE_NAME;
use super::keymap::CharMap;

/// Pause after every key event so compositors and apps keep up.
const KEY_DELAY: Duration = Duration::from_millis(2);

pub(super) struct Injector {
    device: VirtualDevice,
    chars: Arc<CharMap>,
    held_count: Arc<AtomicU8>,
    caps_lock: Arc<AtomicBool>,
}

impl Injector {
    pub(super) fn new(
        chars: Arc<CharMap>,
        held_count: Arc<AtomicU8>,
        caps_lock: Arc<AtomicBool>,
    ) -> io::Result<Self> {
        let mut keys = AttributeSet::<KeyCode>::new();
        for code in 1..=248 {
            keys.insert(KeyCode::new(code));
        }
        let device = VirtualDevice::builder()?
            .name(VIRTUAL_DEVICE_NAME)
            .with_keys(&keys)?
            .build()?;
        // Give the compositor time to register the new device.
        thread::sleep(Duration::from_millis(300));
        Ok(Self {
            device,
            chars,
            held_count,
            caps_lock,
        })
    }

    /// Waits until the user has released all modifier keys, so they do not
    /// combine with simulated keys (Shift + Left would select text).
    pub(super) fn wait_for_modifier_release(&self, timeout: Duration) {
        let start = Instant::now();
        while self.held_count.load(Ordering::Acquire) > 0 && start.elapsed() < timeout {
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn key(&mut self, code: KeyCode, down: bool) -> io::Result<()> {
        self.device
            .emit(&[InputEvent::new(EventType::KEY.0, code.0, i32::from(down))])?;
        thread::sleep(KEY_DELAY);
        Ok(())
    }

    fn tap(&mut self, code: KeyCode) -> io::Result<()> {
        self.key(code, true)?;
        self.key(code, false)
    }

    fn chord(&mut self, modifiers: &[KeyCode], code: KeyCode) -> io::Result<()> {
        for m in modifiers {
            self.key(*m, true)?;
        }
        let result = self.tap(code);
        for m in modifiers.iter().rev() {
            self.key(*m, false)?;
        }
        result
    }

    fn repeat(&mut self, code: KeyCode, times: usize) -> io::Result<()> {
        for _ in 0..times {
            self.tap(code)?;
        }
        Ok(())
    }

    pub(super) fn tap_backspace(&mut self, times: usize) -> io::Result<()> {
        self.repeat(KeyCode::KEY_BACKSPACE, times)
    }

    pub(super) fn tap_left(&mut self, times: usize) -> io::Result<()> {
        self.repeat(KeyCode::KEY_LEFT, times)
    }

    pub(super) fn tap_right(&mut self, times: usize) -> io::Result<()> {
        self.repeat(KeyCode::KEY_RIGHT, times)
    }

    /// `true` if every character exists on the layout.
    pub(super) fn can_type(&self, text: &str) -> bool {
        text.chars()
            .all(|c| matches!(c, '\n' | '\r' | '\t') || self.chars.contains(c))
    }

    pub(super) fn type_text(&mut self, text: &str, newline: NewlineMode) -> io::Result<()> {
        for c in text.chars() {
            match c {
                '\r' => {}
                '\n' => match newline {
                    NewlineMode::Enter => self.tap(KeyCode::KEY_ENTER)?,
                    NewlineMode::ShiftEnter => {
                        self.chord(&[KeyCode::KEY_LEFTSHIFT], KeyCode::KEY_ENTER)?;
                    }
                },
                '\t' => self.tap(KeyCode::KEY_TAB)?,
                c => self.type_char(c)?,
            }
        }
        Ok(())
    }

    fn type_char(&mut self, c: char) -> io::Result<()> {
        let Some(stroke) = self.chars.get(c) else {
            return Ok(());
        };
        let caps = self.caps_lock.load(Ordering::Acquire) && c.is_alphabetic();
        let shift = stroke.shift != caps;
        let mut modifiers = Vec::with_capacity(2);
        if stroke.altgr {
            modifiers.push(KeyCode::KEY_RIGHTALT);
        }
        if shift {
            modifiers.push(KeyCode::KEY_LEFTSHIFT);
        }
        self.chord(&modifiers, stroke.code)
    }

    /// Ctrl plus the key that produces `letter` on this layout.
    fn ctrl_letter(&mut self, letter: char) -> io::Result<()> {
        let code = self
            .chars
            .get(letter)
            .filter(|s| !s.shift && !s.altgr)
            .map_or(
                if letter == 'v' {
                    KeyCode::KEY_V
                } else {
                    KeyCode::KEY_C
                },
                |s| s.code,
            );
        self.chord(&[KeyCode::KEY_LEFTCTRL], code)
    }

    pub(super) fn paste_shortcut(&mut self) -> io::Result<()> {
        self.ctrl_letter('v')
    }

    pub(super) fn copy_shortcut(&mut self) -> io::Result<()> {
        self.ctrl_letter('c')
    }
}
