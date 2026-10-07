# Devin Brief: README 门面重写 + Release notes + Linux/macOS 跨平台实现（P1-P3）

## 项目
`/home/haa/sites/promptkey`，当前 `master` HEAD `baa37fe`（2.0.4 已发布）。新建分支 `feat/readme-release-notes-and-cross-platform`。

## 背景

用户已装 2.0.4，轮盘、拖拽排序都已验收通过。现在要求两件事：

1. **仓库门面全面更新** —— README 重写、Release notes 写好、截图、设计文档、组件说明、功能清单。README 现有 146 行但内容是 2.0 早期状态，多处已过时。
2. **Linux 和 macOS 也要做** —— 用户原话：「而且linux，macos也要做...当然要写好因为没有测试，不确定能否正常使用」。

用户明确知道**没有真机测试**，所以要求"写好"即：**工程质量要扎实、文档要诚实标注验证状态、不能假装已测**。

`.loom/design/CROSS_PLATFORM_PLAN.md`（125 行）已有详尽计划，**先读完再动手**，按其 P1-P4 阶段推进，本轮做到 P3。

---

# 任务总览

## Task A：跨平台实现（P1 抽象 + P2 Linux-X11 + P3 macOS）

### P1：抽 trait + Windows 加固（必做）

`.loom/design/CROSS_PLATFORM_PLAN.md:32` 已给出结构判断：

- `injector` / `hotkey` / `context` 三个模块正好对应三个 trait 抽象
- service 主循环除 `HWND` 类型外本身平台中立
- **`AppContext.window_handle` 需改为 `u64`/`isize` 不透明句柄**（Windows 存 HWND bit，macOS 存 AXUIElement ref 或 0）

要求：
- 抽出 `Injector` / `Context` / `Hotkey` 三个 trait，Windows 实现迁到 `*_impl` 后不动逻辑
- **Windows 行为必须零变化** —— 这是已验收的可用版本，任何回归都是 P0
- 保留 service 内嵌线程架构（`src/main.rs:95-98`），**不做独立守护进程**

### P2：Linux-X11（必做）

按 plan §3：
| 能力 | 选择 |
|---|---|
| 全局热键 | `global-hotkey`（X11 稳定） |
| 剪贴板 | `arboard`（X11 Tier-1） |
| 注入 | `enigo`（x11rb 后端） |
| 前台上下文 | X11 焦点窗口查询 |
| 自动启动 | `~/.config/autostart/promptkey.desktop`（XDG Autostart），不引入 systemd user unit |

**Wayland 检测**：检测到 Wayland 会话时，**明确提示降级**（"当前会话为 Wayland，自动注入需额外授权"），但管理功能全可用。

打包：`tauri build` 原生产出 `.deb` + `.AppImage`。

### P3：macOS（必做）

按 plan §3 与 §3 的 UX 阻塞点：
- 注入：**AX API**（`AXUIElement`）为主，剪贴板兜底
- 全局热键：macOS 原生（`global-hotkey` 的 macos 后端或 CGEventTap，你论证选型）
- 剪贴板：`arboard`（macOS Tier-1）
- **首启 AX 授权引导页（必须做）**：`AXIsProcessTrustedWithOptions(prompt=true)` 弹系统对话框 + 应用内权限状态轮询 + **权限缺失时降级为"复制到剪贴板"模式**
- 自动启动：`SMLoginItemSetEnabled` 或 `plugin-autostart`
- 图标：macOS `.icns`（`iconutil` 从 iconset 生成，脚本化）；tray icon 单独做**单色可模板化**版本（macOS template image 要求）
- 打包：`.dmg` / `.app`；**公证需付费账号，文档说明**

### P4 Wayland 增强（可选，本轮可不做）

`ashpd` GlobalShortcuts portal + `ydotool` opt-in。**如果时间/风险不允许，明确列为"未做"并写清原因**，不要留半成品。

---

## Task B：README 全面重写

现有 README 的问题：
- 功能描述停在 2.0 早期（如默认热键写 `Ctrl+Alt+A`，实际是 `Ctrl+Alt+Q`）
- 没有截图
- 没有设计文档/组件说明
- 没有 2.0.x 的新功能（点按录制式热键录入、链路诊断、pointer 拖拽排序、模板库、capabilities 修复等）
- 没有跨平台章节（Linux/macOS 状态与限制必须诚实写）

### 必须包含

1. **Hero 区** —— 一句话定位 + logo + badges（保持现有风格，更新技术栈徽章：加 Linux/macOS）
2. **功能特性** —— 按 2.0.4 实际功能重写，**逐条与代码核对**，不写不存在的功能
3. **截图区** —— 见 Task C
4. **快速开始** —— 安装（Windows/Linux/macOS 三平台分别说明）+ 使用 + 默认热键表（**热键值必须与代码一致，去核实**）
5. **配置说明** —— 配置项名称与代码核对
6. **从源码构建** —— 三平台分别写
7. **项目结构** —— 更新到当前真实结构（注意 `src/` 与 `service/` 的实际划分、`capabilities/` 目录、`.loom/` 设计文档位置）
8. **设计文档索引** —— 链接 `.loom/design/` 下的研究/设计文档，每个一两句摘要
9. **组件说明** —— 见 Task D
10. **平台支持矩阵** —— 诚实标注每个平台的能力与验证状态（**见"诚实性要求"**）
11. **更新日志** —— 链接 CHANGELOG.md
12. **贡献/致谢** —— 保留现有结尾风格

## Task C：截图（重要，但不能造假）

**没有真机运行环境，禁止用"看起来像"的假图。**

允许的做法（按优先级）：
1. **从 e2e 截图产物来** —— `tests/e2e/wheel_sort_drag_e2e.py` 等测试会产出截图，检查是否有可用产物，挑选/命名后放入 `docs/screenshots/`
2. **从原型 HTML 渲染** —— `.loom/design/prototypes/` 下四个 HTML，用 Playwright headless 截图，**在 README 中明确标注"原型渲染，非最终产品截图"**
3. **ASCII/结构图** —— 用 mermaid 画架构图、轮盘交互流程图

**每个截图/图必须带 `alt` 文字**。

如果某个图只能给原型示意，**在图片下方明确标注**。宁可少而真，不要多而假。

## Task D：组件说明（新增 `docs/COMPONENTS.md`）

前端是 vanilla ES module + 设计系统，需要一份组件清单让人能接手：

- **模块划分** —— `src/js/` 下每个文件的职责（app/store/dom/toast/theme/icons/i18n/wheel/hotkey_recorder + views/*）
- **设计系统** —— 色彩 tokens（明暗双主题）、间距、圆角、字号层级、动效规范。**从 `src/styles.css` 实际提取**，不要编
- **组件** —— 按钮体系（`.btn`/`.btn-primary`/`.btn-ghost`/`.mini-link`）、开关（`.switch`）、分段控件（`.seg`）、抽屉（`.drawer`）、toast、热键录入器、轮盘（`.petal`/`.glass`/`.center`）、状态行（`.hk-state`）、错误横幅（`.env-err`）—— 每个说明用途 + 关键 class + 使用时注意
- **i18n 机制** —— 运行时切换怎么做的，新增语言要怎么加
- **图标** —— 图标从哪来（lucide + 品牌图标），怎么加新图标
- **测试资产** —— `tests/e2e/` 四个 e2e 各自覆盖什么，怎么跑

## Task E：Release notes 写好 2.0.0-2.0.4

`gh release edit` 或 API 更新 **v2.0.0 / v2.0.1 / v2.0.2 / v2.0.3 / v2.0.4** 五个 Release 的 notes。

要求：
- 每个版本写清：**做了什么、为什么、验证到什么程度**
- **2.0.1-2.0.4 是修 bug 的版本，notes 里要写清症状与根因**（这对用户和理解者都有价值），例如 2.0.2 的 `present_wheel` emit 顺序、2.0.3 的 capabilities 缺失、2.0.4 的拖拽排序
- 保持诚实：**未真机验证的能力要标注**
- 格式统一，风格专业（可参考 GitHub 上成熟项目的 release notes）

## Task F：文档清单核对

除 README + COMPONENTS.md 外，确认以下文档状态并补全：
- `CHANGELOG.md` —— 更新到 2.0.4，格式统一
- `.loom/design/` 下的设计文档 —— 在 README 里做索引，**但不要搬位置**
- 跨平台的页面（Linux 构建、macOS 公证、Wayland 降级）写清楚，放 README 的平台章节或独立 `docs/PLATFORMS.md`

---

# 诚实性要求（最高优先级）

用户原话：「**当然要写好因为没有测试，不确定能否正常使用**」

这意味着：

1. **绝不宣称"已测试通过"** —— Linux/macOS 路径本机没跑过（本机是 Linux 但项目当前 build 依赖 Windows-only crate，CI 是否覆盖 Linux 由你核实并报告）
2. **README 平台支持矩阵必须分三列写清**：功能支持性 / 打包产物 / **验证状态**（✅ 已实机验证 / ⚠️ 代码就绪未验证 / ❌ 不支持）
3. **Wayland 限制、macOS 授权摩擦、未公证** 都要明说，不藏在脚注
4. **CI 如果不覆盖 Linux/macOS，明确说"CI 未覆盖"**，并评估是否该加（如果要加，告诉我成本，不要擅自加一堆跑不通的 job）
5. 你做的任何截图，如果是原型渲染的，**标注原型**

---

# 硬约束

- 无 cargo，本机不编译。**CI 必须全绿**
- Windows 行为**零回归** —— P1 抽 trait 时尤其小心，这是已验收的可用版本
- Playwright 全绿；如为跨平台加了新测试，也要跑
- 遵守设计系统与 i18n（新文案中英双语）
- 文档里的每个技术声明都要**能在代码里找到依据**
- **不要动 `capabilities/` 的既有结论**
- commit 规范：可多个 commit（按 P1/P2/P3/README/COMPONENTS/RELEASE-NOTES 分开），push，PR `--base master`，**不合并**
- 遵守 LOOM 流程

---

## 最终交付

```
## 跨平台实现（P1/P2/P3）
（每阶段：做了什么、选型理由、**验证到什么程度**（编译？CI？真机？）、遗留风险）
### P4 状态
（做了/没做，原因）

## README 重写
（新增哪些章节；逐条核对过哪些过时内容并修正——特别是热键值；截图来源清单）

## COMPONENTS.md
（模块/设计系统/组件/i18n/图标/测试 六部分摘要）

## Release notes
（5 个版本 notes 的要点；诚实性怎么体现的）

## 验证状态汇总
（一张表：能力 × 平台 × 验证状态；CI 覆盖情况；**哪些是本机绝对测不了的**）

## 用户在 Linux/macOS 上首次使用的预期
（会看到什么；Wayland 会话什么表现；macOS 首次授权流程；出问题时怎么反馈）

## 未编译验证清单
```

```json
COMPLETION_NOTIFY
source: assistant
task: promptkey-readme-release-and-crossplatform
deliverables:
- P1 traits extracted with zero Windows regression
- P2 Linux-X11 implemented (global-hotkey + arboard + enigo + XDG autostart)
- P3 macOS implemented (AX inject + permission onboarding + icns + template tray)
- Wayland/macOS limits documented honestly with verification status matrix
- README fully rewritten with verified facts, screenshots (real or clearly-labelled prototypes), design docs index
- docs/COMPONENTS.md covering modules, design system, components, i18n, icons, tests
- CHANGELOG.md updated to 2.0.4
- release notes rewritten for v2.0.0-v2.0.4 with symptom→root-cause honesty
- CI green, no fabricated test claims
- branch feat/readme-release-notes-and-cross-platform + PR against master (not merged)
status: success
errors: none
```
