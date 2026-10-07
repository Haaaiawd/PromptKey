# Devin Brief: 修轮盘热键唤不出 + 热键录入改为按键捕获（P0）

## 项目
`/home/haa/sites/promptkey`，分支从当前 `master`（HEAD `30a95ff`，即 2.0.1）新建 `fix/wheel-hotkey-ipc-and-recorder`。

## 用户实测 2.0.1 后的反馈（两个 P0）

> 「轮盘无法被热键唤出，倒是注入可以」
> 「热键录入不太对吧，不应该是让我按这些按键录入吗。。。现在怎么跟一个文本框似得」

两条都要修。

---

## Bug 1：id=4（轮盘）热键失效，id=5（直注）正常

### 已定位的链路差异（我读完了，你直接从这里深入）

两条路径的架构完全不同：

```
id=4 轮盘：
  service/src/main.rs:114-125
    → context_manager.get_foreground_context()
    → ipc_client.send_show_wheel()               service/src/ipc/mod.rs
        → OpenOptions::new().write(true).open("\\\\.\\pipe\\promptkey_selector")
        → write_all("SHOW_WHEEL\n")
    → GUI 端 src/ipc_listener.rs:start_ipc_listener()
        → ServerOptions::new().first_pipe_instance(false).create(PIPE_NAME)
        → server.connect().await
        → read → "SHOW_WHEEL" → crate::present_wheel(&app)   src/main.rs:451
            → window.monitor_from_point + setPosition + show + set_focus
            → emit('wheel-show')  →  wheel.js 的 onShow() → prepare() → 花瓣+键盘

id=5 直注：
  service/src/main.rs:126-138
    → handle_injection_request(...)   ← service 自己干，完全不经过 GUI/named pipe
```

**关键推论**：id=5 不依赖 GUI 和 pipe，所以它正常；id=4 依赖整条 named pipe 链路，**断点必然在 pipe 或 pipe 之后的某一环**。

### 你要查的三个具体方向（按优先级）

1. **`ipc_listener` 到底有没有跑起来**
   - `src/ipc_listener.rs` 的 `start_ipc_listener(app)` 在 `src/main.rs` 的哪里被调用？调用一次还是每次窗口创建都调？如果没被调用，pipe server 根本不存在 —— service 侧 `open(pipe)` 会返回 `Err`，被 `log::warn!` 吞掉，用户看到的就是"按了没反应"。
   - 注意 service 侧的失败处理：`service/src/ipc/mod.rs` 的 `send_show_wheel` 里 `Err(e) => { log::warn!(...); Err(...) }`，**service 主循环的 `4 => { ... let _ = ipc_client.send_show_wheel(); }` 把返回值丢掉了**（`let _ =`）。所以失败了 GUI 侧零反馈。

2. **`first_pipe_instance(false)` 的行为**
   - Tauri 2 / tokio 的 named pipe ServerOptions：`first_pipe_instance(true)` 要求这个实例是第一个，`false` 则不要求。反过来说，**如果需要 `true` 而写成了 `false`，或反之，create() 会失败**。
   - 还要看：`create()` 失败时的 `eprintln!` + `sleep(1s)` + `continue` 循环 —— 如果 create 一直失败，这里会**静默无限重试**，界面上什么都看不到。

3. **#11 引擎重启引入的时序窗口**（重点怀疑）
   - #11 给 worker 加了 `shutdown` AtomicBool + `PostThreadMessageW` 唤醒 + join，确保旧热键注册在新 worker 启动前释放。
   - 但 **pipe server 在 GUI 进程（Tauri app），不在 service worker 线程**。要确认：
     - `ipc_listener` 的生命周期与 app 的关系（是否随窗口关闭而 drop？）
     - 引擎重启后，GUI 侧的 pipe server 是否仍然存活？
     - 若 pipe server 的 accept loop 在 `server.connect()` 或 `read()` 上卡住，**它会不会因为只读一次就 drop 连接**？看代码：`read` 一次后注释写着"Disconnect happens when server is dropped or loop restarts"，**loop 会立即重建新 server** —— 这看起来是对的，但要验证重建是否真的成功。

4. **`present_wheel` 本身**（`src/main.rs:451-481`）
   - 它拿到 `app.get_webview_window("wheel")` 后做了什么？如果 wheel 窗口还没创建（懒创建）或被销毁，`get_webview_window("wheel")` 返回 None 会怎样？
   - `show_wheel_window` 命令（Tauri command）和 `present_wheel` 是两回事，别混。

### 硬要求

- **修复必须让故障可见**。pipe 打不开 / create 失败 / present_wheel 失败，都要在界面上留有痕迹（至少在日志里明确写，最好设置页显示）。不要再用 `let _ =` 丢掉返回值。
- 修复后写一个**诊断命令**（如 `diagnose_hotkey_pipeline()`）并按需暴露到设置页或开发者模式，返回：pipe server 是否在听、最近一次 pipe 通信是否成功、wheel 窗口是否存在、热键注册状态。**让用户下次遇到问题时能自己看到断在哪一环**，不用你猜。

---

## Bug 2：热键录入是文本框，不是按键捕获

### 现状（已核实）

- `src/index.html:153,155`：`<input class="hotkey-input" id="hotkeyInput" spellcheck="false">` 和 `id="quickHotkeyInput"`
- **没有任何 keydown 捕获逻辑** —— 就是个纯文本框，用户得自己打字输 `Ctrl+Alt+Space`
- `TODO.md` 里 P0 写着「设置页：热键录制（冲突检测、应用后热更新）」—— 从未实现

### 要求

实现真正的**按键录制器**：

1. 点击输入框 → 进入「录制中」状态（视觉上要明确：边框高亮 + 提示文字「请按下组合键…」+ Esc 取消）
2. 捕获 `keydown`，取 `ctrlKey`/`altKey`/`shiftKey`/`metaKey` + `event.code`/`event.key`，**规范化为与 Rust `parse_hotkey` 一致 的字符串格式**（如 `Ctrl+Alt+Space`、`Ctrl+Alt+F9`）—— 注意 Rust 侧现在支持 F1-F24、方向键、Home/End/Ins/Del/PgUp/PgDn、Tab/Esc、标点、小键盘，**前端必须能表达这些**
3. **至少需要一个修饰键**（Ctrl/Alt/Shift/Win）。只有修饰键没有主键 → 提示无效；只有主键没有修饰键 → 提示无效（或明确允许但风险自负，你定并说明）
4. **规范化时注意 `event.code` vs `event.key`**：数字键行 `Digit1` vs `1`，小键盘 `Numpad1`，标点 `Semicolon`/`Comma`/`Period`/`Slash`/`BracketLeft`。用 `code` 更可靠，但要转成 Rust 认识的名字。
5. **取消**：Esc 取消录制并恢复原值（不要清空）
6. 录制完成后**立即校验**（调 #11 加的 `check_hotkeys` 或 `apply_settings`），如果该组合已被占用/注册失败，明确告知，**不要静默保存一个无效热键**
7. 录制中的输入框要 `preventDefault()` 阻止字符真正输入，别让 `Ctrl+Alt+Space` 被"打"进框里
8. i18n：所有新文案中英双语（录制中、请按组合键、需要至少一个修饰键、已取消、该组合已被占用…）

---

## Task 3：可验证

本机无 Windows。必须做的：
- CI 绿（`cargo check/clippy/test` + 前端静态检查 + Playwright）
- **Playwright 覆盖按键录制器**：mock `__TAURI__`，模拟 `keydown` 带不同修饰键与键位，断言
  - `Ctrl+Alt+Space` → 输入框显示 `Ctrl+Alt+Space`
  - `Escape` 在录制中 → 取消且恢复原值（不清空）
  - 单独按 `Space`（无修饰键）→ 提示"需要至少一个修饰键"
  - 录制中不许有字符真的进入输入框
  - 小键盘/功能键能正确规范化
- 如果能为 Bug 1 写 Rust 侧测试（pipe 通信、present_wheel 的窗口查找逻辑），加

## Task 4：文档

`TODO.md` 的对应项勾掉；如有新的已知限制，写进 `.loom` 的 open questions。

---

# 硬约束

- 无 cargo，不编译。Rust 改动逐个核对 **windows 0.58**（service）与 **windows 0.52**（root）的 API 签名 —— 两处版本不同，这个坑已经栽过两次
- 交付列"未编译验证清单"
- 不许把"应该能好"当修好。**Bug 1 的 pipe 链路必须上 Windows 真机验证**，明确告诉用户怎么验
- 遵守设计系统与 i18n（中英双语）
- 一个 commit `fix(hotkey): <what> (#NN)`，push，PR `--base master`，**不合并**
- 遵守 LOOM 流程，不动 00-07 号文档

---

## 最终交付

```
## Bug 1 根因（带证据）
（断在哪一环，代码位置 + 为什么 id=5 不受影响）

## Bug 1 修复
（diff 摘要 + 故障可见性 + diagnose 命令设计）

## Bug 2 实现
（录制器交互 + 规范化格式 + 校验 + 取消 + i18n key 列表）

## 测试
（CI + Playwright 断言明细）

## 用户升级后必测清单
（按顺序，每步写清预期；特别是轮盘唤出与录制器）

## 未编译验证清单
```

```json
COMPLETION_NOTIFY
source: assistant
task: promptkey-fix-wheel-hotkey
deliverables:
- root cause of wheel hotkey failure identified with evidence
- pipe/IPC chain fixed with failure visibility + diagnose command
- hotkey recorder implemented (modifier capture, normalization, cancel, validation)
- CI green + Playwright recorder tests
- branch fix/wheel-hotkey-ipc-and-recorder + PR against master (not merged)
status: success
errors: none
```
