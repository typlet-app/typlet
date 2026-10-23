# 🤝 Contributing to Typlet

Thank you for helping! Typlet should become the text expander anyone can use, and that needs many kinds of help:

- 🐛 **Testing**: try Typlet in the apps you use every day and report what breaks.
- 💻 **Code**: pick an issue labelled `good first issue` or `help wanted`.
- 📝 **Documentation**: fix unclear explanations, add examples.
- 🌍 **Translations**: once the user interface exists.
- 💡 **Ideas**: open a feature request.

Issues and discussions may be written in English or German. Code, comments, commit messages and documentation in the repository are in English.

## Before you start

- For anything larger than a small fix, **open an issue first** so we can agree on the approach.
- Read the [architecture](docs/architecture.md) and the [decision records](docs/adr/README.md).
- Set up your environment as described in [docs/development.md](docs/development.md).

## Workflow

1. Fork the repository and create a branch: `feat/<topic>`, `fix/<topic>` or `docs/<topic>`.
2. Make your change with tests.
3. Run the checks locally:

   ```sh
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

4. Update the documentation (see below).
5. Open a pull request. The title follows [Conventional Commits](https://www.conventionalcommits.org/), for example `feat(core): support ordered lists`.
6. Sign the CLA when the bot asks (once per contributor).

## Documentation is part of every change

A pull request is complete only when:

- the [feature table](README.md#-features) and the [roadmap](docs/roadmap.md) reflect the change,
- [`CHANGELOG.md`](CHANGELOG.md) has an entry under **Unreleased**,
- architectural decisions are recorded as a new [ADR](docs/adr/README.md),
- format changes are reflected in [docs/snippet-format.md](docs/snippet-format.md) and its examples,
- visible UI changes come with updated screenshots.

## Contributor License Agreement

Before your first contribution can be merged, you sign the [Typlet CLA](CLA.md). You keep the copyright to your work; you grant the project the rights it needs to distribute it, including under other licences in the future. The CLA also guarantees that your contribution stays available under an open-source licence. Signing happens with one comment in your pull request.

## Code style

- Formatting: `rustfmt` with the repository settings.
- Lints: clippy must be clean with `-D warnings`.
- Every source file starts with `// SPDX-License-Identifier: GPL-3.0-or-later`.
- Public items are documented.
- No `unwrap`/`expect` in library code; use proper errors.
- Never log, print or store what a user types.

## Reporting bugs

Use the bug report template. Helpful details: operating system and version, the app where the problem happens, what you typed, what you expected and what happened.

Security issues must **not** be reported publicly. See [SECURITY.md](SECURITY.md).

## Code of conduct

Everyone taking part follows our [Code of Conduct](CODE_OF_CONDUCT.md).
