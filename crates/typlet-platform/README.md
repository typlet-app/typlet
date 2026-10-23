# typlet-platform

The operating-system layer of [Typlet](../../README.md). Every backend offers the same capabilities to the engine:

| Capability | Windows | Linux |
|---|---|---|
| Reading keys | Low-level keyboard and mouse hooks | evdev (`/dev/input`), user in the `input` group |
| Keys → characters | `ToUnicodeEx` without touching dead-key state | xkbcommon with the user's layout and compose rules |
| Typing | `SendInput` with Unicode key events | uinput virtual keyboard |
| Clipboard | Text, `HTML Format`, `Rich Text Format`; all formats restored; hidden from clipboard history | Text and HTML via the X11 clipboard (bridged to Wayland) |
| Focused app | Executable name; password fields via UI Automation | Not available on GNOME Wayland |
| Hotkeys | Detected and suppressed in the keyboard hook | Detected in the evdev stream |
| Autostart | `HKCU\…\Run` | `~/.config/autostart/typlet.desktop` |

Typed characters are passed to the engine and forgotten. They are never logged, stored or transmitted.

Platform-independent helpers (`paths`, `instance`, `autostart`) and the virtual-key mapping (`vk`, unit-tested on every OS) live next to the backends. See [docs/architecture.md](../../docs/architecture.md#platform-backends) for the design.

## Running the tests

```sh
cargo test -p typlet-platform
```

The backends themselves are tested manually with the [test checklist](../../docs/testing/manual-test-checklist.md), because they need real keyboards and applications.
