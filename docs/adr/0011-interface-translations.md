# 0011. Interface texts in English with bundled gettext translations

- Status: Accepted
- Date: 2026-10-23

## Context

Typlet's first users speak German; the repository language is English and more languages should be easy to add later. The background program must start instantly and offline, so translations cannot be downloaded or loaded from separate files that might be missing.

## Decision

- Interface texts are written in **English** directly in the `.slint` files with Slint's `@tr()` macro.
- Translations are standard **gettext `.po` files** (`crates/typlet/lang/<language>/LC_MESSAGES/typlet.po`) that the build compiles into the program (*bundled translations*). Texts are keyed by the English text only, without per-window contexts, so a text used in several windows is translated once.
- The language follows the system unless the user picks one in the settings. German and English ship with V1.
- Messages that come from the program (errors, diagnostics results) are passed to the interface as *kinds* plus technical details, and the interface translates the kind.
- `scripts/check-translations.py` fails CI when a German translation is missing.

## Consequences

- Translators work with ordinary `.po` tools; no Rust knowledge needed.
- No translation files to install or lose; switching languages takes effect immediately.
- Technical details inside messages (for example operating-system error texts) stay English; they are meant for support, not as the main message.
- The editor (Tauri) uses its own translation files; both follow the same glossary in [docs/design.md](../design.md).
