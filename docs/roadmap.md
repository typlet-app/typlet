# 🗺️ Roadmap

Typlet V1 targets **Windows** as the polished product and **Linux (Wayland)** at developer quality. The risky parts come first; the beta ships with all V1 features at once.

**Legend:** ✅ done · 🚧 in progress · ⏳ next

## M1 — Foundation ✅

- [x] Repository, licence, contributor agreement, community files
- [x] CI on Linux and Windows: format, clippy, tests, docs, licence and advisory checks
- [x] `typlet-core`: snippet format, validation, rendering (plain, HTML, RTF, `CF_HTML`), trigger matching, storage
- [x] Documentation: architecture, format specification, decision records

## M2 — Keyboard engine ⏳

- [ ] Platform layer interface (`typlet-platform`)
- [ ] Linux backend: evdev input, xkbcommon translation, uinput typing, clipboard
- [ ] Windows backend: keyboard hook, dead-key-safe translation, Unicode typing, clipboard with `CF_HTML`/RTF
- [ ] Clipboard save and restore with all formats, transient marking
- [ ] Background program with tray icon, pause/resume and autostart
- [ ] Manual test matrix across the most-used apps

## M3 — Quick search, forms, hotkeys

- [ ] Quick search window (Slint) that opens instantly and returns focus correctly
- [ ] Fill-in forms for snippets with fields
- [ ] Global and per-snippet hotkeys
- [ ] "Create snippet from selection"

## M4 — Editor

- [ ] Editor app (Tauri + Tiptap): list, search, categories
- [ ] Rich-text editing with placeholders as visual chips
- [ ] Conflict warnings, live preview
- [ ] Settings and app rules
- [ ] Import from espanso

## M5 — Beta

- [ ] Setup assistant with test field
- [ ] Diagnostics ("does not work in app X")
- [ ] Pause in password fields
- [ ] Installers without admin rights (Windows), AppImage and packages (Linux)
- [ ] Code signing via SignPath; Microsoft Store submission
- [ ] Screenshots and user guide in the README

## After V1

- macOS
- Images in snippets
- Sync and shared team libraries
- App profiles on GNOME via a Shell extension
- More languages for the user interface (German and English come first)
