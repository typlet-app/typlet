# Changelog

All notable changes to Typlet are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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

[Unreleased]: https://github.com/typlet-app/typlet/commits/main
