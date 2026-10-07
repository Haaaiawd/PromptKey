#!/usr/bin/env python3
"""Capture real UI screenshots for docs/ — serves src/ over HTTP and stubs
window.__TAURI__ the same way the Playwright e2e suites do. What you see is
the shipped HTML/CSS/JS; only the backend responses are stubbed data.
Outputs PNGs to docs/screenshots/.
"""
import functools, http.server, json, socketserver, sys, threading, time
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "src"
OUT = ROOT / "docs" / "screenshots"

NOW = int(time.time() * 1000)
H = 3600_000

PROMPTS = [
    {"id": 1, "name": "客服-致歉模板", "content": "非常抱歉给您带来不便，我们正在核实您反馈的问题，会在 24 小时内回复处理结果。",
     "tags": ["客服", "邮件"], "content_type": "text", "variables_json": None, "app_scopes_json": None,
     "inject_order": "1", "version": 3, "updated_at": "2025-06-18 14:02", "is_pinned": True,
     "usage_count": 47, "last_used_at": NOW - 2 * H, "frecency": 9.4},
    {"id": 2, "name": "Code Review 意见", "content": "这块逻辑建议抽到独立函数，可读性和单测覆盖都会更好。另注意边界条件。",
     "tags": ["工程", "代码"], "content_type": "text", "variables_json": None, "app_scopes_json": None,
     "inject_order": "2", "version": 1, "updated_at": "2025-06-10 09:31", "is_pinned": True,
     "usage_count": 23, "last_used_at": NOW - 5 * H, "frecency": 5.1},
    {"id": 3, "name": "会议纪要骨架", "content": "## 参会人\\n{{people}}\\n## 结论\\n- \\n## 待办\\n- ",
     "tags": ["工作", "模板"], "content_type": "text", "variables_json": '["people"]', "app_scopes_json": None,
     "inject_order": "3", "version": 2, "updated_at": "2025-05-28 16:45", "is_pinned": True,
     "usage_count": 18, "last_used_at": NOW - 26 * H, "frecency": 3.8},
    {"id": 4, "name": "日报速记", "content": "今日完成：{{content}}\\n明日计划：",
     "tags": ["工作"], "content_type": "text", "variables_json": '["content"]', "app_scopes_json": None,
     "inject_order": "4", "version": 1, "updated_at": "2025-06-01 11:20", "is_pinned": True,
     "usage_count": 12, "last_used_at": NOW - 50 * H, "frecency": 2.6},
    {"id": 5, "name": "英文-礼貌催办", "content": "Hi, a gentle follow-up on the ticket below — do you have an ETA? Thanks!",
     "tags": ["邮件", "英文"], "content_type": "text", "variables_json": None, "app_scopes_json": None,
     "inject_order": "5", "version": 1, "updated_at": "2025-06-12 18:03", "is_pinned": True,
     "usage_count": 8, "last_used_at": NOW - 74 * H, "frecency": 1.9},
    {"id": 6, "name": "SQL 查重模板", "content": "SELECT {{cols}}, COUNT(*) FROM {{table}} GROUP BY {{cols}} HAVING COUNT(*) > 1;",
     "tags": ["数据库"], "content_type": "text", "variables_json": '["cols","table"]', "app_scopes_json": None,
     "inject_order": "6", "version": 1, "updated_at": "2025-05-20 10:00", "is_pinned": True,
     "usage_count": 5, "last_used_at": NOW - 98 * H, "frecency": 1.2},
    {"id": 7, "name": "常用地址", "content": "北京市海淀区中关村大街 27 号", "tags": ["个人"],
     "content_type": "text", "variables_json": None, "app_scopes_json": None,
     "inject_order": None, "version": 1, "updated_at": "2025-05-15 08:12", "is_pinned": False,
     "usage_count": 3, "last_used_at": NOW - 120 * H, "frecency": 0.5},
]

SETTINGS = {
    "wheel_hotkey": "Ctrl+Alt+Space",
    "quick_hotkey": "Ctrl+Alt+A",
    "autostart": False,
    "theme": "dark",
    "language": "zh-CN",
}

PLATFORM_STATUS = {
    "os": "linux", "session": "x11",
    "injection": "ok", "hotkeys": "ok", "context": "ok", "notes": [],
}

# Same shape the e2e suites install: core.invoke dispatches by command name,
# event.listen records callbacks so the page can be driven afterwards.
INIT = f"""
window.__LISTENERS__ = {{}};
window.__TAURI__ = {{
  core: {{
    invoke: (cmd, args) => {{
      const d = {{
        get_prompts_view: {json.dumps(PROMPTS, ensure_ascii=False)},
        get_settings: {json.dumps(SETTINGS)},
        get_app_settings: {json.dumps(SETTINGS)},
        get_platform_status: {json.dumps(PLATFORM_STATUS)},
        get_launch_at_login: false,
        check_service_status: {{ok: true}},
        check_hotkeys: {{engine: "ok", hotkeys: [
          {{id: 4, combo: "Ctrl+Alt+Space", canonical: "Ctrl+Alt+Space", status: "ok"}},
          {{id: 5, combo: "Ctrl+Alt+A", canonical: "Ctrl+Alt+A", status: "ok"}},
        ]}},
        get_service_log: "",
        get_prompt_stats: {{total: 7, pinned: 6, used_24h: 4}},
        get_diagnostics: {{ok: true, details: {{}}}},
      }};
      return Promise.resolve(d[cmd] !== undefined ? d[cmd] : null);
    }}
  }},
  event: {{
    listen: (name, cb) => {{ window.__LISTENERS__[name] = cb; return Promise.resolve(() => {{}}); }},
    emit: () => Promise.resolve(),
  }},
  window: {{
    getCurrentWindow: () => ({{
      outerPosition: () => Promise.resolve({{x: 0, y: 0}}),
      outerSize: () => Promise.resolve({{width: 320, height: 320}}),
      currentMonitor: () => Promise.resolve({{scaleFactor: 1, position: {{x:0,y:0}}, size: {{width:1920,height:1080}}}}),
      setPosition: () => Promise.resolve(),
    }}),
    PhysicalPosition: class {{ constructor(x, y) {{ this.x = x; this.y = y; }} }},
  }},
}};
"""


def serve():
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(SRC))
    srv = socketserver.ThreadingTCPServer(("127.0.0.1", 0), handler)
    srv.daemon_threads = True
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv, srv.server_address[1]


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    srv, port = serve()
    shots = []
    with sync_playwright() as p:
        browser = p.chromium.launch(args=["--force-color-profile=srgb"])
        page = browser.new_page(viewport={"width": 980, "height": 700})
        page.add_init_script(INIT)
        page.goto(f"http://127.0.0.1:{port}/index.html")
        page.wait_for_selector(".card", timeout=5000)
        page.wait_for_timeout(600)
        page.screenshot(path=str(OUT / "prompts-view.png"))
        shots.append("prompts-view.png")

        page.click('.nav-item[data-view="settings"]')
        page.wait_for_selector("#hotkeyInput", timeout=5000)
        page.wait_for_timeout(600)
        page.screenshot(path=str(OUT / "settings-view.png"))
        shots.append("settings-view.png")

        # Wheel overlay: transparent window in production — capture on a dark
        # backdrop the way it would appear over an editor.
        wheel = browser.new_page(viewport={"width": 520, "height": 460})
        wheel.add_init_script(INIT)
        wheel.goto(f"http://127.0.0.1:{port}/wheel.html")
        wheel.wait_for_timeout(400)
        wheel.evaluate("""() => {
            document.body.style.background = 'radial-gradient(circle at 40% 40%, #2a2d33, #141619)';
            window.__LISTENERS__['wheel-show']();
        }""")
        wheel.wait_for_selector(".petal", timeout=5000)
        wheel.wait_for_timeout(700)
        wheel.screenshot(path=str(OUT / "wheel-overlay.png"))
        shots.append("wheel-overlay.png")
        browser.close()
    srv.shutdown()
    for s in shots:
        print("saved", OUT / s)


if __name__ == "__main__":
    main()
