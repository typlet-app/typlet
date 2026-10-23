# typlet-core

The platform-independent heart of [Typlet](../../README.md). It contains everything that does not need to talk to the operating system, so it can be tested quickly and identically on every platform.

| Module | Responsibility |
|---|---|
| `document` | Rich-text model of a snippet body: paragraphs, lists, formatting marks and placeholders |
| `snippet` | A snippet (trigger, label, content, form fields) and its validation rules |
| `render` | Turns a snippet into plain text, HTML, RTF and the Windows `CF_HTML` clipboard format |
| `matcher` | Detects typed triggers in the keystroke stream (instant expansion, longest match, backspace undo) |
| `plan` | Turns a rendered snippet into concrete steps (delete trigger, type or paste, move cursor) and plans undo |
| `library` | Loads and saves snippets as one JSON file per snippet, with folders as categories |
| `search` | Finds snippets by trigger, name and text, ignoring case and accents |
| `settings` | User settings and per-application rules, with built-in rules for common apps |
| `hotkey` | Keyboard shortcuts such as `Ctrl+Shift+Space`, with English and German key names |

Nothing in this crate uses `unsafe` code, touches the keyboard or reads the clipboard. Platform-specific code lives in separate crates (see [docs/architecture.md](../../docs/architecture.md)).

## Running the tests

```sh
cargo test -p typlet-core
```
