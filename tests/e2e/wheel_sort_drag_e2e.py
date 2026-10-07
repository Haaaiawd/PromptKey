#!/usr/bin/env python3
"""Wheel-mirror manual sort e2e — regression guard for the pointer-drag fix.

The previous implementation armed `draggable` inside mousedown and relied on
HTML5 drag-and-drop: racy in WebView2, and hard-cancelled by preventDefault()
whenever wheelSort was 'auto' (the default) — users saw a grip icon but could
never drag. This test drives REAL pointer input (mouse.move/down/move/up), so
it fails if the rows do not physically reorder.

The mock backend is durable: prompts live in localStorage and set_pin_order
writes inject_order through (1-based strings, same as Rust's
`(i + 1).to_string()`), so a committed order survives page.reload() exactly
like the real SQLite backend — and a UI that commits but never re-renders is
caught, not masked.

Covers:
  1. plain click on a mirror row does not call set_pin_order
  2. dragging a row to a new slot reorders the DOM, calls set_pin_order once
     with the exact new id sequence, and submitted ids == rendered DOM order
  3. a committed order survives a full page reload (durable mock backend)
  4. a rejected set_pin_order rolls back: DOM returns to the pre-drag order,
     no mid-state residue, backend data untouched
  5. a drag landing in the original slot commits nothing
  6. the drawer no longer renders #fApps / #fOrder, and update_prompt passes
     app_scopes_json / inject_order through instead of wiping them
  7. keyboard reorder (ArrowDown on the grip) commits via the same path
  8. inject_order "0" is an explicit position (ranks first, not last) —
     regression guard for pinOrder's "only >0 counts" bug, where a 0-based or
     weight-0 write silently sorted to the tail and the mirror never moved

Run:  python3 tests/e2e/wheel_sort_drag_e2e.py
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

PROMPTS = [
    {"id": 1, "name": "Alpha", "content": "alpha body", "tags": ["dev"],
     "is_pinned": 1, "usage_count": 9, "frecency": 40.0, "last_used_at": 100,
     "content_type": "text", "variables_json": None,
     "app_scopes_json": '["Code.exe"]', "inject_order": "2",
     "version": 3, "updated_at": "2026-10-01 10:00"},
    {"id": 2, "name": "Beta", "content": "beta body", "tags": [],
     "is_pinned": 1, "usage_count": 5, "frecency": 30.0, "last_used_at": 90,
     "content_type": "text", "variables_json": None,
     "app_scopes_json": None, "inject_order": "1",
     "version": 1, "updated_at": "2026-10-01 10:00"},
    {"id": 3, "name": "Gamma", "content": "gamma body", "tags": [],
     "is_pinned": 1, "usage_count": 2, "frecency": 20.0, "last_used_at": 80,
     "content_type": "text", "variables_json": None,
     "app_scopes_json": None, "inject_order": "4",
     "version": 1, "updated_at": "2026-10-01 10:00"},
    {"id": 4, "name": "Delta", "content": "delta body", "tags": [],
     "is_pinned": 1, "usage_count": 1, "frecency": 10.0, "last_used_at": 70,
     "content_type": "text", "variables_json": None,
     "app_scopes_json": None, "inject_order": "3",
     "version": 1, "updated_at": "2026-10-01 10:00"},
    {"id": 5, "name": "Unpinned", "content": "not on wheel", "tags": [],
     "is_pinned": 0, "usage_count": 99, "frecency": 99.0, "last_used_at": 200,
     "content_type": "text", "variables_json": None,
     "app_scopes_json": None, "inject_order": None,
     "version": 1, "updated_at": "2026-10-01 10:00"},
]

INIT = """
localStorage.setItem('pk-lang', 'zh-CN');
window.__CALLS__ = [];
// Durable mock backend: the db lives in localStorage so writes survive a
// page reload exactly like the real SQLite backend. A window-scoped array
// would silently reset on reload and could never test persistence.
const DB_KEY = '__pk_e2e_prompts__';
let db = JSON.parse(localStorage.getItem(DB_KEY) || 'null');
if (!db) {
  db = JSON.parse(JSON.stringify(%s));
  localStorage.setItem(DB_KEY, JSON.stringify(db));
}
window.__PROMPTS__ = db;
const persist = () => localStorage.setItem(DB_KEY, JSON.stringify(db));
window.__TAURI__ = {
  core: {
    invoke: (cmd, args) => {
      switch (cmd) {
        case 'get_prompts_view':
          return Promise.resolve(JSON.parse(JSON.stringify(db)));
        case 'set_pin_order':
          window.__CALLS__.push({ cmd, ids: [...args.ids] });
          if (window.__FAIL_PIN_ORDER__) return Promise.reject('mock backend down');
          args.ids.forEach((id, i) => {
            const p = db.find(x => x.id === id);
            if (p) p.inject_order = String(i + 1);
          });
          persist();
          return Promise.resolve(null);
        case 'update_prompt': {
          window.__CALLS__.push({ cmd, prompt: args.prompt });
          const p = db.find(x => x.id === args.prompt.id);
          if (p) { Object.assign(p, args.prompt); persist(); }
          return Promise.resolve(null);
        }
        case 'toggle_prompt_pin': {
          const p = db.find(x => x.id === args.id);
          if (p) { p.is_pinned = p.is_pinned ? 0 : 1; persist(); }
          return Promise.resolve(null);
        }
        case 'check_service_status': return Promise.resolve(true);
        default: return Promise.resolve(null);
      }
    }
  },
  event: { listen: () => Promise.resolve(() => {}) }
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


def mirror_names(page):
    return page.eval_on_selector_all(
        "#wheelMirror .wm-item .nm", "els => els.map(e => e.textContent)")


def mirror_ids(page):
    return page.eval_on_selector_all(
        "#wheelMirror .wm-item", "els => els.map(e => e.dataset.wheelId)")


def set_pin_order_calls(page):
    return page.evaluate("window.__CALLS__.filter(c => c.cmd === 'set_pin_order')")


def wait_mirror_order(page, names, timeout=3000):
    """Bounded wait for the mirror to show `names`; returns False instead of
    raising so a missing re-render reports as a named FAIL, not a crash."""
    try:
        page.wait_for_function(
            "[...document.querySelectorAll('#wheelMirror .wm-item .nm')]"
            ".map(e => e.textContent).join() === " + json.dumps(",".join(names)),
            timeout=timeout)
        return True
    except Exception:
        return False


def drag_row_to(page, wheel_id, target_loc, target_y_offset):
    """Real pointer drag: press the row's grip, move past the threshold, drop
    on the target row at target_y_offset ('top' = above it, 'bottom' = below)."""
    row = page.locator(f'.wm-item[data-wheel-id="{wheel_id}"] .drag')
    rb = row.bounding_box()
    tb = target_loc.bounding_box()
    sx, sy = rb["x"] + rb["width"] / 2, rb["y"] + rb["height"] / 2
    ty = tb["y"] + 3 if target_y_offset == "top" else tb["y"] + tb["height"] - 2
    page.mouse.move(sx, sy)
    page.mouse.down()
    page.mouse.move(sx, sy - 20, steps=4)
    page.mouse.move(tb["x"] + tb["width"] / 2, ty, steps=12)
    page.mouse.up()


def main():
    srv, port = serve()
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page()
        page.add_init_script(INIT)
        page.goto(f"http://127.0.0.1:{port}/index.html")
        page.wait_for_selector("#wheelMirror .wm-item")

        # ---- 0. baseline: auto order is frecency desc -> Alpha Beta Gamma Delta ----
        check("mirror lists 4 pinned (unpinned hidden)",
              mirror_names(page) == ["Alpha", "Beta", "Gamma", "Delta"],
              str(mirror_names(page)))
        check("sort toggle starts in auto", "按使用" in (page.locator("#wheelSortBtn").text_content() or ""))

        # ---- 1. plain click commits nothing ----
        row4 = page.locator('.wm-item[data-wheel-id="4"]')
        box = row4.bounding_box()
        page.mouse.move(box["x"] + box["width"] / 2, box["y"] + box["height"] / 2)
        page.mouse.down()
        page.mouse.up()
        page.wait_for_timeout(150)
        check("click without move: no set_pin_order", set_pin_order_calls(page) == [])

        # ---- 2. real pointer drag: Delta (last) -> above Beta (slot 2) ----
        row4 = page.locator('.wm-item[data-wheel-id="4"]')
        row2 = page.locator('.wm-item[data-wheel-id="2"]')
        b4, b2 = row4.bounding_box(), row2.bounding_box()
        start_x, start_y = b4["x"] + b4["width"] / 2, b4["y"] + b4["height"] / 2
        target_y = b2["y"] + 3  # above Beta's midpoint -> insert before Beta
        page.mouse.move(start_x, start_y)
        page.mouse.down()
        page.mouse.move(start_x, start_y - 20, steps=4)
        check("drag lift: row gets .dragging",
              "dragging" in (row4.get_attribute("class") or ""))
        check("drag lift: insertion slot marker shown",
              page.locator("#wheelMirror .wm-insert").count() == 1)
        page.mouse.move(start_x, target_y, steps=12)
        page.mouse.up()
        page.wait_for_function("window.__CALLS__.filter(c => c.cmd === 'set_pin_order').length === 1")

        calls = set_pin_order_calls(page)
        check("drop calls set_pin_order once with [1,4,2,3]",
              calls == [{"cmd": "set_pin_order", "ids": [1, 4, 2, 3]}], str(calls))
        check("DOM re-rendered to committed order",
              wait_mirror_order(page, ["Alpha", "Delta", "Beta", "Gamma"]),
              str(mirror_names(page)))
        check("submitted ids == rendered DOM order",
              [str(i) for i in calls[0]["ids"]] == mirror_ids(page),
              f"ids={calls[0]['ids']} dom={mirror_ids(page)}")
        check("auto -> manual flip persisted",
              page.evaluate("localStorage.getItem('pk-wheel-sort')") == "manual")
        check("sort toggle now shows manual", "手动" in (page.locator("#wheelSortBtn").text_content() or ""))

        # ---- 3. committed order survives a real page reload ----
        page.reload()
        page.wait_for_selector("#wheelMirror .wm-item")
        page.wait_for_timeout(200)
        check("order persisted after reload",
              mirror_names(page) == ["Alpha", "Delta", "Beta", "Gamma"],
              str(mirror_names(page)))
        check("manual sort restored after reload",
              "手动" in (page.locator("#wheelSortBtn").text_content() or ""))

        # ---- 4. rejected commit rolls back the visual order ----
        page.evaluate("window.__FAIL_PIN_ORDER__ = true")
        drag_row_to(page, 3, page.locator('.wm-item[data-wheel-id="1"]'), "top")
        page.wait_for_function("window.__CALLS__.filter(c => c.cmd === 'set_pin_order').length === 1")
        page.wait_for_timeout(300)
        check("failed commit: DOM rolls back to pre-drag order",
              mirror_names(page) == ["Alpha", "Delta", "Beta", "Gamma"],
              str(mirror_names(page)))
        check("failed commit: no mid-state residue",
              page.locator("#wheelMirror .wm-insert").count() == 0
              and page.locator("#wheelMirror .wm-item.dragging").count() == 0)
        stored = page.evaluate(
            "Object.fromEntries(window.__PROMPTS__.filter(p => p.is_pinned).map(p => [p.id, p.inject_order]))")
        check("failed commit: backend data untouched",
              stored == {"1": "1", "2": "3", "3": "4", "4": "2"}, str(stored))
        page.evaluate("window.__FAIL_PIN_ORDER__ = false")

        # ---- 5. drag back to the same slot commits nothing ----
        row3 = page.locator('.wm-item[data-wheel-id="3"]')
        b3 = row3.bounding_box()
        page.mouse.move(b3["x"] + b3["width"] / 2, b3["y"] + b3["height"] / 2)
        page.mouse.down()
        page.mouse.move(b3["x"] + b3["width"] / 2, b3["y"] + b3["height"] / 2 + 8, steps=3)
        page.mouse.up()
        page.wait_for_timeout(150)
        check("no-op drop: still exactly one set_pin_order",
              len(set_pin_order_calls(page)) == 1)

        # ---- 6. drawer: internal fields gone, save preserves them ----
        page.click('.card[data-id="1"]')
        page.wait_for_selector("#drawer.show")
        check("drawer has no #fApps input", page.locator("#fApps").count() == 0)
        check("drawer has no #fOrder input", page.locator("#fOrder").count() == 0)
        body_text = page.locator("#drawer .drawer-body").text_content() or ""
        check("drawer hides 应用范围/轮盘位置 labels",
              "应用范围" not in body_text and "轮盘位置" not in body_text)
        page.click("#saveBtn")
        page.wait_for_function("window.__CALLS__.some(c => c.cmd === 'update_prompt')")
        upd = page.evaluate("window.__CALLS__.find(c => c.cmd === 'update_prompt').prompt")
        check("save preserves app_scopes_json", upd["app_scopes_json"] == '["Code.exe"]', str(upd))
        check("save preserves inject_order", upd["inject_order"] == "1", str(upd))

        # ---- 7. keyboard reorder on the grip ----
        page.locator('.wm-item[data-wheel-id="1"] .drag').focus()
        page.keyboard.press("ArrowDown")
        page.wait_for_function("window.__CALLS__.filter(c => c.cmd === 'set_pin_order').length === 2")
        calls = set_pin_order_calls(page)
        check("ArrowDown on grip commits [4,1,2,3]",
              calls[-1]["ids"] == [4, 1, 2, 3], str(calls[-1]))
        check("keyboard commit: DOM re-rendered to committed order",
              wait_mirror_order(page, ["Delta", "Alpha", "Beta", "Gamma"]),
              str(mirror_names(page)))
        check("keyboard commit: submitted ids == rendered DOM order",
              [str(i) for i in calls[-1]["ids"]] == mirror_ids(page),
              f"ids={calls[-1]['ids']} dom={mirror_ids(page)}")
        check("focus returned to moved row's grip",
              page.evaluate("document.activeElement?.dataset?.drag") == "1")

        # ---- 8. inject_order "0" is an explicit position ----
        # Rewrite the durable db: give pinned prompts spaced weights and set
        # Beta's inject_order to "0". A parse that only accepts >0 would dump
        # Beta to the 9999 tail; it must rank FIRST.
        page.evaluate("""() => {
          const db = JSON.parse(localStorage.getItem('__pk_e2e_prompts__'));
          for (const p of db) if (p.is_pinned) p.inject_order = String(p.id * 10);
          db.find(p => p.id === 2).inject_order = '0';
          localStorage.setItem('__pk_e2e_prompts__', JSON.stringify(db));
        }""")
        page.reload()
        page.wait_for_selector("#wheelMirror .wm-item")
        page.wait_for_timeout(200)
        check("inject_order '0' ranks first, not last",
              mirror_ids(page) == ["2", "1", "3", "4"], str(mirror_ids(page)))

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
