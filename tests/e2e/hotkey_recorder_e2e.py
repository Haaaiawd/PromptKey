#!/usr/bin/env python3
"""Hotkey recorder e2e (P0 fix: settings fields capture chords, not text).

Serves src/ statically, stubs window.__TAURI__, opens Settings, and drives the
readonly .hotkey-input fields with real key events:
  - Ctrl+Alt+Space records "Ctrl+Alt+Space" and commits via apply_settings
  - Escape cancels and restores the previous combo (never clears it)
  - bare Space is rejected ("needs a modifier") and stays in recording
  - typing letters cannot insert characters into the field
  - Numpad keys normalize to NumN / NumAdd (the Rust canonical spellings)
  - a conflicting combo is surfaced AND rolled back, not silently saved

Run:  python3 tests/e2e/hotkey_recorder_e2e.py
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

# Tauri IPC stub. apply_settings records the applied hotkey so check_hotkeys
# can answer dynamically: Ctrl+Alt+F9 is always "held by another app", every
# other combo registers fine.
INIT = """
localStorage.setItem('pk-lang', 'zh-CN');
window.__CALLS__ = [];
window.__MOCK_DATA__ = window.__MOCK_DATA__ || {};
window.__TAURI__ = {
  core: {
    invoke: (cmd, args) => {
      const d = window.__MOCK_DATA__;
      window.__CALLS__.push({ cmd, args });
      switch (cmd) {
        case 'get_settings':
          return Promise.resolve({
            hotkey: 'Ctrl+Alt+Space', quick_hotkey: 'Ctrl+Alt+Q',
            allow_clipboard: true, restore_clipboard: true, secure_gate: false });
        case 'get_app_settings':
          return Promise.resolve({ default_prompt_mode: 'last_used', default_prompt_id: 0 });
        case 'get_prompts_view':    return Promise.resolve([]);
        case 'check_service_status': return Promise.resolve(true);
        case 'apply_settings':
          d.lastHotkey = (args && args.hotkey) || d.lastHotkey;
          return d.applyError ? Promise.reject(d.applyError)
                              : Promise.resolve('设置已保存并已重启服务');
        case 'check_hotkeys': {
          const hk = d.lastHotkey || 'Ctrl+Alt+Space';
          const bad = hk === 'Ctrl+Alt+F9';
          return Promise.resolve({ engine: 'running', engine_error: null, hotkeys: [
            { id: 4, combo: hk, canonical: hk,
              status: bad ? 'conflict' : 'ok',
              detail: bad ? 'combo is held by another application' : null },
            { id: 5, combo: 'Ctrl+Alt+Q', canonical: 'Ctrl+Alt+Q',
              status: 'ok', detail: null },
          ]});
        }
        default: return Promise.resolve(null);
      }
    }
  }
};
"""

BASE_SETTINGS = {
    "hotkey": "Ctrl+Alt+Space",
    "quick_hotkey": "Ctrl+Alt+Q",
}


class Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *a):
        pass


def serve():
    handler = functools.partial(Quiet, directory=str(SRC))
    srv = socketserver.ThreadingTCPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv, srv.server_address[1]


def open_settings(browser, port, data=None):
    page = browser.new_page()
    page.add_init_script(INIT)
    if data is not None:
        page.add_init_script(f"window.__MOCK_DATA__ = {json.dumps(data)};")
    page.goto(f"http://127.0.0.1:{port}/index.html")
    page.click('.nav-item[data-view="settings"]')
    page.wait_for_function(
        "document.querySelector('#hotkeyWheelState')?.textContent.length > 0 "
        "&& !document.querySelector('#hotkeyWheelState')?.textContent.includes('…')")
    page.evaluate("window.__CALLS__ = []")  # ignore boot traffic
    return page


CHECKS = []


def check(name, cond, extra=""):
    CHECKS.append((name, bool(cond), extra))
    print(f"  {'PASS' if cond else 'FAIL'}  {name}" + (f"  [{extra}]" if extra and not cond else ""))


def apply_calls(page):
    return page.evaluate("window.__CALLS__.filter(c => c.cmd === 'apply_settings')")


def main():
    srv, port = serve()
    with sync_playwright() as p:
        browser = p.chromium.launch()

        # ---- 1. record Ctrl+Alt+Space ----
        page = open_settings(browser, port)
        inp = page.locator("#hotkeyInput")
        inp.click()
        check("recording: armed class", "recording" in (inp.get_attribute("class") or ""))
        check("recording: hint shown",
              "组合键" in (page.locator("#hotkeyWheelState").text_content() or ""))
        page.keyboard.press("Control+Alt+Space")
        page.wait_for_function("document.querySelector('#hotkeyInput').value === 'Ctrl+Alt+Space'")
        check("record: value is Ctrl+Alt+Space", inp.input_value() == "Ctrl+Alt+Space")
        calls = page.evaluate("window.__CALLS__")
        check("record: apply_settings committed",
              any(c["cmd"] == "apply_settings" and c["args"].get("hotkey") == "Ctrl+Alt+Space"
                  for c in calls), extra=json.dumps(calls))
        check("record: left recording", "recording" not in (inp.get_attribute("class") or ""))
        page.close()

        # ---- 2. Escape cancels and restores ----
        page = open_settings(browser, port)
        inp = page.locator("#hotkeyInput")
        inp.click()
        check("esc: field cleared while armed", inp.input_value() == "")
        page.keyboard.down("Control")
        page.keyboard.press("Escape")
        page.keyboard.up("Control")
        check("esc: original restored", inp.input_value() == "Ctrl+Alt+Space")
        check("esc: cancel hint", "已取消" in (page.locator("#hotkeyWheelState").text_content() or ""))
        check("esc: nothing committed", len(apply_calls(page)) == 0)
        page.close()

        # ---- 3. bare Space is rejected, still recording ----
        page = open_settings(browser, port)
        inp = page.locator("#hotkeyInput")
        inp.click()
        page.keyboard.press("Space")
        check("bare key: need-modifier hint",
              "修饰键" in (page.locator("#hotkeyWheelState").text_content() or ""))
        check("bare key: still recording", "recording" in (inp.get_attribute("class") or ""))
        check("bare key: field untouched", inp.input_value() == "Ctrl+Alt+Space")
        check("bare key: nothing committed", len(apply_calls(page)) == 0)
        page.keyboard.press("Escape")
        page.close()

        # ---- 4. recording never types characters ----
        page = open_settings(browser, port)
        inp = page.locator("#hotkeyInput")
        inp.click()
        page.keyboard.press("x")
        page.keyboard.press("Space")
        check("typing: no chars entered", inp.input_value() == "Ctrl+Alt+Space")
        check("typing: nothing committed", len(apply_calls(page)) == 0)
        page.keyboard.press("Escape")
        page.close()

        # ---- 5. numpad + function keys normalize to Rust canonical names ----
        page = open_settings(browser, port)
        inp = page.locator("#hotkeyInput")
        inp.click()
        page.keyboard.press("Control+Alt+Numpad5")
        page.wait_for_function("document.querySelector('#hotkeyInput').value.length > 0")
        check("numpad: Numpad5 → Ctrl+Alt+Num5", inp.input_value() == "Ctrl+Alt+Num5",
              extra=inp.input_value())
        inp.click()
        page.keyboard.press("Control+Alt+NumpadAdd")
        page.wait_for_function("document.querySelector('#hotkeyInput').value === 'Ctrl+Alt+NumAdd'")
        check("numpad: NumpadAdd → Ctrl+Alt+NumAdd (round-trip safe)",
              inp.input_value() == "Ctrl+Alt+NumAdd", extra=inp.input_value())
        inp.click()
        page.keyboard.press("Control+Alt+F9")
        # F9 is the mocked conflict combo — it commits, gets rejected by the
        # engine check, then the field and config roll back to NumAdd.
        page.wait_for_function(
            "document.querySelector('#hotkeyInput').value === 'Ctrl+Alt+NumAdd'",
            timeout=8000)
        calls = apply_calls(page)
        check("fkey: F9 normalized to Ctrl+Alt+F9 in the commit",
              any(c["args"].get("hotkey") == "Ctrl+Alt+F9" for c in calls),
              extra=json.dumps(calls))
        check("conflict: error toast shown",
              page.locator("#toasts .toast.err").count() > 0)
        check("conflict: bad combo rolled back", inp.input_value() == "Ctrl+Alt+NumAdd")
        page.close()

        # ---- 6. second field is independent ----
        page = open_settings(browser, port)
        qinp = page.locator("#quickHotkeyInput")
        qinp.click()
        page.keyboard.press("Control+Shift+ArrowUp")
        page.wait_for_function("document.querySelector('#quickHotkeyInput').value.length > 0")
        check("quick: Ctrl+Shift+ArrowUp → Ctrl+Shift+Up",
              qinp.input_value() == "Ctrl+Shift+Up", extra=qinp.input_value())
        calls = page.evaluate("window.__CALLS__")
        check("quick: committed via quickHotkey arg",
              any(c["cmd"] == "apply_settings" and c["args"].get("quickHotkey") == "Ctrl+Shift+Up"
                  for c in calls))
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
