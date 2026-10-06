# Project structure

## Source code

- `src/` — Tauri 壳（`main.rs`）与 vanilla 前端：`index.html`（主窗口）、`wheel.html/css/js`（运行时轮盘）、`styles.css`、`fuse.min.js`（vendored）。ES module 代码放 `src/js/`（`app.js`/`store.js`/`i18n.js`/`toast.js`/`views/*.js`/`i18n/*.js`）。
- `service/src/` — 内嵌服务线程（Rust lib）：`db.rs`（SQLite/rusqlite）、`injector/`、`context/`、`hotkey/`、`ipc/`、`config/`。trait 抽象后放 `service/src/traits.rs`，Windows 实现留原模块。
- `src/icons/` — Lucide 静态 SVG（`src/icons/lucide/`）+ `LICENSE` + `ICONS.md` 溯源；`src/icons/README.md` 记录规则。
- `src/packs/` — 内置精选 `promptkey-pack` JSON。
- `src/icons/brand/` — 从 `PromptKey.ico` / `PromptKey_aiextract.png` 派生的品牌资产（不改造型/配色）。

## Tests

无独立测试目录。前端用 `node --check` 逐文件语法验证 + 静态服务浏览器走查；Rust 本机不可编译，以签名核对 + PR 注明未验证文件为准。临时验证脚本放 `/tmp`，不入库。

## Documents

- `North_Star.md` — 产品北极星与 KPI/边界（不改）。
- `.loom/design/` — 设计文档；大写文件为正文，小写同名文件为 LOOM 索引。
- `.loom/` — PROJECT.md（入口）、STRUCTURE.md（本文件）、tasks.json、deliverables.json、DECISIONS.md、capabilities/。
- `README.md`、`AGENTS.md`（LOOM 连续性规则）。

## Configuration and build

- `Cargo.toml` / `service/Cargo.toml` / `build.rs` — Windows 依赖须 gate 进 `[target.'cfg(windows)'.dependencies]`，`winres` 同。
- `tauri.conf.json` — `frontendDist` 与 `beforeBuildCommand` 为受保护配置，不改。
- `service/config.yaml`（运行时生成）— 服务配置；`app_settings` 表存 UI 侧键值设置。

## Assets and fixtures

- `PromptKey.ico`（32K，在用应用图标）、`PromptKey_aiextract.png`（253K，品牌 logo 母素材）。
- `prompts.json` — 旧版提示词导出样例数据。

## Conventions

- 前端零构建 vanilla ES modules；用户可见文案一律走 i18n key（中英双语同时给值），禁裸 `alert()`/`confirm()`。
- 图标只用 Lucide（`currentColor` stroke，16/20/24 三档），禁 emoji/字符图标。
- Rust IPC 命令 snake_case，命名 `<verb>_<noun>`；DB 迁移用 `ALTER TABLE … ADD COLUMN` + `PRAGMA user_version`。
- Commit 格式 `feat: <what> (LOOM TASK-00X)`，一 Task 一 commit，六分支各 PR `--base master` 不合并。
