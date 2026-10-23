// SPDX-License-Identifier: GPL-3.0-or-later
//! Linux backend (Wayland and X11).
//!
//! * **Input:** keyboards and pointing devices are read directly from evdev
//!   (`/dev/input/event*`). This works on every compositor, including GNOME
//!   on Wayland, and requires the user to be in the `input` group.
//! * **Translation:** key codes become characters with xkbcommon, using the
//!   user's keyboard layout and compose rules (dead keys work).
//! * **Injection:** a uinput virtual keyboard types characters that exist on
//!   the layout; anything else is pasted.
//! * **Clipboard:** through the X11 clipboard (XWayland on Wayland), which
//!   GNOME and KDE keep in sync with the Wayland clipboard.
//! * **Focus:** not available on GNOME Wayland; all apps use the default rule.

mod clipboard;
mod devices;
mod held;
mod inject;
mod keymap;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::Duration;

use crossbeam_channel::Sender;
use typlet_core::{Action, ClipboardContent, Hotkey};

use crate::{Controls, Event, FocusedApp, InjectOptions, PlatformError, Shared, Status};
use clipboard::Clipboard;
use held::HeldKeys;
use inject::Injector;
use keymap::{CharMap, Layout};

/// Shown when evdev or uinput cannot be opened.
const PERMISSION_HELP: &str = "Typlet needs access to the keyboard. Run `sudo usermod -aG input $USER` \
     and install the udev rule from the Typlet documentation, then log out and back in.";

/// One-time set-up: join the `input` group (read keyboards) and allow that
/// group to use uinput (simulate key presses). The packages install the udev
/// rule automatically; the rule file is `packaging/linux/60-typlet-uinput.rules`.
pub(crate) const SETUP_COMMANDS: Option<&str> = Some(
    "sudo usermod -aG input $USER\n\
     echo 'KERNEL==\"uinput\", GROUP=\"input\", MODE=\"0660\", OPTIONS+=\"static_node=uinput\"' \
     | sudo tee /etc/udev/rules.d/60-typlet-uinput.rules\n\
     sudo udevadm control --reload-rules && sudo udevadm trigger",
);

pub(crate) struct Backend {
    controls: Controls,
    layout: Layout,
    held_keys: Arc<HeldKeys>,
    caps_lock: Arc<AtomicBool>,
    injector: Option<Injector>,
    clipboard: Clipboard,
}

impl Backend {
    pub(crate) fn new() -> Result<Self, PlatformError> {
        let layout = Layout::detect();
        let char_map = Arc::new(CharMap::build(&layout).ok_or_else(|| {
            PlatformError::Other(format!("cannot load keyboard layout `{layout}`"))
        })?);
        let held_keys = Arc::new(HeldKeys::default());
        let caps_lock = Arc::new(AtomicBool::new(false));
        let injector = Injector::new(
            Arc::clone(&char_map),
            Arc::clone(&held_keys),
            Arc::clone(&caps_lock),
        )
        .map_err(|e| {
            log::warn!("uinput unavailable: {e}");
            PlatformError::Permission(PERMISSION_HELP.into())
        })?;
        Ok(Self {
            controls: Controls::default(),
            layout,
            held_keys,
            caps_lock,
            injector: Some(injector),
            clipboard: Clipboard::default(),
        })
    }

    pub(crate) fn start(&mut self, sink: Sender<Event>) -> Result<(), PlatformError> {
        // Check before spawning anything, so a failed start can be retried
        // without leaving a second device watcher behind.
        if devices::count_readable_keyboards() == 0 {
            return Err(PlatformError::Permission(PERMISSION_HELP.into()));
        }
        let shared = Shared::new(sink, &self.controls);
        let (raw_tx, raw_rx) = crossbeam_channel::unbounded();
        devices::spawn_watcher(raw_tx);

        // xkbcommon objects are not `Send`, so the translator is built on the
        // thread that uses it. The layout already compiled in `new()`.
        let layout = self.layout.clone();
        let held = Arc::clone(&self.held_keys);
        let caps = Arc::clone(&self.caps_lock);
        thread::Builder::new()
            .name("typlet-translate".into())
            .spawn(move || {
                let Some(mut translator) = keymap::Translator::new(&layout, shared, held, caps)
                else {
                    log::error!("cannot load keyboard layout {layout}");
                    return;
                };
                for raw in raw_rx {
                    translator.handle(raw);
                }
            })?;
        log::info!("listening with layout {}", self.layout);
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
        let injector = self
            .injector
            .as_mut()
            .ok_or_else(|| PlatformError::Permission(PERMISSION_HELP.into()))?;
        injector.begin();

        for action in actions {
            match action {
                Action::Backspace(n) => injector.tap_backspace(*n)?,
                Action::MoveLeft(n) => injector.tap_left(*n)?,
                Action::MoveRight(n) => injector.tap_right(*n)?,
                Action::Type(text) => {
                    if injector.can_type(text) {
                        injector.type_text(text, options.newline)?;
                    } else {
                        let content = ClipboardContent {
                            plain: text.clone(),
                            html: None,
                            rtf: None,
                        };
                        paste(&mut self.clipboard, injector, &content, options)?;
                    }
                }
                Action::Paste(content) => paste(&mut self.clipboard, injector, content, options)?,
            }
        }
        Ok(())
    }

    pub(crate) fn focused_app(&mut self) -> FocusedApp {
        FocusedApp::default()
    }

    pub(crate) fn clipboard_text(&mut self) -> Option<String> {
        self.clipboard.text()
    }

    pub(crate) fn copy_selection(&mut self) -> Option<String> {
        let injector = self.injector.as_mut()?;
        injector.begin();
        let saved = self.clipboard.text();
        self.clipboard.clear();
        injector.copy_shortcut().ok()?;
        thread::sleep(Duration::from_millis(200));
        let selection = self.clipboard.text();
        if let Some(saved) = saved {
            self.clipboard.set_plain(&saved);
        }
        selection.filter(|text| !text.is_empty())
    }

    pub(crate) fn arm_undo(&self, _armed: bool) {
        // evdev cannot keep a key press from reaching the application, so the
        // engine always sees `suppressed: false` on Linux.
        self.controls.arm_undo(false);
    }

    pub(crate) fn remember_focus(&mut self) {}

    pub(crate) fn restore_focus(&mut self) {}
}

fn paste(
    clipboard: &mut Clipboard,
    injector: &mut Injector,
    content: &ClipboardContent,
    options: &InjectOptions,
) -> Result<(), PlatformError> {
    let saved = clipboard.text();
    if !clipboard.set(content) {
        return Err(PlatformError::Other(
            "the clipboard is not available".into(),
        ));
    }
    thread::sleep(Duration::from_millis(60));
    injector.paste_shortcut()?;
    thread::sleep(options.restore_delay);
    match saved {
        Some(text) => clipboard.set_plain(&text),
        None => clipboard.clear(),
    }
    Ok(())
}

pub(crate) fn activate_window(_handle: isize) {}

pub(crate) fn place_window(_handle: isize) {}

pub(crate) fn system_language() -> Option<String> {
    crate::language_from_env()
}

pub(crate) fn status() -> Status {
    let keyboards = devices::count_readable_keyboards();
    let uinput = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/uinput")
        .is_ok();
    let mut problems = Vec::new();
    if keyboards == 0 {
        problems.push(format!("No keyboard can be read. {PERMISSION_HELP}"));
    }
    if !uinput {
        problems.push(format!(
            "Key presses cannot be simulated. {PERMISSION_HELP}"
        ));
    }
    Status {
        keyboard_access: keyboards > 0,
        injection_access: uinput,
        focus_detection: false,
        problems,
    }
}
