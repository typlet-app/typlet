# 🧭 Architecture decision records

An ADR captures one significant decision: the context, what was decided and what follows from it. ADRs are never rewritten after acceptance. A changed decision gets a new ADR that supersedes the old one.

| # | Decision | Status |
|---|---|---|
| [0001](0001-rewrite-in-rust.md) | Build from scratch in Rust instead of forking espanso | Accepted |
| [0002](0002-native-background-process.md) | Native background process with Slint, no browser engine | Accepted |
| [0003](0003-separate-editor-process.md) | Rich-text editor as a separate Tauri + Tiptap program | Accepted |
| [0004](0004-gpl-license-with-cla.md) | GPL-3.0-or-later with a contributor licence agreement | Accepted |
| [0005](0005-file-per-snippet-storage.md) | One JSON file per snippet, folders as categories | Accepted |
| [0006](0006-v1-platform-scope.md) | V1 platforms: Windows as product, Linux at developer quality | Accepted |
| [0007](0007-instant-expansion.md) | Instant expansion, longest match, backspace undo | Accepted |
| [0008](0008-code-signing.md) | Unsigned beta, then free signing via SignPath and the Microsoft Store | Accepted |

## Writing a new ADR

Copy the structure of an existing record, use the next number and keep it short:

```markdown
# NNNN. Title

- Status: Proposed | Accepted | Superseded by NNNN
- Date: YYYY-MM-DD

## Context
## Decision
## Consequences
```
