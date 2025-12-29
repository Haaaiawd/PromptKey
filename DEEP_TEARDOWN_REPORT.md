# PromptKey Deep Teardown & Recovery Report

## 1. 核心问题扫描 (Problem Audit)

在实现 **PromptWheel** 的过程中，系统出现了严重的失稳，主要表现为：
1. **Service 启动即崩溃**：日志显示 `os error 2`。
2. **GUI 启动 Sidecar 失败**：出现 `os error 50`。
3. **构建流程锁死**：`cargo build` 频繁提示“拒绝访问”。

## 2. 深度拆解分析 (Architectural Root Causes)

### A. 命名管道服务端误用 (The IPC Socket Bug)
- **现象**：Service 无法创建 `promptkey_inject` 管道。
- **原因**：在将异步代码同步化的过程中，代码错误地使用了 `std::fs::OpenOptions`。在 Windows 上，`std::fs` 只能作为管道客户端，尝试打开一个不存在的管道服务端会导致 `os error 2` 或崩溃。
- **影响**：后端注入功能彻底瘫痪，服务线程陷入死循环或退出。

### B. Sidecar 加载链路脆弱 (Path Resolution Fragility)
- **现象**：GUI 报 `os error 50` (不支持该操作)。
- **原因**：`resolve_service_exe_path` 返回的是相对路径，且在 `Command::spawn` 时没有进行绝对路径转换。当 GUI 和 CWD 不一致时，路径失效。此外，`CREATE_NO_WINDOW` 标志在路径失效或二进制文件损坏时会抛出混淆的系统错误。
- **影响**：GUI 无法可靠地拉起后端引擎。

### C. 僵尸进程与构建冲突 (Zombie Process Interference)
- **现象**：构建报错 `Access Denied`。
- **原因**：之前的 Service 进程虽然逻辑崩溃，但句柄未释放，导致产生的 `.exe` 文件被锁定。连续的增量构建尝试在文件锁争用下失败。

## 3. 修复方案 (The Recovery Strategy)

1. **重构 IPC 服务端**：放弃原生 Win32 API 调用（易错），改用 `tokio` 的命名管道服务端，但在 Service 内部通过专用线程运行本地运行时。这保证了**服务端能力**的同时维持了主循环的**同步简洁性**。
2. **绝对化路径管理**：在 GUI 中引入 `std::fs::canonicalize`，确保传递给系统的命令路径永远是**绝对路径**。
3. **环境深度清理**：通过强制任务管理器级的 `taskkill`，扫除所有残留的 `promptkey` 和 `service` 实例。

## 4. 当前状态与后续建议

- **状态**：Service 编译已通过。GUI 正在进行最后的 Link 阶段。
- **建议**：
  - 启动时务必确认没有旧版图标留在托盘。
  - 使用 `Ctrl + Alt + Q` 作为轮盘唤起热键，避免与常用软件冲突。
  - 定期清理 `target` 目录以保持构建产物的纯净。
