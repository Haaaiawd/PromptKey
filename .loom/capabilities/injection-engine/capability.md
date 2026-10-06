# 注入机制（Windows/Linux/macOS 文本注入策略）

## Field identity and boundary

操作系统级文本注入 / 输入模拟工程。涵盖：Win32 消息与 SendInput、UI Automation（读侧）、macOS Accessibility (AXUIElement) 与 CGEvent、Linux XTEST/uinput/虚拟键盘协议、剪贴板语义与完整性级别/权限模型。不涵盖：UI 设计（→ design-system）、产品形态决策（→ product-architecture）。

## Project scenario

Windows 桌面文本注入工具：Clipboard/SendInput 双策略、密码框门禁、跨平台 trait 边界，用户隐私边界是硬约束

## Decision tree

### C1: 目标控件是否为安全输入控件

- entry_when: 每次注入前的第一道门禁
- options:
  - A: 检测到密码/安全控件 → 拒绝注入并记日志 → leads_to: 输出 Err("refused: secure field")
  - B: 探测结果为普通控件或探测失败 → 放行（fail-open + log）→ leads_to: C2
- decide_by: Windows=UIA `IsPassword`/`ControlType`（只读探测）；macOS=AXRole==`AXSecureTextField`；Linux=无可靠探测，声明降级
- source: phase1-synthesis.md（原始依据：North_Star.md 边界条款；UIA 被删前代码含 `IsPassword` 判定（git: `cc41dd0` 删除体）；当前代码该检测**已缺失**）
- counterexample: 探测 API 本身不可用时若 fail-closed 会杀死全部注入——故选 fail-open + 日志
- output: 注入许可位

### C2: 目标是否为经典 Win32 Edit/RichEdit（Windows 限定快速通道）

- entry_when: C1 放行后，Windows 平台
- options:
  - A: className ∈ {Edit, RichEdit20W*, RICHEDIT50W} → `EM_REPLACESEL`/`WM_PASTE` 直投，<10ms 不碰剪贴板 → leads_to: DONE
  - B: 否则 → leads_to: C3
- decide_by: `GetClassNameW` 结果
- source: phase1-synthesis.md（原始依据：Win32 控件消息语义；Chromium/Electron 不消费 WM_* 是反例来源）
- counterexample: 对话框式 NMEdit/自绘控件类名相似但消息语义不同——命中白名单类名才走
- output: 成功注入或落入 C3

### C3: 主路径——剪贴板+粘贴键

- entry_when: 常规控件（浏览器/Electron/WPF/终端外的大多数）
- options:
  - A: 备份全格式→SetClipboard→模拟粘贴键（Win Ctrl+V；mac ⌘V；终端类换 Ctrl+Shift+V）→恢复剪贴板 → leads_to: DONE
  - B: 失败/超时/目标禁粘贴 → leads_to: C4
- decide_by: 剪贴板 API 返回值 + 平台/终端能力判定（ConsoleWindowClass/WindowsTerminal 类名）
- source: phase1-synthesis.md（原始依据：现有实现 `service/src/injector/mod.rs:82-254`；PHASE0 实测 2930 字符 264ms）
- counterexample: 远程桌面剪贴板隔离、剪贴板管理器抢占 → 捕获错误降级
- output: 成功注入或落入 C4

### C4: 兜底——逐字符合成输入

- entry_when: C3 失败或配置禁剪贴板
- options:
  - A: Win `SendInput KEYEVENTF_UNICODE`（现有）/ mac CGEvent / Linux XTEST 或 uinput → leads_to: DONE
  - B: 也失败 → 记日志 + UI 明确报错（不静默）
- decide_by: 平台实现表
- source: phase1-synthesis.md（原始依据：`injector/mod.rs:256-323`；macOS `CGEventKeyboardSetUnicodeString`；Linux enigo x11rb/ydotool）
- counterexample: 万字符级文本应拒绝走此路（慢且易丢）——长文本在 C3 之前就该走剪贴板
- output: 注入结果 + 真实 strategy/duration 落库

### C5: Wayland 会话判定（Linux）

- entry_when: Linux 启动时探测 `XDG_SESSION_TYPE`
- options:
  - A: x11/xwayland → X11 全栈 → leads_to: C2–C4 等价实现
  - B: wayland → 检查 ydotoold/uinput 可用性：有→注入模式 opt-in；无→降级"复制到剪贴板+提示手动粘贴"→ leads_to: 降级输出
- decide_by: 会话类型 + `/dev/uinput` 可写性 + portal 可用性
- source: phase1-synthesis.md（原始依据：Mutter 未实现 virtual-keyboard-v1（GNOME 不支持 wtype）；ydotool uinput 全栈）
- counterexample: 勿承诺 Wayland 全功能——平台设计禁止任意注入
- output: Linux 运行模式标记

## Stance and rejected defaults

保留 Clipboard→SendInput 主链路不动；UIA 仅以"只读门禁"形式回归（探测密码框），不恢复 UIA 注入路径（历史文档证明其在 Chromium/Swing/Scintilla 覆盖差且复杂度是当年事故源）。拒绝按应用名黑名单驱动策略（能力探测替代）；拒绝远程剪贴板/云端通道；拒绝为 Wayland 编造兼容性。

## Failure signals

- 日志里 strategy 全同值/耗时 0ms → 日志前置 bug 未修（`service/src/main.rs:139-153` 先于 inject 写 success=true）
- 剪贴板恢复后非文本内容丢失 → 只备份了 CF_UNICODETEXT
- 用户报告"注入了但内容进了别的窗口" → 缺 SetForegroundWindow 或注入间焦点被偷
- Wayland 会话下承诺全功能 → 虚假兼容性

## Relationships without merger

- → design-system：注入成功/失败的 toast 与状态呈现归它管
- → product-architecture：策略开关暴露到设置页的颗粒度归它管
- 张力：注入栈追求"多策略全覆盖"，产品架构要求"用户可理解的少选项"——折中：设置页只露"允许剪贴板/恢复剪贴板"两个开关，策略细节藏内部
