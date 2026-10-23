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

- `crates/typlet-core`: format, validation, matching, rendering, storage
- `docs/`: architecture, format spec, roadmap, development guide, ADRs, tested examples
- Planned crates: `typlet-platform` (OS backends), `typlet` (background program, Slint), `typlet-editor` (Tauri + Tiptap)

## Toolchain

The Rust version is pinned in `rust-toolchain.toml`. When upgrading, also change `rust-version` in `Cargo.toml` and the toolchain in `.github/workflows/ci.yml`.
