# 🛠️ Development

## Prerequisites

| Tool | Why | Install |
|---|---|---|
| Rust via rustup | Compiler; the version is pinned in [`rust-toolchain.toml`](../rust-toolchain.toml) and installed automatically | <https://rustup.rs> |
| Git | Version control | Your package manager |
| Node.js 22+ | The editor's web interface | <https://nodejs.org> or your package manager |
| Linux libraries | Keyboard layouts, fonts, Wayland (background program); WebKitGTK and D-Bus (editor) | `sudo apt install libxkbcommon-dev libfontconfig1-dev libwayland-dev libwebkit2gtk-4.1-dev libdbus-1-dev` |
| Python 3 *(optional)* | Translation check, icon rendering and screenshot scripts | Your package manager |
| cargo-deny *(optional)* | Licence and advisory checks, also run in CI | `cargo install cargo-deny --locked` |

On Windows, the Visual Studio Build Tools (C++ workload) are needed, as for any Rust project; the editor uses WebView2, which ships with Windows 10 and 11.

Typlet is developed on **Ubuntu 26.04 LTS (GNOME, Wayland)** and **Windows 11**. Both are covered by CI.

## Everyday commands

```sh
cargo test --workspace                                      # all tests
cargo clippy --workspace --all-targets -- -D warnings       # lints, as strict as CI
cargo fmt --all                                             # format
cargo doc --workspace --no-deps --open                      # API documentation
cargo deny check                                            # licences, advisories, sources
python3 scripts/check-translations.py                       # every text translated
(cd crates/typlet-editor/web && npm ci && npm run typecheck && npm test)   # editor interface
```

CI runs exactly these checks on Linux and Windows for every push and pull request, and builds unsigned test versions ([how to try them](testing/test-build.md)).

## Running the background program

```sh
cargo run -p typlet                       # first start opens the set-up assistant
cargo run -p typlet -- --search           # open the quick search right away
TYPLET_LOG=debug cargo run -p typlet      # more detail in the log
```

On Linux, keyboard access needs a one-time set-up (the assistant shows the same commands):

```sh
sudo usermod -aG input $USER
sudo cp packaging/linux/60-typlet-uinput.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
# then log out and back in
```

Development builds never register themselves for autostart. To start from scratch, quit Typlet and delete `~/.config/typlet` and `~/.local/share/typlet` (Windows: `%APPDATA%\Typlet`).

## Running the editor

```sh
cd crates/typlet-editor/web && npm ci && cd ..
web/node_modules/.bin/tauri dev           # editor with hot reload (starts the Vite dev server)
```

`cargo run -p typlet-editor` alone builds a development editor that expects the dev server on port 5173. Without Tauri, `VITE_DEMO=1 npm run dev` in `web/` shows the interface with sample data in any browser. See [crates/typlet-editor](../crates/typlet-editor/README.md) for details.

## Working on the user interface

- Windows are written in [Slint](https://slint.dev) in [`crates/typlet/ui`](../crates/typlet/ui). Colours, spacing and type sizes come from [`theme.slint`](../crates/typlet/ui/theme.slint); see [design.md](design.md).
- Write every visible text in English with `@tr("…")`, then add the German translation to [`typlet.po`](../crates/typlet/lang/de/LC_MESSAGES/typlet.po). `python3 scripts/check-translations.py --list` prints the missing entries ready to paste.
- The editor's texts live in [`en.json`](../crates/typlet-editor/web/src/i18n/en.json) and [`de.json`](../crates/typlet-editor/web/src/i18n/de.json); both must have the same keys (checked by the same script). The editor's colours are CSS variables with the same values as `theme.slint`.
- Regenerate the screenshots after visible changes and commit them:

  ```sh
  cargo run -p typlet --example screenshots -- docs/images
  ```

  The example renders every window with sample data in English and German using the software renderer, so it needs no display. Editor screenshots come from `python3 scripts/editor-screenshots.py` (needs `pip install playwright pillow` and Playwright's Chromium). Finally shrink the images with `python3 scripts/optimise-images.py`.
- Icons are rendered from [`assets/logo.svg`](../assets/logo.svg) with `python3 scripts/render-icons.py` (needs `pip install cairosvg`). The PNGs are committed. The editor's icons are generated with `web/node_modules/.bin/tauri icon <1024px PNG of the logo> -o icons` in `crates/typlet-editor` (delete the `android` and `ios` folders it creates), and `web/public/logo.svg` is a copy of the logo.

## Checking Windows code on Linux

The Windows backend can be type-checked without a Windows machine. Install the standard library sources once (`rustup component add rust-src`), then:

```sh
RUSTC_BOOTSTRAP=1 cargo clippy -Zbuild-std=std,panic_abort \
  --target x86_64-pc-windows-msvc -p typlet-platform -p typlet -p typlet-editor --all-targets -- -D warnings
```

This catches compile errors and lints; running and linking still need Windows (CI does both).

## Conventions

### Commits and branches

- [Conventional Commits](https://www.conventionalcommits.org/): `feat(core): add ordered lists`, `fix(matcher): reset on focus change`, `docs: explain insert modes`.
- Branches: `feat/<topic>`, `fix/<topic>`, `docs/<topic>`.
- `main` is always releasable. Changes arrive via pull requests with green CI.

### Code

- Every source file starts with `// SPDX-License-Identifier: GPL-3.0-or-later`.
- Public items are documented (`missing_docs` is enabled).
- `typlet-core` contains no `unsafe` code and no OS access.
- Errors use `thiserror`; library code does not `unwrap` or `expect` (enforced by clippy outside tests).
- Never log or persist typed characters or snippet text. See the [security model](architecture.md#security-and-privacy-model).
- Platform code that needs `unsafe` explains each block with a `// SAFETY:` comment.

### Documentation is part of the change

A change is done when its documentation is done:

1. Update the [feature table](../README.md#-features) and the [roadmap](roadmap.md).
2. Add an entry under **Unreleased** in [`CHANGELOG.md`](../CHANGELOG.md).
3. Record architectural decisions as a new [ADR](adr/README.md).
4. Keep [snippet-format.md](snippet-format.md) and its [examples](examples/) in sync with the format; the tests load every example.
5. Add or update screenshots in the README whenever the user interface changes visibly (see above).
6. Keep the [user guide](user-guide.md) and the [test checklist](testing/manual-test-checklist.md) in step with new features.

## Upgrading the Rust toolchain

1. Change the version in [`rust-toolchain.toml`](../rust-toolchain.toml), in `rust-version` in [`Cargo.toml`](../Cargo.toml) and `RUST_TOOLCHAIN` in every workflow in [`.github/workflows`](../.github/workflows).
2. Run all checks above and fix new lints.
3. Commit as `chore: update Rust to 1.xx`.

## Releasing

See [releasing.md](releasing.md): version tags build the installers in CI and create a draft release.
