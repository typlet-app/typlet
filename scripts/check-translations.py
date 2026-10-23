#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks that every user-visible text in the Slint UI has a German translation.

Usage: python3 scripts/check-translations.py [--list]

Finds all @tr("...") strings in crates/typlet/ui/*.slint and compares them
with crates/typlet/lang/de/LC_MESSAGES/typlet.po. Exits with an error if a
translation is missing or empty, and warns about translations nobody uses.
With --list it prints the missing entries in .po syntax, ready to fill in.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
UI = ROOT / "crates" / "typlet" / "ui"
PO = ROOT / "crates" / "typlet" / "lang" / "de" / "LC_MESSAGES" / "typlet.po"

TR = re.compile(r'@tr\(\s*"((?:[^"\\]|\\.)*)"(?:\s*\|\s*"((?:[^"\\]|\\.)*)")?')


def ui_strings():
    found = {}
    for path in sorted(UI.glob("*.slint")):
        for match in TR.finditer(path.read_text(encoding="utf-8")):
            found[match.group(1)] = match.group(2)
    return found


def po_entries():
    """Returns msgid -> list of translations (one per plural form)."""
    entries = {}
    msgid = None
    current = None
    for line in PO.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith("msgid "):
            msgid = line[6:].strip('"')
            current = None
        elif line.startswith("msgid_plural "):
            continue
        elif line.startswith("msgstr"):
            current = line.split(" ", 1)[1].strip('"')
            entries.setdefault(msgid, []).append(current)
        elif line.startswith('"') and msgid is not None and current is None:
            msgid += line.strip('"')
    entries.pop("", None)
    return entries


def main():
    strings = ui_strings()
    entries = po_entries()
    missing = [s for s in strings if not any(entries.get(s, []))]
    unused = [s for s in entries if s not in strings]
    for text in unused:
        print(f"warning: unused translation: {text}")
    if "--list" in sys.argv:
        for text in missing:
            plural = strings[text]
            print(f'msgid "{text}"')
            if plural:
                print(f'msgid_plural "{plural}"\nmsgstr[0] ""\nmsgstr[1] ""\n')
            else:
                print('msgstr ""\n')
    for text in missing:
        print(f"error: missing German translation: {text}", file=sys.stderr)
    print(f"{len(strings)} texts, {len(missing)} missing, {len(unused)} unused")
    return 1 if missing else 0


if __name__ == "__main__":
    sys.exit(main())
