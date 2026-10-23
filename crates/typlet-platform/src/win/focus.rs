// SPDX-License-Identifier: GPL-3.0-or-later
//! Focused application, password-field detection and window activation.

use windows::Win32::Foundation::{CloseHandle, HWND, POINT, RECT};
use windows::Win32::Globalization::GetUserDefaultLocaleName;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentProcessId, GetCurrentThreadId, OpenProcess, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation, IUIAutomation2};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId,
    IsIconic, SW_RESTORE, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetForegroundWindow,
    SetWindowPos, ShowWindow,
};
use windows::core::{Interface, PWSTR};

use crate::FocusedApp;

/// UI Automation must not stall the engine if the target app hangs.
const UIA_TIMEOUT_MS: u32 = 300;

pub(super) struct Focus {
    automation: Option<IUIAutomation>,
    remembered: Option<HWND>,
}

impl Focus {
    pub(super) fn new() -> Self {
        // SAFETY: COM initialisation for this (the engine) thread; creating
        // the UI Automation client object. Microsoft recommends a
        // multithreaded apartment for UI Automation clients, which also
        // needs no message loop.
        let automation = unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()
        };
        if let Some(automation) = &automation
            && let Ok(automation2) = automation.cast::<IUIAutomation2>()
        {
            // SAFETY: valid interface pointer.
            unsafe {
                let _ = automation2.SetConnectionTimeout(UIA_TIMEOUT_MS);
                let _ = automation2.SetTransactionTimeout(UIA_TIMEOUT_MS);
            }
        }
        Self {
            automation,
            remembered: None,
        }
    }

    pub(super) fn focused_app(&self) -> FocusedApp {
        // SAFETY: no preconditions.
        let window = unsafe { GetForegroundWindow() };
        FocusedApp {
            app: executable_name(window),
            password_field: self.is_password_focused(),
        }
    }

    fn is_password_focused(&self) -> bool {
        let Some(automation) = &self.automation else {
            return false;
        };
        // SAFETY: valid interface pointer; errors mean "not a password field".
        unsafe {
            automation
                .GetFocusedElement()
                .and_then(|element| element.CurrentIsPassword())
                .is_ok_and(|is_password| is_password.as_bool())
        }
    }

    pub(super) fn remember(&mut self) {
        // SAFETY: no preconditions.
        let window = unsafe { GetForegroundWindow() };
        // Opened from the tray, the taskbar is in front; use the window the
        // user last typed in instead.
        let window = if window.0.is_null() || is_own_window(window) || is_shell_window(window) {
            HWND(super::hook::last_external_window() as *mut core::ffi::c_void)
        } else {
            window
        };
        self.remembered = (!window.0.is_null()).then_some(window);
    }

    pub(super) fn restore(&mut self) {
        if let Some(window) = self.remembered.take() {
            activate(window.0 as isize);
        }
    }
}

/// Returns `true` if the window belongs to Typlet itself.
pub(super) fn is_own_window(window: HWND) -> bool {
    let mut pid = 0u32;
    // SAFETY: querying the owner of a window handle.
    unsafe {
        GetWindowThreadProcessId(window, Some(&raw mut pid));
        pid == GetCurrentProcessId()
    }
}

/// Returns `true` for the taskbar and the notification area overflow.
fn is_shell_window(window: HWND) -> bool {
    let mut buffer = [0u16; 64];
    // SAFETY: the buffer is local and its length is passed implicitly.
    let len = unsafe { GetClassNameW(window, &mut buffer) };
    let len = usize::try_from(len).unwrap_or(0).min(buffer.len());
    let class = String::from_utf16_lossy(&buffer[..len]);
    matches!(
        class.as_str(),
        "Shell_TrayWnd"
            | "Shell_SecondaryTrayWnd"
            | "NotifyIconOverflowWindow"
            | "TopLevelWindowForOverflowXamlIsland"
    )
}

fn executable_name(window: HWND) -> Option<String> {
    if window.0.is_null() {
        return None;
    }
    let mut pid = 0u32;
    // SAFETY: querying the process of a window; the handle is closed below.
    unsafe {
        GetWindowThreadProcessId(window, Some(&raw mut pid));
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 1024];
        let mut len = u32::try_from(buffer.len()).unwrap_or(0);
        let result = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &raw mut len,
        );
        let _ = CloseHandle(process);
        result.ok()?;
        let path = String::from_utf16_lossy(&buffer[..usize::try_from(len).unwrap_or(0)]);
        path.rsplit('\\').next().map(str::to_lowercase)
    }
}

/// Brings a window to the foreground. Windows only allows this for the
/// process that received the last input, so the input queues are attached
/// briefly.
pub(super) fn activate(handle: isize) {
    let window = HWND(handle as *mut core::ffi::c_void);
    if window.0.is_null() {
        return;
    }
    // SAFETY: plain window-management calls; attachment is undone.
    unsafe {
        let foreground = GetForegroundWindow();
        let foreground_thread = GetWindowThreadProcessId(foreground, None);
        let current_thread = GetCurrentThreadId();
        let attach = foreground_thread != 0 && foreground_thread != current_thread;
        if attach {
            let _ = AttachThreadInput(current_thread, foreground_thread, true);
        }
        if IsIconic(window).as_bool() {
            let _ = ShowWindow(window, SW_RESTORE);
        }
        let _ = SetForegroundWindow(window);
        if attach {
            let _ = AttachThreadInput(current_thread, foreground_thread, false);
        }
    }
}

/// Centers a window horizontally on the monitor with the mouse pointer and
/// places it in the upper third, where launchers usually appear.
pub(super) fn place(handle: isize) {
    let window = HWND(handle as *mut core::ffi::c_void);
    if window.0.is_null() {
        return;
    }
    // SAFETY: plain window-management calls with local out-parameters.
    unsafe {
        let mut cursor = POINT::default();
        if GetCursorPos(&raw mut cursor).is_err() {
            return;
        }
        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: u32::try_from(size_of::<MONITORINFO>()).unwrap_or(0),
            ..MONITORINFO::default()
        };
        if !GetMonitorInfoW(monitor, &raw mut info).as_bool() {
            return;
        }
        let mut rect = RECT::default();
        if GetWindowRect(window, &raw mut rect).is_err() {
            return;
        }
        let work = info.rcWork;
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let x = work.left + (work.right - work.left - width) / 2;
        let y = work.top + ((work.bottom - work.top - height) / 4).max(0);
        let _ = SetWindowPos(
            window,
            None,
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

/// The user's locale, for example `de-DE`.
pub(super) fn user_locale() -> Option<String> {
    let mut buffer = [0u16; 85];
    // SAFETY: the buffer is local and its length is passed implicitly.
    let len = unsafe { GetUserDefaultLocaleName(&mut buffer) };
    let len = usize::try_from(len).ok()?.checked_sub(1)?;
    Some(String::from_utf16_lossy(&buffer[..len]))
}
