// SPDX-License-Identifier: GPL-3.0-or-later
//! Simulated key presses with `SendInput`.

use std::mem::size_of;
use std::thread;
use std::time::{Duration, Instant};

use typlet_core::NewlineMode;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, SendInput, VIRTUAL_KEY,
};

use crate::PlatformError;
use crate::vk;

/// Inputs sent per `SendInput` call; small batches keep slow apps in sync.
const BATCH: usize = 64;

fn key_input(vk_code: u16, up: bool) -> INPUT {
    let mut flags = KEYBD_EVENT_FLAGS(0);
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    if vk::is_extended(vk_code) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk_code),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn unicode_input(unit: u16, up: bool) -> INPUT {
    let mut flags = KEYEVENTF_UNICODE;
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: unit,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send(inputs: &[INPUT]) -> Result<(), PlatformError> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // INPUT is small
    let size = size_of::<INPUT>() as i32;
    for chunk in inputs.chunks(BATCH) {
        // SAFETY: the slice contains initialised INPUT structures.
        let sent = unsafe { SendInput(chunk, size) };
        if usize::try_from(sent).unwrap_or(0) != chunk.len() {
            return Err(PlatformError::Other(
                "Windows blocked the simulated input (is the target app running as administrator?)"
                    .into(),
            ));
        }
        if inputs.len() > BATCH {
            thread::sleep(Duration::from_millis(5));
        }
    }
    Ok(())
}

/// Presses a key `times` times.
pub(super) fn tap(vk_code: u16, times: usize) -> Result<(), PlatformError> {
    let mut inputs = Vec::with_capacity(times * 2);
    for _ in 0..times {
        inputs.push(key_input(vk_code, false));
        inputs.push(key_input(vk_code, true));
    }
    send(&inputs)
}

/// Presses Ctrl plus a key, for example Ctrl+V.
pub(super) fn ctrl(vk_code: u16) -> Result<(), PlatformError> {
    send(&[
        key_input(vk::CONTROL, false),
        key_input(vk_code, false),
        key_input(vk_code, true),
        key_input(vk::CONTROL, true),
    ])
}

/// Types text with Unicode key events.
pub(super) fn type_text(text: &str, newline: NewlineMode) -> Result<(), PlatformError> {
    let mut inputs = Vec::with_capacity(text.len() * 2);
    for c in text.chars() {
        match c {
            '\r' => {}
            '\n' => {
                if newline == NewlineMode::ShiftEnter {
                    inputs.push(key_input(vk::SHIFT, false));
                }
                inputs.push(key_input(vk::RETURN, false));
                inputs.push(key_input(vk::RETURN, true));
                if newline == NewlineMode::ShiftEnter {
                    inputs.push(key_input(vk::SHIFT, true));
                }
            }
            '\t' => {
                inputs.push(key_input(vk::TAB, false));
                inputs.push(key_input(vk::TAB, true));
            }
            c => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    inputs.push(unicode_input(*unit, false));
                    inputs.push(unicode_input(*unit, true));
                }
            }
        }
    }
    send(&inputs)
}

/// Waits until the user releases modifier keys, so they do not combine with
/// simulated keys (Shift + Left would select text).
pub(super) fn wait_for_modifier_release(timeout: Duration) {
    let start = Instant::now();
    let held = || {
        [vk::SHIFT, vk::CONTROL, vk::MENU, vk::LWIN, vk::RWIN]
            .iter()
            // SAFETY: no preconditions.
            .any(|key| unsafe { GetAsyncKeyState(i32::from(*key)) } < 0)
    };
    while held() && start.elapsed() < timeout {
        thread::sleep(Duration::from_millis(10));
    }
}
