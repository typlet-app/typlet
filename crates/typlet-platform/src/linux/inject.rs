// SPDX-License-Identifier: GPL-3.0-or-later
//! Simulated key presses through a uinput virtual keyboard.

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, EventType, InputEvent, KeyCode};
use typlet_core::NewlineMode;

use super::devices::VIRTUAL_DEVICE_NAME;
use super::held::HeldKeys;
use super::keymap::{CharMap, MODIFIER_CODES};

/// Pause after every key event so compositors and apps keep up.
const KEY_DELAY: Duration = Duration::from_millis(2);

/// How long one injection waits in total for the user to let go of keys.
const RELEASE_TIMEOUT: Duration = Duration::from_secs(1);

/// Pause after the user let go of a key, so the compositor has processed the
/// release before Typlet's virtual keyboard presses anything.
const SETTLE_DELAY: Duration = Duration::from_millis(10);

pub(super) struct Injector {
    device: VirtualDevice,
    chars: Arc<CharMap>,
    held: Arc<HeldKeys>,
    caps_lock: Arc<AtomicBool>,
    /// End of the waiting time of the current injection ([`Self::begin`]).
    deadline: Instant,
}

impl Injector {
    pub(super) fn new(
        chars: Arc<CharMap>,
        held: Arc<HeldKeys>,
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
            held,
            caps_lock,
            deadline: Instant::now(),
        })
    }

    /// Starts an injection: waits until the user has released all modifier
    /// keys, so they do not combine with simulated keys (Shift + Left would
    /// select text).
    pub(super) fn begin(&mut self) {
        self.deadline = Instant::now() + RELEASE_TIMEOUT;
        if !self.wait_released(&MODIFIER_CODES.map(|code| code.0)) {
            log::debug!("modifier keys still held, typing anyway");
        }
    }

    /// Waits (within the injection's waiting time) until the user has let go
    /// of `codes`. Returns `false` if one is still held.
    fn wait_released(&self, codes: &[u16]) -> bool {
        if !codes.iter().any(|code| self.held.is_held(*code)) {
            return true;
        }
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        let released = self.held.wait_released(codes, remaining);
        if released {
            thread::sleep(SETTLE_DELAY);
        }
        released
    }

    fn key(&mut self, code: KeyCode, down: bool) -> io::Result<()> {
        if down {
            // The compositor ignores a press of a key the user still holds
            // (see `held`), so wait for the user to let go of it first.
            self.wait_released(&[code.0]);
        }
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
