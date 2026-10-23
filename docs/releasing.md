# 🚀 Releasing

Releases are built by the [Release workflow](../.github/workflows/release.yml) when a version tag is pushed. It produces the installers ([ADR 0013](adr/0013-installers.md)) and a **draft** GitHub release; nothing is published until a maintainer publishes the draft.

| File | Built on | Contents |
|---|---|---|
| `Typlet_<version>_x64-setup.exe` | `windows-latest` | NSIS installer, per user, no admin rights |
| `Typlet_<version>_amd64.deb` | `ubuntu-22.04` | Debian package with the udev rule; adds the installing user to `input` |
| `Typlet_<version>_amd64.AppImage` | `ubuntu-22.04` | Portable Linux build |
| `latest.json` | | The update feed: version, notes and the updater signature of every installer ([ADR 0015](adr/0015-automatic-updates.md)) |
| `SHA256SUMS.txt` | | Checksums |

Every build also installs and uninstalls its packages once on the runner (smoke test) and lists the files in the job summary. The same build runs for pull requests that change the packaging, and on demand from **Actions → Release → Run workflow**, without creating a release; testers find the installers under the run's **Artifacts** ([test builds](testing/test-build.md)).

Versions follow [Semantic Versioning](https://semver.org). `0.x` versions are titled **Beta** but published as normal releases, because the updater reads `latest.json` from the latest normal release; versions with a suffix (`1.0.0-rc.1`) become pre-releases, which the updater ignores. Installed copies see a new version only after the draft is published.

## Checklist

1. All CI checks on `main` are green and the [manual test checklist](testing/manual-test-checklist.md) has been run on Windows and Linux with a test build.
2. On a branch `release/vX.Y.Z`:
   - set `version` in [`Cargo.toml`](../Cargo.toml) (workspace) and in [`crates/typlet-editor/web/package.json`](../crates/typlet-editor/web/package.json); run `cargo build` to update `Cargo.lock`,
   - in [`CHANGELOG.md`](../CHANGELOG.md), rename **Unreleased** to `[X.Y.Z] – YYYY-MM-DD`, add a new empty **Unreleased** section and the comparison links,
   - review [`docs/release-notes.md`](release-notes.md) (the text of the GitHub release),
   - open a pull request `chore: release X.Y.Z` and merge it.
3. Tag the merge commit and push the tag:

   ```sh
   git tag -a vX.Y.Z -m "Typlet X.Y.Z"
   git push origin vX.Y.Z
   ```

   The workflow refuses to build if the tag and `Cargo.toml` disagree.
4. When the workflow is done, open the draft under **Releases**, check the files and the text, and publish it.

## Update signing key

Installed copies only accept updates signed with the project's update key ([ADR 0015](adr/0015-automatic-updates.md)). Its public half is `plugins.updater.pubkey` in [`tauri.conf.json`](../crates/typlet-editor/tauri.conf.json); the private half lives in two repository secrets (**Settings → Secrets and variables → Actions**):

| Secret | Content |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | The private key (one line of base64) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Its password |

The Release workflow signs every installer after Windows code signing, including its version (`tauri signer sign --app-version`), and writes `latest.json` ([`scripts/updater-manifest.py`](../scripts/updater-manifest.py)). Without the secrets the installers are built without updater signatures and the release has no `latest.json`.

**Keep an offline backup of the key and its password** (for example in a password manager). If the key is lost, installed copies cannot update themselves any more and every user has to install the next version by hand. If the key leaks, generate a new pair (`tauri signer generate`), replace the public key and the secrets, and publish a release that users install by hand once.

## Building installers locally

```sh
cargo build --release -p typlet
mkdir -p crates/typlet-editor/binaries
cp target/release/typlet crates/typlet-editor/binaries/typlet-$(rustc -vV | sed -n 's/host: //p')   # add .exe on Windows
cd crates/typlet-editor/web && npm ci && cd ..
node web/node_modules/@tauri-apps/cli/tauri.js build --config ../../packaging/tauri.release.json --bundles deb   # or nsis, appimage
```

The packages end up in `target/release/bundle/`.

## Code signing

Until signing is set up, Windows shows "unknown publisher" and Smart App Control blocks the installer ([ADR 0008](adr/0008-code-signing.md)). The workflow already contains the signing step; it runs as soon as these are configured:

1. Apply for the free open-source plan of the [SignPath Foundation](https://signpath.org) with this repository.
2. In SignPath, create the project `typlet` with a signing policy `release-signing` and an artifact configuration that signs the NSIS installer (and, for signed program files, the PE files `typlet.exe` and `typlet-editor.exe` inside it).
3. In the GitHub repository settings, add the variable `SIGNPATH_ORGANIZATION_ID` and the secret `SIGNPATH_API_TOKEN`.

SignPath only signs files built by GitHub Actions from this repository, which is why the release is built in CI.

## Microsoft Store

The Store signs packages itself, which removes the SmartScreen warning without a certificate. Steps (not automated yet):

1. Register a free individual developer account in the Microsoft Partner Center.
2. Reserve the name "Typlet".
3. Either submit the signed NSIS installer as a Win32 app (requires code signing, see above) or create an MSIX package with the MSIX Packaging Tool from the installer and submit that (the Store signs MSIX).
4. Note for the listing: Typlet reads keyboard input to recognise triggers; the privacy policy is the "Privacy" section of the [user guide](user-guide.md#privacy).
