// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows clipboard: saving, setting rich content and restoring.

use std::ptr;
use std::thread;
use std::time::Duration;

use typlet_core::{ClipboardContent, wrap_cf_html};
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
    GetClipboardSequenceNumber, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows::core::{PCWSTR, w};

use crate::PlatformError;

const CF_UNICODETEXT: u32 = 13;

/// Formats whose data is a GDI handle or otherwise not plain global memory;
/// they cannot be copied byte-wise. Windows synthesises bitmaps from `CF_DIB`.
fn is_copyable(format: u32) -> bool {
    !matches!(format, 2 | 3 | 9 | 14 | 0x80..=0x8E | 0x200..=0x3FF)
}

/// Upper bound for saved clipboard data, to avoid copying huge images.
const MAX_SAVED_BYTES: usize = 64 * 1024 * 1024;

/// A snapshot of all copyable clipboard formats.
#[derive(Debug, Default)]
pub(super) struct Saved {
    formats: Vec<(u32, Vec<u8>)>,
}

/// Clipboard access without an owner window.
///
/// The clipboard is opened without an owner (`OpenClipboard(NULL)`). Windows
/// sends messages such as `WM_DESTROYCLIPBOARD` to the owner window; an owner
/// on the engine thread, which does not pump messages, would make other
/// applications hang while they copy.
pub(super) struct Clipboard {
    html_format: u32,
    rtf_format: u32,
    exclude_monitor: u32,
    history: u32,
    cloud: u32,
}

impl Clipboard {
    pub(super) fn new() -> Self {
        let register = |name: PCWSTR| {
            // SAFETY: registering a named format has no preconditions.
            unsafe { RegisterClipboardFormatW(name) }
        };
        Self {
            html_format: register(w!("HTML Format")),
            rtf_format: register(w!("Rich Text Format")),
            exclude_monitor: register(w!("ExcludeClipboardContentFromMonitorProcessing")),
            history: register(w!("CanIncludeInClipboardHistory")),
            cloud: register(w!("CanUploadToCloudClipboard")),
        }
    }

    /// The clipboard sequence number; it changes whenever the content changes.
    pub(super) fn sequence(&self) -> u32 {
        // SAFETY: no preconditions.
        unsafe { GetClipboardSequenceNumber() }
    }

    fn open(&self) -> bool {
        for _ in 0..20 {
            // SAFETY: opening without an owner window has no preconditions.
            if unsafe { OpenClipboard(None) }.is_ok() {
                return true;
            }
            thread::sleep(Duration::from_millis(15));
        }
        log::warn!("clipboard is busy");
        false
    }

    fn close() {
        // SAFETY: only called after a successful `open`.
        let _ = unsafe { CloseClipboard() };
    }

    pub(super) fn save(&self) -> Saved {
        let mut saved = Saved::default();
        if !self.open() {
            return saved;
        }
        let mut total = 0;
        let mut format = 0;
        loop {
            // SAFETY: the clipboard is open.
            format = unsafe { EnumClipboardFormats(format) };
            if format == 0 {
                break;
            }
            if !is_copyable(format) {
                continue;
            }
            if let Some(bytes) = read_global(format) {
                total += bytes.len();
                if total > MAX_SAVED_BYTES {
                    break;
                }
                saved.formats.push((format, bytes));
            }
        }
        Self::close();
        saved
    }

    pub(super) fn restore(&self, saved: &Saved) {
        if !self.open() {
            return;
        }
        // SAFETY: the clipboard is open.
        let _ = unsafe { EmptyClipboard() };
        for (format, bytes) in &saved.formats {
            set_global(*format, bytes);
        }
        Self::close();
    }

    /// Puts content on the clipboard and returns the new sequence number.
    pub(super) fn set(&self, content: &ClipboardContent) -> Result<u32, PlatformError> {
        if !self.open() {
            return Err(PlatformError::Other("the clipboard is busy".into()));
        }
        // SAFETY: the clipboard is open.
        let _ = unsafe { EmptyClipboard() };

        let text = content.plain.replace('\n', "\r\n");
        let wide: Vec<u8> = text
            .encode_utf16()
            .chain(std::iter::once(0))
            .flat_map(u16::to_le_bytes)
            .collect();
        set_global(CF_UNICODETEXT, &wide);

        if let Some(html) = &content.html {
            let mut bytes = wrap_cf_html(html).into_bytes();
            bytes.push(0);
            set_global(self.html_format, &bytes);
        }
        if let Some(rtf) = &content.rtf {
            let mut bytes = rtf.clone().into_bytes();
            bytes.push(0);
            set_global(self.rtf_format, &bytes);
        }

        // Keep the temporary content out of clipboard history and cloud sync.
        set_global(self.exclude_monitor, &[0]);
        set_global(self.history, &0u32.to_le_bytes());
        set_global(self.cloud, &0u32.to_le_bytes());

        Self::close();
        Ok(self.sequence())
    }

    pub(super) fn text(&self) -> Option<String> {
        if !self.open() {
            return None;
        }
        let bytes = read_global(CF_UNICODETEXT);
        Self::close();
        let bytes = bytes?;
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .take_while(|unit| *unit != 0)
            .collect();
        Some(String::from_utf16_lossy(&units).replace("\r\n", "\n"))
    }
}

/// Copies the data of a clipboard format. The clipboard must be open.
fn read_global(format: u32) -> Option<Vec<u8>> {
    // SAFETY: the clipboard is open; the handle is only used while locked.
    unsafe {
        let handle = GetClipboardData(format).ok()?;
        let global = HGLOBAL(handle.0);
        let size = GlobalSize(global);
        if size == 0 {
            return None;
        }
        let data = GlobalLock(global);
        if data.is_null() {
            return None;
        }
        let mut bytes = vec![0u8; size];
        ptr::copy_nonoverlapping(data.cast::<u8>(), bytes.as_mut_ptr(), size);
        let _ = GlobalUnlock(global);
        Some(bytes)
    }
}

/// Places bytes on the clipboard. The clipboard must be open and emptied.
fn set_global(format: u32, bytes: &[u8]) {
    // SAFETY: memory is allocated, filled while locked and handed to the
    // clipboard, which takes ownership on success; on failure we free it.
    unsafe {
        let Ok(global) = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1)) else {
            return;
        };
        let data = GlobalLock(global);
        if data.is_null() {
            let _ = GlobalFree(Some(global));
            return;
        }
        ptr::copy_nonoverlapping(bytes.as_ptr(), data.cast::<u8>(), bytes.len());
        let _ = GlobalUnlock(global);
        if SetClipboardData(format, Some(HANDLE(global.0))).is_err() {
            let _ = GlobalFree(Some(global));
        }
    }
}
