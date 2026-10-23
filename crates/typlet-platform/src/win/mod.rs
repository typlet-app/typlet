// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows backend.
//!
//! * **Input:** low-level keyboard and mouse hooks on a dedicated thread. The
//!   hook callbacks only translate and enqueue; Windows removes hooks that
//!   answer too slowly.
//! * **Translation:** `ToUnicodeEx` with the "do not change keyboard state"
//!   flag, so the user's dead keys (`^`, `´`) keep working in the target app.
//! * **Injection:** `SendInput` with Unicode key events, independent of the
//!   keyboard layout. Injected events are ignored by the hook.
//! * **Clipboard:** `CF_UNICODETEXT`, `HTML Format` and `Rich Text Format`,
//!   marked as transient for clipboard history; all previous formats are
//!   restored afterwards.
//! * **Focus:** executable name of the foreground window; password fields via
//!   UI Automation.

mod clipboard;
mod focus;
mod hook;
mod inject;

use std::thread;
use std::time::Duration;

use crossbeam_channel::Sender;
use typlet_core::{Action, ClipboardContent, Hotkey};

use crate::{Controls, Event, FocusedApp, InjectOptions, PlatformError, Shared, Status};
use clipboard::Clipboard;
use focus::Focus;

pub(crate) struct Backend {
    controls: Controls,
    clipboard: Clipboard,
    focus: Focus,
    started: bool,
}

impl Backend {
    pub(crate) fn new() -> Result<Self, PlatformError> {
        Ok(Self {
            controls: Controls::default(),
            clipboard: Clipboard::new(),
            focus: Focus::new(),
            started: false,
        })
    }

    pub(crate) fn start(&mut self, sink: Sender<Event>) -> Result<(), PlatformError> {
        if self.started {
            return Ok(());
        }
        let shared = Shared::new(sink, &self.controls);
        hook::start(shared)?;
        self.started = true;
        Ok(())
    }

    pub(crate) fn set_hotkeys(&self, hotkeys: &[Hotkey]) {
        self.controls.set_hotkeys(hotkeys);
    }

    pub(crate) fn execute(
        &mut self,
        actions: &[Action],
        options: &InjectOptions,
    ) -> Result<(), PlatformError> {
        inject::wait_for_modifier_release(Duration::from_secs(1));
        for action in actions {
            match action {
                Action::Backspace(n) => inject::tap(crate::vk::BACK, *n)?,
                Action::MoveLeft(n) => inject::tap(crate::vk::LEFT, *n)?,
                Action::MoveRight(n) => inject::tap(crate::vk::RIGHT, *n)?,
                Action::Type(text) => inject::type_text(text, options.newline)?,
                Action::Paste(content) => self.paste(content, options)?,
            }
        }
        Ok(())
    }

    fn paste(
        &mut self,
        content: &ClipboardContent,
        options: &InjectOptions,
    ) -> Result<(), PlatformError> {
        let saved = self.clipboard.save();
        let sequence = self.clipboard.set(content)?;
        thread::sleep(Duration::from_millis(30));
        inject::ctrl(crate::vk::KEY_V)?;
        thread::sleep(options.restore_delay);
        // Only restore if nobody else changed the clipboard in the meantime.
        if self.clipboard.sequence() == sequence {
            self.clipboard.restore(&saved);
        }
        Ok(())
    }

    pub(crate) fn focused_app(&mut self) -> FocusedApp {
        self.focus.focused_app()
    }

    pub(crate) fn clipboard_text(&mut self) -> Option<String> {
        self.clipboard.text()
    }

    pub(crate) fn copy_selection(&mut self) -> Option<String> {
        inject::wait_for_modifier_release(Duration::from_secs(1));
        let saved = self.clipboard.save();
        let before = self.clipboard.sequence();
        inject::ctrl(crate::vk::KEY_C).ok()?;
        // Wait until the application has put the selection on the clipboard.
        for _ in 0..25 {
            thread::sleep(Duration::from_millis(20));
            if self.clipboard.sequence() != before {
                break;
            }
        }
        let changed = self.clipboard.sequence() != before;
        let text = if changed { self.clipboard.text() } else { None };
        if changed {
            self.clipboard.restore(&saved);
        }
        text.filter(|t| !t.is_empty())
    }

    pub(crate) fn arm_undo(&self, armed: bool) {
        self.controls.arm_undo(armed);
    }

    pub(crate) fn remember_focus(&mut self) {
        self.focus.remember();
    }

    pub(crate) fn restore_focus(&mut self) {
        self.focus.restore();
    }
}

pub(crate) fn activate_window(handle: isize) {
    focus::activate(handle);
}

pub(crate) fn place_window(handle: isize) {
    focus::place(handle);
}

pub(crate) fn system_language() -> Option<String> {
    focus::user_locale()
}

pub(crate) const SETUP_COMMANDS: Option<&str> = None;

pub(crate) fn status() -> Status {
    Status {
        keyboard_access: true,
        injection_access: true,
        focus_detection: true,
        problems: Vec::new(),
    }
}
