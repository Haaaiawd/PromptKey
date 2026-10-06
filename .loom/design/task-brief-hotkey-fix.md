# Devin Brief: 修 2.0.0 全局热键无法唤起 + 无法修改（P0 阻塞）

## 项目
`/home/haa/sites/promptkey`，分支从当前 `master`（HEAD `75f6687`）新建 `fix/hotkey-regression`。

## 问题（用户装机实测，P0 阻塞级）

用户装了 CI 出的 **PromptKey 2.0.0**（`PromptKey_2.0.x64-setup.exe`），报告：

> 「全局热键似乎有问题，没法唤起，并且没法正常修改」

两个症状：
1. **热键按了没反应**，轮盘不弹出
2. **设置里改热键无效**

默认热键是 `Ctrl+Alt+Space`（轮盘）+ `Ctrl+Alt+Q`（快捷直注）。

## 已知代码位置（我已读过，你直接从这里深入）

`service/src/hotkey/windows_impl.rs` 是 Phase 2 Task8 抽 trait 后的新实现。**这个文件从来没在 Windows 上真跑过**（CI 只做 `cargo check`/`clippy`，不打热键）。

### 我看到的可疑点（**不要盲信，逐个验证**）

1. **消息循环用 `PeekMessageW` + `sleep(10ms)` 轮询**，不是 `GetMessageW` 阻塞等待。`WM_HOTKEY` 由系统投递到线程消息队列，`PeekMessage` + sleep 理论上能收到，但轮询式循环在高负载或时序敏感场景下可能丢消息。**要判断这是不是根因**。

2. **`start()` 里 spawn 线程后，线程内重新构造了一个 `HotkeyManager`**（`tx: tx.clone(), rx: mpsc::channel().1` —— rx 是 dummy）。`register()` 在这个线程里调用。**确认 `RegisterHotKey` 的调用线程和 `PeekMessageW` 的循环线程是同一个**（应该是，但要验证 `thread::spawn` 闭包捕获的 `tx` 是否真的进了同一个线程）。

3. **引擎重启路径**（Phase 2 修过 F01）：`src/main.rs` 的 `ServiceState` 有 `shutdown: Option<AtomicBool>` + `worker: Option<JoinHandle>`，`stop_service` 会 signal + join。**检查改热键时的完整时序**：
   - 用户改热键 → `apply_settings` → 是否 `stop_service` → `start_service`？
   - 旧线程退出前 `UnregisterHotKey(None, 4/5)`，新线程注册同名/新热键 —— **有没有竞态窗口**：旧线程还没反注册完，新线程已注册，导致 `RegisterHotKey` 返回 `ERROR_HOTKEY_ALREADY_REGISTERED`（1409）然后被 `log::error!` 吞掉？
   - **`log::error!` 之后没有重试、没有降级** —— 如果注册失败，界面上看不到任何提示，用户完全不知道热键没注册上。这可能就是"没法正常修改"的直接体感。

4. **`parse_hotkey` 的按键表极其有限**：只有 `SPACE`/`ENTER`/`Q`/`W`/`H` + 任意单字符。如果用户在设置里录入了别的键（比如 `F1`、`Home`、`,`、`.`），会走到 `_ => return Err("不受支持的按键零件")`。**要确认设置界面的热键录制组件到底产出什么格式**，以及**失败时用户看到什么**（很可能静默失败）。

5. **`MOD_CONTROL | MOD_ALT` 没有加 `MOD_NOREPEAT`** —— 不是致命问题，但要评估是否需要。

6. **注册失败的返回值被 `log::error!` 吞掉，前端无感**。这条极可能是"无法唤起的直接原因 + 无法修改的体感来源"的合击。

## 任务

### Task 1：定位根因（必须给出证据，不许猜）

读代码 + 逻辑推演，把两个症状分别归因。如果是多个缺陷叠加，每个都要说清。

**特别要查清**：
- 消息循环用 `PeekMessageW`+sleep 是否会丢 `WM_HOTKEY`
- `RegisterHotKey` 的线程与消息循环线程是否同一线程
- 引擎重启时 `RegisterHotKey` 的竞态（ERROR_HOTKEY_ALREADY_REGISTERED）
- 注册失败是否有任何用户可见反馈
- 设置界面的热键录制到底产出什么字符串（去 `src/js/views/settings.js` 和 `src/main.rs` 的 `apply_settings` 找）

### Task 2：修

按你定位的根因修，硬要求：

1. **消息循环改成阻塞式**（`GetMessageW` 或等价），不要轮询 sleep。如果保留 sleep 有充分理由，说明
2. **注册失败必须让用户看到**：toast / 设置页错误提示，至少说清"热键被占用/不受支持，当前未生效"
3. **改热键的竞态**：旧引擎完全退出（反注册 + join 完成）后再注册新的，或新注册失败时保留旧注册
4. **`parse_hotkey` 补齐常见键**（至少 F1-F12、方向键、Home/End/Insert/Delete、`,`.`/`;`等符号，以及小键盘键），不支持的键要给出明确错误而不是静默
5. **`MOD_NOREPEAT`** 视情况加上
6. 加**启动时的自检**：服务起来后检查两个热键是否都注册成功，失败则在界面上明示

### Task 3：可验证的测试

本机无 Windows，不能真按热键。但你可以：
- 写一个 Windows 侧的**自检命令**（Tauri command，如 `check_hotkeys()`），返回每个热键 id 的注册状态（已注册/失败+原因），设置页调用它显示
- 用 CI 跑 `cargo check` + `clippy` 确认编译干净
- Playwright 前端测试：mock `check_hotkeys()`，验证设置页在"注册失败"状态下**有可见提示**（这是防回归的关键）
- 如果 CI 能加一步热键注册的 Windows 冒烟测试（比如注册一个不冲突热键再反注册），加上

### Task 4：一并带上已知的未合入项

这两个已在别的分支，**由我合并，你不要动**：
- `fix/wheel-quickcreate-content`（PR #9，quickCreate 修复）
- `feat/packs-prismix-vela-tornpaper`（三个新 pack）

你的分支只做热键修复。**避免和它们冲突**：如果你必须改 `src/main.rs`，注意 PR #9 也改了它（加了 `present_main_window_new_prompt`）—— **你的分支要基于最新 master，且 rebase 时遇到 `main.rs` 冲突要正确合并不是覆盖**。

---

# 硬约束

- 本机无 cargo，**不编译**。Rust 改动逐个核对 `windows` 0.58 的 API 签名（`service/Cargo.toml` 用的是 **0.58**，不是根 Cargo 的 0.52 —— 这个版本差异已经坑过一次）
- 交付时列"未编译验证清单"
- **不许把"应该能好"当成修好了** —— 明确说明哪些只能上 Windows 真机验证
- 前端改动守 i18n（中英双语 key 都加）与已有设计系统
- 一个 commit：`fix(hotkey): <what> (#NN)`，push，开 PR `--base master`，**不合并**
- 遵守 LOOM 流程（record/deliverables），不要动 00-07 号文档

---

## 最终交付

```
## 根因（带证据）
（两个症状各自的归因，代码位置 + 为什么）

## 修复内容
（diff 摘要 + 新 i18n key + check_hotkeys 设计）

## 测试
（CI 结果 + Playwright 断言 + 哪些必须在 Windows 真机验）

## 用户升级后要测什么
（具体步骤，让用户一次到位）

## 未编译验证清单
```

```json
COMPLETION_NOTIFY
source: assistant
task: promptkey-fix-hotkey
deliverables:
- root cause identified with evidence for both symptoms
- hotkey loop + registration race + silent-failure + key coverage fixed
- check_hotkeys() self-check command wired into settings
- CI cargo check/clippy green + Playwright failure-visibility test
- branch fix/hotkey-regression + PR against master (not merged)
status: success
errors: none
```
