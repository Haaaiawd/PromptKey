# PromptKey 注入机制研究报告

> Loom slug: `injection-research` | Kind: research | Phase 1 交付物

## 结论

保留 Clipboard→SendInput 主链路。三个缺口：(1) Phase0 删 UIA 时**连密码框检测一起删了**，"不注入密码框"边界当前零实现——建议以"只读探测"拿回 UIA/AX 门禁用途，不恢复整条 UIA 注入路径；(2) 剪贴板有 ~300ms 污染窗口期且只备份文本格式；(3) 跨平台应抽象统一 `Injector` trait：Windows=现有 Win32，macOS=AXUIElement→CGEvent，Linux-X11=XTEST+剪贴板，Wayland=uinput 或明确降级——Wayland 无免权限注入方案，这是平台设计使然。

---

## 1. 现状链路（带证据）

```
用户按 Ctrl+Alt+Q（默认热键，service/src/config/mod.rs:101）
  → HotkeyService 消息循环，PeekMessageW 收到 WM_HOTKEY id=4
      (service/src/hotkey/mod.rs:106-124)
  → service 主循环捕获上下文（前台 hwnd/标题/进程名）
      (service/src/main.rs:63-76 → context/mod.rs:27-62)
  → IPCClient.send_show_wheel() 写 "\\.\pipe\promptkey_selector" 文本 "SHOW_WHEEL\n"
      (service/src/ipc/mod.rs:66-94)
  → GUI 端 ipc_listener 收到 → wheel-panel 窗口 show+focus
      (src/ipc_listener.rs:62-73)
  → 用户点击花瓣 → trigger_wheel_injection → 写 "\\.\pipe\promptkey_inject" "INJECT_PROMPT:{id}\n"
      (src/wheel.js:127-141 → src/inject_pipe_client.rs:11-22)
  → inject_server 解析 prompt_id 投递给主循环
      (service/src/ipc/inject_server.rs:44-82)
  → handle_injection_request(force_id) → db.get_prompt_by_id → injector.inject()
      (service/src/main.rs:86-174)
  → inject(): Clipboard 主 → SendInput 回退
      (service/src/injector/mod.rs:43-78)
      · Clipboard: SetForegroundWindow → sleep(pre_inject_delay) →
        OpenClipboard×5 → 备份旧文本(≤1M UTF-16) → EmptyClipboard →
        SetClipboardData → sleep(200ms) → SendInput Ctrl+V →
        sleep(100ms) → 恢复旧剪贴板   (injector/mod.rs:82-254)
      · SendInput: SetForegroundWindow → 逐 UTF-16 字符
        KEYEVENTF_UNICODE down/up     (injector/mod.rs:256-323)
```

**调研中发现的与 Brief 不符的事实（如实报告）**：

| Brief 陈述 | 实际代码 | 证据 |
|---|---|---|
| `find_prompt_for_context()` 按 app_scopes 匹配 | 参数 `_app_name`/`_window_title` 被忽略，只返回 `selected_prompt` 行 | `service/src/db.rs:401-414` |
| "逐字符 SendInput with UNICODE" 是"没有的新方向" | 已实现——现有回退就是逐字符 `KEYEVENTF_UNICODE` | `service/src/injector/mod.rs:287-320` |
| service 是独立进程（`service.exe`） | service 已内嵌为 GUI 子线程；`resolve_service_exe_path` 是 dead code | `src/main.rs:95-98,112` |
| 选择器面板可用（Fuse.js 搜索） | 是死代码：热键 ID 3 已移除，`SHOW_SELECTOR` 永远不会被发送 | `service/src/hotkey/mod.rs:103`（注释）、`service/src/main.rs:64-77`（只处理 id=4） |
| "不注入密码框"边界 | **无任何实现**：密码框检测随 UIA 一并被删，当前对任意焦点控件都会粘贴 | 全文件无 `IsPassword`/`ControlType` 引用（grep 可证） |

另发现：使用日志在注入**之前**写入且 `success=true` 硬编码（`service/src/main.rs:139-153` vs `163`），日志面板里的成功率和耗时数据不可信——这影响"注入成功率"KPI 的度量能力，见 §7 加固建议。

---

## 2. UIA 为什么被移除？（git 考古）

**结论：不是"UIA 不可用"，而是"当时的 UIA 实现是项目最大的复杂度与安全隐患集中点"，删除是有意的降复杂度决策，并且事后证明是对的。**

证据链：

1. **动机**：`blueprint/00_PRD_UIA_REMOVAL.md` L14-18 — `injector/mod.rs` 当时 CCN 72、技术债分 648、839 行，内含两个真实安全问题：`probe_selection_via_clipboard()`（为探测选区而**篡改用户剪贴板**）和 unsafe 越界读取。PRD 的 Non-Goal 明确"接受某些特殊应用（如禁用粘贴的终端）可能无法工作"。
2. **执行**：commit `cc41dd0` 删 `inject_via_uia` 等 7 函数 ~440 LOC；`c12d180`/`503d69c`/`d909f0b`/`d1fdd09` 完成 enum、配置兼容；`295f603` 清理 EditorType 残骸。
3. **结果**：`PHASE0_EXECUTIVE_SUMMARY.md` — 净删 475 行，CCN 72→9，剪贴板路径实测 2930 字符 264ms、测试内 100% 成功。
4. **UIA 不可靠的历史证据**：已删除的 `.qoder/quests/project-review-uia-troubleshooting.md`（commit `e577276` 引入、`7331a12` 删除）记录 ValuePattern 在 VS Code/IntelliJ/Notepad++/Sublime 全部不可用或半可用，UIA 注入实测成功率约 85%（记事本）——即 **UIA 并没有兑现它承诺的可靠性**，反而 clipboard 更稳。

**是否值得重新引入？** 分两种用途分别回答：

- **作为注入手段**：不值得完整恢复。ValuePattern.SetValue 对 Chromium/Electron/Java Swing/Scintilla 系控件覆盖差（历史文档已证），恢复它要同时恢复编辑器类型检测的复杂度，正是当年被删的东西。**可选的轻量版本**见 §4。
- **作为只读门禁**：值得。North_Star 边界"不注入密码框"当前零实现，而 UIA `IsPassword`/`ControlType` 查询是最可靠的检测手段（Windows）；macOS 对应 AX role `AXSecureTextField`。**只读探测 ~50 LOC，不碰注入路径**，复杂度可控。这符合"可降级"原则：探测失败时按"非密码框"放行还是按"未知即拒绝"需要产品决策（建议放行但记日志，密码框检测只做"明确检出才拦截"）。

---

## 3. 策略适用边界与测试矩阵

### 3.1 各策略真实边界

**Clipboard + Ctrl+V（现有主策略）**
- 适用：几乎所有接受标准粘贴的控件（Win32 Edit/RichEdit、WPF、Electron/Chromium 编辑器、浏览器文本域、Electron 应用、Word、各类聊天框）
- 不适用/失败点：禁粘贴控件（罕见）；终端类（Windows Terminal/CMD 用 Ctrl+Shift+V 或右键粘贴，Ctrl+V 是字面字符）；部分游戏的自绘 UI；远程桌面/虚拟机剪贴板隔离场景
- 副作用：剪贴板污染窗口期（当前实现备份→粘贴→恢复，约 300ms 窗口）；若用户剪贴板里是**非文本**（图片/文件），当前代码只备份 CF_UNICODETEXT——`injector/mod.rs:117` 只检测 format 13，富格式内容会**永久丢失**。这是一个实际存在的数据丢失边界，需如实告知。
- IME：无干扰（走粘贴不走键盘事件）

**SendInput UNICODE 逐字（现有回退）**
- 适用：禁粘贴场景、把文本当作"按键流"的场景
- 不适用/慢：长文本（逐字符 SendInput，每字符 2 个 INPUT 事件；万字符级提示词会明显拖慢且有丢字符风险）；注入期间用户按键会**交错混入**；换行 `\n` 在 UNICODE 模式下映射为 Return 还是换行字符依控件而异；某些应用对快速输入做节流
- 中文环境：`KEYEVENTF_UNICODE` 直接注入 UTF-16 码元，**不经过 IME**，因此中文输入法开着也不会被截获——这恰是它优于虚拟键码方案的地方，现有实现方向正确
- 风险：注入中途焦点被偷走会把文本打进错误的窗口

**UIA ValuePattern.SetValue（已删）**
- 适用：Win32 Edit、WPF/WinUI TextBox、部分表单
- 不适用：密码框（`IsPassword=true` 时 SetValue 返回错误——这其实是特性）；Chromium contenteditable 历史上不稳；Sublime/自绘控件无 UIA 树
- 权限：受 UIPI（User Interface Privilege Isolation）限制——无法向更高完整性级别的窗口注入（如管理员身份运行的应用），此边界对 SendInput/剪贴板路径同样成立，见 §6

**UIA TextPattern（只读定位）**
- 能读不能写（粘贴/输入仍需别的手段）；用于插入点定位、选区检测，历史实现复杂度大头就在这里——不建议恢复

**新策略评估（brief 要求 ≥2 个代码外方向）：**

| 方向 | 评估 | 结论 |
|---|---|---|
| `WM_PASTE`/`EM_REPLACESEL`/`WM_CHAR` 消息直投 | 只对经典 Win32 Edit/RichEdit 控件有效（记事本、旧式对话框）；Electron/浏览器/Electron 全部忽略（它们不消费 WM_*）；SentMessage 路线**绕过剪贴板**且不改变焦点语义 | **有价值但窄**：作为 Clipboard 之前的"快速通道"只在 `className ∈ {Edit, RichEdit*, *EditWnd*}` 时启用，能力驱动判定，命中即毫秒级且不污染剪贴板；命中率低但成本极低 |
| macOS `AXSelectedText`/`AXValue` 直写 | `AXUIElementSetAttributeValue(el, kAXValueAttribute, text)` 或写 `kAXSelectedTextAttribute` 在光标处插入——**不碰剪贴板**、可读 `AXSecureTextField` role 做密码框拒绝；macOS 上这是 UIA 的正对应物 | **macOS 首选路径**（见 §5） |
| Linux uinput（ydotool 模式） | 内核级虚拟键盘，X11/Wayland/GNOME 全通吃，无协议依赖；代价是 `/dev/uinput` 访问权限（input 组或 systemd user unit 跑 ydotoold） | **Linux Wayland 唯一通用答案** |
| `wtype`/`virtual-keyboard-v1` | wlroots（sway/Hyprland）✅、KDE 部分、**GNOME 明确不支持**（Mutter 未实现该协议）；enigo 0.6 的 wayland feature 即此路线，标 Experimental | 只能作条件分支，不能做主路径 |
| XDG RemoteDesktop portal | GNOME/KDE 可用，但每次会话要授权弹窗，UX 割裂 | 备选，不推荐主用 |
| TSF（Text Services Framework） | enigo 0.6.1 的 windows feature 已含 `Win32_UI_TextServices`——面向 IME 感知输入，学习成本高 | 暂列观察项，不建议本期采用 |
| Chrome DevTools Protocol / 浏览器扩展 | 对浏览器内容最可靠但只对浏览器 | 范围外 |

### 3.2 可复现测试矩阵（Phase 2 执行用）

**矩阵维度**：策略 × 目标应用 × 文本形态。文本形态分四档：ASCII 短文本（<200 字符）、中文混合（"请帮我润色这段话：……"）、长文本（3k 字符）、含换行/引号/emoji。

**Windows 目标应用清单**（沿用 TODO.md 既有清单扩展）：
Notepad（Edit）/ WordPad（RichEdit）/ VS Code（Electron+Monaco）/ Chrome 地址栏+textarea+contenteditable / Windows Terminal / WeChat 输入框 / IntelliJ（Swing）/ 记事本++（Scintilla）/ 高完整性进程（以管理员运行的记事本）/ 密码框（`type=password`，验证拒绝行为）。

**判定通过标准**：注入文本逐字一致（含换行）；剪贴板恢复后内容与注入前一致（含非文本格式）；耗时记录。

跑法建议：手工为主 + 每应用固定 3 次取中位；把结果写进 `usage_logs.strategy` 做真实分布统计（先修 §7 的日志前置 bug）。

---

## 4. 推荐策略顺序 + 判定逻辑

**原则**：能力驱动、小步扩展、每步可降级。不建议恢复"按应用名查表"的旧路（`config.applications` 里遗留的 per-app strategy 已无人消费，见 `service/src/config/mod.rs:222-298`——这些配置条目引用的 `uia`/`textpattern_enhanced` 策略已不存在，属死配置）。

```
Windows 推荐链路（v2）：

  0. Gate（新增，~只读）
     focused = UIA_GetFocusedElement()            // 仅探测，非注入
     if focused.IsPassword || focused.ControlType==SecureEdit → ABORT(记日志)
     探测失败 → 放行（fail-open，但 log "gate_unchecked"）

  1. FastPath（新增，可选）
     if GetClassName(hwnd) ∈ {Edit, RichEdit20W*, RICHEDIT50W}
        → SendMessage(EM_REPLACESEL / WM_PASTE)   // 不碰剪贴板，<10ms
        成功 → DONE

  2. Clipboard（现有，主路径）
     备份全格式 → 写入 → Ctrl+V → 恢复
     *修复点*：备份要枚举全部剪贴板格式（IsClipboardFormatAvailable 逐个查），
     至少区分 text/image/file，避免丢非文本内容；或改用
     延迟渲染（delayed rendering）式的零拷贝备份——待验证

  3. SendInput UNICODE（现有，兜底）
     逐字符；建议加：分批 yield + 注入期间锁热键 + 中止键(Esc 监听)

  判失败条件：
  - 任何一步返回 Err → 下一级；全失败 → 日志 + UI 提示（不静默）
  - "粘贴了但目标是终端" → 用 Ctrl+Shift+V 重试一次（按 ConsoleWindowClass/
    WindowsTerminal 类名做能力判断，不是应用黑名单）

伪代码：
fn inject(text, ctx):
    gate = probe_secure_field(ctx.hwnd)          // UIA 只读或 className 启发式
    if gate == ConfirmedSecure: return Err("refused: secure field")
    if is_classic_edit(ctx.hwnd):                 // WM_* 能力检测
        try msg_paste(); if ok return
    if config.allow_clipboard:
        try clipboard_paste(text); if ok return
    return sendinput_unicode(text)
```

---

## 5. 三平台方案

### 抽象结论（brief 必答项）

**统一 trait，而非 service 层分支。** 调用点只有 `injector.inject()` 一处（`service/src/main.rs:163`），天然就是 trait 边界。建议：

```rust
// service/src/injector/mod.rs（Phase 2 结构，本 Phase 不写代码）
pub trait Injector {
    fn inject(&self, text: &str, ctx: &InjectionContext)
        -> Result<(StrategyName, u64 /*ms*/), InjectError>;
}
#[cfg(windows)] mod win32_impl;    // 现有代码搬入
#[cfg(target_os = "macos")] mod macos_impl;
#[cfg(target_os = "linux")] mod linux_impl;
```

同理 `context`（前台窗口/进程名）、`hotkey` 各做一个 trait。平台差异收进各自 impl，service 主循环零 `cfg`。

### Windows（现状即可用）
- 注入：现有 Clipboard/SendInput（`windows` crate 0.58，`service/Cargo.toml`）
- 热键：`RegisterHotKey` + 消息循环（`service/src/hotkey/mod.rs`）
- 上下文：`GetForegroundWindow`/`GetWindowTextW`/`K32GetProcessImageFileNameW`（`context/mod.rs:31,68,116`）
- 门禁新增：UIA 只读探测 → `windows` crate `Win32_UI_Accessibility` feature 重新启用（现 Cargo.toml 还留着该 feature，删了代码没删 feature——小技术债）

### macOS
- **注入首选**：AXUIElement。`system.focused_ui_element` → 读 `AXRole`（拒绝 `AXSecureTextField`——密码框天生标记）→ 写 `AXSelectedText`（光标处插入，不碰剪贴板）或 `AXValue`（整体覆写，慎用）。crate：**`axuielement` v0.10**（活跃维护，Swift bridge over C API，覆盖 Attribute 读写/FocusedElement/ProcessTrust）或底层 `accessibility`/`accessibility-sys`。
- **注入回退**：NSPasteboard（`arboard`）写文本 + `CGEvent` 模拟 ⌘V（`core-graphics` crate 或 enigo）。
- **权限硬约束**：AX 读写与 CGEvent 注入都需要"辅助功能"权限（`AXIsProcessTrusted`）；首启必须做权限引导页（系统设置 → 隐私与安全性 → 辅助功能），这是 macOS 用户体验的最大成本，需接受为产品现实。全局热键用 `global-hotkey` crate（Carbon RegisterEventHotKey 路线，macOS 支持、不需 AX 权限）。
- **上下文**：`NSWorkspace.frontmostApplication`（`objc2-app-kit`）+ AXFocusedWindow 标题。

### Linux — 分 X11 / Wayland 两套现实

**X11**：`x11rb` 或 enigo 的 XTEST（enigo 0.6.1 默认 feature `x11rb` 已不需要外部 xdotool 二进制）；剪贴板 `arboard`（X11 Tier-1）；全局热键 `global-hotkey`（X11 支持）；前台窗口 `_NET_ACTIVE_WINDOW`/`_NET_WM_NAME` + `/proc/<pid>/comm`。**结论：X11 下全功能可做。**

**Wayland（必须诚实说清）**：
- 全局热键：协议层禁止任意应用监听全局按键。可行路径只有三条——(a) XDG GlobalShortcuts portal（KDE Plasma 6 / GNOME 48.8+ 可用；`global-hotkey` PR #162 已实现但截至 2025-11 仍未合并进 release；也可自己走 `ashpd`）；(b) 读 `/dev/input` evdev（要 `input` 组权限或 udev 规则——超出普通桌面应用权限模型）；(c) 让用户在 DE 设置里自定义快捷键指向一个 CLI 命令（`promptkey --inject <id>`，走 socket 通知主进程——最朴素但最可靠）。
- 注入：`wtype`（virtual-keyboard-v1）GNOME 不支持；`ydotool`（uinput）全栈可用但要 `/dev/uinput` 权限 + 用户态 daemon；RemoteDesktop portal 每次会话要授权弹窗；**纯协议级没有免权限方案**。
- 剪贴板：`arboard` 需 `wayland-data-control` feature（ext-data-control-v1/wlr-data-control），GNOME/KDE 支持该扩展但属 Tier-2。

**Linux 落地建议**："**X11 全功能；Wayland 全功能依赖 ydotool + evdev 权限，做成显式 opt-in 的'高级模式'，否则降级为"复制到剪贴板+提示用户手动粘贴"**"——这保留了产品价值（提示词管理/搜索/复制仍可用），不假装 Wayland 能和 Windows 等价。

### crate 选型汇总

| 用途 | Windows | macOS | Linux-X11 | Linux-Wayland |
|---|---|---|---|---|
| 文本注入 | 现有 windows crate | `axuielement` 0.10 + `core-graphics`(CGEvent) 回退 | `enigo` 0.6.1 (x11rb) | enigo `wayland` feature（实验）/ ydotool 子进程 / portal `ashpd` |
| 剪贴板 | 现有 Win32 | `arboard` 3.6.1 | `arboard` 3.6.1 | `arboard` + `wayland-data-control` |
| 全局热键 | 现有 RegisterHotKey | `global-hotkey` | `global-hotkey` | ashpd GlobalShortcuts portal / evdev / DE 快捷键+CLI |
| 前台上下文 | 现有 Win32 | `objc2-app-kit` + AX | `x11rb` EWMH | 部分可得（kdotool/wlr foreign-toplevel，桌面相关） |

维护状态核实：enigo 0.6.1（2025-08 更新，活跃）、arboard 3.6.1（2025-08，1Password 维护）、axuielement 0.10（活跃）、global-hotkey（Tauri 官方，X11-only 稳定版）。`rdev`（全局输入监听）更新偏慢、Wayland 支持弱，不建议引入。**均需 Phase 2 真机验证版本兼容性。**

---

## 6. 安全边界对齐（North_Star → 新策略）

| 边界 | 现状 | 对齐措施 |
|---|---|---|
| 不注入密码框 | **缺失**（随 UIA 删除而消失） | 恢复"只读门禁"：Win=UIA IsPassword/ControlType；mac=AXRole==AXSecureTextField；Linux=无法检测（能力声明降级，文档明示） |
| 不越权高完整性窗口 | 无显式处理（SendInput/SetForegroundWindow 会静默失败） | 注入前比较双方完整性级别（GetProcessIntegrityLevel），失败给出明确错误而非"看起来成功" |
| 剪贴板隐私 | 备份仅文本格式，非文本内容丢失 | 备份枚举全部格式；或提供"不恢复剪贴板"选项；日志不记录剪贴板内容 |
| Named Pipe 安全 | 无 ACL，本机任意进程可发 `INJECT_PROMPT:`（scout/SCOUT_RISK_REPORT.md §Risk#2 已标记） | Phase 2 加安全描述符或改用 localhost socket+token |
| Wayland 权限模型 | — | uinput/evdev 需要显式授权；不提供就意味着功能降级而非绕过 |

---

## 7. 附带发现：观测性缺陷（影响 KPI 度量）

`service/src/main.rs:139-153` 在调用 `inject()` **之前**就写入 `log_usage(..., success=true, "Injected")`——日志面板显示的"成功率/耗时"恒为成功/0ms，实际注入结果从未落库。Phase 2 必须先把 `inject()` 返回值接到日志，否则"注入成功率 ≥99%"这条北极星 KPI 无法被度量。

---

## 8. 待验证清单（需真机）

- [ ] Windows：WM_PASTE/EM_REPLACESEL 在记事本/WordPad/Win32 Edit 的命中率；终端类 Ctrl+Shift+V 探测
- [ ] Windows：UIA 只读探测对主流应用焦点元素的获取成功率（Chromium/Electron/Java/Sublime）
- [ ] Windows：剪贴板全格式备份+恢复的正确性（图片/文件复制场景）
- [ ] macOS：axuielement 在 Safari/Chrome 输入框/Electron 应用的 AXSelectedText 写入成功率；AX 权限引导流程
- [ ] Linux-X11：enigo x11rb 文本注入与 global-hotkey 注册稳定性
- [ ] Linux-Wayland：ydotool daemon 用户态部署可行性；GNOME/KDE 上 GlobalShortcuts portal 实测
- [ ] 三平台：轮盘窗口"跟随光标弹出"在多显示器/DPI 缩放下的定位正确性
