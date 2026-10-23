# 0015. Automatic updates through the editor

- Status: Accepted
- Date: 2026-10-23

## Context

Typlet is made for people who do not follow release pages. Without updates, every fix has to be downloaded and installed by hand, and most users would stay on their first version for good.

At the same time, the background program sees every keystroke and has no network code ([ADR 0002](0002-native-background-process.md)). That promise must stay: it is the main reason a text expander can be trusted.

The editor already uses Tauri, which offers a signed updater for NSIS installers, Debian packages and AppImages.

## Decision

- **Only the editor connects to the network**, and only to check for and download updates. The background program stays offline.
- **Once a day** the background program starts `typlet-editor --check-updates`. The editor checks without a window, writes the result to `update.json` in the data folder and exits. The background program reads that file, shows "Install update …" in the tray menu and announces a new version once. The check can be switched off (`check_for_updates` in the settings).
- **Installing** happens in the editor's update page (from the tray, the message, the home page or the settings). Windows runs the new installer in passive mode, which stops the background program, installs and restarts the editor. On Linux the updater replaces the AppImage or installs the Debian package (asking for the password through `pkexec`); the editor then asks the background program to quit (a `quit-request` file next to the settings, which the background program watches) and restarts. The restarted editor starts the new background program.
- **Updates are signed** with a minisign key that belongs to the project (Tauri's updater signatures, independent of Windows code signing). The public key is in `tauri.conf.json`; the private key is a GitHub secret. Signatures include the version, and the updater requires that (`requireSignedVersion`), so an old installer cannot be served as a new one. A test verifies a fixture signed with the release key against the configured public key.
- **The update feed** is `latest.json` in the newest GitHub release (`releases/latest/download/latest.json`), written by the release workflow after Windows code signing. Releases only become visible to the updater when a maintainer publishes the draft. Because GitHub's "latest" skips pre-releases, 0.x versions are normal releases titled "Beta"; only versions with a suffix (`1.0.0-rc.1`) are pre-releases.
- The updater is built without its ZIP support: Typlet ships plain installers, and the ZIP library had open security advisories.

## Consequences

- Users get fixes without visiting GitHub; an update is one click and about a minute.
- The editor contacts GitHub once a day (the IP address is visible to GitHub, like any web request). Nothing about the user's snippets is sent. The privacy section of the user guide says so, and the check can be turned off.
- If the private key is lost, installed copies can no longer update themselves; users would have to install the next version by hand. The key therefore has an offline backup ([releasing](../releasing.md#update-signing-key)).
- Development builds cannot update themselves; their update page links to the download page instead.
