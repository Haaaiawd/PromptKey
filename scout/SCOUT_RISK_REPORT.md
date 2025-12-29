# PromptKey System - Scout 2.0 风险报告

**任务**: 结构化拆解 PromptKey 系统，识别组件边界、通信线路和架构风险  
**执行日期**: 2025-12-29  
**分析范围**: 构建系统、运行时架构、IPC 机制

---

## 🗺️ System Fingerprint (系统指纹)

**项目类型**: Tauri 桌面应用 (Rust + WebView)  
**构建系统**: Cargo Workspace  
**目标平台**: Windows (从 Named Pipe 和 Windows API 推断)  
**架构模式**: **单进程 + 多线程** (过去曾是 Sidecar 多进程架构，已重构)

### 核心发现
- **主应用**: `promptkey` (GUI + 主控逻辑)
- **服务库**: `service` (热键监听 + 注入引擎 + IPC 服务端)
- **架构转变**: service 从独立 sidecar 进程 → 内嵌库线程 (重大架构演进！)
- **遗留物**: `sidecar/service-x86_64-pc-windows-msvc.exe` (3.18MB, 已废弃但未清理)

---

## 🏗️ Component Map (组件清单)

### Build Roots (构建根)

| Build Root | 类型 | 成员 | 拓扑角色 |
|:---|:---:|:---|:---|
| `/Cargo.toml` | **Workspace** | `["service"]` | 统一指挥，主构建根 |
| `/service/Cargo.toml` | Workspace Member | - | service 库 (可作为 lib 或 bin) |
| `/scripts/Cargo.toml` | **🔴 独立包** | - | 测试工具 (未纳入 workspace) |

### 🔴 风险标记：独立王国 (Polyrepo Hell Lite)

`scripts/` 包**未在 workspace members 里**！这意味着：
- ✅ **当前状态**: 仅用于测试，不影响生产环境
- ⚠️ **潜在风险**: 如果将来依赖 scripts 里的工具，版本会漂移
- 💡 **建议**: 在 workspace 配置中添加 `exclude = ["scripts"]` 明确标注"故意分离"

---

## 🔄 Build Topology (构建拓扑)

**判定**: 🟢 **Cargo Workspace (单一版本)**

### 产物列表

| 产物 | 来源 | 类型 | 运行方式 |
|:---|:---|:---:|:---|
| `promptkey.exe` | `/Cargo.toml` [[bin]] | 可执行文件 | 主进程 (Tauri) |
| `service` (lib) | `/service/Cargo.toml` [lib] | 库 | 内嵌线程 (通过 `thread::spawn`) |
| `~~service.exe~~` | `sidecar/` (历史遗留) | ❌ 已废弃 | 过去独立进程,现已废弃 |
| `test_uia` | `scripts/` [[bin]] | 测试工具 | 独立运行 (手动) |
| `ide_compatibility_test` | `scripts/` [[bin]] | 测试工具 | 独立运行 (手动) |

### Critical Insight: 架构转变的痕迹

**证据链**:
1. `service/Cargo.toml` → `[lib] path = "src/main.rs"` (罕见配置！通常库入口是 `lib.rs`)
2. `service/src/main.rs` → 同时存在 `pub fn run_service()` 和 `fn main()`
3. `tauri.conf.json` → `"externalBin": []` (Sidecar 列表为空)
4. `sidecar/` → 包含预编译的 `service.exe` (未删除)
5. `src/main.rs:96-98` → `std::thread::spawn(|| { service::run_service(); });`

**结论**: 系统从 **Sidecar 多进程架构</s> → **内嵌单进程架构**  
**用意**: 简化部署，单一可执行文件

**🟡 技术债**:  
- `service/src/main.rs` 的 `fn main()` 已无用，但保留是为了兼容性 (注释说明："为了作为二进制文件运行时兼容")
- `sidecar/service.exe` 应删除以避免混淆

---

## 🔌 Runtime Topology (运行时拓扑)

### Process Roots (进程入口)

| 入口点 | 路径 | 角色 | 运行方式 |
|:---|:---|:---:|:---|
| `main()` | `/src/main.rs` | GUI + 主控 | 主进程 (Tauri) |
| `run_service()` | `/service/src/main.rs` | 热键 + 注入引擎 | 子线程 (Embedded) |
| `~~main()~~` | `/service/src/main.rs` | ❌ 历史遗留 | 过去独立进程,现弃用 |

### 🛡️ 进程生命周期

**NO Sidecar! NO Process Spawning!**  
- ❌ 没有 `Command::new` 调用
- ❌ 没有 `subprocess.Popen`
- ✅ 只有 `std::thread::spawn` (线程创建)
- ✅ 只有 `tauri::async_runtime::spawn` (Tokio async task)

**判定**: 🟢 **单进程架构，无僵尸进程风险** (Process lifecycle fully managed by Tauri)

---

## 📡 Communication Map (通信线路)

### IPC Surfaces (通信表面)

#### 1. Named Pipe: Service → GUI (显示窗口命令)

| 属性 | 值 |
|:---|:---|
| **Pipe Name** | `\\.\\pipe\\promptkey_selector` |
| **方向** | Service → GUI |
| **用途** | 触发 Selector/Wheel 窗口显示 |
| **Server** | `src/ipc_listener.rs` (GUI 端) |
| **Client** | `service/src/ipc/mod.rs::IPCClient` |
| **协议** | 🟡 **Plain Text** (`"SHOW_SELECTOR\n"`, `"SHOW_WHEEL\n"`) |

**代码证据**:
```rust
// service → GUI
let message = "SHOW_SELECTOR\n"; // Plain string
pipe.write_all(message.as_bytes())?;
```

#### 2. Named Pipe: GUI → Service (注入指令)

| 属性 | 值 |
|:---|:---|
| **Pipe Name** | `\\.\\pipe\\promptkey_inject` |
| **方向** | GUI → Service |
| **用途** | 用户选择 prompt 后触发注入 |
| **Server** | `service/src/ipc/inject_server.rs` |
| **Client** | `src/inject_pipe_client.rs` |
| **协议** | 🟡 **Semi-Structured** (`"INJECT_PROMPT:{id}\n"`) |

**代码证据**:
```rust
// GUI → service
let message = format!("INJECT_PROMPT:{}\n", prompt_id);
pipe.write_all(message.as_bytes())?;
```

#### 3. SQLite 数据库 (共享存储层)

| 属性 | 值 |
|:---|:---|
| **路径** | 通过 `Config::load().database_path` 动态获取 |
| **访问者** | GUI (`src/main.rs::open_db`) + Service (`service/src/db.rs`) |
| **并发模式** | WAL (Write-Ahead Logging) |
| **Schema** | `prompts`, `usage_logs`, `selected_prompt` |

**证据**:
```rust
// GUI 端 (src/main.rs:732)
let cfg = load_or_default_config()?;
let database_path = cfg.database_path;

// Service 端 (service/src/main.rs:18-23)
let config = crate::config::Config::load().unwrap_or_default();
let database = db::Database::new(&config.database_path)?;
```

**潜在风险**: 如果 `Config::load()` 在 GUI 和 Service 两端读取不同的配置文件或解析逻辑有差异，会导致**数据库路径不一致**！

---

## 🛡️ Contract Status (契约状态分析)

### IPC 协议强度评估

| 通道 | 协议格式 | 版本握手 | 强度 | 风险等级 |
|:---|:---:|:---:|:---:|:---:|
| `promptkey_selector` | Plain Text<br>(`SHOW_SELECTOR` / `SHOW_WHEEL`) | ❌ 无 | 🟡 **Weak** | 中 |
| `promptkey_inject` | Semi-Structured<br>(`INJECT_PROMPT:{id}`) | ❌ 无 | 🟡 **Weak** | 中 |
| SQLite (共享) | Schema-based | ✅ 隐式<br>(表结构) | 🟢 **Medium** | 低 |

### 🔴 关键风险

#### Risk #1: 协议漂移 (Protocol Mismatch)
**问题**: 没有显式的版本握手机制  
**场景**: 如果未来修改了消息格式 (如添加参数 `INJECT_PROMPT:{id}:{mode}`)，旧版本会静默失败

**举例**:
```rust
// 当前 (Weak Contract)
if msg_clean == "SHOW_SELECTOR" { ... } // 硬编码字符串比较

// 未来如果改成
"SHOW_SELECTOR:v2" // GUI 不认识,会忽略!
```

**老师傅建议** (来自 runtime-inspector skill):
> ⚠️ **协议漂移 (Protocol Mismatch)**: Channel 存在，但无 Handshake/Version → 在新功能规划中**强制添加版本握手任务**

**处方**:
```rust
// 建议添加握手协议
enum IPCMessage {
    Handshake { version: u32 },
    ShowSelector,
    ShowWheel,
    InjectPrompt { id: i32 },
}
```

#### Risk #2: Named Pipe 权限漏洞 (Windows Security)

**问题**: 未发现显式的Security Descriptor设置  
**默认行为**: Windows Named Pipe 默认可能允许 **Everyone** 访问  
**攻击面**: 恶意进程可以伪造 `INJECT_PROMPT:xxx` 指令

**代码审查**:
```rust
// service/src/ipc/inject_server.rs:48-50
let mut server = ServerOptions::new()
    .first_pipe_instance(true)
    .create(PIPE_NAME)?; // ❌ 未设置 ACL!
```

**老师傅警报** (来自 runtime-inspector skill):
> 🔴 **Named Pipe 权限漏洞 (Windows)**: 使用 Named Pipe 但未显式设置 Security Descriptor → 高危：默认可能允许 Everyone 访问！

**处方** (参考 Windows Security Best Practice):
```rust
use windows::Win32::Security::{
    SecurityDescriptor, SECURITY_ATTRIBUTES
};

// 设置仅当前用户可访问
let security_descriptor = "D:(A;;GA;;;WD)"; // 示例，需调整
ServerOptions::new()
    .access_inbound(true)
    .pipe_mode(...)
    // .security_attributes(...) // Tokio API 限制,需用底层 winapi
    .create(PIPE_NAME)?;
```

#### Risk #3: 数据库路径漂移
**问题**: GUI 和 Service 通过独立的 `Config::load()` 获取路径  
**风险**: 如果配置文件被修改、环境变量不同步，可能访问不同的数据库

**证据**: 两端都有独立的配置加载逻辑，无共享单例

**建议**:
- 启动时 GUI 将配置传递给 Service
- 或在 Service 启动时从 GUI 接收数据库路径 (通过 IPC handshake)

---

## 🔥 Hotspot Analysis (热点模块)

*基于文件大小和关键性推断 (未执行完整 Git forensics)*

| 模块 | 规模 | 复杂度推断 | 风险 | 理由 |
|:---|---:|:---:|:---:|:---|
| `src/main.rs` | 42KB<br>1165行 | 🔴 高 | 🔴 高 | 单一文件包含所有 Tauri 命令<br>数据库操作、UI 事件、IPC 调用混在一起 |
| `service/src/db.rs` | 14.9KB | 🟡 中 | 🟡 中 | 核心数据层，Schema 变更会影响全局 |
| `service/src/main.rs` | 7KB | 🟢 低 | 🟢 低 | 逻辑清晰的事件循环 |

**🔴 优先重构建议**: `src/main.rs`  
**Strategy**: 拆分成子模块
```
src/
  commands/       # Tauri commands
  database/       # DB logic
  ipc/            # IPC clients
  main.rs         # App setup only
```

---

## 🚧 Feature Landing Guide (新功能落地指南)

### 如果你要添加新 IPC 命令...

**必须注意**:
1. **协议破坏风险**: 修改现有消息格式会破坏兼容性
2. **安全风险**: 新增 Named Pipe 需考虑权限设置
3. **测试策略**: 必须测试 GUI 和 Service 两端的消息收发

**推荐流程**:
```
1. 定义新消息 enum (建议使用 serde JSON 序列化)
2. 在 service/src/ipc/ 添加服务端处理
3. 在 src/ipc_listener.rs 添加客户端发送
4. 编写集成测试 (模拟 pipe 通信)
5. 考虑添加版本握手机制 (Version: 1)
```

### 如果你要修改数据库 Schema...

**必须注意**:
1. **并发冲突**: GUI 和 Service 同时访问 SQLite (WAL 模式下相对安全)
2. **Migration**: 需要添加 Schema 迁移逻辑 (当前通过 `ALTER TABLE` 动态添加列)
3. **一致性**: 确保 GUI 和 Service 都执行相同的 Schema 初始化

**发现**: 当前已有动态 Schema 升级机制
```rust
// src/main.rs:784
ensure_usage_logs_schema(&conn)?; // 动态添加新列
```

---

## 📌 Summary (摘要)

### ✅ 优势
- 🟢 **单一 Workspace**: 版本一致性有保障
- 🟢 **单进程架构**: 无进程间生命周期管理风险
- 🟢 **WAL 数据库**: 支持并发读写

### ⚠️ 警告
- 🟡 **弱类型 IPC 协议**: 无版本握手，字符串硬编码
- 🟡 **技术债**: 历史遗留代码 (`sidecar/`, `service/main()`)
- 🟡 **独立 scripts 包**: 未纳入 workspace，版本可能漂移

### 🔴 高危风险
- 🔴 **Named Pipe 权限**: 可能默认允许任意进程访问
- 🔴 **协议漂移**: 缺少显式契约和版本检查
- 🔴 **单一巨型文件**: `src/main.rs` 过大，影响可维护性

---

## 🎯 Recommended Actions (推荐行动)

### 立即执行 (Critical)
1. ✅ **添加 Named Pipe ACL**: 设置安全描述符，限制访问权限
2. ✅ **设计 IPC 版本握手**: 引入 `enum IPCMessage` 和版本号

### 近期执行 (High Priority)
3. ✅ **清理技术债**: 删除 `sidecar/service.exe`
4. ✅ **重构 `src/main.rs`**: 拆分成模块化结构
5. ✅ **标记 scripts 分离**: 在 workspace 配置中 `exclude = ["scripts"]`

### 长期规划 (Low Priority)
6. ✅ **Git 耦合分析**: 运行 git-forensics 找出隐性耦合
7. ✅ **配置单例化**: 统一 GUI 和 Service 的配置加载逻辑

---

**Generated by**: Scout 2.0 - Structure Analyzer  
**Next Phase**: 执行 `/blueprint` 进入第二阶段：需求分析与架构设计
