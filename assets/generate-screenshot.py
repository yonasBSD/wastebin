#!/usr/bin/env -S uv run --quiet
# /// script
# requires-python = ">=3.11"
# dependencies = ["playwright>=1.50", "pillow>=10"]
# ///
"""Regenerate assets/screenshot.webp.

Boots a temporary wastebin server, posts `crates/wastebin_server/src/main.rs`
as a paste, captures the paste view in the light and dark themes via headless
Chromium with lines 5-8 anchored (`#L5-L8`) to showcase line highlighting,
then composites them along a top-left → bottom-right diagonal (dark
upper-right triangle, light lower-left triangle).

Run `uv run assets/generate-screenshot.py` from the repo root. On first use,
Playwright's Chromium must be installed with `uv run --with playwright
playwright install chromium`. The script expects the server binary at
`target/release/wastebin`; run `cargo build --release -p wastebin` first.
"""

import asyncio
import contextlib
import json
import os
import signal
import socket
import subprocess
import sys
import tempfile
import time
import urllib.parse
import urllib.request
from pathlib import Path

from PIL import Image
from playwright.async_api import async_playwright

REPO = Path(__file__).resolve().parent.parent
BINARY = REPO / "target" / "release" / "wastebin"
PASTE_SOURCE = REPO / "crates" / "wastebin_server" / "src" / "main.rs"
OUTPUT = REPO / "assets" / "screenshot.webp"
HIGHLIGHT_FRAGMENT = "#L5-L8"

WIDTH, HEIGHT = 1714, 900
# Integer device scale factor keeps glyph rasterization sharp and yields exact
# output dimensions without any PIL resize.
DSF = 2
VIEWPORT_W, VIEWPORT_H = WIDTH // DSF, HEIGHT // DSF


def pick_free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


@contextlib.contextmanager
def wastebin_server(port: int, db_path: Path):
    env = {
        **os.environ,
        "WASTEBIN_ADDRESS_PORT": f"127.0.0.1:{port}",
        "WASTEBIN_DATABASE_PATH": str(db_path),
    }
    proc = subprocess.Popen(
        [str(BINARY)],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        preexec_fn=os.setsid,
    )
    try:
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=0.5)
                break
            except Exception:
                time.sleep(0.1)
        else:
            raise RuntimeError("wastebin server did not become ready in time")
        yield
    finally:
        os.killpg(proc.pid, signal.SIGTERM)
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            os.killpg(proc.pid, signal.SIGKILL)


def post_paste(base_url: str, source: Path) -> str:
    payload = json.dumps(
        {"text": source.read_text(), "extension": "rs", "title": "main.rs"}
    ).encode()
    req = urllib.request.Request(
        base_url + "/",
        data=payload,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req) as resp:
        path = json.loads(resp.read())["path"]
    # The id can contain characters (notably '+') that must be percent-encoded.
    head, _, tail = path.rpartition("/")
    return head + "/" + urllib.parse.quote(tail, safe=".")


async def capture(paste_url: str, pref: str, out_path: Path) -> None:
    async with async_playwright() as p:
        browser = await p.chromium.launch()
        context = await browser.new_context(
            viewport={"width": VIEWPORT_W, "height": VIEWPORT_H},
            device_scale_factor=DSF,
        )
        await context.add_cookies(
            [
                {
                    "name": "pref",
                    "value": pref,
                    "domain": "127.0.0.1",
                    "path": "/",
                    "sameSite": "Strict",
                }
            ]
        )
        page = await context.new_page()
        await page.goto(paste_url, wait_until="networkidle")
        # paste.js reads window.location.hash on load and adds the
        # .line-highlight class to the matching rows. scrollIntoView for the
        # range start is clamped by the scroll container, so lines near the
        # top stay at the top.
        await page.wait_for_timeout(200)
        await page.screenshot(path=str(out_path), full_page=False, type="png")
        await browser.close()


def diagonal_composite(light_path: Path, dark_path: Path, out_path: Path) -> None:
    light = Image.open(light_path).convert("RGBA")
    dark = Image.open(dark_path).convert("RGBA")
    assert light.size == (WIDTH, HEIGHT), light.size
    assert dark.size == (WIDTH, HEIGHT), dark.size

    # Dark fills the upper-right triangle (TL, TR, BR); diagonal runs TL→BR.
    mask = Image.new("L", (WIDTH, HEIGHT), 0)
    px = mask.load()
    for y in range(HEIGHT):
        x_boundary = int(round(WIDTH * y / HEIGHT))
        for x in range(x_boundary, WIDTH):
            px[x, y] = 255

    Image.composite(dark, light, mask).convert("RGB").save(
        out_path, "WEBP", lossless=True, method=6
    )


async def run() -> None:
    if not BINARY.exists():
        sys.exit(
            f"error: {BINARY.relative_to(REPO)} not found; "
            "run `cargo build --release -p wastebin` first"
        )

    port = pick_free_port()
    base_url = f"http://127.0.0.1:{port}"

    with tempfile.TemporaryDirectory(prefix="wastebin-screenshot-") as tmp:
        tmp = Path(tmp)
        db_path = tmp / "state.db"
        with wastebin_server(port, db_path):
            paste_path = post_paste(base_url, PASTE_SOURCE)
            paste_url = base_url + paste_path + HIGHLIGHT_FRAGMENT

            light_png = tmp / "light.png"
            dark_png = tmp / "dark.png"
            await capture(paste_url, "light", light_png)
            await capture(paste_url, "dark", dark_png)
            diagonal_composite(light_png, dark_png, OUTPUT)

    print(f"wrote {OUTPUT.relative_to(REPO)}")


if __name__ == "__main__":
    asyncio.run(run())
