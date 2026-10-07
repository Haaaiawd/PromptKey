# Devin Brief: 拖拽提交后 UI 不刷新（P0，Linux 真机实测抓到）

## 项目
`/home/haa/sites/promptkey`，当前 `master` HEAD `baa37fe`（2.0.4）。新建分支 `fix/wheel-drag-commit-no-refresh`。

## 背景

我在 **Linux 上用 Playwright 真实驱动前端**（mock `__TAURI__` IPC），跑完整个冒烟套件。16/17 通过，**抓到一个 P0 真 bug** —— 这正是你在 Windows 上装 2.0.4 后拖排序会遇到的"拖了没反应"。

## 实测证据（完整链路，逐环都有）

```
MID-DRAG:   {target row gets `.dragging`, insertion slot appears}          ✅ 交互识别正常
AFTER DROP: set_pin_order 收到 { ids: [2, 1] }                            ✅ 顺序计算完全正确
FINAL DOM:  #wheelMirror 顺序仍是 ["1","2"]                               ❌ 提交后没有重新渲染
```

**即：拖拽逻辑对、提交参数对，唯一断点是"提交成功后 UI 没刷新"。**

这解释了为什么 2.0.4 的 e2e 用例声称"断言 DOM reorder + `set_pin_order` 被调用"却全绿 —— 见下方"测试盲区"。

## 疑似位置

`src/js/views/prompts.js` 的拖拽提交函数（PR #14 的 pointer 实现，约 `commitPinOrder` / drop handler 一带）：

```js
try {
  await ipc('set_pin_order', { ids: pins });
  await loadPrompts(); rebuildIndex(); renderPrompts();   // ← 这条链看起来有，但实测没生效
} catch { /* toasted */ }
```

请先**逐行读实际代码**，查清为什么刷新没发生。至少这几种可能，请逐一证伪/证实：

1. **刷新被调用了但抛错被 catch 吞掉**（`loadPrompts` 内部依赖某个 mock 里没有的东西，或某个调用 throw）
2. **`renderPrompts()` 只重渲染 `#grid`，根本不碰 `#wheelMirror`** —— 那顺序就永远不更新。**这个可能性最大，优先查**
3. `wheelPrompts()` 的排序在 `wheelSort !== 'manual'` 时不认 `inject_order`，mock 切了 manual 但真实路径有问题
4. 提交后 `state.prompts` 被替换但 mirror 的渲染走的是别的缓存
5. drop 之后某个 return 提前退出，刷新语句根本没执行到

## 修复要求

1. **提交成功后 `#wheelMirror` 必须立即反映新顺序**（不需要刷新页面、不需要切页面再切回）
2. **顺序要真的持久化** —— 重载页面后顺序不变（我的 mock 里已实现"真后端"语义，能验证这点）
3. **失败时回滚视觉状态**：IPC 失败要恢复到拖拽前的顺序，不能停在中间态
4. 我实测发现：**拖拽必须按在 `.drag` 句柄上才触发**（按行 body 无效）—— 这是 PR #14 的合理设计，**不要改掉**，但请在交付里确认这是有意为之，并考虑要不要给用户更明显的视觉提示（句柄 hover/光标变化），你论证

## 附带：修测试盲区（必须）

2.0.4 的 e2e 号称断言"DOM reorder"，却漏掉了这个 bug。原因是：

- e2e 的 mock `set_pin_order` 返回 `null`，**不真的持久化顺序**
- 且断言只看"调用发生"，**没断言松手后 DOM 顺序真的变了**

请修 `tests/e2e/wheel_sort_drag_e2e.py`：

1. mock 的 `set_pin_order` 必须**真的写回顺序**（真后端语义）
2. 断言必须包含：**松手后 `#wheelMirror` 的 DOM 顺序确实变化**
3. 再断言一次 `set_pin_order` 收到的 `ids` 顺序 = DOM 新顺序
4. 加一个失败回滚用例：mock `set_pin_order` reject → UI 恢复原顺序
5. 加一个"刷新后顺序仍在"用例（重新 load 页面）

---

# 硬约束

- **我在 Linux 上给你复现步骤**（见下），你可以自己复现验证
- 无 cargo，本机不编译，CI 全绿即可
- 不得回归 Windows 行为
- 一个 commit `fix(wheel): re-render mirror after pin order commit`，push，PR `--base master`，**不合并**
- 遵守 LOOM 流程

## 我在 Linux 上的复现方式（你可以照做）

```bash
cd /home/haa/sites/promptkey/src && python3 -m http.server 8913   # 后台起静态服务
# 然后用 Playwright + mock __TAURI__（mock 必须实现 set_pin_order 真写回）
# 我的冒烟脚本在 /tmp/pk_linux_smoke.py，可直接参考或复用
```

关键：**mock 一定要让 `set_pin_order` 真的更新数据**，否则这个 bug 测不出来（这正是原 e2e 漏掉它的原因）。

---

## 最终交付

```
## 根因
（为什么提交后不刷新，逐行证据）

## 修复
（改了什么；为什么这样改）

## 测试盲区修正
（e2e 怎么改的；现在能抓住这个 bug 吗——**请真的跑一次证明**）

## 复现验证
（你在 Linux 上跑的证据：拖拽前/中/后 DOM + set_pin_order 参数 + 刷新后顺序）

## 顺带观察
（拖拽句柄的视觉提示、以及你发现的任何其他问题）
```

```json
COMPLETION_NOTIFY
source: assistant
task: promptkey-wheel-drag-refresh
deliverables:
- root cause of "commits but UI never re-renders" proven with code lines
- mirror re-renders immediately after successful commit and survives reload
- failure rolls back visual order
- e2e mock now persists order and asserts post-drop DOM order + received ids + reload persistence + rollback
- e2e actually re-run to prove it now catches the bug (or would have)
- CI green
- branch fix/wheel-drag-commit-no-refresh + PR against master (not merged)
status: success
errors: none
```
