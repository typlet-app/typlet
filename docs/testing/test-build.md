# 🧪 Trying a test build

There are two kinds of test builds. Both are meant for testing, not for everyday use, and both are **unsigned**.

| Kind | Built by | Artifacts |
|---|---|---|
| **Installers** (recommended) | [Release](../../.github/workflows/release.yml) workflow: on pull requests that change the packaging, and on demand (**Actions → Release → Run workflow**, branch `main`) | `installers-x86_64-pc-windows-msvc` (setup `.exe`), `installers-x86_64-unknown-linux-gnu` (`.deb`, `.AppImage`) |
| **Plain programs** | [Test builds](../../.github/workflows/build.yml) workflow: every pull request and every change on `main` | `typlet-windows-x64`, `typlet-linux-x64` |

Download: open the workflow run on GitHub → **Artifacts**. The installers are used exactly like the released ones; see [Installing](../user-guide.md#installing) in the user guide. The rest of this page is about the plain programs.

## Windows 10/11

1. Unzip the download, for example into `Documents\Typlet`. It contains the background program `typlet.exe` and the editor `typlet-editor.exe`; keep them in the same folder.
2. Double-click `typlet.exe` (or `typlet-editor.exe`, which starts the background program too).
3. Windows SmartScreen warns about an unknown publisher, because test builds are not signed. Click **More info → Run anyway**. On PCs with *Smart App Control* switched on, unsigned programs cannot run at all; use a PC without it until the signed beta (see [ADR 0008](../adr/0008-code-signing.md)).
4. The set-up assistant opens on the first start. Type the example trigger in its test field.
5. Typlet lives in the notification area next to the clock (click the **^** arrow if the icon is hidden).

Typlet stores its data in `%APPDATA%\Typlet`. To start fresh, quit Typlet and delete that folder.

## Linux (Ubuntu 22.04 or newer)

1. Unpack the download and make the programs executable: `chmod +x typlet typlet-editor`. The editor needs WebKitGTK (`libwebkit2gtk-4.1-0`, installed on Ubuntu by default).
2. Give your user access to keyboards and virtual keyboards once, then **log out and back in**:

   ```sh
   sudo usermod -aG input $USER
   sudo cp 60-typlet-uinput.rules /etc/udev/rules.d/
   sudo udevadm control --reload-rules && sudo udevadm trigger
   ```

3. Start `./typlet`. The set-up assistant shows whether access works.
4. The tray icon needs a desktop with StatusNotifier support. Ubuntu ships it; on other GNOME systems install the *AppIndicator and KStatusNotifierItem Support* extension.

Typlet stores its data in `~/.config/typlet` and `~/.local/share/typlet`. The log is `~/.local/share/typlet/typlet.log`; it never contains typed text.

## What to test

Work through the [manual test checklist](manual-test-checklist.md) and report results in an issue, including the report from **Diagnostics → Copy report**.
