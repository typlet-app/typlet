# 0002. Native background process with Slint, no browser engine

- Status: Accepted
- Date: 2026-10-23

## Context

The background program runs permanently and sees every keystroke. The windows users see most often (quick search, fill-in forms) must appear instantly and hand focus back to the target app correctly. Options considered: Electron, Tauri (system webview), Qt, Slint, native UI per OS.

A webview adds start-up latency, permanent memory use and a browser engine inside the keystroke process. On Linux, Tauri depends on WebKitGTK, whose quality varies between distributions.

## Decision

The background program is native Rust with **[Slint](https://slint.dev)** for its user interface (tray, quick search, forms). Windows are created once and hidden, so they appear instantly. Slint is used under the GPL-3.0 licence. Slint's tray support covers Windows, macOS and Linux (StatusNotifierItem).

## Consequences

- Low memory use, instant windows, accessibility via AccessKit.
- No browser engine and no network code in the process that sees keystrokes.
- A second UI technology is needed for rich-text editing (see [ADR 0003](0003-separate-editor-process.md)).
- On GNOME, tray icons require the AppIndicator extension (Ubuntu ships it); a hotkey remains the fallback entry point.
