# 0012. The editor signals its focus with a lock file

- Status: Accepted
- Date: 2026-10-23

## Context

The background program expands triggers in every application. In the editor this is wrong: a user typing `:mfg` as the trigger of a new snippet, or writing about a trigger in a snippet's text, must not have it replaced.

On Windows the background program knows the focused application and could switch itself off for `typlet-editor.exe`. On GNOME Wayland it cannot see the focused application at all. The two programs deliberately have no network connection or IPC channel ([ADR 0003](0003-separate-editor-process.md)).

## Decision

While the editor window has the keyboard focus, the editor holds an exclusive lock on `editor-focus.lock` in Typlet's data folder; it releases the lock when the window loses the focus or closes. Before inserting a snippet, the engine checks whether the lock is held and skips the expansion if so.

The check uses the same operating-system file lock as the single-instance check (`File::try_lock`), so a crashed editor never leaves a stale lock behind.

## Consequences

- Works identically on Windows and Linux, including GNOME Wayland, without IPC.
- The check costs one file open per *expansion*, not per key press.
- Hotkeys (quick search) still work while the editor has the focus.
- Other Typlet windows with text fields (search, forms, new snippet) use the engine's in-process "quiet" mode instead; the set-up assistant's test field deliberately expands.
