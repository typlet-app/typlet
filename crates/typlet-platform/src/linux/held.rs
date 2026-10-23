// SPDX-License-Identifier: GPL-3.0-or-later
//! Which keys the user is holding down, counted across all keyboards.
//!
//! Compositors merge every keyboard into one seat and pass a key press on
//! only if no keyboard holds that key already. When Typlet's virtual keyboard
//! presses a key the user still holds (typically the last letters of the
//! trigger), the press is lost and the character is missing. The injector
//! therefore waits until the user has let go of a key before pressing it.

use std::sync::atomic::{AtomicU8, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// Key codes below this are tracked; the virtual keyboard only uses these.
const TRACKED: usize = 256;

const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Press counts per key code, shared between the input and typing threads.
pub(super) struct HeldKeys {
    counts: [AtomicU8; TRACKED],
}

impl Default for HeldKeys {
    fn default() -> Self {
        Self {
            counts: std::array::from_fn(|_| AtomicU8::new(0)),
        }
    }
}

impl HeldKeys {
    fn slot(&self, code: u16) -> Option<&AtomicU8> {
        self.counts.get(usize::from(code))
    }

    /// A keyboard reported the key as pressed.
    pub(super) fn press(&self, code: u16) {
        if let Some(count) = self.slot(code) {
            // The closure always returns `Some`, so the update cannot fail.
            let _ = count.fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                Some(n.saturating_add(1))
            });
        }
    }

    /// A keyboard reported the key as released. Releases of keys pressed
    /// before Typlet started are ignored.
    pub(super) fn release(&self, code: u16) {
        if let Some(count) = self.slot(code) {
            let _ = count.fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                Some(n.saturating_sub(1))
            });
        }
    }

    /// `true` if any keyboard holds the key.
    pub(super) fn is_held(&self, code: u16) -> bool {
        self.slot(code)
            .is_some_and(|count| count.load(Ordering::Acquire) > 0)
    }

    /// Waits until none of `codes` is held. Returns `false` if one is still
    /// held after `timeout` (a zero timeout only checks).
    pub(super) fn wait_released(&self, codes: &[u16], timeout: Duration) -> bool {
        let start = Instant::now();
        while codes.iter().any(|code| self.is_held(*code)) {
            if start.elapsed() >= timeout {
                return false;
            }
            thread::sleep(POLL_INTERVAL);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    const KEY_F: u16 = 33;
    const KEY_G: u16 = 34;

    #[test]
    fn counts_presses_across_keyboards() {
        let held = HeldKeys::default();
        held.press(KEY_F);
        held.press(KEY_F); // the same key on a second keyboard
        held.release(KEY_F);
        assert!(held.is_held(KEY_F));
        held.release(KEY_F);
        assert!(!held.is_held(KEY_F));
    }

    #[test]
    fn ignores_releases_without_press_and_unknown_codes() {
        let held = HeldKeys::default();
        held.release(KEY_G);
        assert!(!held.is_held(KEY_G));
        held.press(KEY_G);
        assert!(held.is_held(KEY_G));

        held.press(0x2ff);
        held.release(0x2ff);
        assert!(!held.is_held(0x2ff));
    }

    /// Regression: `:mfg` produced "Mit reundlichen rüßen" because the user
    /// still held `f` and `g` when typing started.
    #[test]
    fn waits_until_the_user_lets_go() {
        let held = Arc::new(HeldKeys::default());
        held.press(KEY_F);
        held.press(KEY_G);
        let user = Arc::clone(&held);
        let releasing = thread::spawn(move || {
            thread::sleep(Duration::from_millis(40));
            user.release(KEY_F);
            thread::sleep(Duration::from_millis(40));
            user.release(KEY_G);
        });

        let start = Instant::now();
        assert!(held.wait_released(&[KEY_F, KEY_G], Duration::from_secs(5)));
        assert!(start.elapsed() >= Duration::from_millis(70));
        assert!(releasing.join().is_ok());
    }

    #[test]
    fn gives_up_after_the_timeout() {
        let held = HeldKeys::default();
        held.press(KEY_F);
        assert!(!held.wait_released(&[KEY_F], Duration::ZERO));
        assert!(!held.wait_released(&[KEY_F], Duration::from_millis(20)));
        assert!(held.wait_released(&[KEY_G], Duration::ZERO));
    }
}
