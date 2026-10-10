# 平台实现与验证状态

本文档按平台列出每个能力的**实现选型**、**代码位置**与**验证状态**。
验证状态只有三档：

- ✅ **已实机验证** — 在该平台真机上运行并验收过
- ⚠️ **代码就绪** — 编译通过 / 单元或 E2E 测试通过，但**未在真机运行**
- ❌ **不支持** — 有意不实现（会说明原因与降级行为）

> **总原则**：Windows 2.0.4 是唯一经用户实机验收的版本。
> Linux 与 macOS 代码全部经过 `cargo check --workspace`（Linux）与 CI 跨平台编译验证，
> service 单元测试 9/9 通过，前端 E2E 4 套件全绿——但**均未在对应平台真机运行**。
> 首次使用如遇问题请带「设置 → 平台状态」截图与日志反馈。

---

## 架构：一条边界，三个 trait

引擎（`service/`）与平台的边界是三个 trait，定义于 `service/src/{injector,context,hotkey}/mod.rs`：

| Trait | 职责 | Windows | Linux (X11) | macOS |
|-------|------|---------|-------------|-------|
| `Injector` | 把文本注入前台输入框 | `windows_impl.rs`（剪贴板 + SendInput） | `x11_impl.rs`（arboard + enigo/XTEST） | `macos_impl.rs`（AX 写值 + ⌘V 兜底） |
| `Context` | 前台窗口/进程探测 | `windows_impl.rs`（GetForegroundWindow） | `x11_impl.rs`（x11rb `_NET_ACTIVE_WINDOW` + `/proc`） | `macos_impl.rs`（NSWorkspace frontmost） |
| `Hotkey` | 全局热键注册/消息泵 | `windows_impl.rs`（RegisterHotKey + GetMessageW） | `unix_impl.rs`（global-hotkey 事件循环） | `unix_impl.rs`（同上） |

`AppContext.window_handle` 为不透明 `u64`：Windows 存 HWND 位模式，X11 存 window id，macOS 存 AXUIElement 引用或 0。
引擎主循环（`service/src/main.rs`）平台中立，不做独立守护进程——service 作为线程内嵌在 GUI 进程内。

平台探测集中在 `service/src/platform/mod.rs`：`os()`、`session()`（X11/Wayland/Native/Unknown）、
`status_json()`（供 GUI `get_platform_status` 命令消费，字段：`os` `session` `injection` `hotkeys` `context` `notes`）。

---

## Windows（✅ 实机验收基线）

| 能力 | 实现 | 验证 |
|------|------|------|
| 全局热键 | `RegisterHotKey` + 阻塞 `GetMessageW` + `PostThreadMessageW` 唤醒 | ✅ 2.0.4 验收 |
| 注入 | 剪贴板粘贴（全格式备份/恢复）+ SendInput 逐键兜底 | ✅ |
| 密码框门禁 | ES_PASSWORD + UIA `IsPassword` | ✅ |
| 前台上下文 | `GetForegroundWindow` + 进程名 | ✅ |
| IPC | 命名管道 `\\.\pipe\promptkey_selector` / `promptkey_inject` | ✅ |
| 自动启动 | `tauri-plugin-autostart`（注册表 Run 键） | ⚠️ 代码就绪（插件新增，UI 开关已接） |
| 托盘 | 托盘图标 + 双击切换主窗 | ✅ |

**零回归约束**：P1 抽 trait 时 Windows 三个实现仅迁移为 `*_impl.rs`，逻辑未改；
IPC 仍走命名管道；引擎重启/防抖语义不变（失败重发不锁防抖）。

## Linux · X11（⚠️ 代码就绪）

| 能力 | 实现 | 验证 |
|------|------|------|
| 全局热键 | `global-hotkey` 0.6（XRecord/X11 后端），独立线程持有 manager | ⚠️ 编译 + 单测通过，未真机 |
| 注入 | `arboard` 3.6 剪贴板粘贴 + `enigo` 0.6（x11rb/XTEST）逐键兜底；注入后 x11rb 回焦原窗口 | ⚠️ 同上 |
| 前台上下文 | `x11rb` 读 `_NET_ACTIVE_WINDOW` → WM_CLASS / `_NET_WM_PID` → `/proc/<pid>/comm` | ⚠️ 同上 |
| 剪贴板 | `arboard`（X11 Tier-1） | ⚠️ 同上 |
| IPC | Unix domain socket（`$XDG_RUNTIME_DIR` 或 `$TMPDIR/promptkey-$UID/`，目录 0700 / socket 0600） | ⚠️ 同上 |
| 自动启动 | `tauri-plugin-autostart` → `~/.config/autostart/promptkey.desktop`（XDG Autostart） | ⚠️ 同上 |
| 配置路径 | `${XDG_CONFIG_HOME:-~/.config}/promptkey/` | ⚠️ 同上 |
| 打包 | Release workflow `build-linux`（ubuntu-22.04 runner，刻意压低 glibc 下限）→ `*_amd64.deb` + `*_amd64.AppImage` | ⚠️ CI 构建 + 格式/体积校验，未真机安装 |

**已知风险（未真机验证推断）**：
- XTEST 在某些发行版被禁/受限时会表现为「粘贴可用、逐键无效」——诊断可见注入策略耗时。
- 多显示器/HiDPI 下轮盘落位逻辑沿用 Windows 同款 clamp，X11 坐标语义一致但未经实机确认。
- IBus/Fcitx 输入法激活时的逐键注入行为未验证（剪贴板路径不受影响）。

## Linux · Wayland（⚠️ 降级，有意为之）

检测到 `WAYLAND_DISPLAY`（且无 `DISPLAY` 可用 XWayland 语义）时 `status_json()` 报告
`session=wayland` + `notes=["wayland_degraded"]`，`injection/hotkeys/context` 记为 `xwayland-only`。

- 管理界面、模板库、记录、剪贴板 —— 全部可用。
- 全局热键/自动注入/前台上下文 —— **只能到达 XWayland 客户端**；对原生 Wayland 窗口不承诺。
- 设置页显示降级提示（中英双语），不做静默失败。
- 纯无显示环境（无 `DISPLAY`/`WAYLAND_DISPLAY`）：`session=unknown` + `notes=["no_display"]`，注入能力记 `unavailable`。

**P4（Wayland 原生增强）未做**：`ashpd` GlobalShortcuts portal + `ydotool` 属可选路线
（见 `CROSS_PLATFORM_PLAN.md` P4），本轮未实现——portal 授权模型与 compositor 差异太大，
半做比不做更误导用户。列为后续工作。

## macOS（⚠️ 代码就绪）

| 能力 | 实现 | 验证 |
|------|------|------|
| 全局热键 | `global-hotkey` 0.6（Carbon `RegisterEventHotKey` 后端） | ⚠️ 编译通过（CI macos），未真机 |
| 注入 · 主 | Accessibility：`AXUIElementSetAttributeValue(kAXValueAttribute)` 直写焦点控件 | ⚠️ 同上 |
| 注入 · 兜底 | `arboard` 剪贴板写值 + `enigo` 模拟 ⌘V + 恢复剪贴板 | ⚠️ 同上 |
| 权限 | `AXIsProcessTrustedWithOptions`（`service/src/injector/macos_ax.rs`）；GUI `request_ax_permission` 命令带 `prompt=true` 触发系统对话框 | ⚠️ 同上 |
| 前台上下文 | `NSWorkspace.frontmostApplication`（`context/macos_impl.rs`，objc） | ⚠️ 同上 |
| IPC | Unix domain socket（同 Linux） | ⚠️ 同上 |
| 自动启动 | `tauri-plugin-autostart`（SMAppService） | ⚠️ 同上 |
| 配置路径 | `~/Library/Application Support/PromptKey/` | ⚠️ 同上 |
| 图标 | `icons/icon.icns`（`scripts/make_icons.py` 生成，PNG-payload icns）+ 单色 template tray icon | ⚠️ 生成产物已验证，视觉未真机 |
| 轮盘窗口 | 透明无边框需要 Tauri `macos-private-api` feature（未文档化的 WKWebView API）——已按 `cfg(macos)` 作用域启用；**App Store 审核风险不适用**（本应用走未签名 dmg 直发） | ⚠️ 同上 |
| 打包 | Release workflow `build-macos` matrix（arm64 runner 原生 aarch64 + 交叉 x86_64）→ `*_aarch64.dmg` + `*_x64.dmg`；**未签名/未公证**，workflow 内设 `signingIdentity` 护栏 | ⚠️ CI 构建 + 格式/体积校验，未真机运行 |

**权限缺失时的行为**：`status_json()` 报告 `injection=needs-permission` + `notes=["ax_permission_missing"]`，
设置页出现「辅助功能权限」行可一键触发系统授权；授权前注入降级为「复制到剪贴板并提示」而非静默失败。

**已知风险**：
- `AXUIElementSetAttributeValue` 只对实现 `AXValue` 的控件生效（Cocoa 原生输入框多数支持；Electron/Chromium 应用支持不一）——兜底 ⌘V 路径覆盖大部分场景。
- Carbon 热键无需权限即可注册，但⌘V 合成需要 Input Monitoring 权限提示（未授权时注入失败会在日志可见）。
- tray template icon 按 macOS template-image 惯例生成（单色 + alpha），未在真机确认渲染。

## IPC 协议（跨平台一致）

一行一消息，纯文本 UTF-8：

- `SHOW_WHEEL` — service → GUI（热键命中 → 弹轮盘）
- `INJECT_PROMPT:<id>` / `INJECT_PROMPT:<id>:VARS:<json>` — GUI → service（触发注入；vars JSON 校验失败拒绝）
- 防抖语义：**失败重发不锁防抖**（管道/socket 断线重试期间不吞后续真实按键）；成功发送后按既有窗口防抖。

Windows 端为命名管道，Unix 端为 UDS——协议字节级一致，`platform::selector_endpoint()` / `inject_endpoint()` 统一生成地址。

## 权限与系统集成一览

| 平台 | 需要的权限/集成 | 未授权时 |
|------|-----------------|----------|
| Windows | 无（WebView2 Runtime 需预装） | — |
| Linux X11 | 无；XTEST 需 X server 允许 | 逐键路径失败 → 剪贴板路径仍可用 |
| Wayland | 无原生支持（P4 才有 portal） | 明确降级提示，管理功能可用 |
| macOS | 「辅助功能」权限（AX）；Input Monitoring（⌘V 合成）；未公证需 Gatekeeper 绕过 | 注入降级为复制+提示，设置页可重新请求 |

## 验证矩阵（汇总）

| 能力 | Windows | Linux-X11 | Wayland | macOS |
|------|---------|-----------|---------|-------|
| 编译 | ✅ | ✅（本地+CI） | — | ✅（CI） |
| 单元测试 | ✅ | ✅ | — | ⚠️（CI 编译，未跑） |
| 前端 E2E | ✅ | ✅（Chromium） | — | ⚠️（同前端代码，未在 WKWebView 验证） |
| 热键注册/触发 | ✅ | ⚠️ | ⚠️（XWayland） | ⚠️ |
| 注入（剪贴板） | ✅ | ⚠️ | ⚠️ | ⚠️ |
| 注入（逐键/AX） | ✅ | ⚠️ | ❌ | ⚠️ |
| 前台上下文 | ✅ | ⚠️ | ⚠️ | ⚠️ |
| IPC | ✅ | ⚠️ | ⚠️ | ⚠️ |
| 自动启动 | ⚠️ | ⚠️ | ⚠️ | ⚠️ |
| 打包产物 | ✅（Release 产出 NSIS+MSI） | ⚠️（Release 产出 deb+AppImage） | — | ⚠️（Release 产出双架构 dmg，未公证） |

✅=实机/实际验证；⚠️=代码就绪未实机；❌=有意不支持。

---

## Release 打包产物与流水线（`.github/workflows/release.yml`）

推送 `v*` tag 触发：`prepare-release` 先建/更新 GitHub Release（避免并行 job 抢建产生 422/重复 release），
随后 `build-windows` / `build-linux` / `build-macos` 三平台**并行**构建、各自把产物追加到同一个 Release——
任一平台失败不阻塞其余平台发布（互相无 `needs` 于彼此）。

| 平台 | Job / Runner | 产物文件 | 打包前校验 | 安装方式 |
|------|-------------|----------|-----------|----------|
| Windows | `build-windows` / windows-latest | `PromptKey_*_x64-setup.exe`（NSIS）、`PromptKey_*_x64_en-US.msi` | capabilities 解析 + 产物存在性 | 运行安装向导；需 WebView2 Runtime |
| Linux | `build-linux` / **ubuntu-22.04** | `*_amd64.deb`、`*_amd64.AppImage` | pkg-config 依赖门 + `scripts/verify_release_artifacts.sh`（ar members / ELF+`AI\x02` / 体积下限） | `sudo apt install ./*.deb`；或 `chmod +x` AppImage 直接运行（需 FUSE2） |
| macOS | `build-macos` / macos-latest（arm64） | `*_aarch64.dmg`（Apple Silicon）、`*_x64.dmg`（Intel，交叉编译） | unsigned 护栏（`signingIdentity` 必须为未配置）+ `koly` trailer / 体积下限 | 拖入 Applications；Gatekeeper「仍要打开」；注入需辅助功能授权 |

**产物诚实性约定**：
- Linux 在 ubuntu-22.04 构建 → glibc 下限 2.35，与「Ubuntu 22.04+ / Debian 12+」的要求一致；不用 ubuntu-latest 以免下限被悄悄抬高，也规避 linuxdeploy 在 24.04 的已知故障（tauri-apps/tauri#14796）。
- macOS 出**两个单架构 dmg** 而非 universal2：各自是原生 `tauri build` 路径，风险最低、可独立验证；Intel Mac 仍被覆盖。
- 三平台全部**未签名/未公证**，Release 说明与本文档如实标注；`verify_release_artifacts.sh` 只做格式与体积健全性检查，不验证内容正确性。
