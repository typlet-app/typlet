// SPDX-License-Identifier: GPL-3.0-or-later
//! Low-level keyboard and mouse hooks.
//!
//! Hook callbacks run on the hook thread for every key press system-wide and
//! must return quickly. They read the key, translate it and push an event
//! into a channel; all real work happens on the engine thread.

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;

use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, GetKeyboardLayout, ToUnicodeEx,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
    HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, SetWindowsHookExW, TranslateMessage,
    WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_RBUTTONDOWN,
    WM_SYSKEYDOWN, WM_XBUTTONDOWN,
};

use crate::vk;
use crate::{Event, Modifiers, PlatformError, Shared};

/// Keep `ToUnicodeEx` from changing the keyboard state, so dead keys still
/// compose in the target application (Windows 10 1607 and later).
const TO_UNICODE_KEEP_STATE: u32 = 0x4;

struct HookContext {
    shared: Arc<Shared>,
    last_foreground: AtomicIsize,
    dead_key_pending: AtomicBool,
}

static CONTEXT: OnceLock<HookContext> = OnceLock::new();

/// The last window of another program the user typed in. Used to return
/// the focus when a Typlet window was opened from the tray.
static LAST_EXTERNAL: AtomicIsize = AtomicIsize::new(0);

/// Returns the last window of another program that received key presses.
pub(super) fn last_external_window() -> isize {
    LAST_EXTERNAL.load(Ordering::Acquire)
}

/// Installs the hooks on a new thread with its own message loop.
pub(super) fn start(shared: Arc<Shared>) -> Result<(), PlatformError> {
    CONTEXT
        .set(HookContext {
            shared,
            last_foreground: AtomicIsize::new(0),
            dead_key_pending: AtomicBool::new(false),
        })
        .map_err(|_| PlatformError::Other("keyboard hook already installed".into()))?;

    let (ready_tx, ready_rx) = crossbeam_channel::bounded(1);
    thread::Builder::new()
        .name("typlet-hook".into())
        .spawn(move || run_hook_thread(&ready_tx))?;

    match ready_rx.recv() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(message)) => Err(PlatformError::Other(message)),
        Err(_) => Err(PlatformError::Other("keyboard hook thread ended".into())),
    }
}

fn run_hook_thread(ready: &crossbeam_channel::Sender<Result<(), String>>) {
    // SAFETY: plain Win32 calls on this thread; the hook procedures are
    // `extern "system"` functions with the documented signature.
    unsafe {
        let module = GetModuleHandleW(None).ok().map(|m| HINSTANCE(m.0));
        let keyboard = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0);
        let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0);
        if let Err(e) = &keyboard {
            let _ = ready.send(Err(format!("cannot install keyboard hook: {e}")));
            return;
        }
        if let Err(e) = &mouse {
            log::warn!("cannot install mouse hook: {e}");
        }
        let _ = ready.send(Ok(()));

        let mut message = MSG::default();
        while GetMessageW(&raw mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&raw const message);
            DispatchMessageW(&raw const message);
        }
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32
        && let Some(context) = CONTEXT.get()
    {
        // SAFETY: for WH_KEYBOARD_LL, lparam points to a KBDLLHOOKSTRUCT.
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let message = u32::try_from(wparam.0).unwrap_or_default();
        let injected = info.flags.0 & LLKHF_INJECTED.0 != 0;
        if !injected
            && (message == WM_KEYDOWN || message == WM_SYSKEYDOWN)
            && context.key_down(info.vkCode, info.scanCode)
        {
            return LRESULT(1);
        }
    }
    // SAFETY: passing the unmodified arguments to the next hook.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32
        && let Some(context) = CONTEXT.get()
    {
        let message = u32::try_from(wparam.0).unwrap_or_default();
        if matches!(
            message,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        ) {
            context.shared.take_undo_armed();
            context.shared.send(Event::Reset);
        }
    }
    // SAFETY: passing the unmodified arguments to the next hook.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn is_down(key: u16) -> bool {
    // SAFETY: GetAsyncKeyState has no preconditions.
    let state = unsafe { GetAsyncKeyState(i32::from(key)) };
    state < 0
}

impl HookContext {
    /// Handles a key press. Returns `true` to keep it from the application.
    fn key_down(&self, vk_code: u32, scan_code: u32) -> bool {
        // SAFETY: no preconditions.
        let foreground = unsafe { GetForegroundWindow() };
        let id = foreground.0 as isize;
        if self.last_foreground.swap(id, Ordering::AcqRel) != id {
            self.shared.send(Event::Reset);
            if !super::focus::is_own_window(foreground) {
                LAST_EXTERNAL.store(id, Ordering::Release);
            }
        }

        let Ok(vk_code16) = u16::try_from(vk_code) else {
            return false;
        };
        if vk::is_modifier_or_lock(vk_code16) {
            return false;
        }

        // AltGr is reported as Ctrl+Alt; it types characters, not shortcuts.
        let altgr = is_down(vk::RMENU);
        let mods = Modifiers {
            ctrl: is_down(vk::CONTROL) && !altgr,
            alt: is_down(vk::MENU) && !altgr,
            shift: is_down(vk::SHIFT),
            meta: is_down(vk::LWIN) || is_down(vk::RWIN),
        };

        if let Some(key) = vk::to_key(vk_code16)
            && let Some(index) = self.shared.hotkey_index(mods, key)
        {
            self.shared.take_undo_armed();
            self.shared.send(Event::Hotkey(index));
            return true;
        }
        if mods.ctrl || mods.alt || mods.meta {
            self.shared.take_undo_armed();
            self.shared.send(Event::Reset);
            return false;
        }
        if vk_code16 == vk::BACK {
            let suppressed = self.shared.take_undo_armed();
            self.shared.send(Event::Backspace { suppressed });
            return suppressed;
        }
        // Undo is only offered for a Backspace directly after an expansion.
        self.shared.take_undo_armed();
        if vk::resets_position(vk_code16) {
            self.shared.send(Event::Reset);
            return false;
        }

        self.translate(vk_code, scan_code, foreground);
        false
    }

    fn translate(
        &self,
        vk_code: u32,
        scan_code: u32,
        foreground: windows::Win32::Foundation::HWND,
    ) {
        let mut state = [0u8; 256];
        for key in [
            vk::SHIFT,
            vk::LSHIFT,
            vk::RSHIFT,
            vk::CONTROL,
            vk::LCONTROL,
            vk::RCONTROL,
            vk::MENU,
            vk::LMENU,
            vk::RMENU,
        ] {
            if is_down(key) {
                state[usize::from(key)] = 0x80;
            }
        }
        // SAFETY: no preconditions.
        if unsafe { GetKeyState(i32::from(vk::CAPITAL)) } & 1 != 0 {
            state[usize::from(vk::CAPITAL)] = 0x01;
        }

        let mut buffer = [0u16; 8];
        // SAFETY: the keyboard layout of the foreground thread; buffers are
        // local and sized as passed.
        let count = unsafe {
            let thread = GetWindowThreadProcessId(foreground, None);
            let layout = GetKeyboardLayout(thread);
            ToUnicodeEx(
                vk_code,
                scan_code,
                &state,
                &mut buffer,
                TO_UNICODE_KEEP_STATE,
                Some(layout),
            )
        };

        if count < 0 {
            // A dead key; the next key composes a character we cannot know.
            self.dead_key_pending.store(true, Ordering::Release);
            self.shared.send(Event::Reset);
            return;
        }
        if count == 0 {
            return;
        }
        if self.dead_key_pending.swap(false, Ordering::AcqRel) {
            self.shared.send(Event::Reset);
            return;
        }
        let len = usize::try_from(count).unwrap_or(0).min(buffer.len());
        for c in char::decode_utf16(buffer[..len].iter().copied()).flatten() {
            if !c.is_control() {
                self.shared.send(Event::Char(c));
            }
        }
    }
}
