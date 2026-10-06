# 跨平台工程化计划（Linux / macOS）

> Loom slug: `cross-platform-plan` | Kind: research | Phase 1 交付物

## 结论

**可做，但须诚实分层：Windows 全功能；Linux-X11 全功能；macOS 全功能（代价是首启辅助功能授权引导）；Linux-Wayland 只承诺"降级可用"（管理+复制可用，自动注入需 ydotool/portal 授权）。** 四阶段：P1 抽 `Injector`/`Context`/`Hotkey` 三 trait+Windows 加固 → P2 Linux-X11 → P3 macOS（AX+公证 DMG）→ P4 Wayland 尽力而为。service 维持内嵌线程架构（`src/main.rs:95-98`），不做独立守护进程。

---

## 1. 平台耦合点清单（文件:行号证据）

| 位置 | 内容 | 耦合类型 |
|---|---|---|
| `src/main.rs:1` | `#![windows_subsystem = "windows"]` | 编译属性（Windows-only 属性，需 `cfg_attr` 门控） |
| `src/main.rs:95-98` | `std::thread::spawn` 内嵌启动 `service::run_service()` | 生命周期：内嵌线程 ✅天然跨平台 |
| `src/main.rs:113-120` | `resolve_service_exe_path()` 区分 `service.exe`/`service`，dead_code | 历史残骸，删除或保留皆可 |
| `service/src/injector/mod.rs:4-7` | `windows::Win32::*` 全家桶导入 | **硬耦合**：整个模块 Win32 |
| `service/src/hotkey/mod.rs:6` | `use windows::Win32::...` | 硬耦合 |
| `service/src/hotkey/mod.rs:34-39,112-124` | `RegisterHotKey` + `PeekMessageW` 消息循环 | 硬耦合 |
| `service/src/context/mod.rs:2-9` | `GetForegroundWindow`/`OpenProcess`/`K32GetProcessImageFileNameW` 导入 | 硬耦合 |
| `service/src/context/mod.rs:27-61` | 前台 hwnd/标题/进程名采集 | 硬耦合 |
| `service/src/context/mod.rs:89-147` | `get_process_name` OpenProcess 路径 | 硬耦合 |
| `service/src/main.rs:102-104` | `HWND(null_mut())` 构造 fallback ctx | 类型耦合：`AppContext.window_handle` 是 `HWND` 类型 |
| `service/Cargo.toml:18-19` | `[target.'cfg(windows)'] windows = 0.58` | ✅已正确门控 |
| `Cargo.toml:39` | `windows = 0.52` 写在**全局** `[dependencies]` | 半耦合：非 Windows 平台会编译该 crate（虽不致命，应收进 cfg 块） |
| `Cargo.toml:30` | `winres = 0.1`（Windows 资源嵌入） | 需 `[target.'cfg(windows)'.build-dependencies]` |
| `tauri.conf.json:29-32` | `targets:"all"` + `icon:["PromptKey.ico"]` | 配置：`all` 会尝试全平台打包但图标只有 .ico |
| `tauri.conf.json:47` | `windows.icon` 又是 `PromptKey.ico` | 同上 |
| `src/index.html:12` | `body class="theme-light"`（硬编码） | UI 层，非平台但顺手记录 |

**结构性结论**：耦合集中且边界清晰——`injector`/`hotkey`/`context` 三个模块正好对应三个 trait 抽象，service 主循环（`service/src/main.rs`）除 `HWND` 类型外本身平台中立。`AppContext.window_handle` 需改为 `u64`/`isize` 不透明句柄（Windows 存 HWND bit，macOS 存 AXUIElement ref 或 0）。

## 2. 每平台工作量与阻塞点

### Linux

| 能力 | X11 | Wayland |
|---|---|---|
| 注入 | `enigo`(x11rb)/`xdotool`/`XTEST`——**可用** | `wtype`（GNOME 不支持 Mutter 未实现 virtual-keyboard-v1）/ `ydotool`（uinput 需权限）/ RemoteDesktop portal（每会话授权弹窗）——**没有免权限方案** |
| 全局热键 | `global-hotkey`（X11 支持，稳定） | XDG GlobalShortcuts portal（KDE6/GNOME48.8+；global-hotkey PR#162 未合入，需自走 `ashpd`）/ 或让用户在 DE 设置自定义快捷键→CLI |
| 剪贴板 | `arboard`（X11 Tier-1） | `arboard` + `wayland-data-control`（Tier-2，合成器扩展） |
| 前台上下文 | EWMH `_NET_ACTIVE_WINDOW`+`/proc/pid/comm` | 桌面相关，部分可得 |

**阻塞点**：Wayland 的权限模型是**设计使然而非缺陷**——任意应用注入输入=键盘记录器同型，平台明确禁止。诚实的工程答案是分层降级（§4）。打包：`tauri build` 原生产出 `.deb` + `.AppImage`；Flatpak 需处理 uinput/input 组权限 manifest，作为可选后续。

### macOS

| 能力 | 方案 | 阻塞点 |
|---|---|---|
| 注入主路径 | `axuielement` 0.10：`AXSelectedText`/`AXValue` 直写（不碰剪贴板，可读 `AXSecureTextField` 拒密码框） | 需"辅助功能"权限 |
| 注入回退 | `arboard` 写 NSPasteboard + `core-graphics` CGEvent ⌘V | 同上需 AX 权限（CGEventPost 也要） |
| 全局热键 | `global-hotkey`（Carbon RegisterEventHotKey） | **不需** AX 权限，可独立工作 |
| 上下文 | `objc2-app-kit` NSWorkspace.frontmostApplication + AXFocusedWindow | AX 权限 |
| 打包 | DMG（`targets:"dmg"`）+ **Developer ID 签名 + Apple 公证** | 无公证=Gatekeeper 拦截，必须开发者账号（$99/年） |

**首启 UX 阻塞点**：macOS 的 AX 授权必须用户手动进"系统设置→隐私与安全性→辅助功能"开启。需要首启引导页：`AXIsProcessTrustedWithOptions(prompt=true)` 弹系统对话框 + 应用内状态轮询 + 权限缺失时降级为"复制到剪贴板"模式。这是产品体验成本，不可绕开。

### Windows
现状即目标态，仅需 P1 修复项（INJECTION_RESEARCH §6-7：密码框门禁、剪贴板全格式备份、日志前置 bug）。

## 3. 图标资源方案

现状：仅 `PromptKey.ico`（`tauri.conf.json:31,47`）。方案：

1. **源资产**：提供一张 1024×1024 透明底 PNG 母图（`resources/icon-master.png`）作为唯一事实源
2. **生成产物**：
   - Windows `.ico`（16/32/48/256 多分辨率合一）——现有文件保留或重新生成
   - macOS `.icns`（`iconutil` 从 iconset 生成，CI/脚本化）
   - Linux PNG 组（32/64/128/256/512，`tauri.conf.json` icon 数组按 Tauri 约定列出）
   - tray icon 单独做"单色可模板化"版本（macOS template image 要求）
3. **工作流**：`scripts/gen-icons.sh`（imagemagick + iconutil），产物入 `icons/` 目录提交；母图变更才需重跑
4. `tauri.conf.json` icon 数组改为 `[32.png, 128.png, 128@2x.png, icon.icns, PromptKey.ico]`（Tauri 约定）

## 4. Service 生命周期与自启

**关键事实**：当前 service **不是独立进程**，是 GUI 内嵌线程（`src/main.rs:95-98` `thread::spawn`），`resolve_service_exe_path` 是 dead code。三平台统一维持"内嵌线程"架构：

| 平台 | 进程形态 | 自启机制 | 说明 |
|---|---|---|---|
| Windows | 内嵌线程（现状） | 设置项"开机自启"→ `HKCU\...\Run` 注册表或 Startup 文件夹快捷方式 | Tauri `plugin-autostart` 可直接用 |
| Linux | 内嵌线程 | `~/.config/autostart/promptkey.desktop`（XDG Autostart） | 同上 plugin；不引入 systemd user unit（GUI app 不需要 daemon 语义） |
| macOS | 内嵌线程 | `SMLoginItemSetEnabled` / `plugin-autostart`（LaunchAgent 封装） | 同上 |

**何时需要拆独立进程**：仅当出现"GUI 崩溃不能带走 service"或"无 GUI 后台驻留"需求时——目前两个需求都不存在（注入只在交互时发生）。**不做**。

## 5. 分阶段路线

```
P0  Phase 1 本文档                      ✅ 当前
P1  Windows 加固 + trait 抽象           injector/hotkey/context 抽 trait，
     (不动功能)                          Win32 实现原样搬入 impl，编译验证无回归
P2  Linux-X11 发布                      enigo(x11rb)+arboard+global-hotkey+
                                        EWMH context；deb+AppImage；
                                        Wayland 检测→提示"当前会话为 Wayland，
                                        自动注入需配置 ydotool（高级）"
P3  macOS 发布                          AX 权限引导页 + AXUIElement 注入 +
                                        CGEvent 回退 + 公证 + DMG
P4  Wayland 增强（可选）                 ashpd GlobalShortcuts portal +
                                        ydotool/uinput 集成文档 + RemoteDesktop
                                        portal 兜底
```

**P1 验收**：Windows 三平台编译通过（`cargo check --target`）、注入回归测试通过。
**P2 验收**：X11 会话下热键→轮盘→注入全链路；Wayland 会话下提示降级文案且管理功能全可用。
**P3 验收**：未授权时引导页+降级模式；授权后注入成功率与 Windows 同水平。

## 6. 硬限制（必须如实告知用户）

| 限制 | 影响 | 缓解 |
|---|---|---|
| **Wayland 无免权限注入** | "做了 Linux"≠Wayland 下热键注入开箱即用 | ydotool opt-in；portal；降级"复制+手动粘贴" |
| **Wayland 无全局热键协议** | 同上 | GlobalShortcuts portal（仅新桌面）；DE 自定义快捷键→`promptkey --inject` |
| **macOS AX 权限必须手动授** | 首启有引导摩擦 | 引导页+权限状态检测+降级模式 |
| **macOS 公证需付费账号** | 无公证用户需右键打开 | 文档说明；后续可考虑签 GitHub release |
| **XWayland 混合会话** | 目标窗是 XWayland 时 X11 注入可达，纯 Wayland 窗不可达 | 能力探测后降级 |
| **Flatpak 沙箱** | uinput/全局热键需额外 portal 权限 | 打包清单内声明，或先发 deb/AppImage |
| **CI 无 macOS runner 即无签名链** | 需 owner 提供开发者证书 | 列入阻塞项待确认 |

## 7. 待 owner 确认

- Q-001：接受 Wayland"半可用"叙事？（建议接受，诚实降级好于虚假承诺）
- Q-002：接受 macOS 首启授权引导摩擦？（无替代方案，只能接受）
- 是否有 Apple 开发者账号用于公证？
- Linux 首发目标发行版优先级（deb 系 > rpm > AppImage 通用包）
