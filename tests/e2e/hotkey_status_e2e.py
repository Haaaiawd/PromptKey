#!/usr/bin/env python3
"""Hotkey self-check visibility e2e (brief Task 3, failure-visibility regression guard).

Serves src/ statically, stubs window.__TAURI__.core.invoke, loads the app,
opens Settings, and asserts that a FAILED hotkey registration produces a
visible error — the bug being fixed was exactly "registration failed, toast
said saved".

Run:  python3 tests/e2e/hotkey_status_e2e.py
Reqs: python3 -m playwright (browsers installed)
"""

import functools
import http.server
import json
import socketserver
import sys
import threading
from pathlib import Path

from playwright.sync_api import sync_playwright

SRC = Path(__file__).resolve().parents[2] / "src"

# Installed before any page script runs: Tauri IPC stub + Chinese UI.
INIT = """
localStorage.setItem('pk-lang', 'zh-CN');
window.__TAURI__ = {
  core: {
    invoke: (cmd, args) => {
      const d = window.__MOCK_DATA__ || {};
      switch (cmd) {
        case 'check_hotkeys':       return Promise.resolve(d.report);
        case 'get_settings':        return Promise.resolve(d.settings);
        case 'get_app_settings':    return Promise.resolve({ default_prompt_mode: 'last_used', default_prompt_id: 0 });
        case 'get_prompts_view':    return Promise.resolve([]);
        case 'check_service_status': return Promise.resolve(true);
        case 'apply_settings':
          return d.applyError ? Promise.reject(d.applyError) : Promise.resolve('设置已保存并已重启服务');
        default: return Promise.resolve(null);
      }
    }
  }
};
"""

BASE_SETTINGS = {
    "hotkey": "Ctrl+Alt+Space",
    "quick_hotkey": "Ctrl+Alt+Q",
    "allow_clipboard": True,
    "restore_clipboard": True,
    "secure_gate": False,
}


def report(**kw):
    d = {
        "engine": "running",
        "engine_error": None,
        "hotkeys": [
            {"id": 4, "combo": "Ctrl+Alt+Space", "canonical": "Ctrl+Alt+Space",
             "status": "ok", "detail": None},
            {"id": 5, "combo": "Ctrl+Alt+Q", "canonical": "Ctrl+Alt+Q",
             "status": "ok", "detail": None},
        ],
    }
    d.update(kw)
    return d


class Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *a):
        pass


def serve():
    handler = functools.partial(Quiet, directory=str(SRC))
    srv = socketserver.ThreadingTCPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv, srv.server_address[1]


def open_settings(page, port, data):
    page.add_init_script(INIT)
    page.add_init_script(f"window.__MOCK_DATA__ = {json.dumps(data)};")
    page.goto(f"http://127.0.0.1:{port}/index.html")
    page.click('.nav-item[data-view="settings"]')
    # wait until the self-check resolves (checking text replaced)
    page.wait_for_function(
        "document.querySelector('#hotkeyWheelState')?.textContent.length > 0 "
        "&& !document.querySelector('#hotkeyWheelState')?.textContent.includes('…')")


CHECKS = []


def check(name, cond, extra=""):
    CHECKS.append((name, bool(cond), extra))
    print(f"  {'PASS' if cond else 'FAIL'}  {name}" + (f"  [{extra}]" if extra and not cond else ""))


def main():
    srv, port = serve()
    with sync_playwright() as p:
        browser = p.chromium.launch()

        # ---- 1. conflict: wheel hotkey held by another app ----
        page = browser.new_page()
        rep = report(hotkeys=[
            {"id": 4, "combo": "Ctrl+Alt+Space", "canonical": "Ctrl+Alt+Space",
             "status": "conflict", "detail": "combo is held by another application"},
            {"id": 5, "combo": "Ctrl+Alt+Q", "canonical": "Ctrl+Alt+Q",
             "status": "ok", "detail": None},
        ])
        open_settings(page, port, {"settings": BASE_SETTINGS, "report": rep})
        w = page.locator("#hotkeyWheelState")
        check("conflict: wheel state visible", w.is_visible())
        check("conflict: wheel state has err class", "err" in (w.get_attribute("class") or ""))
        check("conflict: wheel state says occupied", "占用" in (w.text_content() or ""))
        q = page.locator("#hotkeyQuickState")
        check("conflict: quick state ok", "已生效" in (q.text_content() or ""))
        page.close()

        # ---- 2. ok: both registered ----
        page = browser.new_page()
        open_settings(page, port, {"settings": BASE_SETTINGS, "report": report()})
        check("ok: wheel state shows active",
              "已生效" in (page.locator("#hotkeyWheelState").text_content() or ""))
        check("ok: wheel state ok class",
              "ok" in (page.locator("#hotkeyWheelState").get_attribute("class") or ""))
        page.close()

        # ---- 3. engine failed (the env_logger-panic regression) ----
        page = browser.new_page()
        rep = report(engine="failed",
                     engine_error="attempted to set a logger after the logging system was already initialized")
        open_settings(page, port, {"settings": BASE_SETTINGS, "report": rep})
        w = page.locator("#hotkeyWheelState")
        check("engine failed: wheel state err", "err" in (w.get_attribute("class") or ""))
        check("engine failed: mentions service error", "服务异常" in (w.text_content() or ""))
        page.close()

        # ---- 4. record flow: apply_settings ok but registration conflict ----
        # The fields are key-capture recorders now — drive the real chord
        # instead of fill()+change (readonly inputs reject fill()).
        page = browser.new_page()
        rep = report(hotkeys=[
            {"id": 4, "combo": "Ctrl+Alt+F9", "canonical": "Ctrl+Alt+F9",
             "status": "conflict", "detail": "combo is held by another application"},
            {"id": 5, "combo": "Ctrl+Alt+Q", "canonical": "Ctrl+Alt+Q",
             "status": "ok", "detail": None},
        ])
        open_settings(page, port, {"settings": BASE_SETTINGS, "report": rep})
        inp = page.locator("#hotkeyInput")
        inp.click()
        page.keyboard.press("Control+Alt+F9")
        err_toast = page.locator("#toasts .toast.err")
        err_toast.first.wait_for(timeout=5000)
        check("save: error toast on inactive hotkey", "热键未生效" in (err_toast.first.text_content() or ""))
        w = page.locator("#hotkeyWheelState")
        check("save: wheel state shows conflict", "占用" in (w.text_content() or ""))
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
