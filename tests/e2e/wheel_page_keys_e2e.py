#!/usr/bin/env python3
"""Wheel pagination hotkey e2e — regression guard for the page-2 digit bug.

The wheel pages 6 petals at a time and badges each petal with a page-local
digit (1..n). The bug: on page 2 the digits 7/8/9/0 fell through the
selection branch into the typing filter — `query += '7'` plus `page = 0`
snapped the user back to page 1 of a "7"-filtered list. The key contract is
now uniform: every digit key selects the petal showing that badge; a digit
with no petal gets an on-screen hint and NEVER touches the query.

Covers:
  1. page 1 shows 6 petals badged 1..6, center reads "1/2"
  2. digit 3 on page 1 injects prompt #3 (first-page regression)
  3. ArrowRight -> page 2: petals badged 1..2, center reads "2/2"
  4. digit 1 on page 2 injects global item #7 (badge = key, page-local)
  5. digit 7/8/0 injects nothing, leaves no .filter-tag, shows a range toast
  6. ArrowRight on the last page is a harmless no-op (no crash, no filter)
  7. ArrowLeft returns to page 1
  8. digit past the visible count on a short page (5 on a 2-petal page) hints
  9. letters still filter (x -> filter-tag, empty-tip), Backspace clears
 10. a digit pressed while the filter has no matches does NOT append to query

Run:  python3 tests/e2e/wheel_page_keys_e2e.py
Reqs: python3 -m playwright (browsers installed)
"""

import functools
import http.server
import json
import socketserver
import sys
import threading
import time
from pathlib import Path

from playwright.sync_api import sync_playwright

SRC = Path(__file__).resolve().parents[2] / "src"

# 8 pinned prompts; auto wheel sort is frecency-desc, so ids 1..8 map to
# page 1 [1..6] and page 2 [7,8]. No {{vars}} in content -> pick injects
# directly via trigger_wheel_injection.
PROMPTS = [
    {"id": i, "name": f"P{i}", "content": f"body {i}", "tags": [],
     "is_pinned": 1, "usage_count": 0, "frecency": 100 - i * 10,
     "last_used_at": 1000 - i, "content_type": "text", "variables_json": None,
     "app_scopes_json": None, "inject_order": None,
     "version": 1, "updated_at": "2026-10-01 10:00"}
    for i in range(1, 9)
]

INIT = """
localStorage.setItem('pk-lang', 'zh-CN');
window.__CALLS__ = [];
window.__LISTENERS__ = {};
window.__TAURI__ = {
  core: {
    invoke: (cmd, args) => {
      window.__CALLS__.push({ cmd, promptId: args && args.promptId });
      switch (cmd) {
        case 'get_prompts_view': return Promise.resolve(JSON.parse(JSON.stringify(%s)));
        case 'check_service_status': return Promise.resolve(true);
        default: return Promise.resolve(null);
      }
    }
  },
  event: {
    listen: (name, cb) => { window.__LISTENERS__[name] = cb; return Promise.resolve(() => {}); }
  }
};
""" % json.dumps(PROMPTS)


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


def wait_js(page, expr, timeout=5.0):
    """Poll `expr` in the page until truthy. More reliable than
    wait_for_function on this near-static wheel page, where rAF-driven
    polling can stall before first paint."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        if page.evaluate(expr):
            return True
        page.wait_for_timeout(40)
    return False


def show_wheel(page):
    page.evaluate("window.__LISTENERS__['wheel-show']()")
    page.wait_for_selector("#wheel.show .petal")


def petal_names(page):
    return page.eval_on_selector_all("#wheel .petal .pn", "els => els.map(e => e.textContent)")


def petal_badges(page):
    return page.eval_on_selector_all("#wheel .petal .k", "els => els.map(e => e.textContent)")


def inject_calls(page):
    return page.evaluate("window.__CALLS__.filter(c => c.cmd === 'trigger_wheel_injection')")


def wait_inject(page, n):
    assert wait_js(
        page, f"window.__CALLS__.filter(c => c.cmd === 'trigger_wheel_injection').length === {n}")


def center_text(page):
    return page.locator("#centerTxt").text_content() or ""


def main():
    srv, port = serve()
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page()
        page.add_init_script(INIT)
        page.goto(f"http://127.0.0.1:{port}/wheel.html")
        assert wait_js(page, "!!(window.__LISTENERS__ && window.__LISTENERS__['wheel-show'])")

        # ---- 1. open -> page 1, six petals badged 1..6 ----
        show_wheel(page)
        check("page 1: six petals", len(petal_names(page)) == 6, str(petal_names(page)))
        check("page 1: badges 1..6", petal_badges(page) == ["1", "2", "3", "4", "5", "6"],
              str(petal_badges(page)))
        check("page 1: names P1..P6 (frecency order)",
              petal_names(page) == ["P1", "P2", "P3", "P4", "P5", "P6"], str(petal_names(page)))
        check("page 1: center shows 1/2", "1/2" in center_text(page), center_text(page))

        # ---- 2. first-page regression: digit 3 picks P3 ----
        page.keyboard.press("3")
        wait_inject(page, 1)
        calls = inject_calls(page)
        check("digit 3 on page 1 injects P3", calls[-1]["promptId"] == 3, str(calls))
        page.wait_for_timeout(350)  # let hide() settle before re-showing

        # ---- 3. ArrowRight -> page 2 shows badges 1..2 (page-local) ----
        show_wheel(page)
        page.keyboard.press("ArrowRight")
        assert wait_js(page, "document.querySelectorAll('#wheel .petal').length === 2")
        check("page 2: two petals P7,P8", petal_names(page) == ["P7", "P8"], str(petal_names(page)))
        check("page 2: badges are page-local 1,2", petal_badges(page) == ["1", "2"],
              str(petal_badges(page)))
        check("page 2: center shows 2/2", "2/2" in center_text(page), center_text(page))

        # ---- 4. digit 1 on page 2 picks the FIRST petal of page 2 (P7) ----
        page.keyboard.press("1")
        wait_inject(page, 2)
        calls = inject_calls(page)
        check("digit 1 on page 2 injects P7 (badge=key)",
              calls[-1]["promptId"] == 7, str(calls))
        page.wait_for_timeout(350)

        # ---- 5. digits 7/8/0 never select and never pollute the filter ----
        show_wheel(page)
        page.keyboard.press("ArrowRight")
        assert wait_js(page, "document.querySelectorAll('#wheel .petal').length === 2")
        before = len(inject_calls(page))
        for key in ("7", "8", "0"):
            page.keyboard.press(key)
            page.wait_for_timeout(120)
        check("digits 7/8/0 inject nothing", len(inject_calls(page)) == before,
              str(inject_calls(page)))
        check("digits 7/8/0 leave no filter-tag",
              page.locator("#wheel .filter-tag").count() == 0)
        check("digits 7/8/0 keep page 2 rendered",
              petal_names(page) == ["P7", "P8"], str(petal_names(page)))
        check("out-of-range digit shows range hint toast",
              page.locator("#toasts .toast").count() >= 1
              and "1–2" in (page.locator("#toasts").text_content() or ""),
              page.locator("#toasts").text_content() or "")
        check("still on page 2 after stray digits", "2/2" in center_text(page),
              center_text(page))

        # ---- 5b. holding an out-of-range digit must NOT stack toasts ----
        # Regression guard for Devin Review on PR #18: keydown auto-repeat used
        # to spawn one toast per repeat (up to 5), and the stacked toasts sit
        # over the lower petals with pointer events enabled -> clicks hit a
        # toast instead of selecting a prompt. Keyed toasts coalesce into one.
        page.keyboard.down("7")
        page.wait_for_timeout(700)          # let OS key-repeat fire
        page.keyboard.up("7")
        page.wait_for_timeout(120)
        n_toasts = page.locator("#toasts .toast").count()
        check("held out-of-range digit coalesces to one toast", n_toasts == 1,
              f"toasts={n_toasts}")
        check("toast does not intercept petal clicks",
              "none" == page.evaluate(
                  "() => getComputedStyle("
                  "document.querySelector('#toasts .toast')).pointerEvents"),
              page.evaluate(
                  "() => getComputedStyle("
                  "document.querySelector('#toasts .toast')).pointerEvents"))

        # ---- 6. ArrowRight on last page: no crash, no wrap, no filter ----
        page.keyboard.press("ArrowRight")
        page.wait_for_timeout(120)
        check("last-page ArrowRight is a no-op",
              petal_names(page) == ["P7", "P8"] and "2/2" in center_text(page)
              and page.locator("#wheel .filter-tag").count() == 0,
              f"{petal_names(page)} {center_text(page)}")

        # ---- 7. ArrowLeft returns to page 1 ----
        page.keyboard.press("ArrowLeft")
        assert wait_js(page, "document.querySelectorAll('#wheel .petal').length === 6")
        check("ArrowLeft back to page 1", "1/2" in center_text(page)
              and petal_names(page) == ["P1", "P2", "P3", "P4", "P5", "P6"],
              f"{petal_names(page)} {center_text(page)}")

        # ---- 8. digit past visible count on a short page still hints ----
        page.keyboard.press("ArrowRight")
        assert wait_js(page, "document.querySelectorAll('#wheel .petal').length === 2")
        n_toasts = page.locator("#toasts .toast").count()
        n_inject = len(inject_calls(page))
        page.keyboard.press("5")
        page.wait_for_timeout(120)
        # wave44-review: a keyed toast REPLACES the live one instead of stacking,
        # so the count may stay flat — assert the hint is present and correct
        # (and still nothing injected), not that the stack grew.
        check("digit 5 on 2-petal page hints, does not inject",
              len(inject_calls(page)) == n_inject
              and "1–2" in (page.locator("#toasts").text_content() or ""),
              page.locator("#toasts").text_content() or "")

        # ---- 9. letters still filter; 10. digit never appends to query ----
        page.keyboard.press("x")  # zero matches -> empty-tip, petals gone
        page.wait_for_selector("#wheel .empty-tip")
        check("letter x filters (filter-tag shown)",
              "x" in (page.locator("#wheel .filter-tag").text_content() or ""))
        before_q = page.locator("#wheel .filter-tag").text_content()
        page.keyboard.press("9")
        page.wait_for_timeout(120)
        check("digit 9 does not append to query",
              (page.locator("#wheel .filter-tag").text_content() or "") == before_q,
              page.locator("#wheel .filter-tag").text_content() or "")
        page.keyboard.press("Backspace")
        assert wait_js(page, "document.querySelectorAll('#wheel .petal').length === 6")
        check("Backspace clears filter back to page 1",
              petal_names(page) == ["P1", "P2", "P3", "P4", "P5", "P6"],
              str(petal_names(page)))

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
