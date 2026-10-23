# 🛠️ Development

## Prerequisites

| Tool | Why | Install |
|---|---|---|
| Rust via rustup | Compiler; the version is pinned in [`rust-toolchain.toml`](../rust-toolchain.toml) and installed automatically | <https://rustup.rs> |
| Git | Version control | Your package manager |
| Linux libraries | Keyboard layouts, fonts and Wayland for the background program | `sudo apt install libxkbcommon-dev libfontconfig1-dev libwayland-dev` |
| Python 3 *(optional)* | Translation check and icon rendering scripts | Your package manager |
| cargo-deny *(optional)* | Licence and advisory checks, also run in CI | `cargo install cargo-deny --locked` |

On Windows, the Visual Studio Build Tools (C++ workload) are needed, as for any Rust project. The editor (milestone M4) will add Node.js and WebKitGTK; they will be listed here then.

Typlet is developed on **Ubuntu 26.04 LTS (GNOME, Wayland)** and **Windows 11**. Both are covered by CI.

## Everyday commands

```sh
cargo test --workspace                                      # all tests
cargo clippy --workspace --all-targets -- -D warnings       # lints, as strict as CI
cargo fmt --all                                             # format
cargo doc --workspace --no-deps --open                      # API documentation
cargo deny check                                            # licences, advisories, sources
python3 scripts/check-translations.py                       # every text translated
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

## Working on the user interface

- Windows are written in [Slint](https://slint.dev) in [`crates/typlet/ui`](../crates/typlet/ui). Colours, spacing and type sizes come from [`theme.slint`](../crates/typlet/ui/theme.slint); see [design.md](design.md).
- Write every visible text in English with `@tr("…")`, then add the German translation to [`typlet.po`](../crates/typlet/lang/de/LC_MESSAGES/typlet.po). `python3 scripts/check-translations.py --list` prints the missing entries ready to paste.
- Regenerate the screenshots after visible changes and commit them:

  ```sh
  cargo run -p typlet --example screenshots -- docs/images
  ```

  The example renders every window with sample data in English and German using the software renderer, so it needs no display.
- Icons are rendered from [`assets/logo.svg`](../assets/logo.svg) with `python3 scripts/render-icons.py` (needs `pip install cairosvg`). The PNGs are committed.

## Checking Windows code on Linux

The Windows backend can be type-checked without a Windows machine. Install the standard library sources once (`rustup component add rust-src`), then:

```sh
RUSTC_BOOTSTRAP=1 cargo clippy -Zbuild-std=std,panic_abort \
  --target x86_64-pc-windows-msvc -p typlet-platform -p typlet --all-targets -- -D warnings
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

1. Change the version in [`rust-toolchain.toml`](../rust-toolchain.toml), in `rust-version` in [`Cargo.toml`](../Cargo.toml) and in the CI workflow.
2. Run all checks above and fix new lints.
3. Commit as `chore: update Rust to 1.xx`.

## Releasing

Releases are tagged `vMAJOR.MINOR.PATCH` following [Semantic Versioning](https://semver.org). The release workflow (build, sign, publish installers) will be added with milestone M5 and documented here.
