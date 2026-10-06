# Devin Brief: PromptKey 大改造 Phase 1 — 调研 + 设计（不写实现代码）

## 项目
`/home/haa/sites/promptkey` — Tauri 2.8.5 (Rust) + SQLite 桌面提示词管理器，当前 1.2.1。
Git: `https://github.com/Haaaiawd/PromptKey.git`，分支 `master`，工作区干净。

---

# 背景与用户原话

用户（项目 owner）的原话，按重要性排序：

1. **「首先，我们的注入等机制研究好...市场的话想想...啥机制，要不要做」**
2. **「另外是提示词和轮盘直接合并」**
3. **「在UI和UX体验上需要全部革新」** — 美观、动效、稳定性
4. **「特别是轮盘的UI也需要美化，并且太大了，缩小，并且样式要改，想一下，UX角度」**
5. **「让其先思考好原型长啥样，需要的功能就新加，不需要的就不要。并且中英文双版，明暗模式要做」**
6. **「老功能搜索有没有更优策略，当然现在的策略就还不错哦，但没人不喜欢更好的办法，实在不行也可以降级」**
7. **「没什么了，保持稳定可用，保持优雅简约」**
8. **「哦对，那顺便把我们的linux和mac端也给做了吧」** — 跨平台
9. **「让devin先研究设计」** — 本 Phase 只做研究+设计，实现留给后续 Phase

## 设计原则（用户钦定，不可违背）

- **优雅简约** — 每个新功能都要回答"没有它会死吗"，不会就别加
- **符合 UX** — 一切缩小/合并/新形态以真实 UX 为准，不做拍脑袋
- **可降级** — 新策略不成熟时保留旧路径，不赌一把
- **稳定可用优先于功能丰富**

---

# Phase 1 任务：只研究、只设计、只写文档。禁止修改任何 `src/`、`service/`、`Cargo.toml` 代码。

## Task 1：注入机制研究报告

### 现状（已核实，别重复调研这些事实）

- `service/src/injector/mod.rs` — **只有两级策略**：Clipboard（主）→ SendInput（回退）
- **UIA (UI Automation ValuePattern) 已被完全移除**（TODO.md 里写了 UIA 主路径，但 `injector/mod.rs` 注释明确 "UIA-specific, no longer used"，`InjectionStrategy` enum 只剩 `Clipboard` / `SendInput`）
- Clipboard 路径有：快照备份 → 大小上限截断 → SetClipboardData → Ctrl+V → 恢复原剪贴板
- 剪贴板路径会**污染用户剪贴板**（先备份再恢复，期间有窗口期）
- `service/src/context/mod.rs` — 上下文感知：取前台窗口 hwnd / 窗口标题 / 进程名
- `service/src/db.rs` — `find_prompt_for_context()` 按 app_scopes 匹配提示词

### 研究要求

1. **为什么 UIA 被移除？** 从 git 历史挖（`git log --all -p -- service/src/injector/`），给出原因判断：是真不可用，还是当时实现有问题？如果是后者，值不值得重新引入
2. **各策略的适用边界**（要给出可复现的测试矩阵思路，不要空谈）：
   - UIA ValuePattern：适合/不适合哪些控件类型，密码框拒绝，完整性级别限制
   - Clipboard：哪些应用会拦截（终端、浏览器地址栏、Electron 应用、Chromium 编辑器）
   - SendInput：长文本分批、IME 干扰、中文输入法环境下的行为差异
   - **新策略调研**：至少评估 2 个当前代码里没有的方向，例如
     - WM_PASTE / WM_CHAR 消息直投（不走剪贴板）
     - 逐字符 SendInput with UNICODE 标志
     - UI Automation TextPattern（区别于 ValuePattern）
     - 其他你认为更优的（给出证据）
3. **给出建议的策略顺序 + 判定条件**（能力驱动，不做应用名特例黑名单）
4. **跨平台差异**（这是新要求，重点）：
   - Windows: 现有实现
   - **Linux**: X11 vs Wayland 分别怎么注入？(`xdotool` / `XSendEvent` / `XTEST` / `wtype` / `ydotool`；Wayland 下全局热键和注入的根本限制)
   - **macOS**: Accessibility API (AXUIElement) / CGEvent / AppleScript
   - 明确哪些 Rust crate 可用（`enigo` / `rdev` / `arboard` / `x11rb` / `accessibility-sys` 等），版本与维护状态
   - **结论必须包含**：三平台的注入能力是否应该抽象成统一 trait，还是在 service 层按平台分支
5. **安全边界**（North_Star.md 已定，但要对齐到新策略）：不注入密码框、不越权高完整性窗口、Linux 下 Wayland 的权限模型

### 交付

`.loom/design/INJECTION_RESEARCH.md`，含：
- 现状链路图（文字描述即可，标注文件:行号）
- 策略对比矩阵（策略 × 适用场景 × 成功率预期 × 副作用 × 平台支持）
- 推荐顺序 + 判定逻辑（伪代码/决策树）
- 三平台注入方案 + crate 选型
- 待验证清单（哪些结论需要在真机上测）

---

## Task 2：市场机制 — 做还是不做的决策报告

### 现状

- `src/index.html:111-127` 有 market-panel 骨架
- `src/main_simple.js:563-571` — **刷新市场按钮 `alert('市场功能暂未实现')`**
- `src/main_simple.js:646-656` — 市场搜索按钮只 `updateDebugInfo()`，无实际行为
- 即：市场是一个**纯 UI 占位的空壳**

### 研究要求

用户明确要一个决策，不要含糊。请评估：

1. **本地优先产品的市场应该是什么形态**？可选方向（可提你自己的）：
   - A. 内置精选提示词库（本地打包一批高质量模板，一键导入）
   - B. 社区分享（需后端、需审核、需版权处理）
   - C. GitHub raw / 用户自建源（用户提供 URL，拉取 JSON/YAML）
   - D. 从剪贴板/文件导入（这不叫市场，但可能是更实在的需求）
2. 每个方向的：实现成本、维护成本、法律/版权风险、与"本地优先、隐私"北极星是否冲突
3. **明确给一个推荐 + 理由**，并明确"本版本做/不做"
4. 如果建议做：给出最小可行形态（数据格式、来源、审核方式、UI 形态）
5. 如果建议不做：明确 market-panel 怎么处理（删除？改成"导入"？保留占位但说明）

### 交付

`.loom/design/MARKET_DECISION.md`，开头 200 字内必须是结论。

---

## Task 3：提示词 + 轮盘合并设计

### 现状

- 主窗口 (`src/index.html`) 5 个 nav：**提示词 / 轮盘 / 设置 / 市场 / 日志**
- 「提示词」页 = CRUD 列表 + 卡片/列表视图 + 标签
- 「轮盘」页 (`src/index.html` wheel-panel) = 配置哪些提示词置顶到轮盘（`toggle_prompt_pin` / `get_all_prompts_with_pin`）
- 轮盘本体是独立窗口 (`src/wheel.html`)，500px 六扇区花瓣，分页 PageUp/PageDown
- `service/src/db.rs:132` 有 `selected_prompt` 表

### 用户要求

「提示词和轮盘直接合并」

### 设计要求

1. **先回答"合并到底是什么意思"**：是把两个页面合成一个？还是让"置顶到轮盘"变成提示词列表上的一个内联操作（无需独立页面）？还是重构成一个统一视图？给出你的判断和理由
2. **合并后的信息架构**：导航变成几个？每个页面职责是什么？
3. **围绕合并后的新形态，哪些功能值得加，哪些不值得加**（用户原话：「想想看围绕我们的这个需求，还有哪些功能值得添加很必要的就做，没必要的就不做」）
   - 提示：从"一个 AI 重度用户每天真实的工作流"出发倒推，而不是从功能清单出发堆叠
   - 例如可以考虑的方向（**不限定这些，也不要求都做**）：变量填充、最近使用、使用频率排序、快速试运行、提示词版本对比、模板继承、导入导出格式
   - **每个候选功能必须给"必要性评级"**：必需 / 有价值可延后 / 不做（并各给一句理由）
4. **与"日志"页的关系**是否也要重新考虑（日志是否配得上独立 tab）

### 交付

`.loom/design/MERGE_ARCHITECTURE.md`，含合并后的信息架构图（文字/mermaid 均可）+ 功能必要性评级表。

---

## Task 4：UI/UX 全面革新设计稿

### 现状（已核实）

- 主窗口：`src/index.html` (215行) + `src/styles.css` (1150行) + `src/main_simple.js` (1365行)，**单 HTML 无框架**
- 轮盘：`src/wheel.html` + `src/wheel.css` (258行) + `src/wheel.js` (204行)，iOS 玻璃拟态深色，**`--wheel-size: 500px`**
- 选择器面板：`src/selector.html` + `src/selector.css` + `src/selector.js` (387行，Fuse.js 模糊搜索)
- 窗口定义：`tauri.conf.json` — main 1000x700 可缩放；轮盘窗口在 `src/main.rs` 里创建（`show_wheel_window`）
- 设计系统：`GUI_RENOVATION_GUIDE.md` 定了 Zinc 深色调色板，但**只有深色一套，无明暗切换，无 i18n**

### 设计要求

**这是本 Phase 最重的交付。用户原话：「美观，用这个UI库，延续，但是需要加入动效，稳定性也需要提高」**

1. **UI 库选型（必须先决策）**：
   - 现状是裸 HTML + 手写 CSS。要不要引入框架/库？
   - 评估方向：保持 vanilla + 设计系统化（最小依赖，最稳）/ 引入 Tailwind / 引入 React + 组件库（shadcn 风格是 GUI_RENOVATION_GUIDE.md 提过的方向）
   - **用户说"用这个UI库，延续"** — 指延续 Zinc/slate 深色工业风的设计语言。请给出选型建议 + 理由，考虑：Tauri 打包体积、构建复杂度、动画能力、长期维护
   - 明确：如果建议引入框架，`frontendDist` 和 `beforeBuildCommand` 需要怎么改
2. **动效规范**：列出每个交互的动效（时长 / 缓动 / 触发条件），遵守 `prefers-reduced-motion`。**重点是"加入动效"但不能影响性能与稳定性**
3. **轮盘重新设计（用户点名）**：
   - 500px 太大 — **尺寸、形态、位置都要按 UX 重新想**，不要只做等比缩小
   - 考虑方向（不限定）：缩到多小合适？鼠标悬停区域/命中区怎么保证？要不要跟随鼠标位置弹出而不是屏幕正中？扇区数量固定 6 还是自适应？改为列表/网格形态是否更快？高分屏 DPI 缩放
   - 给出 2-3 个候选形态 + 你推荐的一个 + 理由（从"热键到完成注入"的操作步数和视觉干扰两方面论证）
4. **中英文双版（i18n）**：
   - 现状全中文硬编码在 HTML 里
   - 给出 i18n 方案（运行时切换？跟随系统？t 函数还是字典文件？）
   - 要把现有所有 UI 文案抽出来做成 key 清单（中文值先填好）
5. **明暗模式**：
   - CSS 变量双层 + `prefers-color-scheme` + 手动覆盖 + 持久化
   - 注意轮盘窗口是透明的，明暗两套都要保证在任意桌面背景下可读
6. **稳定性提高**：识别当前前端有哪些不稳定因素（如 `alert()` 兜底、无错误边界、DOM 操作无防护、`updateDebugInfo` 当日志用等），给出加固清单

### 交付

- `.loom/design/UI_UX_REDESIGN.md` — 总纲：设计系统（色板/字阶/间距/圆角/阴影/动效 token，明暗两套）+ 信息架构 + 每个页面的布局说明
- **高保真原型**（用户要的是能看出长什么样的，不是线稿）：
  - 用单文件 HTML + 内联 CSS 实现，放 `.loom/design/prototypes/` 下
  - 至少覆盖：主窗口（提示词/合并后的视图）、轮盘新形态（2-3 个候选都做，用真数据填充，可交互看动效）
  - 明暗两套都要能看到（用按钮切换）
  - 中英文都能看到（用按钮切换）
  - **不要提交 PR，直接留文件**（用户明确说过这种预览不走 PR）

---

## Task 5：LOOM 初始化

项目当前没有 `.loom/`，但 loom CLI 2.1.3 在位。

1. `loom init` 建立骨架
2. 把 North_Star.md 的核心内容 `loom record` 进去（愿景/KPI/边界/里程碑）
3. 为 Phase 1 的四个研究方向建立 capability 条目（注入机制 / 市场机制 / 产品合并架构 / UI 设计系统），每个写出你的专业判断
4. `loom deliverable add` 划分交付面
5. **不要 `loom task plan` / `loom task start`** — 那是 Phase 2 的事，等用户看完设计
6. 不要创建/modify `00_READ_ME_FIRST.md` 到 `07_DECISIONS_AND_OPEN_QUESTIONS.md` 里的内容除非任务要求

---

## Task 6：跨平台工程化调研（新要求）

用户原话：「那顺便把我们的linux和mac端也给做了吧」

### 调研要求

1. **当前代码的平台耦合点**（逐一列出 文件:行号）：
   - `src/main.rs` — `#![windows_subsystem = "windows"]`、service 启动名 `service.exe` vs `service`
   - `service/src/injector/mod.rs` — Win32 API 直调
   - `service/src/hotkey/mod.rs` — 全局热键实现
   - `service/src/context/mod.rs` — 前台窗口检测
   - `tauri.conf.json` — bundle targets "all"、icon 只有 `.ico`（Windows only）
2. **每平台的工作量与阻塞点**：
   - Linux: X11/Wayland 双栈、全局热键（Wayland 限制）、注入（Wayland 限制）、打包（deb/AppImage）
   - macOS: 辅助功能权限（需用户手动授权，影响首启体验）、notarization/签名、DMG
3. **图标资源**：现只有 `PromptKey.ico`。macOS 需 `.icns`，Linux 需 PNG 多尺寸。给出生成方案
4. **service 独立进程**：三平台的进程名/路径/自启机制（Windows 注册表 / Linux systemd user unit / macOS LaunchAgent）
5. **给出分阶段落地路线**（哪些先做、哪些可以后做），以及**哪些是"做了 linux/mac 但不一定能用"的硬限制**（必须诚实说，例如 Wayland 下模拟输入大概率不可行）

### 交付

`.loom/design/CROSS_PLATFORM_PLAN.md`，开头 300 字内给结论 + 阶段路线表。

---

# 硬约束

- **Phase 1 不写任何实现代码。** 不改 `src/*`、`service/src/*`、`Cargo.toml`、`tauri.conf.json`
- 允许：新建 `.loom/`、新建 `.loom/design/*.md`、新建 `.loom/design/prototypes/*.html`
- 可以跑只读命令做调研（`git log`、读文件、`loom` 子命令）
- 本机**没有 cargo**，不要尝试 `cargo build` / `cargo check`，会浪费时间
- 报告里所有涉及"现状"的陈述必须带 `文件:行号` 证据
- 不要臆造 Fuse.js / crate 的行为，不确定就标"需验证"

# 交付格式

所有 `.loom/design/*.md` 用中文写（项目是中文项目，UI 也是中文）。代码/标识符保持原文。
每个文档开头用「## 结论」小节，300 字内给结论，然后才是论证。

# Git

- 新建分支 `feat/phase1-research-design`
- 只提交文档与 loom 骨架和原型文件，**不要提交任何源码改动**
- push 到 origin，**不要开 PR，不要合并**（用户 review 后再说）

```json
COMPLETION_NOTIFY
source: assistant
task: promptkey-phase1-research
deliverables:
- INJECTION_RESEARCH.md (strategy matrix + 3-platform plan)
- MARKET_DECISION.md (build or not, with recommendation)
- MERGE_ARCHITECTURE.md (prompts+wheel merge IA + feature necessity ratings)
- UI_UX_REDESIGN.md (design system incl. dark/light + motion tokens)
- hi-fi prototypes: main window + 2-3 wheel forms, i18n + theme switchable
- CROSS_PLATFORM_PLAN.md (linux/macos route with honest limits)
- .loom initialized with capabilities recorded
- branch feat/phase1-research-design pushed (no PR, no merge)
status: success
errors: none
```
