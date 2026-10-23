// SPDX-License-Identifier: GPL-3.0-or-later
//! Keyboard and mouse input through Raw Input, shortcuts through
//! `RegisterHotKey` (ADR 0014 in `docs/adr`).
//!
//! A hidden window on its own thread receives a copy of every key press and
//! mouse click (`RIDEV_INPUTSINK`), also while other programs have the
//! focus. Raw input only reads: it cannot change or withhold a key press.
//! Typlet's shortcuts are therefore registered with Windows, which reports
//! them as `WM_HOTKEY` and keeps them from the focused app.
//!
//! Raw input is registered once per process, and the Slint interface
//! (winit) registers it as well. A timer checks every second that the
//! registration still points at this window.

use std::cell::RefCell;
use std::mem::size_of;
use std::sync::Arc;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use typlet_core::Hotkey;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, GetKeyboardLayout, HOT_KEY_MODIFIERS, RegisterHotKey,
    ToUnicodeEx, UnregisterHotKey,
};
use windows::Win32::UI::Input::{
    GetRawInputData, GetRegisteredRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE,
    RAWINPUTHEADER, RID_INPUT, RIDEV_INPUTSINK, RIM_TYPEKEYBOARD, RIM_TYPEMOUSE,
    RegisterRawInputDevices,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetForegroundWindow, GetMessageW,
    GetWindowThreadProcessId, MSG, PostMessageW, RI_MOUSE_BUTTON_4_DOWN, RI_MOUSE_BUTTON_5_DOWN,
    RI_MOUSE_LEFT_BUTTON_DOWN, RI_MOUSE_MIDDLE_BUTTON_DOWN, RI_MOUSE_RIGHT_BUTTON_DOWN,
    RegisterClassW, SetTimer, TranslateMessage, WM_APP, WM_HOTKEY, WM_INPUT, WM_TIMER, WNDCLASSW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_OVERLAPPED,
};
use windows::core::w;

use crate::raw_keys::{KeyAction, KeyState, RawKey, classify};
use crate::{Event, PlatformError, Shared, vk};

/// Keep `ToUnicodeEx` from changing the keyboard state, so dead keys still
/// compose in the target application (Windows 10 1607 and later).
const TO_UNICODE_KEEP_STATE: u32 = 0x4;

/// HID usage page and usages of keyboards and mice.
const USAGE_PAGE_GENERIC: u16 = 0x01;
const USAGE_MOUSE: u16 = 0x02;
const USAGE_KEYBOARD: u16 = 0x06;

/// Posted to the input window when the hotkeys changed.
const UPDATE_HOTKEYS: u32 = WM_APP + 1;

const MAINTENANCE_TIMER: usize = 1;
const MAINTENANCE_INTERVAL_MS: u32 = 1000;
/// Shortcuts another program owned are tried again this often (in ticks).
const HOTKEY_RETRY_TICKS: u32 = 10;
/// Modifiers still "held" after this long without key events are checked
/// against the real keyboard state.
const STALE_MODIFIERS: Duration = Duration::from_millis(500);

const MOUSE_BUTTON_DOWN: u32 = RI_MOUSE_LEFT_BUTTON_DOWN
    | RI_MOUSE_RIGHT_BUTTON_DOWN
    | RI_MOUSE_MIDDLE_BUTTON_DOWN
    | RI_MOUSE_BUTTON_4_DOWN
    | RI_MOUSE_BUTTON_5_DOWN;

/// The hidden input window, once it exists.
static WINDOW: AtomicIsize = AtomicIsize::new(0);

/// The last window of another program the user typed in. Used to return
/// the focus when a Typlet window was opened from the tray.
static LAST_EXTERNAL: AtomicIsize = AtomicIsize::new(0);

thread_local! {
    static STATE: RefCell<Option<InputThread>> = const { RefCell::new(None) };
}

/// Returns the last window of another program that received key presses.
pub(super) fn last_external_window() -> isize {
    LAST_EXTERNAL.load(Ordering::Acquire)
}

/// Starts the input thread with its hidden window and message loop.
pub(super) fn start(shared: Arc<Shared>) -> Result<(), PlatformError> {
    let (ready_tx, ready_rx) = crossbeam_channel::bounded(1);
    thread::Builder::new()
        .name("typlet-input".into())
        .spawn(move || run(shared, &ready_tx))?;

    match ready_rx.recv() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(message)) => Err(PlatformError::Other(message)),
        Err(_) => Err(PlatformError::Other("keyboard input thread ended".into())),
    }
}

/// Tells the input thread to register the current hotkeys.
pub(super) fn hotkeys_changed() {
    let window = WINDOW.load(Ordering::Acquire);
    if window != 0 {
        // SAFETY: posting to our own window; a failure only means the
        // thread has ended.
        let _ = unsafe { PostMessageW(Some(hwnd(window)), UPDATE_HOTKEYS, WPARAM(0), LPARAM(0)) };
    }
}

fn hwnd(raw: isize) -> HWND {
    HWND(raw as *mut core::ffi::c_void)
}

fn size_u32<T>() -> u32 {
    u32::try_from(size_of::<T>()).unwrap_or(u32::MAX)
}

fn run(shared: Arc<Shared>, ready: &crossbeam_channel::Sender<Result<(), String>>) {
    let window = match create_window() {
        Ok(window) => window,
        Err(message) => {
            let _ = ready.send(Err(message));
            return;
        }
    };
    if let Err(e) = register_raw_input(window) {
        let _ = ready.send(Err(format!("cannot read the keyboard: {e}")));
        return;
    }
    WINDOW.store(window.0 as isize, Ordering::Release);

    // SAFETY: no preconditions. Later changes come from the raw events.
    let caps_lock = unsafe { GetKeyState(i32::from(vk::CAPITAL)) } & 1 != 0;
    STATE.with(|state| {
        *state.borrow_mut() = Some(InputThread {
            shared,
            window,
            keys: KeyState::new(caps_lock),
            last_key: Instant::now(),
            last_foreground: 0,
            dead_key_pending: false,
            registered: Vec::new(),
            ticks: 0,
        });
    });
    with_state(InputThread::update_hotkeys);
    // SAFETY: a timer on our own window, handled in `window_proc`.
    unsafe {
        SetTimer(
            Some(window),
            MAINTENANCE_TIMER,
            MAINTENANCE_INTERVAL_MS,
            None,
        )
    };
    let _ = ready.send(Ok(()));

    let mut message = MSG::default();
    // SAFETY: a standard message loop on the thread that owns the window.
    unsafe {
        while GetMessageW(&raw mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&raw const message);
            DispatchMessageW(&raw const message);
        }
    }
}

fn create_window() -> Result<HWND, String> {
    // SAFETY: registering a window class whose procedure has the documented
    // signature, then creating a hidden window of it on this thread.
    unsafe {
        let instance = GetModuleHandleW(None)
            .map(|module| HINSTANCE(module.0))
            .map_err(|e| format!("cannot create the input window: {e}"))?;
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: w!("TypletInput"),
            ..WNDCLASSW::default()
        };
        // Registering twice fails harmlessly; creating the window tells.
        RegisterClassW(&raw const class);
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("TypletInput"),
            w!("Typlet input"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance),
            None,
        )
        .map_err(|e| format!("cannot create the input window: {e}"))
    }
}

fn devices(window: HWND) -> [RAWINPUTDEVICE; 2] {
    [USAGE_KEYBOARD, USAGE_MOUSE].map(|usage| RAWINPUTDEVICE {
        usUsagePage: USAGE_PAGE_GENERIC,
        usUsage: usage,
        dwFlags: RIDEV_INPUTSINK,
        hwndTarget: window,
    })
}

fn register_raw_input(window: HWND) -> windows::core::Result<()> {
    // SAFETY: the array is fully initialised and its size is passed.
    unsafe { RegisterRawInputDevices(&devices(window), size_u32::<RAWINPUTDEVICE>()) }
}

/// `true` if keyboard raw input of this process still goes to `window`.
fn raw_input_intact(window: HWND) -> bool {
    let mut registered = [RAWINPUTDEVICE::default(); 8];
    let mut count = u32::try_from(registered.len()).unwrap_or(0);
    // SAFETY: the buffer holds `count` entries of the size passed.
    let found = unsafe {
        GetRegisteredRawInputDevices(
            Some(registered.as_mut_ptr()),
            &raw mut count,
            size_u32::<RAWINPUTDEVICE>(),
        )
    };
    // On failure the answer is unknown; leave the registration as it is
    // rather than registering again every second.
    if found == u32::MAX {
        return true;
    }
    let found = usize::try_from(found).unwrap_or(0);
    registered.iter().take(found).any(|device| {
        device.usUsagePage == USAGE_PAGE_GENERIC
            && device.usUsage == USAGE_KEYBOARD
            && device.hwndTarget == window
            && device.dwFlags.contains(RIDEV_INPUTSINK)
    })
}

fn read_raw_input(handle: HRAWINPUT) -> Option<RAWINPUT> {
    // SAFETY: RAWINPUT is plain data; all-zero is a valid value.
    let mut input: RAWINPUT = unsafe { core::mem::zeroed() };
    let mut size = size_u32::<RAWINPUT>();
    // SAFETY: the buffer is a RAWINPUT of `size` bytes; keyboard and mouse
    // data always fit (Typlet registers no other devices).
    let read = unsafe {
        GetRawInputData(
            handle,
            RID_INPUT,
            Some((&raw mut input).cast()),
            &raw mut size,
            size_u32::<RAWINPUTHEADER>(),
        )
    };
    (read != u32::MAX && read != 0).then_some(input)
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_INPUT => {
            if let Some(input) = read_raw_input(HRAWINPUT(lparam.0 as *mut core::ffi::c_void)) {
                with_state(|state| state.raw_input(&input));
            }
            // WM_INPUT must reach DefWindowProc, which frees the input data.
        }
        WM_HOTKEY => {
            with_state(|state| state.hotkey(wparam.0));
            return LRESULT(0);
        }
        WM_TIMER if wparam.0 == MAINTENANCE_TIMER => {
            with_state(InputThread::maintain);
            return LRESULT(0);
        }
        UPDATE_HOTKEYS => {
            with_state(InputThread::update_hotkeys);
            return LRESULT(0);
        }
        _ => {}
    }
    // SAFETY: passing the unmodified arguments on.
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

fn with_state(f: impl FnOnce(&mut InputThread)) {
    STATE.with(|cell| {
        if let Ok(mut state) = cell.try_borrow_mut()
            && let Some(state) = state.as_mut()
        {
            f(state);
        }
    });
}

/// State of the input thread.
struct InputThread {
    shared: Arc<Shared>,
    window: HWND,
    keys: KeyState,
    last_key: Instant,
    last_foreground: isize,
    dead_key_pending: bool,
    /// The hotkeys in the engine's order (the id is the index) and whether
    /// Windows accepted them.
    registered: Vec<(Hotkey, bool)>,
    ticks: u32,
}

impl InputThread {
    fn raw_input(&mut self, input: &RAWINPUT) {
        if input.header.dwType == RIM_TYPEMOUSE.0 {
            // SAFETY: the header says the union holds mouse data.
            let buttons = unsafe { input.data.mouse.Anonymous.Anonymous.usButtonFlags };
            if u32::from(buttons) & MOUSE_BUTTON_DOWN != 0 {
                self.shared.send(Event::Reset);
            }
            return;
        }
        // Simulated key presses (Typlet's own and other programs') come
        // without a device and are ignored.
        if input.header.dwType != RIM_TYPEKEYBOARD.0 || input.header.hDevice.0.is_null() {
            return;
        }
        // SAFETY: the header says the union holds keyboard data.
        let keyboard = unsafe { input.data.keyboard };
        let raw = RawKey {
            make_code: keyboard.MakeCode,
            flags: keyboard.Flags,
            vkey: keyboard.VKey,
        };
        let Some(press) = self.keys.update(raw) else {
            return;
        };
        self.last_key = Instant::now();
        if press.down {
            self.key_down(press.vk, u32::from(keyboard.MakeCode));
        }
    }

    fn key_down(&mut self, vk_code: u16, scan_code: u32) {
        // SAFETY: no preconditions.
        let foreground = unsafe { GetForegroundWindow() };
        let id = foreground.0 as isize;
        if self.last_foreground != id {
            self.last_foreground = id;
            self.shared.send(Event::Reset);
            if !super::focus::is_own_window(foreground) {
                LAST_EXTERNAL.store(id, Ordering::Release);
            }
        }

        let mods = self.keys.modifiers();
        let is_hotkey =
            vk::to_key(vk_code).is_some_and(|key| self.shared.hotkey_index(mods, key).is_some());
        match classify(vk_code, mods, is_hotkey) {
            KeyAction::Ignore => {}
            KeyAction::Reset => self.shared.send(Event::Reset),
            // The Backspace reaches the app; the undo accounts for it.
            KeyAction::Backspace => self.shared.send(Event::Backspace { suppressed: false }),
            KeyAction::Translate => self.translate(vk_code, scan_code, foreground),
        }
    }

    fn translate(&mut self, vk_code: u16, scan_code: u32, foreground: HWND) {
        let state = self.keys.keyboard_state();
        let mut buffer = [0u16; 8];
        // SAFETY: the keyboard layout of the foreground thread; buffers are
        // local and sized as passed.
        let count = unsafe {
            let thread = GetWindowThreadProcessId(foreground, None);
            let layout = GetKeyboardLayout(thread);
            ToUnicodeEx(
                u32::from(vk_code),
                scan_code,
                &state,
                &mut buffer,
                TO_UNICODE_KEEP_STATE,
                Some(layout),
            )
        };

        if count < 0 {
            // A dead key; the next key composes a character we cannot know.
            self.dead_key_pending = true;
            self.shared.send(Event::Reset);
            return;
        }
        if count == 0 {
            return;
        }
        if std::mem::take(&mut self.dead_key_pending) {
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

    fn hotkey(&self, id: usize) {
        if self.registered.get(id).is_some_and(|(_, ok)| *ok) {
            self.shared.send(Event::Hotkey(id));
        }
    }

    /// Registers the engine's hotkeys with Windows if they changed.
    fn update_hotkeys(&mut self) {
        let wanted = self.shared.hotkeys();
        if self
            .registered
            .iter()
            .map(|(hotkey, _)| *hotkey)
            .eq(wanted.iter().copied())
        {
            return;
        }
        for (index, (_, ok)) in self.registered.iter().enumerate() {
            if *ok && let Ok(id) = i32::try_from(index) {
                // SAFETY: a hotkey this thread registered on its window.
                let _ = unsafe { UnregisterHotKey(Some(self.window), id) };
            }
        }
        let mut registered = Vec::with_capacity(wanted.len());
        for (index, hotkey) in wanted.into_iter().enumerate() {
            let ok = match self.register(index, &hotkey) {
                Ok(()) => true,
                Err(e) => {
                    log::warn!("shortcut {hotkey} is not available: {e}");
                    self.shared.send(Event::HotkeyUnavailable(index));
                    false
                }
            };
            registered.push((hotkey, ok));
        }
        self.registered = registered;
    }

    /// Registers one hotkey; fails if another program owns it.
    fn register(&self, index: usize, hotkey: &Hotkey) -> windows::core::Result<()> {
        let id = i32::try_from(index).map_err(|_| windows::core::Error::empty())?;
        let (modifiers, key) = vk::registration(hotkey).ok_or_else(windows::core::Error::empty)?;
        // SAFETY: registering on our own window from its thread.
        unsafe {
            RegisterHotKey(
                Some(self.window),
                id,
                HOT_KEY_MODIFIERS(modifiers),
                u32::from(key),
            )
        }
    }

    /// Runs every second.
    fn maintain(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);

        if !raw_input_intact(self.window) {
            match register_raw_input(self.window) {
                Ok(()) => log::info!("keyboard input registered again"),
                Err(e) => log::warn!("cannot register keyboard input again: {e}"),
            }
        }

        if self.keys.any_modifier_down() && self.last_key.elapsed() > STALE_MODIFIERS {
            // SAFETY: GetAsyncKeyState has no preconditions.
            self.keys
                .forget_released(|key| unsafe { GetAsyncKeyState(i32::from(key)) } < 0);
        }

        if self.ticks.is_multiple_of(HOTKEY_RETRY_TICKS) {
            let retry: Vec<(usize, Hotkey)> = self
                .registered
                .iter()
                .enumerate()
                .filter(|(_, (_, ok))| !ok)
                .map(|(index, (hotkey, _))| (index, *hotkey))
                .collect();
            for (index, hotkey) in retry {
                if self.register(index, &hotkey).is_ok() {
                    log::info!("shortcut {hotkey} is available now");
                    if let Some(entry) = self.registered.get_mut(index) {
                        entry.1 = true;
                    }
                }
            }
        }
    }
}
