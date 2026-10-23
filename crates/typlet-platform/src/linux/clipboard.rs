// SPDX-License-Identifier: GPL-3.0-or-later
//! Clipboard access via arboard.
//!
//! The arboard handle is kept alive for the whole session: on X11/XWayland
//! the clipboard content is served by the owning process, so dropping the
//! handle right after setting content would lose it.

use typlet_core::ClipboardContent;

#[derive(Default)]
pub(super) struct Clipboard {
    inner: Option<arboard::Clipboard>,
}

impl Clipboard {
    fn handle(&mut self) -> Option<&mut arboard::Clipboard> {
        if self.inner.is_none() {
            match arboard::Clipboard::new() {
                Ok(clipboard) => self.inner = Some(clipboard),
                Err(e) => log::warn!("clipboard unavailable: {e}"),
            }
        }
        self.inner.as_mut()
    }

    pub(super) fn text(&mut self) -> Option<String> {
        self.handle()?.get_text().ok()
    }

    /// Puts content on the clipboard; HTML is offered with a plain-text
    /// alternative so plain fields still receive text.
    pub(super) fn set(&mut self, content: &ClipboardContent) -> bool {
        let Some(clipboard) = self.handle() else {
            return false;
        };
        let result = match &content.html {
            Some(html) => clipboard.set_html(html.as_str(), Some(content.plain.as_str())),
            None => clipboard.set_text(content.plain.as_str()),
        };
        result.is_ok()
    }

    pub(super) fn set_plain(&mut self, text: &str) {
        if let Some(clipboard) = self.handle() {
            let _ = clipboard.set_text(text);
        }
    }

    pub(super) fn clear(&mut self) {
        if let Some(clipboard) = self.handle() {
            let _ = clipboard.clear();
        }
    }
}
