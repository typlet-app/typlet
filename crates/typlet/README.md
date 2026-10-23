# typlet

The background program of [Typlet](../../README.md): it watches for triggers, inserts snippets and shows the small windows users see every day.

| Part | Source | What it does |
|---|---|---|
| Engine | `src/engine.rs` | Runs on its own thread: matcher → render → plan → platform. Tested with a fake platform. |
| Controller | `src/app.rs` | Main thread: tray icon, windows, settings, talks to the engine with commands |
| Watcher | `src/watcher.rs` | Reloads the library and settings when the editor saves files |
| User interface | `ui/*.slint` | Tray menu, quick search, forms, new snippet, set-up assistant, diagnostics, messages ([Slint](https://slint.dev)) |
| Translations | `lang/de/LC_MESSAGES/typlet.po` | German texts; English is the source language |

## Running

```sh
cargo run -p typlet              # start (opens the set-up assistant on first run)
cargo run -p typlet -- --search  # open the quick search right away
TYPLET_LOG=debug cargo run -p typlet
```

Development builds never register themselves for autostart.

## Screenshots

```sh
cargo run -p typlet --example screenshots -- docs/images
```

renders every window with sample data in English and German, without a display.
