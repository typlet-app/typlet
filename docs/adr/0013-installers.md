# 0013. One installer per platform, built by the Tauri bundler

- Status: Accepted
- Date: 2026-10-23

## Context

Typlet consists of two programs that must be installed together: the background program (`typlet`) and the editor (`typlet-editor`). Users without admin rights (company laptops) must be able to install it. On Linux, keyboard access needs a udev rule and membership in the `input` group.

The editor is a Tauri application, and Tauri's bundler already produces Windows (NSIS, MSI), Debian and AppImage packages, including icons, start menu entries, desktop files and uninstallers.

## Decision

- The **Tauri bundler** builds all installers from the editor crate, with the background program included as a **sidecar** binary (`externalBin`), installed next to the editor.
- **Windows:** an NSIS installer in per-user mode (`%LOCALAPPDATA%\Typlet`, no admin rights). Installer hooks stop a running background program before updating or uninstalling and remove the start-at-login entry on uninstall. User data in `%APPDATA%\Typlet` is kept.
- **Linux:** a Debian package that installs the udev rule for `/dev/uinput`, reloads udev and, when installed with `sudo`, adds the installing user to the `input` group; and an AppImage for other distributions, which needs the one-time set-up shown in the set-up assistant.
- The start menu / desktop entry opens the **editor**, which starts the background program if needed. At login the background program starts directly (Windows: `HKCU\…\Run`; Linux: an autostart entry, which for the AppImage calls the editor with `--autostart`, starting only the background program).
- Bundle settings live in [`packaging/tauri.release.json`](../../packaging/tauri.release.json) and are merged in only for release builds, so everyday builds do not need the sidecar.
- Releases are built by CI from version tags and published as drafts for review ([releasing.md](../releasing.md)).

## Consequences

- One tool for all packages; no separate installer scripts to maintain.
- Per-user installation works on locked-down company PCs, but programs running as administrator cannot receive simulated input from an unsigned, non-UIAccess Typlet ([ADR 0008](0008-code-signing.md)).
- The AppImage is large (about 90 MB) because it carries WebKitGTK; the Debian package uses the system's.
- MSIX for the Microsoft Store needs a separate packaging step (see [releasing.md](../releasing.md)).
