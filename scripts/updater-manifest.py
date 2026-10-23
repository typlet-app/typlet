#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Writes latest.json, the file the updater reads (ADR 0015).

Usage: scripts/updater-manifest.py DIST TAG REPOSITORY [CHANGELOG]

DIST holds the installers and their updater signatures (FILE.sig, written by
`tauri signer sign`). The manifest lists every installer that has a
signature; without signatures (no signing key configured) nothing is written.
The release notes are the CHANGELOG section of the version, if there is one.
"""

import datetime
import json
import pathlib
import re
import sys

# Updater targets: `{os}-{arch}-{bundle}` as the updater looks them up, and
# the plain `{os}-{arch}` as a fallback for Windows.
TARGETS = [
    ("*-setup.exe", ["windows-x86_64-nsis", "windows-x86_64"]),
    ("*.deb", ["linux-x86_64-deb"]),
    ("*.AppImage", ["linux-x86_64-appimage"]),
]


def release_notes(changelog: pathlib.Path, version: str) -> str:
    """The CHANGELOG section `## [version] …` without its heading."""
    if not changelog.is_file():
        return ""
    text = changelog.read_text(encoding="utf-8")
    match = re.search(
        rf"^## \[{re.escape(version)}\][^\n]*\n(.*?)(?=^## \[|^\[[^\]]+\]: |\Z)",
        text,
        flags=re.M | re.S,
    )
    return match.group(1).strip() if match else ""


def manifest(dist: pathlib.Path, tag: str, repository: str, notes: str) -> dict | None:
    base = f"https://github.com/{repository}/releases/download/{tag}/"
    platforms = {}
    for pattern, targets in TARGETS:
        for installer in sorted(dist.glob(pattern)):
            signature = installer.with_name(installer.name + ".sig")
            if not signature.is_file():
                continue
            entry = {
                "signature": signature.read_text(encoding="utf-8").strip(),
                "url": base + installer.name,
            }
            for target in targets:
                platforms[target] = entry
    if not platforms:
        return None
    version = tag.removeprefix("v")
    now = datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0)
    return {
        "version": version,
        "notes": notes or f"Typlet {version}: https://github.com/{repository}/releases/tag/{tag}",
        "pub_date": now.isoformat().replace("+00:00", "Z"),
        "platforms": platforms,
    }


def main() -> int:
    if len(sys.argv) not in (4, 5):
        print(__doc__, file=sys.stderr)
        return 2
    dist, tag, repository = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3]
    changelog = pathlib.Path(sys.argv[4] if len(sys.argv) == 5 else "CHANGELOG.md")
    data = manifest(dist, tag, repository, release_notes(changelog, tag.removeprefix("v")))
    if data is None:
        print("no updater signatures found; latest.json is not written")
        return 0
    (dist / "latest.json").write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    print(f"latest.json: {', '.join(sorted(data['platforms']))}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
