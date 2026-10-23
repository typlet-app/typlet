// SPDX-License-Identifier: GPL-3.0-or-later
//! # typlet-platform
//!
//! Operating-system backends for Typlet. Each backend provides the same small
//! set of capabilities:
//!
//! * **listening**: turn physical key presses into [`Event`]s (characters,
//!   backspace, resets and hotkeys) on background threads,
//! * **injecting**: execute [`typlet_core::Action`]s in the focused app,
//! * **clipboard**: read text, paste rich content and restore the clipboard,
//! * **focus**: report the focused application and password fields.
//!
//! Privacy rule for every backend: typed characters are passed to the engine
//! and forgotten. They are never logged, stored or transmitted.

pub mod autostart;
pub mod instance;
pub mod paths;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;

#[cfg(any(windows, test))]
mod raw_keys;
#[cfg(any(windows, test))]
mod vk;
#[cfg(windows)]
mod win;
#[cfg(windows)]
use win as imp;

#[cfg(not(any(target_os = "linux", windows)))]
mod unsupported;
#[cfg(not(any(target_os = "linux", windows)))]
use unsupported as imp;

use std::io;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crossbeam_channel::Sender;
use typlet_core::{Action, Hotkey, Key, NewlineMode};

/// What the listening threads report to the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// A character was typed.
    Char(char),
    /// Backspace was pressed. `suppressed` is `true` if the platform kept the
    /// key from reaching the application (only possible on some platforms).
    Backspace {
        /// Whether the key press was withheld from the target application.
        suppressed: bool,
    },
    /// The typing position may have changed (click, navigation key, shortcut,
    /// focus change). The engine forgets partially typed triggers.
    Reset,
    /// A registered hotkey was pressed; the value is its index in the list
    /// passed to [`Platform::set_hotkeys`].
    Hotkey(usize),
    /// The hotkey with this index cannot be used because another program
    /// has already claimed it (Windows).
    HotkeyUnavailable(usize),
}

/// The application that currently has the keyboard focus.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FocusedApp {
    /// Executable or application id, if the platform can tell.
    pub app: Option<String>,
    /// `true` if the focused control is a password field.
    pub password_field: bool,
}

/// Options for injecting text, usually derived from an
/// [`typlet_core::EffectiveRule`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InjectOptions {
    /// How line breaks are typed.
    pub newline: NewlineMode,
    /// How long pasted content stays on the clipboard before the previous
    /// content is restored.
    pub restore_delay: Duration,
}

impl Default for InjectOptions {
    fn default() -> Self {
        Self {
            newline: NewlineMode::Enter,
            restore_delay: Duration::from_millis(u64::from(
                typlet_core::settings::DEFAULT_RESTORE_DELAY_MS,
            )),
        }
    }
}

/// Errors from the platform layer.
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    /// The current operating system is not supported yet.
    #[error("this operating system is not supported yet")]
    Unsupported,
    /// Typlet lacks a permission; the text explains how to grant it.
    #[error("missing permission: {0}")]
    Permission(String),
    /// An operating-system call failed.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// Another failure.
    #[error("{0}")]
    Other(String),
}

/// Health of the platform integration, shown in diagnostics and the setup
/// assistant.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Status {
    /// Typlet can read keyboard input.
    pub keyboard_access: bool,
    /// Typlet can simulate key presses.
    pub injection_access: bool,
    /// Typlet can detect the focused application.
    pub focus_detection: bool,
    /// Human-readable problems with instructions, in English.
    pub problems: Vec<String>,
}

/// What the engine needs from a platform. Implemented by [`Platform`] and by
/// test doubles.
pub trait Backend {
    /// Executes actions in the focused application, in order.
    ///
    /// # Errors
    ///
    /// Fails if key presses cannot be simulated or the clipboard is
    /// unavailable.
    fn execute(&mut self, actions: &[Action], options: &InjectOptions)
    -> Result<(), PlatformError>;

    /// Reports the focused application.
    fn focused_app(&mut self) -> FocusedApp;

    /// Reads the clipboard as text.
    fn clipboard_text(&mut self) -> Option<String>;

    /// Copies the current selection in the focused app (by sending the copy
    /// shortcut), returns it as text and restores the previous clipboard.
    fn copy_selection(&mut self) -> Option<String>;

    /// After an expansion, the next Backspace may be withheld from the app
    /// so that the undo can restore the trigger exactly. Windows and Linux
    /// cannot withhold keys (they only read them), so they ignore this and
    /// always report `suppressed: false`.
    fn arm_undo(&self, armed: bool);
}

/// Hotkeys, owned by the backend and shared with the listening threads.
#[derive(Debug, Clone, Default)]
pub(crate) struct Controls {
    pub(crate) hotkeys: Arc<RwLock<Vec<Hotkey>>>,
}

impl Controls {
    pub(crate) fn set_hotkeys(&self, hotkeys: &[Hotkey]) {
        if let Ok(mut current) = self.hotkeys.write() {
            *current = hotkeys.to_vec();
        }
    }
}

/// State shared between the engine and the listening threads.
#[derive(Debug)]
pub(crate) struct Shared {
    pub(crate) sink: Sender<Event>,
    pub(crate) hotkeys: Arc<RwLock<Vec<Hotkey>>>,
}

impl Shared {
    pub(crate) fn new(sink: Sender<Event>, controls: &Controls) -> Arc<Self> {
        Arc::new(Self {
            sink,
            hotkeys: Arc::clone(&controls.hotkeys),
        })
    }

    /// Sends an event; a closed engine simply means Typlet is shutting down.
    pub(crate) fn send(&self, event: Event) {
        let _ = self.sink.send(event);
    }

    /// Returns the index of the hotkey matching these modifiers and key.
    pub(crate) fn hotkey_index(&self, mods: Modifiers, key: Key) -> Option<usize> {
        let hotkeys = self.hotkeys.read().ok()?;
        find_hotkey(&hotkeys, mods, key)
    }

    /// A copy of the registered hotkeys.
    #[cfg_attr(not(windows), allow(dead_code))] // Windows registers them with the system
    pub(crate) fn hotkeys(&self) -> Vec<Hotkey> {
        self.hotkeys
            .read()
            .map(|hotkeys| hotkeys.clone())
            .unwrap_or_default()
    }
}

/// Modifier keys that are held down.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // independent flags
pub(crate) struct Modifiers {
    pub(crate) ctrl: bool,
    pub(crate) alt: bool,
    pub(crate) shift: bool,
    pub(crate) meta: bool,
}

/// Reads the language from the usual POSIX environment variables.
#[cfg_attr(windows, allow(dead_code))] // Windows asks the system instead
pub(crate) fn language_from_env() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .map(|value| {
            value
                .split(['.', '@'])
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .find(|value| !value.is_empty() && value != "C" && value != "POSIX")
}

pub(crate) fn find_hotkey(hotkeys: &[Hotkey], mods: Modifiers, key: Key) -> Option<usize> {
    hotkeys.iter().position(|h| {
        h.key == key
            && h.ctrl == mods.ctrl
            && h.alt == mods.alt
            && h.shift == mods.shift
            && h.meta == mods.meta
    })
}

/// Brings one of Typlet's own windows to the foreground. `handle` is the
/// native window handle (an `HWND` on Windows). Can be called from any
/// thread. Platforms where the window manager handles focus ignore the call.
pub fn activate_window(handle: isize) {
    imp::activate_window(handle);
}

/// Moves one of Typlet's windows to the upper middle of the screen that
/// shows the mouse pointer, like a launcher. `handle` is the native window
/// handle. Platforms where the window manager places windows ignore the call.
pub fn place_window(handle: isize) {
    imp::place_window(handle);
}

/// Shell commands that give Typlet keyboard access, for platforms that need
/// a one-time set-up (Linux). `None` where nothing has to be done.
#[must_use]
pub fn setup_commands() -> Option<&'static str> {
    imp::SETUP_COMMANDS
}

/// The user's preferred language as a tag such as `de-DE` or `en_US`, if the
/// system reports one.
#[must_use]
pub fn system_language() -> Option<String> {
    imp::system_language()
}

/// The backend for the current operating system.
pub struct Platform {
    inner: imp::Backend,
}

impl Platform {
    /// Creates the backend. Call this on the thread that will execute
    /// actions (the engine thread); some platforms bind resources to it.
    ///
    /// # Errors
    ///
    /// Fails if the operating system is unsupported or required permissions
    /// are missing.
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self {
            inner: imp::Backend::new()?,
        })
    }

    /// Starts the listening threads. Events are sent to `sink`.
    ///
    /// # Errors
    ///
    /// Fails if keyboard input cannot be read.
    pub fn start(&mut self, sink: Sender<Event>) -> Result<(), PlatformError> {
        self.inner.start(sink)
    }

    /// Replaces the registered hotkeys.
    pub fn set_hotkeys(&self, hotkeys: &[Hotkey]) {
        self.inner.set_hotkeys(hotkeys);
    }

    /// Checks permissions and capabilities without starting anything.
    #[must_use]
    pub fn status() -> Status {
        imp::status()
    }

    /// Remembers the currently focused window so it can be re-focused with
    /// [`Platform::restore_focus`] after one of Typlet's windows closes.
    pub fn remember_focus(&mut self) {
        self.inner.remember_focus();
    }

    /// Re-focuses the window remembered by [`Platform::remember_focus`].
    pub fn restore_focus(&mut self) {
        self.inner.restore_focus();
    }
}

impl Backend for Platform {
    fn execute(
        &mut self,
        actions: &[Action],
        options: &InjectOptions,
    ) -> Result<(), PlatformError> {
        self.inner.execute(actions, options)
    }

    fn focused_app(&mut self) -> FocusedApp {
        self.inner.focused_app()
    }

    fn clipboard_text(&mut self) -> Option<String> {
        self.inner.clipboard_text()
    }

    fn copy_selection(&mut self) -> Option<String> {
        self.inner.copy_selection()
    }

    fn arm_undo(&self, armed: bool) {
        self.inner.arm_undo(armed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkeys_match_exact_modifiers_only() {
        let hotkeys = [Hotkey::default_search(), Hotkey::default_selection()];
        let ctrl_shift = Modifiers {
            ctrl: true,
            shift: true,
            ..Modifiers::default()
        };
        assert_eq!(find_hotkey(&hotkeys, ctrl_shift, Key::Space), Some(0));
        assert_eq!(
            find_hotkey(
                &hotkeys,
                Modifiers {
                    ctrl: true,
                    ..Modifiers::default()
                },
                Key::Space
            ),
            None
        );
        let ctrl_alt = Modifiers {
            ctrl: true,
            alt: true,
            ..Modifiers::default()
        };
        assert_eq!(find_hotkey(&hotkeys, ctrl_alt, Key::Letter('S')), Some(1));
    }
}
