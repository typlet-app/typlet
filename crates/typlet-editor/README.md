# typlet-editor

The editor of [Typlet](../../README.md): a window for creating and editing snippets, changing settings and importing from espanso. Built with [Tauri](https://tauri.app) (Rust) and a React + [Tiptap](https://tiptap.dev) interface in [`web/`](web).

| Part | Source | What it does |
|---|---|---|
| Commands | `src/commands.rs` | All file access through `typlet-core`: load and save snippets by id, delete, validate, preview with the real renderer, settings, espanso import |
| Window | `src/main.rs` | Single instance (a second start focuses the window and passes `--open <id>` / `--settings`), starts the background program if needed, holds the editor-focus lock |
| Interface | `web/src` | Snippet list, rich-text editor with placeholder chips, form fields, preview, settings, import |
| Conversion | `web/src/convert.ts` | Typlet document ⇄ Tiptap JSON, tested against every documented example |
| Texts | `web/src/i18n/{en,de}.json` | English and German |

The editor never talks to the background program directly. It writes files; the background program notices and reloads. While the editor window has the keyboard focus it holds a lock file, so triggers typed into the editor are not expanded ([ADR 0012](../../docs/adr/0012-editor-focus-lock.md)).

## Developing

```sh
cd crates/typlet-editor/web
npm ci                   # once
npm test                 # converter, date formatting, trigger checks
npm run dev              # interface only, in the browser (needs the Rust backend for data)

cd ..                    # crates/typlet-editor
web/node_modules/.bin/tauri dev     # the real editor with hot reload
```

`VITE_DEMO=1 npm run dev` runs the interface in a normal browser with sample snippets and no Rust backend. `python3 scripts/editor-screenshots.py` uses that mode for the documentation screenshots.

Release builds embed the interface: `npm run build` in `web/`, then `cargo build --release -p typlet-editor --features typlet-editor/custom-protocol` (CI does this for the test builds).
