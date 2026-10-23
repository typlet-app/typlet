// SPDX-License-Identifier: GPL-3.0-or-later
//! Discovers keyboards and pointing devices and reads their key events.
//!
//! Every device gets its own reader thread. A watcher rescans periodically so
//! keyboards plugged in later (USB, Bluetooth) are picked up. Typlet's own
//! virtual keyboard is skipped, otherwise injected text would be matched
//! again.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossbeam_channel::Sender;
use evdev::{Device, EventSummary, KeyCode};

/// Name of the uinput device Typlet creates for typing.
pub(super) const VIRTUAL_DEVICE_NAME: &str = "Typlet Virtual Keyboard";

const RESCAN_INTERVAL: Duration = Duration::from_secs(3);

/// A key or button event as reported by evdev.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RawKey {
    /// Linux input event code (`KEY_*` or `BTN_*`).
    pub(super) code: u16,
    /// 0 = released, 1 = pressed, 2 = auto-repeat.
    pub(super) value: i32,
}

fn is_ours(device: &Device) -> bool {
    device.name() == Some(VIRTUAL_DEVICE_NAME)
}

fn is_keyboard(device: &Device) -> bool {
    device.supported_keys().is_some_and(|keys| {
        keys.contains(KeyCode::KEY_A)
            && keys.contains(KeyCode::KEY_Z)
            && keys.contains(KeyCode::KEY_SPACE)
    })
}

fn is_pointer(device: &Device) -> bool {
    device
        .supported_keys()
        .is_some_and(|keys| keys.contains(KeyCode::BTN_LEFT) || keys.contains(KeyCode::BTN_TOUCH))
}

/// Starts reading all current devices and watching for new ones.
pub(super) fn spawn_watcher(tx: Sender<RawKey>) {
    let active = Arc::new(Mutex::new(HashSet::new()));
    scan(&tx, &active);

    let spawned = thread::Builder::new()
        .name("typlet-devices".into())
        .spawn(move || {
            loop {
                thread::sleep(RESCAN_INTERVAL);
                scan(&tx, &active);
            }
        });
    if let Err(e) = spawned {
        log::warn!("cannot start device watcher: {e}");
    }
}

fn scan(tx: &Sender<RawKey>, active: &Arc<Mutex<HashSet<PathBuf>>>) {
    for (path, device) in evdev::enumerate() {
        if is_ours(&device) {
            continue;
        }
        if !is_keyboard(&device) && !is_pointer(&device) {
            continue;
        }

        let is_new = active.lock().is_ok_and(|mut set| set.insert(path.clone()));
        if is_new {
            spawn_reader(path, device, tx.clone(), Arc::clone(active));
        }
    }
}

fn spawn_reader(
    path: PathBuf,
    mut device: Device,
    tx: Sender<RawKey>,
    active: Arc<Mutex<HashSet<PathBuf>>>,
) {
    let name = device.name().unwrap_or("unnamed").to_owned();
    let result = thread::Builder::new()
        .name("typlet-input".into())
        .spawn(move || {
            log::info!("reading input device {name}");
            let mut pressed = HashSet::new();
            'read: loop {
                let Ok(events) = device.fetch_events() else {
                    break;
                };
                for event in events {
                    if let EventSummary::Key(_, code, value) = event.destructure() {
                        match value {
                            1 => {
                                pressed.insert(code.0);
                            }
                            0 => {
                                pressed.remove(&code.0);
                            }
                            _ => {}
                        }
                        if tx
                            .send(RawKey {
                                code: code.0,
                                value,
                            })
                            .is_err()
                        {
                            break 'read;
                        }
                    }
                }
            }
            log::info!("input device {name} went away");
            // A keyboard unplugged with a key down never reports the release;
            // otherwise Typlet would wait for that key before every expansion.
            for code in pressed {
                if tx.send(RawKey { code, value: 0 }).is_err() {
                    break;
                }
            }
            if let Ok(mut set) = active.lock() {
                set.remove(&path);
            }
        });
    if let Err(e) = result {
        log::warn!("cannot start reader thread: {e}");
    }
}

/// Counts keyboards Typlet can read, without starting anything.
pub(super) fn count_readable_keyboards() -> usize {
    evdev::enumerate()
        .filter(|(_, device)| is_keyboard(device) && !is_ours(device))
        .count()
}
