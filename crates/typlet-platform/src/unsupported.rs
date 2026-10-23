// SPDX-License-Identifier: GPL-3.0-or-later
//! Placeholder backend for operating systems that are not supported yet
//! (macOS until after V1).

use crossbeam_channel::Sender;
use typlet_core::{Action, Hotkey};

use crate::{Event, FocusedApp, InjectOptions, PlatformError, Status};

pub(crate) struct Backend;

impl Backend {
    pub(crate) fn new() -> Result<Self, PlatformError> {
        Err(PlatformError::Unsupported)
    }

    pub(crate) fn start(&mut self, _sink: Sender<Event>) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }

    pub(crate) fn set_hotkeys(&self, _hotkeys: &[Hotkey]) {}

    pub(crate) fn execute(
        &mut self,
        _actions: &[Action],
        _options: &InjectOptions,
    ) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }

    pub(crate) fn focused_app(&mut self) -> FocusedApp {
        FocusedApp::default()
    }

    pub(crate) fn clipboard_text(&mut self) -> Option<String> {
        None
    }

    pub(crate) fn copy_selection(&mut self) -> Option<String> {
        None
    }

    pub(crate) fn arm_undo(&self, _armed: bool) {}

    pub(crate) fn remember_focus(&mut self) {}

    pub(crate) fn restore_focus(&mut self) {}
}

pub(crate) fn activate_window(_handle: isize) {}

pub(crate) fn place_window(_handle: isize) {}

pub(crate) fn system_language() -> Option<String> {
    crate::language_from_env()
}

pub(crate) const SETUP_COMMANDS: Option<&str> = None;

pub(crate) fn status() -> Status {
    Status {
        problems: vec!["This operating system is not supported yet.".into()],
        ..Status::default()
    }
}
