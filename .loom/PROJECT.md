# Project Whole and Document Map

## Intended result

PromptKey 是一个系统级提示词管理器（Tauri 桌面应用，Windows 首发）：全局热键唤出跟随光标的径向迷你轮盘，
在任何应用里把高质量提示词模板一键注入当前输入框。Phase 2 的具体形态：明暗双主题 + 中英运行时切换的设计系统，
提示词与轮盘合并为"pin 是提示词属性、轮盘是 pin 投影"的心智模型，5 项必需功能 + 5 项延后功能全部落地，
本地优先的模板库（内置精选包 + URL/文件导入导出），注入层补齐密码框门禁与全格式剪贴板备份、日志真实可信，
以及面向 Linux-X11/macOS 的 trait 抽象（仅抽象，不实装）。

## People and operating reality

单一用户的自用工具。用户重度使用多个 AI 应用，对 UX 顺畅度是最终验收人，偏好中文界面但要求中英双版；
维护者是单 Agent 会话链，**本机无 cargo、注入层依赖 Windows API**，Rust 代码交付时逐文件列"未编译验证"清单，
前端用 `node --check` 验证。用户强烈偏好多个小 PR 而非一个大 PR。

## Whole experience or behavior

主窗口四导航：提示词 / 模板库 / 记录 / 设置。提示词卡片内联 pin、搜索（`#tag`/`@name`/`c:` 前缀）、
frecency 排序、编辑抽屉（变量/标签/app 作用域/版本）。全局热键弹出 280px 轮盘，跟随光标、敲字过滤、
数字键直选、中心圆点显示状态；选中后渲染 `{{变量}}`（`{{clipboard}}`/`{{date}}` 自动，自定义变量弹表单），
再经剪贴板或 SendInput 注入目标窗口，密码框一律拒绝。模板库导入 `promptkey-pack` v1 JSON 包。
一切失败对用户可见（toast），不静默吞错。

## Boundaries and consequential assumptions

- 不引前端框架/npm 依赖/构建链；`tauri.conf.json` 的 `frontendDist` 与 `beforeBuildCommand` 不动。
- 图标一律来自 Lucide 静态 SVG（禁手画几何/字符图标），品牌资产只从 `PromptKey.ico`/`PromptKey_aiextract.png` 派生。
- 设计明确不做：版本 diff UI、模板继承、多轮盘、自动应用匹配、云同步、团队协作、独立守护进程。
- Wayland 诚实降级，自动注入依赖外部工具；本 Phase 不实装 Linux/macOS。
- 不改 Phase 1 五份设计文档；设计变更走 `loom decision`。
- 六个 PR 全部 `--base master`，不 auto-merge。

## Design document map

- `.loom/design/UI_UX_REDESIGN.md` — 设计系统 token、动效、i18n、轮盘 A/B/C、页面布局、稳定性清单（正文）；`ui-ux-redesign.md` 为索引。
- `.loom/design/MERGE_ARCHITECTURE.md` — 合并信息架构、功能评级表、后端最小改动。
- `.loom/design/MARKET_DECISION.md` — 模板库形态与 `promptkey-pack` 格式。
- `.loom/design/INJECTION_RESEARCH.md` — 注入缺口、密码框门禁、剪贴板、日志修复。
- `.loom/design/CROSS_PLATFORM_PLAN.md` — 三平台 trait 抽象路线（本 Phase 仅 P1）。
- `.loom/design/task-brief-phase2.md` — 本 Phase 任务分解与用户验收原话。
- `.loom/design/prototypes/` — 高保真可交互原型（视觉/交互基准）。

## Professional capability map

- `.loom/capabilities/design-system/capability.md` — 主题/语言切换、信息层级、错误呈现形态。
- `.loom/capabilities/injection-engine/capability.md` — 安全字段门禁、剪贴板策略、平台边界。
- `.loom/capabilities/market-model/capability.md` — 包来源信任模型、格式契约。
- `.loom/capabilities/product-architecture/capability.md` — 概念一级化/属性化、隐式耦合、死代码处置。

## Project structure

见 `.loom/STRUCTURE.md`：源码 `src/`（Tauri 壳 + vanilla 前端）与 `service/src/`（内嵌服务线程）；
资产 `src/icons/`；设计/任务/交付物在 `.loom/`。

## Work map

`.loom/tasks.json`：TASK-001..006 对应 Phase 2 六个分支，各带 acceptance/verify_by/evidence。

## Decision history

`.loom/DECISIONS.md`，经 `loom decision --json-file` 追加。

## Completion and failure

成功 = 最终交付清单逐项可验（功能实装、6 PR 开而不并、Rust 未验证清单明示、LOOM 证据齐）。
假完成风险：Rust 写了无法编译但声称可用；UI 只改样式不改交互语义；日志/统计仍乐观造假。

## Staged visibility and review

每个 Task 一个分支 + 一个 commit + 一个 PR，PR 描述写"改了什么/为什么/验收方式/未编译验证项"。
前端改动用静态服务+浏览器原型级走查；每批结束跑 `loom check`。
