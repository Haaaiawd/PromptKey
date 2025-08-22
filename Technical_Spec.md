# Technical Spec — Prompt Manager (Windows MVP)

## 1. 总体架构
- 进程：后台常驻服务（Rust），UI 后续使用 Tauri 连接（MVP 暂不实现）。
- 模块：
  - hotkey（Win32 RegisterHotKey + 消息循环）
  - injector：UIA/Clipboard/SendInput 三段式流水线与回退
  - context：前台窗口/进程解析与按应用路由
  - config：YAML 加载/热更新（后续）
  - db：SQLite（WAL），rusqlite 访问
  - logging：env_logger

## 2. 技术选型
- 语言：Rust 1.76+（MSVC 工具链）。
- Windows API：windows crate（UIA、剪贴板、SendInput、窗口/进程）。
- DB：rusqlite + SQLite（WAL）。
- 配置：serde_yaml + `%APPDATA%/PromptManager/config.yaml`。

## 3. 关键实现
### 3.1 全局热键
- `RegisterHotKey(HWND(0), id, MOD_NOREPEAT|mods, vk)` 注册；
- `GetMessageW/DispatchMessageW` 消息循环；
- 解析如 `Ctrl+Alt+Space` → MOD_CONTROL|MOD_ALT + VK_SPACE。

### 3.2 注入流水线
- UIA：
  - 获取 `AutomationElement.FocusedElement`；
  - 支持 `ValuePattern::SetValue` 或 `TextPattern` 选区替换；
  - 检测密码/安全输入控件直接拒绝。
- 剪贴板：
  - 完整快照 `IDataObject`；`CF_UNICODETEXT` 写入；
  - 发送 `Ctrl+V`（SendInput）；等待稳定；恢复原剪贴板；
  - 并发保护（互斥）+ 超时回滚。
- SendInput：
  - UTF-16 批量键入；适当 sleep/分批，避免丢键；
  - 仅在其他方法失败时作为兜底。

### 3.3 按应用上下文
- `GetForegroundWindow` + `GetWindowThreadProcessId` → 进程名；
- 窗口标题 `GetWindowTextW`；
- 用正则匹配选择模板与策略。

## 4. 数据库
- PRAGMA：`journal_mode=WAL`；
- 表结构见 Functional Spec。

## 5. 配置与日志
- 启动时加载配置；未来支持文件变更热加载；
- 日志级别可通过环境变量调整（`RUST_LOG=debug`）。

## 6. 打包与发布（后续）
- Cargo 发布 profile 优化；
- Windows 签名与安装器（后续）。

## 7. 测试策略
- 单元：热键解析、注入策略选择、配置加载、DB 迁移。
- 集成：在记事本中 E2E 注入（CI 可选手动）。
