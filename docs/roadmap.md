# 🗺️ Roadmap

Typlet V1 targets **Windows** as the polished product and **Linux (Wayland)** at developer quality. The risky parts come first; the beta ships with all V1 features at once.

**Legend:** ✅ done · 🚧 in progress · ⏳ next

## M1 — Foundation ✅

- [x] Repository, licence, contributor agreement, community files
- [x] CI on Linux and Windows: format, clippy, tests, docs, licence and advisory checks
- [x] `typlet-core`: snippet format, validation, rendering (plain, HTML, RTF, `CF_HTML`), trigger matching, storage
- [x] Documentation: architecture, format specification, decision records

## M2 — Keyboard engine ✅

- [x] Platform layer interface (`typlet-platform`)
- [x] Linux backend: evdev input, xkbcommon translation, uinput typing, clipboard
- [x] Windows backend: keyboard hook, dead-key-safe translation, Unicode typing, clipboard with `CF_HTML`/RTF
- [x] Clipboard save and restore with all formats, transient marking
- [x] Background program with tray icon, pause/resume and autostart
- [x] App rules with built-in defaults for Excel, chat apps, browsers and remote desktops
- [x] Unsigned test builds for Windows and Linux from CI
- [ ] Manual test matrix across the most-used apps (needs real hardware, see the [checklist](testing/manual-test-checklist.md))

## M3 — Quick search, forms, hotkeys ✅

- [x] Quick search window that opens instantly and returns focus to the previous app
- [x] Fill-in forms for snippets with fields
- [x] Global and per-snippet hotkeys
- [x] "Create snippet from selection"

## M4 — Editor ✅

- [x] Editor app (Tauri + React + Tiptap): list, search, categories
- [x] Rich-text editing with placeholders as visual chips
- [x] Form fields, per-snippet hotkeys and insert mode
- [x] Conflict warnings, live preview rendered by the core
- [x] Settings and app rules
- [x] Import from espanso
- [x] No expansion while typing in the editor

## M5 — Beta ⏳

- [x] Set-up assistant with test field
- [x] Diagnostics ("does not work in app X")
- [x] Pause in password fields (Windows)
- [x] Installers without admin rights (Windows), AppImage and Debian package (Linux)
- [x] Release workflow with draft releases and checksums
- [ ] Code signing via SignPath *(workflow ready; needs the SignPath account, see [releasing](releasing.md#code-signing))*
- [ ] Microsoft Store submission *(steps in [releasing](releasing.md#microsoft-store))*
- [x] Screenshots and user guide in the README
- [ ] First beta release `0.1.0`, after the [manual tests](testing/manual-test-checklist.md) on Windows and Linux with the installers built by CI ([releasing](releasing.md))

## After V1

- macOS
- Images in snippets
- Sync and shared team libraries
- App profiles and password-field detection on GNOME via a Shell extension
- Focus handling on GNOME without the "is ready" notification (XDG activation tokens)
- More languages for the user interface (German and English come first)
