#!/usr/bin/env python3
"""Missing-Tauri / ACL-denied visibility e2e — regression guard for the 2.0.x
capabilities bug.

Every existing e2e mocks window.__TAURI__, which is exactly how 2.0.1/2.0.2
shipped green while real installs had every plugin:* call ACL-denied. This
test is the blind-spot fix:

  1. NO __TAURI__ at all (plain-browser context)  → persistent env banner
  2. __TAURI__ present but event.listen rejects    → ACL-denied banner
  3. wheel.html with no __TAURI__                  → visible error card
  4. wheel.html with listen denied                 → visible error card

A silent failure must never again be the only signal.

Run:  python3 tests/e2e/no_tauri_e2e.py
Reqs: python3 -m playwright (browsers installed)
"""

import functools
import http.server
import socketserver
import sys
import threading
from pathlib import Path

from playwright.sync_api import sync_playwright

SRC = Path(__file__).resolve().parents[2] / "src"

LANG_ZH = "localStorage.setItem('pk-lang', 'zh-CN');"

# Bridge present (app commands would work) but plugin:* is ACL-denied —
# exactly what a missing capabilities/ dir produces on a real install.
ACL_DENIED = """
window.__TAURI__ = {
  core: { invoke: (cmd) => Promise.resolve(null) },
  event: {
    listen: (name) => Promise.reject(
      `Command plugin:event|listen not allowed by ACL (${name})`),
    emit: () => Promise.reject('Command plugin:event|emit not allowed by ACL'),
  },
  window: { getCurrentWindow: () => ({}) },
};
"""


class Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *a):
        pass


def serve():
    handler = functools.partial(Quiet, directory=str(SRC))
    srv = socketserver.ThreadingTCPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv, srv.server_address[1]


CHECKS = []


def check(name, cond, extra=""):
    CHECKS.append((name, bool(cond), extra))
    print(f"  {'PASS' if cond else 'FAIL'}  {name}" + (f"  [{extra}]" if extra and not cond else ""))


def main():
    srv, port = serve()
    with sync_playwright() as p:
        browser = p.chromium.launch()

        # ---- 1. main window, no __TAURI__ injected at all ----
        page = browser.new_page()
        page.add_init_script(LANG_ZH)
        page.goto(f"http://127.0.0.1:{port}/index.html")
        page.locator("#envBanner:not(.hidden)").wait_for(timeout=5000)
        check("no-bridge: banner visible", page.locator("#envBanner").is_visible())
        check("no-bridge: title names the missing bridge",
              "桥接" in (page.locator("#envBannerTitle").text_content() or "")
              or "Tauri" in (page.locator("#envBannerTitle").text_content() or ""))
        check("no-bridge: body is actionable",
              "IPC" in (page.locator("#envBannerBody").text_content() or ""))
        kind = page.evaluate("window.__PK_IPC_ENV__?.kind")
        check("no-bridge: probe reports kind", kind == "no-bridge", f"got {kind!r}")
        page.close()

        # ---- 2. main window, bridge present but plugin:* ACL-denied ----
        page = browser.new_page()
        page.add_init_script(LANG_ZH)
        page.add_init_script(ACL_DENIED)
        page.goto(f"http://127.0.0.1:{port}/index.html")
        page.locator("#envBanner:not(.hidden)").wait_for(timeout=5000)
        check("acl-denied: banner visible", page.locator("#envBanner").is_visible())
        check("acl-denied: title names capabilities",
              "capabilities" in (page.locator("#envBannerTitle").text_content() or ""))
        body = page.locator("#envBannerBody").text_content() or ""
        check("acl-denied: body quotes the denied command",
              "plugin:event|listen" in body or "ACL" in body)
        kind = page.evaluate("window.__PK_IPC_ENV__?.kind")
        check("acl-denied: probe reports kind", kind == "acl-denied", f"got {kind!r}")
        page.close()

        # ---- 3. wheel window, no __TAURI__ ----
        page = browser.new_page()
        page.add_init_script(LANG_ZH)
        page.goto(f"http://127.0.0.1:{port}/wheel.html")
        page.locator("#envErr.show").wait_for(timeout=5000)
        check("wheel no-bridge: error card visible", page.locator("#envErr").is_visible())
        check("wheel no-bridge: card explains",
              "桥接" in (page.locator("#envErr").text_content() or ""))
        check("wheel no-bridge: wheel itself stays hidden",
              "show" not in (page.locator("#wheel").get_attribute("class") or ""))
        page.close()

        # ---- 4. wheel window, listen ACL-denied ----
        page = browser.new_page()
        page.add_init_script(LANG_ZH)
        page.add_init_script(ACL_DENIED)
        page.goto(f"http://127.0.0.1:{port}/wheel.html")
        page.locator("#envErr.show").wait_for(timeout=5000)
        check("wheel acl-denied: error card visible", page.locator("#envErr").is_visible())
        check("wheel acl-denied: card names ACL",
              "ACL" in (page.locator("#envErr").text_content() or ""))
        page.close()

        browser.close()
    srv.shutdown()

    failed = [n for n, ok, _ in CHECKS if not ok]
    print(f"\n{len(CHECKS) - len(failed)}/{len(CHECKS)} checks passed")
    if failed:
        print("FAILED:", ", ".join(failed))
        sys.exit(1)
    print("ALL PASS")


if __name__ == "__main__":
    main()
