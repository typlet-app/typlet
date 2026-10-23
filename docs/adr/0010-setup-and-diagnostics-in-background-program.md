# 0010. Set-up assistant and diagnostics live in the background program

- Status: Accepted (supersedes the set-up assistant part of [ADR 0003](0003-separate-editor-process.md))
- Date: 2026-10-23

## Context

[ADR 0003](0003-separate-editor-process.md) placed the editor, the settings *and the set-up assistant* in the separate editor program. While building the background program it became clear that the assistant and the diagnostics depend on things only the background program knows:

- whether keyboard access and typing actually work (live platform status, engine errors),
- whether a trigger typed into a test field really expands, which proves the whole pipeline end to end,
- what happened during the last expansions (which app, typed or pasted, skipped, failed).

The assistant must also work before the editor is installed or when the editor cannot start (for example without WebView2).

## Decision

The **set-up assistant** (welcome, permission check, test field, start-up option) and the **diagnostics window** are Slint windows in the background program. They are opened on first start, from the tray menu and from error messages.

The editor keeps snippet editing and the full settings, as decided in ADR 0003.

## Consequences

- The test field exercises the real engine, hook and injection; "It works!" means it works.
- Diagnostics can show live status without any communication between the two programs.
- Two small windows more in the native UI; they reuse the shared design tokens.
- The diagnostics history is kept in memory only and contains app names and outcomes, never text.
