#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Renders the PNG icons used by the background program from assets/logo.svg.

Usage: python3 scripts/render-icons.py   (requires `pip install cairosvg`)

The generated files are committed, so building Typlet does not need Python.
"""

from pathlib import Path

import cairosvg

ROOT = Path(__file__).resolve().parent.parent
LOGO = (ROOT / "assets" / "logo.svg").read_text(encoding="utf-8")
OUT = ROOT / "crates" / "typlet" / "ui" / "images"

# The paused icon replaces the brand gradient with neutral greys.
PAUSED = LOGO.replace("#5B5BF0", "#8E939C").replace("#A24DF2", "#5F646D")


def render(svg: str, name: str, size: int) -> None:
    cairosvg.svg2png(
        bytestring=svg.encode("utf-8"),
        write_to=str(OUT / name),
        output_width=size,
        output_height=size,
    )


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    render(LOGO, "tray.png", 64)
    render(PAUSED, "tray-paused.png", 64)
    render(LOGO, "icon.png", 256)


if __name__ == "__main__":
    main()
