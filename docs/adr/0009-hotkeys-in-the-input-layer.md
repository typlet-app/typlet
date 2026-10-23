# 0009. Global hotkeys are detected in the input layer

- Status: Accepted
- Date: 2026-10-23

## Context

Typlet needs global shortcuts: one for the quick search, one for "new snippet from selection" and optional shortcuts per snippet. Each operating system has its own registration API: `RegisterHotKey` on Windows, the XDG *GlobalShortcuts* portal on Wayland, nothing usable on GNOME before version 48. These APIs need a window or message loop of their own, fail silently when another program owns the shortcut and, on Wayland, ask the user to confirm every shortcut in a system dialog.

Typlet already reads every key press to recognise triggers.

## Decision

Hotkeys are recognised by the same code that reads key presses: the Windows keyboard hook and the Linux evdev reader compare each key press with the registered hotkeys and send a `Hotkey` event to the engine.

- On **Windows** the hook also swallows the key press, so the focused app does not see it.
- On **Linux** evdev cannot withhold keys; the app also receives the shortcut. The defaults (`Ctrl+Shift+Space`, `Ctrl+Alt+S`) do nothing in common apps.
- Letters are matched by their position-independent meaning on the user's layout, so `Ctrl+Alt+S` works on QWERTY and QWERTZ alike.

## Consequences

- One code path and one behaviour on every platform; no system dialogs, works on GNOME Wayland today.
- Shortcuts can be changed at runtime by replacing a list, without re-registering anything.
- Typlet cannot detect that another program uses the same shortcut; both react on Linux, Typlet wins on Windows.
- If the platform cannot read keys (missing permissions), hotkeys are unavailable too; the set-up assistant shows this.
