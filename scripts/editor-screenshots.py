#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Takes screenshots of the editor for the documentation.

Usage: python3 scripts/editor-screenshots.py [output-dir]   (default docs/images)

Builds the editor's web interface in demo mode (sample snippets, no Rust
backend), serves it locally and photographs it with a headless Chromium.
Requires Node.js, `pip install playwright pillow` and Playwright's Chromium.
"""

import os
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parent.parent
WEB = ROOT / "crates" / "typlet-editor" / "web"
OUT = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "docs" / "images"
PORT = 4173
BASE = f"http://localhost:{PORT}/"

# Snippet ids from src/demo.ts.
SIGNATURE = "019a0f3c-5d2e-7a10-8b3c-0a1b2c3d4e03"
FORM = "019a0f3c-5d2e-7a10-8b3c-0a1b2c3d4e04"
THANKS = "019a0f3c-5d2e-7a10-8b3c-0a1b2c3d4e06"

SHOTS = [
    ("editor.png", f"open={SIGNATURE}", None),
    ("editor-form.png", f"open={FORM}", None),
    ("editor-date.png", f"open={FORM}", ".chip-date"),
    ("editor-nested.png", f"open={THANKS}", ".chip-snippet"),
    ("editor-settings.png", "settings", None),
    ("editor-update.png", "update", None),
]


def wait_for_server() -> None:
    for _ in range(100):
        try:
            urllib.request.urlopen(BASE, timeout=1)
            return
        except OSError:
            time.sleep(0.1)
    raise SystemExit("preview server did not start")


def main() -> None:
    vite = str(WEB / "node_modules" / "vite" / "bin" / "vite.js")
    subprocess.run(
        ["node", vite, "build", "--outDir", "dist-demo", "--logLevel", "warn"],
        cwd=WEB,
        check=True,
        env={**os.environ, "VITE_DEMO": "1"},
    )
    server = subprocess.Popen(
        ["node", vite, "preview", "--outDir", "dist-demo", "--port", str(PORT), "--strictPort"],
        cwd=WEB,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        wait_for_server()
        with sync_playwright() as p:
            browser = p.chromium.launch()
            page = browser.new_page(
                viewport={"width": 1120, "height": 740},
                device_scale_factor=2,
                color_scheme="light",
            )
            for lang in ("en", "de"):
                (OUT / lang).mkdir(parents=True, exist_ok=True)
                for name, query, click in SHOTS:
                    page.goto(f"{BASE}?lang={lang}&{query}")
                    page.wait_for_selector(".sidebar", timeout=10_000)
                    page.wait_for_timeout(600)
                    if click:
                        page.click(click)
                        page.wait_for_timeout(200)
                    path = OUT / lang / name
                    page.screenshot(path=str(path))
                    print(f"wrote {path}", flush=True)
            browser.close()
    finally:
        server.terminate()
    subprocess.run([sys.executable, str(ROOT / "scripts" / "optimise-images.py"), str(OUT)], check=True)


if __name__ == "__main__":
    main()
