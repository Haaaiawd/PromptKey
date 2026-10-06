# Devin Brief: PromptKey Phase 2 — UI/UX 全面革新 + 5 必需功能 + 5 延后功能

## 项目
`/home/haa/sites/promptkey`，分支从 `feat/phase1-research-design` 起。

## 前置：Phase 1 设计文档已就绪，**必须先读完再动手**

按顺序读这五份，它们是本 Phase 的唯一规格来源：

1. `.loom/design/UI_UX_REDESIGN.md` (225 行) — 设计系统（明暗双 token / 字阶 / 间距 / 动效 / i18n）+ 轮盘三候选 + 页面布局
2. `.loom/design/MERGE_ARCHITECTURE.md` (101 行) — 合并信息架构 + 功能必要性评级表 + 后端最小改动清单
3. `.loom/design/MARKET_DECISION.md` (71 行) — 模板库形态 + pack 数据格式
4. `.loom/design/INJECTION_RESEARCH.md` (245 行) — 注入机制缺口 + 密码框保护 + 日志修复
5. `.loom/design/CROSS_PLATFORM_PLAN.md` — 三平台 trait 抽象路线

**高保真原型（可交互，作为视觉与交互基准）**：
- `.loom/design/prototypes/main-window.html` (636 行) — 主窗口，明暗/中英可切换
- `.loom/design/prototypes/wheel-a-radial.html` — 轮盘 A 案（**已定采用**）
- `.loom/design/prototypes/wheel-b-arc.html` / `wheel-c-palette.html` — 仅参考，B/C 不做

线上可看：`https://box.haaaiawd.live/dl/pk-prototypes/*.html`

## 用户原话（本次 Phase 的验收标准）

> 「把五必须和五延后的都给做一下，另外仍然是 UI UX 方面，一定要做好，关于用户体验的部分」

「**一定要做好**」「**关于用户体验的部分**」是硬要求。用户对 UX 顺畅度是最终验收标准，会反问"人类或 Agent 用这个真的方便吗"。

## 设计原则（不可违背）

- **优雅简约** — 每个新功能都要能回答"没有它会死吗"
- **符合 UX** — 一切形态以真实使用体验为准
- **可降级** — 新策略不成熟保留旧路径
- **稳定可用优先于功能丰富**

---

## Task 0：图标与视觉资产规则（硬约束，优先级最高，先读这个再动手）

用户原话：「比如图标之类的，一定要用我们自己的，不要去自己瞎做嘛。如果可以的话，可以上网找这些图标或者 logo 来用，而不是去这个自己画，因为我们现在的 devin 比较菜。让 devin 懂这个道理」

### 现有资产（已核实，别再问）

| 文件 | 大小 | 用途 |
|---|---|---|
| `PromptKey.ico` | 32K | **Windows 应用图标，已在用** |
| `PromptKey_aiextract.png` | 253K | **从原设计提取的品牌 logo 素材** |
| `src/icons/` | 仅一份 README | 空目录，需生成实际图标文件 |

### 必须遵守

1. **禁止手画 SVG 图标。** 现有代码里那些手写几何 rect 当图标的做法必须全部替换 —— 例如 `src/index.html:44-58` 用 `<rect>` 拼的卡片/列表视图切换图标，`src/wheel.html:77,81` 的 `◀` `▶` 字符，`src/index.html` 里的 `<i class="icon-plus">` 之类。
2. **UI 图标全部来自单一成熟开源图标库**，整组引入、风格统一，禁止东拼西凑混用。
   推荐（按优先级，选一个）：
   - **Lucide** (`https://lucide.dev`) — **首推**。1.5k+ 图标、统一 2px stroke / 24×24 grid、ISC 许可、提供单文件 SVG sprite，最适合 vanilla 零构建项目
   - Tabler Icons (`https://tabler.io`) — 备选，更细、数量更多
   - Phosphor Icons (`https://phosphoricons.com`) — 备选，多粗细变体
   - 选定后**全项目只用这一个库**
3. **品牌 logo / 应用图标必须派生自现有资产，不许重新设计**：
   - 从 `PromptKey.ico` + `PromptKey_aiextract.png` 派生：系统托盘图标、关于页 logo、轮盘中心标记、安装包图标、macOS `.icns`、Linux 多尺寸 PNG
   - **不要"创造"品牌图形**，不要改 logo 的造型/配色/构图
4. **静态引入，不依赖运行时 CDN** —— Tauri 打包后可能离线。把图标文件下载到 `src/icons/` 下（SVG sprite 或单图标 SVG + 一个 `<symbol>` 索引），许可文件（LICENSE）一并放 `src/icons/` 里，注明库名与版本，可追溯。
5. **图标正确性要求**：
   - stroke 图标用 `currentColor`，确保明暗两套主题都可见
   - 尺寸统一（16/20/24 三档），对齐网格
   - 触摸/点击目标 ≥ 24×24（轮盘等密集区域用视觉尺寸 + padding 扩大命中区）
   - 加 `aria-label` / `title`，别让图标成为唯一信息载体

### 验收
- 全项目搜不到手画几何图标与 emoji/字符图标（`◀ ▶ ⭐ ✓ ✕ 🔍 ⚙` 等）
- 图标全部来自选定的单一库，风格与粗细统一
- `src/icons/` 下有真实图标文件 + 许可文件 + 库版本记录
- 品牌资产（logo / 托盘 / 各平台安装包图标）派生自 `PromptKey.ico` 与 `PromptKey_aiextract.png`
- 最终交付里单独列一节 **"Icon sourcing"**：用了哪个库及版本、许可、下载方式、品牌派生关系、新增图标文件清单

---

# Phase 2 任务

## Task 1：UI/UX 基础 —— 设计系统落地 + 明暗 + 中英 + 动效

按 `UI_UX_REDESIGN.md` §1/§2 执行：

1. **不引框架**（Phase 1 已决策：vanilla + 设计系统化）。把 `main_simple.js` (1365 行) 拆成 ES modules：`js/app.js` / `js/store.js` / `js/i18n.js` / `js/toast.js` / `js/views/*.js`，零构建，`tauri.conf.json` 的 `frontendDist` 不改
2. **设计 token 三层**：色板（明暗双套）/ 字阶 / 间距 / 圆角 / 阴影，CSS 变量定义在共享位置
3. **明暗模式**：CSS 变量双层 + `prefers-color-scheme` + 手动覆盖 + 持久化（localStorage）。修掉 `src/index.html:12` 的 `body class="theme-light"` 硬编码
4. **中英双版**：运行时切换 + 跟随系统 + 持久化。现有 UI 文案全部抽成 i18n key（中文值先填好）。轮盘窗口和选择器窗口也要覆盖
5. **动效**：按文档的动效 token 实现每个交互，**必须遵守 `prefers-reduced-motion`**
6. **轮盘 A 案**：500px → 280px，跟随光标弹出，屏幕边缘 clamp 16px，中心 56px 功能圆点（显示过滤状态/页码/Esc），打开即听键（敲字实时过滤），数字键 1-6 直选，选中 pulse 300ms 后收起+注入，`←→` 翻页兼容保留
7. **轮盘透明窗口**：明暗两套都要保证在任意桌面背景下可读
8. **高分屏**：px 逻辑像素 + Tauri scale_factor 换算

### 验收
- 明暗/中英均可在设置里切换并重启后保持
- 轮盘 280px 跟随光标，命中区满足 Fitts
- `prefers-reduced-motion` 生效
- 原型与实现的视觉一致性

## Task 2：提示词 + 轮盘合并（5 必需之本体）

按 `MERGE_ARCHITECTURE.md` 执行：

1. **删除 `wheel-panel` 整个页面**（`src/index.html:99-109`），轮盘条目降级为 `WHERE pinned=1` 筛选
2. **pin 内联化**：每条提示词卡片上加 ⭐ 开关，直接调 `toggle_prompt_pin`
3. 导航 5 → 4：提示词 / 模板库 / 记录 / 设置
4. 删除"适应规则"卡片（写死的演示卡）
5. 提示词页顶部：搜索框 + 标签筛选 + 视图切换；每条 = 名称/摘要/标签/⭐/使用数 + ⋯菜单
6. 详情/编辑抽屉：内容 / 变量 / 标签 / app_scopes / pin / 版本
7. **"轮盘预览"按钮**：触发一次真实轮盘弹出（所见即所得）—— 解决"新手看不到轮盘长什么样"的风险

## Task 3：5 个必需功能

按 `MERGE_ARCHITECTURE.md` §4 评级表"必需"行 + `UI_UX_REDESIGN.md` §7 执行：

| # | 功能 | 要点 |
|---|---|---|
| N1 | pin 内联化 | 见 Task 2 |
| N2 | **frecency 排序** | 轮盘默认序 = 使用频率 + 新近，聚合 `usage_logs`；`get_top_prompts_paginated` 改 `ORDER BY frecency`。注意 Phase 1 发现"最近使用"是把字符串日期当数字算（`db.rs:359-360`），要修 |
| N3 | **轮盘内打字过滤** | 轮盘弹出后敲字实时过滤（Fuse 能力从 `selector.js` 平移进轮盘），突破 6 格限制。全量 prompts 一次给轮盘窗口（<1MB 无压力） |
| N4 | **显式"默认快捷提示词"** | 修 `find_prompt_for_context` 隐式耦合（`db.rs:401-414` 忽略上下文只读 `selected_prompt`）。新增 `default_prompt_mode: "last_used"\|"fixed"` + `default_prompt_id` 设置项 |
| N5 | **导入/导出 JSON** | 模板库的前提 + 用户数据主权底线。格式与 pack 统一 |

**搜索增强（UI_UX_REDESIGN.md §7，属 N3 同源）**：
- 前缀快进语法：`#tag` 按标签、`@name` 只搜名称、`c:xxx` 强制全文
- 保留 Fuse.js，权重 name .6 / tags .25 / content .15
- fallback 方案记录在案（>5000 条时降级 SQL LIKE），**本次不实现**

## Task 4：5 个延后功能（用户要求本次一起做）

| # | 功能 | 要点 |
|---|---|---|
| D1 | **`{{clipboard}}` / `{{date}}` 自动变量** | 零 UI 成本的自动变量，注入时替换。字符串替换即可，不需要表单 |
| D2 | **pin 拖拽排序** | `inject_order` 字段已有（`db.rs:55`）。注意：有了 frecency 自动序，手动排序应是"覆盖自动序"的显式开关，不要搞出两套排序打架 |
| D3 | **快速试运行** | 详情页"预览"按钮渲染变量后的成品文本，不碰目标窗口 |
| D4 | **轮盘内"+ 快速新建"** | 中心按钮长按/右键出"新建"，减少回主窗口的往返 |
| D5 | **变量填充表单** | `variables_json` 字段已备好（`db.rs:53`）。注入前弹表单收集 `{{var}}` 值。注意与 D1 的自动变量共存：自动变量不弹表单，自定义变量才弹 |

## Task 5：模板库（原"市场"改造）

按 `MARKET_DECISION.md` 执行：

1. `market-panel` 改名"**模板库 / Library**"（i18n 两语）
2. **内置精选 JSON 包**，一键导入（复用 `create_prompt`）。内容由你策展一批高质量的常见提示词模板（中英各几类：写作/编程/分析/翻译/总结等），格式遵循 `promptkey-pack` v1
3. **自定义源导入**：粘贴 URL 拉 JSON + 选本地文件（`tauri-plugin-dialog`/`fs` 已在依赖 `Cargo.toml:41-42`）
4. **导出我的提示词**（与 N5 同一格式）
5. **删掉空壳**：`main_simple.js:563-571` 刷新按钮 `alert('市场功能暂未实现')`、`:646-656` 搜索按钮 —— 全部替换为真实功能
6. 加"来源自担"提示（URL 导入时）

## Task 6：注入机制加固（Phase 1 发现的真实缺陷）

按 `INJECTION_RESEARCH.md` 执行，**只做缺口修复，不重构主链路**：

1. **密码框保护**（最重要）：Phase0 删 UIA 时连密码框检测一起删了，North_Star 的"不注入密码框"边界**当前零实现**。用只读探测拿回 UIA/AX 门禁（不恢复整条 UIA 注入路径）。Linux/macOS 用对应平台能力
2. **剪贴板污染窗口**：当前 ~300ms 且只备份文本格式。改备份所有可用格式（`EnumClipboardFormats`），缩短窗口
3. **日志修复**：当前使用日志在注入**之前**写入且 `success=true` 硬编码（`service/src/main.rs:139-153` vs `163`）→ 改为注入后按真实结果写，记录实际策略与耗时。这是 KPI 可观测性的前提
4. **`usage_count` 修正**：当前永远 +1 不论成败 → 只在成功时 +1
5. 修完让"记录"页的成功率/耗时数据**可信**

## Task 7：前端稳定性加固

按 `UI_UX_REDESIGN.md` §8 清单 + 现状问题执行：

1. 消除 `alert()` 兜底（用统一的 toast）
2. DOM 操作加防护（元素存在性检查）
3. `updateDebugInfo()` 当前被当日志用 —— 区分"调试信息"与"用户可见反馈"
4. IPC 调用失败要有用户可见的错误态，不能静默
5. 轮盘/选择器窗口的键盘事件在失焦时正确释放

## Task 8：跨平台 Phase A（trait 抽象）

按 `CROSS_PLATFORM_PLAN.md` 执行 **P1 阶段**：

1. 抽出 `Injector` / `Context` / `Hotkey` 三个 trait
2. Windows 实现迁到 `impl ... for WindowsInjector`
3. `AppContext.window_handle` 改 `u64`/`isize` 不透明句柄（Windows 存 HWND bit）
4. `src/main.rs:1` 的 `#![windows_subsystem]` 用 `cfg_attr` 门控
5. `Cargo.toml:39` 的 `windows = 0.52` 从全局 dependencies 收进 `[target.'cfg(windows)'.dependencies]`
6. `Cargo.toml:30` 的 `winres` 收进 `[target.'cfg(windows)'.build-dependencies]`
7. `src/main.rs:113-120` 的 `resolve_service_exe_path()` 死代码删除
8. service 维持内嵌线程架构，**不做独立守护进程**
9. Linux-X11/macOS 实现留到 Phase 3，本 Phase 只做抽象 + trait 边界

## Task 9：LOOM 任务登记

1. 按 Task 1-8 拆成 LOOM task，每个带验收标准
2. `loom task plan` 后逐个 `loom task start → done`
3. 执行中把验收证据写进 `.loom/deliverables.json`
4. **不要修改 Phase 1 的五份设计文档**，除非实现中发现设计有误 —— 那种情况走 `loom decision --json-file` 记录变更原因，然后更新文档
5. 不要动 `00_READ_ME_FIRST.md` 到 `07_DECISIONS_AND_OPEN_QUESTIONS.md`

---

# 执行约束

## 这不是本机可编译的项目
**本机没有 cargo，且注入层依赖 Windows API。** 所以：
- Rust 代码**写对但无法编译验证** —— 必须格外小心，每处 API 调用核对 `windows` 0.52 / rusqlite 0.32 的实际签名
- 前端代码可以用 node 做基本语法检查（`node --check`）
- **不要跑 `cargo build` / `cargo check`**，浪费时间且必然失败
- 交付时明确列出"未编译验证的 Rust 文件清单"

## 拆分 PR（用户强烈偏好）
用户原话（历史）：「下次记得分多pr进行我笑死了，这一个pr东西太多了」
**按 Task 拆分支 + 各自 PR，全部 `--base master`**：
- `feat/phase2-design-system` (Task 1)
- `feat/phase2-merge-prompts-wheel` (Task 2+3)
- `feat/phase2-template-library` (Task 5)
- `feat/phase2-injection-hardening` (Task 6)
- `feat/phase2-stability` (Task 7)
- `feat/phase2-cross-platform-traits` (Task 8)

D1-D5 归入 `feat/phase2-merge-prompts-wheel` 或独立分支，你按文件重叠判断。
每个 PR 描述里写清：改了什么、为什么、验收方式、**未编译验证项**。

## 其他
- 每 Task 一个 commit，`feat: <what> (LOOM TASK-00X)`
- 不改 `tauri.conf.json` 的 `frontendDist` / `beforeBuildCommand`
- 不引 npm 依赖（保持零构建）
- 新增/修改的 i18n key 必须中英都有值
- **不要 auto-merge**

---

## 最终交付

- 设计系统落地：明暗 + 中英 + 动效 + `prefers-reduced-motion`
- 轮盘 A 案：280px 跟随光标 + 打字过滤
- 提示词/轮盘合并：pin 内联，导航 5→4
- 5 必需 + 5 延后功能全部实现
- 模板库（内置包 + URL/文件导入 + 导出）
- 注入加固：密码框保护 + 剪贴板全格式备份 + 日志真实可信
- trait 抽象完成（Windows 实现迁移）
- 6 个分支 + PR，全部 `--base master`，未合并
- LOOM tasks.json / deliverables.json 更新
- **未编译验证清单**（Rust 文件逐个列）

```json
COMPLETION_NOTIFY
source: assistant
task: promptkey-phase2-implementation
deliverables:
- design system: dark/light + zh/en + motion tokens + reduced-motion
- wheel A: 280px cursor-following + type-to-filter
- prompts+wheel merged: inline pin, nav 5->4
- 5 required + 5 deferred features implemented
- template library: builtin pack + URL/file import + export
- injection hardening: password guard + clipboard all-format backup + truthful logs
- Injector/Context/Hotkey traits extracted, Windows impl migrated
- 6 branches + PRs against master, none merged
- LOOM tasks + deliverables updated
status: success
errors: none
```
