# Devin Brief: 根因是 Tauri 2 capabilities 缺失，window.__TAURI__ 从未注入（P0，第三轮）

## 项目
`/home/haa/sites/promptkey`，分支从当前 `master`（HEAD `b7c1224`，即 2.0.2）新建 `fix/tauri2-capabilities`。

## 背景：三轮修复都没命中根因

用户装了 2.0.1 和 2.0.2，两个症状始终存在：
1. **轮盘永远不显示**（按热键毫无反应，但直注可以）
2. **热键录制器无法录入**（提交时被拒）

前两轮 Devin 分别修了 pipe 链路（重试、顺序、`let _ =`）和录制器交互，**都没解决问题**。我第三轮亲自读完代码，找到根因。

## 根因（已核实，不要再怀疑 pipe / emit / listener）

**Tauri 2 的 capabilities（能力白名单）整个不存在。**

```
tauri.conf.json         → 没有 app.security.capabilities 字段，也没有指向 capabilities 目录
gen/schemas/capabilities.json → {}  （空对象，零 capability）
仓库根/capabilities/    → ❌ 目录不存在
```

Tauri 2 把 IPC 改成了**白名单制**：没有 capability，`window.__TAURI__` **根本不会注入到 webview**，`invoke()` 也调不通。

`withGlobalTauri: true` 是 **Tauri 1 的遗留配置**，在 v2 里它只决定 `__TAURI__` 是不是全局对象，**不等于授予 IPC 权限**。

### 为什么两个症状都指向这里

| 症状 | 链条 |
|---|---|
| 轮盘不显示 | `src/js/wheel.js` → `prepare()` → `loadPrompts()` → `src/js/store.js:ipc()` → `hasTauri()` 检查 `window.__TAURI__?.core?.invoke` → **false** → 抛「无 Tauri」→ `catch { state.prompts = [] }`。没有 pinned 数据，`prepare()` 后续逻辑全部空转 |
| 录制器无法录入 | `src/js/hotkey_recorder.js` 的 `onCommit` → `ipc('apply_settings', …)` → 同样 `hasTauri()` false → `toast('err', t('ipc.noTauri'))` 后 **throw** → 提交被拒 |

### 为什么"直注可以"

`service/src/main.rs` 的 `id=5` 分支（`handle_injection_request`）**完全在 service 内部执行，不经过 GUI、不经过 named pipe、更不经过 Tauri IPC**。所以它能正常工作，**完全不能证明链路是通的**。这是一个误导性线索，前两轮都被它带偏了。

### 佐证

- `src/js/store.js` 的 `hasTauri()` / `ipc()` 写了完整的降级路径（`t('ipc.noTauri')` + `t('ipc.timeout')` + `t('ipc.fail')`），说明作者预期过 Tauri 缺失的情况 —— 但它现在成了常态
- Playwright e2e 全部通过，因为测试是**手动 mock `window.__TAURI__`** 注入的。**mock 掩盖了真实环境下的缺失**。这也是为什么 32 个断言全绿而真机全废

---

# 任务

## Task 1：建立 capabilities（核心）

1. 在仓库根创建 `capabilities/` 目录（Tauri 2 标准位置）
2. 写 `capabilities/default.json`，要求：
   - `identifier` 用 `"default"`（或你判断更合适的名字）
   - `windows` 必须覆盖**两个窗口**：`main` 和 `wheel-panel`（Rust 侧 `src/main.rs:258-272` 创建 wheel 窗口用的 label 就是 `"wheel-panel"`，主窗口是 tauri.conf.json 里的 `"main"`）
   - `permissions` 至少包含 `core:default`，以及项目实际用到的：
     - `core:window:allow-show` / `allow-hide` / `allow-set-focus` / `allow-set-position` / `allow-start-dragging`（如有拖拽）
     - `core:event:default`（`emit` / `listen` 需要，轮盘的 `wheel-show` 事件依赖它）
     - `core:webview:allow-...`（如需）
     - `core:app:default`
   - **逐个权限对应到真实调用点**，在交付里列出"权限 → 谁在用"的映射表。不要凭感觉堆权限，也不要少授导致新的静默失败
3. 想清楚要不要拆多个 capability（比如 `main` 和 `wheel` 分开），说明理由

## Task 2：让缺失不再静默

capabilities 缺失这种致命问题，用户的体感只是"没反应"。要求：

1. 在 `src/js/store.js` 的 `ipc()` 或 `boot()` 里，**当 `hasTauri()` 为 false 时给出明确、醒目、可操作的提示**，而不是一行 toast 就过去。至少要说清"运行环境缺少 Tauri 能力（capabilities），IPC 不可用"，并指向排查方式
2. 如果有办法在前端检测到 capabilities 缺失（对比预期命令列表），列出来哪些命令不可用
3. **修 Playwright 的盲区**：现在的 e2e 全靠 mock `__TAURI__`，所以 capabilities 缺失永远测不出来。想一个办法让 CI 能发现这类问题，例如：
   - 在 CI 里加一步**校验 `capabilities/` 目录存在且 `gen/schemas/capabilities.json` 非空**
   - 或者加一个 Rust 测试断言 capability 已注册
   - 或者两者都做
   - **不要只靠人工 review**

## Task 3：验证真机链路

本机无 Windows，无法真机验证。但你要：

1. 确认 `tauri build` 会把 `capabilities/` 打进包里（查 Tauri 2 的构建行为，`frontendDist` 之外的能力文件是否需要额外配置）
2. 在 CI 里加一步：构建后**检查产物里含 capability 定义**（如果可行）
3. 如果 tauri CLI 有 `tauri info` / `tauri build --verbose` 能打印生效的 capabilities，跑一次把输出贴出来

## Task 4：顺带核查

Tauri 1 → 2 的迁移遗漏可能不止 capabilities 一处。系统性过一遍：

- `withGlobalTauri` 是否还需要（v2 默认行为？）
- `security.csp: null` 在 v2 的语义
- `src/ipc_listener.rs` 用 `tokio::net::windows::named_pipe` —— 这是**应用自己建的 pipe**，不走 Tauri IPC，**和 capabilities 无关**，确认它不需要额外权限
- `src/inject_pipe_client.rs` 同理
- Rust 侧 `#[tauri::command]` 在 v2 是否还需要 `#[allow]` 或注册到某个白名单（除了 `generate_handler!`）
- 有没有别的 v1 遗留配置在 v2 已失效

---

# 硬约束

- 无 cargo，不编译。**但这次必须让 CI 跑 `cargo check`/`clippy`/`test` 全绿**
- Playwright 全部重跑，且**新增一个"不 mock `__TAURI__` 时应该看到明确错误提示"的用例**（防止再被 mock 掩盖）
- 遵守设计系统与 i18n（新文案中英双语）
- 一个 commit `fix(tauri): restore IPC capabilities — window.__TAURI__ was never injected (#NN)`，push，PR `--base master`，**不合并**
- 遵守 LOOM 流程

---

## 最终交付

```
## 根因确认
（capabilities 缺失的证据链 + 为什么 Playwright 全绿而真机全废）

## 改动
（capabilities 文件内容 + 权限→调用点映射表 + 降级提示 + CI 防线）

## v1→v2 迁移排查结果
（Task 4 逐项结论）

## 测试
（CI + Playwright，特别是新增的无-mock 用例）

## 用户升级后必测清单
（按顺序，每步预期；如果还不行，怎么把证据发回来）

## 未编译验证清单
```

```json
COMPLETION_NOTIFY
source: assistant
task: promptkey-fix-capabilities
deliverables:
- capabilities/default.json created covering main + wheel-panel with exact permissions
- permission-to-callsite mapping documented
- loud non-silent degradation when __TAURI__ is missing
- CI guard so capabilities can never silently disappear again
- v1->v2 migration audit completed
- branch fix/tauri2-capabilities + PR against master (not merged)
status: success
errors: none
```
