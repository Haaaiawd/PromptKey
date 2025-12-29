# PromptKey 启动与调试指南

## 🚀 正确启动步骤（确保使用最新构建）

### 方案 A：完整重新构建（推荐）
```bash
# 1. 先杀掉所有旧进程
taskkill /F /IM service.exe 2>nul
taskkill /F /IM promptkey.exe 2>nul

# 2. 重新构建 Service
cd d:\PROJECTALL\Workflow\PromptKey
cargo build --release -p service

# 3. 复制到 sidecar（供 GUI 启动用）
cp target/release/service.exe sidecar/service-x86_64-pc-windows-msvc.exe

# 4. 构建并运行 GUI
cargo run --release
```

### 方案 B：快速测试（分开运行，便于看日志）
```bash
# Terminal 1: 单独运行 Service（可以看到详细日志）
cd d:\PROJECTALL\Workflow\PromptKey
$env:RUST_LOG="info"
./target/release/service.exe

# Terminal 2: 运行 GUI（注释掉 src/main.rs 中的 ServiceState 启动代码）
cargo run --release
```

## 🔍 诊断检查清单

### 1. 确认 Service 版本正确
运行 Service 后查看日志开头，应该看到：
```
[INFO] Configuration loaded successfully
[WARN] ⚠️ Configured hotkey 'Ctrl+Shift+H' conflicts with Selector Panel. Resetting...
```

如果看到这个警告，说明配置自愈生效了！

### 2. 确认 IPC 通道建立
运行 GUI 后查看日志，应该看到：
```
[IPC] Starting listener on \\.\pipe\promptkey_selector
```

### 3. 测试热键
按下 `Ctrl+Shift+H`，Service 日志应该显示：
```
[INFO] Selector hotkey detected (ID=3), sending IPC to GUI
[INFO] IPC: Sent SHOW_SELECTOR to GUI via \\.\pipe\promptkey_selector
```

GUI 日志应该显示：
```
[IPC] Received: SHOW_SELECTOR
[IPC] Selector window shown via IPC
```

## 🐛 如果还是"直接粘贴"

### 问题1：配置冲突未解决
**症状**：日志显示 `Injection hotkey detected (ID=1 or 2)`
**原因**：`%APPDATA%\PromptKey\config.yaml` 中 `hotkey: Ctrl+Shift+H`
**解决**：
```bash
# 删除配置文件，强制重新生成
del "%APPDATA%\PromptKey\config.yaml"
```

### 问题2：旧 Service 进程未杀干净
**症状**：重启后行为没变化
**检查**：
```bash
tasklist | findstr service
```
**解决**：
```bash
taskkill /F /IM service.exe
```

### 问题3：IPC 通道未建立
**症状**：日志有 "Failed to send selector IPC" 或 "Failed to open named pipe"
**原因**：GUI 的 IPC Listener 未启动或崩溃
**检查**：GUI 日志中搜索 "[IPC]"
**解决**：确认 `src/ipc_listener.rs` 存在并在 `src/main.rs` 中正确注册

## 📝 当前状态

✅ **已完成**：
- IPC Listener 已实现（`src/ipc_listener.rs`）
- 配置自愈已添加（`service/src/main.rs`）
- Service 二进制已更新（09:08 构建）
- Sidecar 已更新

⏳ **待验证**：
- 确认新 Service 进程启动
- 确认 IPC 通道连接成功
- 确认 `Ctrl+Shift+H` 触发 Selector

## 🎯 预期行为

正确配置后：
1. **Ctrl+Alt+Space** → 直接粘贴当前选中的 Prompt（注入模式）
2. **Ctrl+Shift+H** → 弹出选择面板（新功能）

## 📊 日志级别设置

如果需要更详细的调试日志：
```bash
# Service
$env:RUST_LOG="debug"
./target/release/service.exe

# GUI (在 .cargo/config.toml 或环境变量)
$env:RUST_LOG="debug"
cargo run --release
```

---

**最后提醒**：每次修改代码后，必须：
1. 重新 `cargo build --release -p service`
2. 复制到 `sidecar/`
3. 杀掉旧 `service.exe` 进程
4. 重新运行 `cargo run --release`
