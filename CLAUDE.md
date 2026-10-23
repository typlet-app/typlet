# Working on Typlet

Typlet is an open-source text expander for non-technical users (see [README.md](README.md)). This file tells AI coding agents how to work in this repository. Humans: see [CONTRIBUTING.md](CONTRIBUTING.md).

## Commands

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo doc --workspace --no-deps
cargo deny check
```

All five must pass before a change is finished. CI runs them on Linux and Windows.

## Non-negotiable rules

1. **Documentation is part of every change.** The maintainer wants a clean, best-practice repository whose documentation is always current. For every change, update in the same commit or pull request:
   - the feature table in `README.md` and `docs/roadmap.md`,
   - `CHANGELOG.md` under **Unreleased**,
   - `docs/architecture.md` when structure or behaviour changes,
   - `docs/snippet-format.md` and `docs/examples/` when the format changes (tests load every example),
   - a new ADR in `docs/adr/` for every architectural decision (never edit accepted ADRs; supersede them),
   - README screenshots whenever the UI changes visibly.
2. **Privacy:** never log, print, persist or transmit typed characters. The background program has no network access.
3. **`typlet-core` stays pure:** no `unsafe`, no OS access, no UI.
4. **Tests accompany code.** Bug fixes include a regression test.
5. Every source file starts with `// SPDX-License-Identifier: GPL-3.0-or-later`.
6. No `unwrap`/`expect` in library code.
7. Repository language is English. The app UI is German-first with English.
8. Commits follow Conventional Commits (`feat(core): …`, `fix(matcher): …`, `docs: …`).
9. Do not copy code from espanso or other GPL-incompatible or unrelated projects.

## Layout

- `crates/typlet-core`: format, validation, matching, rendering, plans, search, settings, storage
- `crates/typlet-platform`: OS backends (Windows raw input/RegisterHotKey/SendInput/clipboard/UIA, Linux evdev/xkbcommon/uinput), paths, autostart
- `crates/typlet`: background program (engine thread, Slint UI in `ui/`, German texts in `lang/`)
- `crates/typlet-editor`: editor (Tauri; React + Tiptap interface in `web/`, texts in `web/src/i18n/`)
- `crates/typlet-import`: espanso import
- `packaging/`: installer configuration (`tauri.release.json`), Windows NSIS hooks, Linux udev rule and post-install script
- `scripts/`: translation check, icon rendering, screenshots, image optimisation
- `docs/`: architecture, design, format specs, roadmap, user guides (EN/DE), development, releasing, testing, ADRs, tested examples, screenshots

Also keep in step: every new interface text needs a German translation (`python3 scripts/check-translations.py`); the editor interface has its own checks (`cd crates/typlet-editor/web && npm run typecheck && npm test`).

## Toolchain

The Rust version is pinned in `rust-toolchain.toml`. When upgrading, also change `rust-version` in `Cargo.toml` and `RUST_TOOLCHAIN` in every workflow in `.github/workflows/`.
