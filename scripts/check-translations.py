#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks that every user-visible text has a German translation.

Usage: python3 scripts/check-translations.py [--list]

Background program: finds all @tr("...") strings in crates/typlet/ui/*.slint
and compares them with crates/typlet/lang/de/LC_MESSAGES/typlet.po.
With --list it prints the missing entries in .po syntax, ready to fill in.

Editor: compares the keys of crates/typlet-editor/web/src/i18n/en.json and
de.json and rejects empty texts.

Exits with an error if a translation is missing or empty, and warns about
translations nobody uses.
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
UI = ROOT / "crates" / "typlet" / "ui"
PO = ROOT / "crates" / "typlet" / "lang" / "de" / "LC_MESSAGES" / "typlet.po"
EDITOR_I18N = ROOT / "crates" / "typlet-editor" / "web" / "src" / "i18n"

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


def check_editor():
    """Returns the number of problems in the editor's translation tables."""
    en_path, de_path = EDITOR_I18N / "en.json", EDITOR_I18N / "de.json"
    if not en_path.exists():
        return 0
    en = json.loads(en_path.read_text(encoding="utf-8"))
    de = json.loads(de_path.read_text(encoding="utf-8"))
    problems = 0
    for key in sorted(set(en) - set(de)):
        print(f"error: editor text without German translation: {key}", file=sys.stderr)
        problems += 1
    for key in sorted(set(de) - set(en)):
        print(f"error: German editor text without English original: {key}", file=sys.stderr)
        problems += 1
    for table, name in ((en, "English"), (de, "German")):
        for key, value in table.items():
            if not value.strip():
                print(f"error: empty {name} editor text: {key}", file=sys.stderr)
                problems += 1
    print(f"editor: {len(en)} texts, {problems} problems")
    return problems


def main():
    editor_problems = check_editor()
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
    print(f"background program: {len(strings)} texts, {len(missing)} missing, {len(unused)} unused")
    return 1 if missing or editor_problems else 0


if __name__ == "__main__":
    sys.exit(main())
