#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Shrinks screenshots by converting them to a 256-colour palette.

Usage: python3 scripts/optimise-images.py [folder]   (default docs/images)

Interface screenshots have few colours, so a palette keeps them crisp at
about a third of the size. Requires Pillow (`pip install pillow`).
"""

import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent


def optimise(folder: Path) -> None:
    before = after = 0
    for path in sorted(folder.rglob("*.png")):
        before += path.stat().st_size
        image = Image.open(path)
        if image.mode != "P":
            image = image.convert("RGB").quantize(
                colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE
            )
        image.save(path, optimize=True)
        after += path.stat().st_size
    print(f"{folder}: {before // 1024} KB -> {after // 1024} KB")


if __name__ == "__main__":
    optimise(Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "docs" / "images")
