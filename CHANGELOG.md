# Changelog

All notable changes to Typlet are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Installers (milestone M5): Windows NSIS installer per user without admin rights, Debian package with udev rule and `input` group set-up, AppImage; built by the new release workflow from version tags as draft releases with checksums, with an install/uninstall smoke test on every build; code-signing step prepared for SignPath (ADR 0013, docs/releasing.md).
- The editor accepts `--autostart`/`--background` to start only the background program (used by the AppImage at login).
- User guide: installing; README: download section.
- Windows: a message when another program already uses one of Typlet's shortcuts, with a button to the settings.
- Day and month names in dates follow the interface language (`Freitag, 23. Oktober 2026`); formats with a time-zone code such as `%z` no longer fail.
- Editor `typlet-editor` (milestone M4), built with Tauri, React and Tiptap:
  - snippet list with search and categories,
  - rich-text editing with bold, italic, underline, links and lists,
  - placeholders as chips: date (format and day shift), cursor, clipboard, form fields,
  - form designer with text, multi-line, choice and yes/no questions,
  - live preview rendered by the same code that inserts snippets,
  - live warnings for duplicate and unreachable triggers,
  - insert mode and per-snippet keyboard shortcut,
  - settings: language, autostart, messages, shortcuts, snippet folder, app rules,
  - import from espanso (`typlet-import` crate),
  - English and German interface,
  - starts the background program if needed; a second start opens the requested snippet.
- Triggers typed into the editor are not expanded (editor focus lock, ADR 0012).
- Category names are validated so snippets cannot be saved outside the library.
- Background program `typlet` (milestones M2 and M3):
  - expands triggers in every app, types short text and pastes long or formatted text,
  - restores the clipboard afterwards and keeps temporary content out of the Windows clipboard history,
  - Backspace right after an expansion brings the trigger back,
  - quick search (`Ctrl+Shift+Space`) that inserts into the previous app,
  - fill-in forms with text, multi-line, choice and yes/no fields,
  - new snippet from selected text (`Ctrl+Alt+S`) with live trigger checks,
  - optional global hotkey per snippet,
  - tray menu with pause, start with the computer, snippet folder, help and quit,
  - set-up assistant with permission check and a test field,
  - diagnostics with live status, library problems, recent expansions and a copyable report without typed text,
  - pause in password fields (Windows),
  - app rules with built-in defaults (Shift+Enter in chat apps, plain text in Excel, longer clipboard delays for browsers and remote desktops),
  - reloads snippets and settings when files change,
  - five example snippets on first start, in English or German,
  - English and German interface.
- Platform layer `typlet-platform` for Windows (keyboard hook, dead-key-safe translation, Unicode typing, clipboard with HTML and RTF, UI Automation) and Linux (evdev, xkbcommon, uinput, X11 clipboard), with autostart and a single-instance lock.
- Core: action plans and undo, settings with app rules, hotkeys with English and German key names, accent-insensitive search, previews with real dates.
- Snippet format: optional `hotkey` field.
- Documentation: user guide in English and German, design tokens and glossary, test-build instructions, manual test checklist, screenshots, decision records 0009–0011.
- CI: German translation check, Linux build libraries, unsigned test builds for Windows and Linux.
- CI: editor interface job (type check, tests, build); test builds contain the editor.
- Documentation: editor chapters in the user guide, editor screenshots, ADR 0012, screenshot and image scripts.
- `typlet-core` crate:
  - snippet file format (version 1) with validation,
  - rich-text documents with bold, italic, underline, links, bullet and numbered lists,
  - placeholders for date/time with offsets, cursor position, clipboard and form fields,
  - rendering to plain text, HTML, RTF and the Windows `CF_HTML` clipboard format,
  - automatic choice between typing and pasting,
  - instant trigger matching with longest-match rule and backspace undo,
  - detection of duplicate and unreachable triggers,
  - snippet library with one JSON file per snippet, categories as folders and atomic saving.
- Documentation: architecture, snippet format, roadmap, development guide and eight decision records.
- Tested example snippets in `docs/examples`.
- CI on Linux and Windows with formatting, clippy, tests, documentation and `cargo-deny` checks.
- Community files: contributing guide, contributor licence agreement, code of conduct, security and trademark policies, issue and pull request templates.

### Fixed

- Linux: characters were missing when the user still held the last keys of the trigger while Typlet typed (`:mfg` gave `Mit reundlichen rüßen`). Typlet now waits until such a key is released.
- The background program reloaded the snippet library over and over after any program read a snippet file (the file watcher treated reading as a change).

### Changed

- Windows: Typlet reads the keyboard with Raw Input instead of a low-level keyboard hook and registers its shortcuts with Windows (`RegisterHotKey`). Raw input only reads, so the Backspace that undoes an expansion now reaches the app first, as on Linux (ADR 0014).
- ADR 0003 is partly superseded by ADR 0010: the set-up assistant lives in the background program.
- `cargo-deny` treats "unmaintained" notices as errors only for direct dependencies.

[Unreleased]: https://github.com/typlet-app/typlet/commits/main
