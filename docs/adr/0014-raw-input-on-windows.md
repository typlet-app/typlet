# 0014. Raw Input and registered shortcuts on Windows

- Status: Accepted
- Date: 2026-10-23
- Supersedes: the Windows part of [ADR 0009](0009-hotkeys-in-the-input-layer.md)

## Context

Typlet read the keyboard on Windows with low-level keyboard and mouse hooks (`SetWindowsHookEx` with `WH_KEYBOARD_LL`, `WH_MOUSE_LL`). A hook sits in the path of every key press of every program, may change or swallow it, and is the classic technique of keyloggers. Typlet used the swallowing for two things: its shortcuts ([ADR 0009](0009-hotkeys-in-the-input-layer.md)) and an exact undo, where the Backspace right after an expansion never reached the app.

On a managed company laptop the unsigned Typlet could not be installed, while espanso could. espanso is signed ([ADR 0008](0008-code-signing.md)), and it reads the keyboard with **Raw Input** instead of a hook. Security software watches unknown programs that install global keyboard hooks closely. Raw input has further advantages:

- It only reads. Windows hands the program a copy of each key press; nothing runs in the path of other programs' input.
- There is no time limit. Windows silently removes a hook that answers too slowly; raw input messages simply wait in the queue.
- It is the documented way to read keyboards in the background (`RIDEV_INPUTSINK`).

## Decision

- On **Windows**, a hidden window on its own thread registers for raw keyboard and mouse input with `RIDEV_INPUTSINK`. Key presses are translated with `ToUnicodeEx` as before. Modifier state is tracked from the raw events themselves, because the messages arrive after the fact ([`raw_keys.rs`](../../crates/typlet-platform/src/raw_keys.rs), unit-tested on every platform). Simulated input has no device and is ignored. Mouse clicks reset matching as before.
- Typlet's **shortcuts** are registered with `RegisterHotKey`. Windows reports them as `WM_HOTKEY` and keeps them from the focused app. If another program already owns a shortcut, Typlet tells the user once and tries again every ten seconds.
- The **Backspace after an expansion reaches the app**, as on Linux. The undo deletes one character less and restores the trigger; the result is the same.
- Raw input is registered per process, and the Slint interface (winit) registers it too. A timer checks every second that the registration still points at Typlet's input window and restores it if not. Slint does not use raw input events.
- Linux is unchanged: evdev only reads, and shortcuts are detected in the evdev stream ([ADR 0009](0009-hotkeys-in-the-input-layer.md)).

## Consequences

- Typlet uses the same input technique as espanso; together with code signing this is the best chance to pass company security policies.
- Shortcuts that another program owns are reported instead of being taken away from it.
- Between a key press and Typlet's reaction the user's Backspace is visible for a moment during an undo; the end result is unchanged.
- Raw input does not see input into programs running as administrator, which Typlet cannot type into anyway without UIAccess ([ADR 0008](0008-code-signing.md)).
- Modifier releases that happen on another desktop (lock screen, user account control) are not reported; a timer reconciles stale modifiers with the real keyboard state after half a second without key presses.
