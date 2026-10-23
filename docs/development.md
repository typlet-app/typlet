# 🛠️ Development

## Prerequisites

| Tool | Why | Install |
|---|---|---|
| Rust via rustup | Compiler; the version is pinned in [`rust-toolchain.toml`](../rust-toolchain.toml) and installed automatically | <https://rustup.rs> |
| Git | Version control | Your package manager |
| cargo-deny *(optional)* | Licence and advisory checks, also run in CI | `cargo install cargo-deny --locked` |

Platform dependencies for the user interface (Slint, Tauri, WebKitGTK) will be listed here when those crates are added in milestones M2–M4.

Typlet is developed on **Ubuntu 26.04 LTS (GNOME, Wayland)** and **Windows 11**. Both are covered by CI.

## Everyday commands

```sh
cargo test --workspace                                      # all tests
cargo clippy --workspace --all-targets -- -D warnings       # lints, as strict as CI
cargo fmt --all                                             # format
cargo doc --workspace --no-deps --open                      # API documentation
cargo deny check                                            # licences, advisories, sources
```

CI runs exactly these checks on Linux and Windows for every push and pull request.

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
- Never log or persist typed characters. See the [security model](architecture.md#security-and-privacy-model).

### Documentation is part of the change

A change is done when its documentation is done:

1. Update the [feature table](../README.md#-features) and the [roadmap](roadmap.md).
2. Add an entry under **Unreleased** in [`CHANGELOG.md`](../CHANGELOG.md).
3. Record architectural decisions as a new [ADR](adr/README.md).
4. Keep [snippet-format.md](snippet-format.md) and its [examples](examples/) in sync with the format; the tests load every example.
5. Add or update screenshots in the README whenever the user interface changes visibly.

## Upgrading the Rust toolchain

1. Change the version in [`rust-toolchain.toml`](../rust-toolchain.toml), in `rust-version` in [`Cargo.toml`](../Cargo.toml) and in the CI workflow.
2. Run all checks above and fix new lints.
3. Commit as `chore: update Rust to 1.xx`.

## Releasing

Releases are tagged `vMAJOR.MINOR.PATCH` following [Semantic Versioning](https://semver.org). The release workflow (build, sign, publish installers) will be added with milestone M5 and documented here.
