# Devin Brief: 手动拖拽排序失效 + 删除两个不该暴露的字段

## 项目
`/home/haa/sites/promptkey`，当前 `master`（HEAD `330e7c4` = PR #13 squash）。新建分支 `fix/wheel-manual-drag-and-hide-fields`。

## 背景

用户已安装 2.0.3 并实测。原话：

> 轮盘很好用，现在是那个排序时的手动排序，我看到你做的有拖拽啊改变排序但是我发现拖不动没法手动调整顺序，请你做调整...

> 另外我看到的每个提示词的这个，可用范围和那个内部的用编号排序的这个就删掉吧，不显示，这个不需要被调整就不要给出去

所以两个任务：

---

## Task 1：手动拖拽排序拖不动（必须修好，真机可用）

### 现状代码（已核实）

拖拽实现在 `src/js/views/prompts.js` 的 `bindMirrorDrag()`，绑定在**左侧「轮盘」镜像列表** `#wheelMirror` 上：

```js
handle.addEventListener('mousedown', () => { row.draggable = true; });
handle.addEventListener('mouseup',   () => { row.draggable = false; });
row.addEventListener('dragstart', e => {
  if (state.wheelSort !== 'manual') { e.preventDefault(); toast('info', t('wheel.sortSwitchToManual')); return; }
  dragId = +row.dataset.wheelId;
  row.classList.add('dragging');
  e.dataTransfer.effectAllowed = 'move';
});
```

### 已知问题（我逐行读过，至少这些）

1. **`mousedown`/`mouseup` 设置 `draggable` 的做法在 Windows WebView2 上极不可靠**——`draggable` 属性的**时序**决定拖拽能否启动，某些情况下 `dragstart` 根本不触发。这是 HTML5 原生拖拽在 WebView2 上的长期坑。
2. **`row.dataset.wheelId` 若渲染时没写 `data-wheel-id`（而是 `data-id` 之类），`dragId` 恒为 `NaN`** —— 那么 `dragstart` 里 `dragId = NaN`，`drop` 里 `if (!dragId || dragId === overId) return;`（NaN 是 truthy 但 `NaN === NaN` 为 false，能过）但 `pins.indexOf(NaN)` = -1 → `if (from < 0 || to < 0) return;` → **完全静默失败**。**先核实 attribute 名字到底叫什么**。
3. **非 manual 模式时只 toast 不进入拖拽**，用户可能连"拖不动"和"不让拖"都分不清。
4. `mousedown` 才置 `draggable=true` 意味着**鼠标按下后到移动之间的窗口很短**，慢一点或先停顿就拖不起来。

### 要求

**直接换成不依赖 HTML5 原生拖拽的实现。** 推荐**指针事件（pointerdown/pointermove/pointerup）+ 手动视觉反馈**的自制排序：

- 按住行 → 行进入拖拽态（视觉：抬升/半透明/影子），可**上下移动**
- 拖动过程中给出**清晰的插入位置指示**（一条分割线或目标行高亮）
- 松手 → 计算目标 index → `ipc('set_pin_order', { ids })` → 重新加载
- **必须支持鼠标和触摸**（`pointerdown` 天然覆盖）
- 保留 `wheel` 镜像列表的**滚动**（拖拽时不能把滚动吞了）：可约定"按住句柄区才拖"或"长按/位移超过阈值才进入拖拽"
- 必须**真的拖得动**。不要"看起来能拖但实现是 HTML5 原生那套"

顺带：

- **拖拽态下禁止 wheel 事件冒泡到行的点击**（拖完误触发预览/注入）
- 排序模式非 manual 时：**要么自动切到 manual 再拖，要么给出明确引导**（哪个体验好你论证后选一个，说明理由）
- 键盘可达性：方向键 + 空格/回车也能移动行（可选，但做更好）
- 视觉风格遵循既有设计系统，动效轻量不廉价

---

## Task 2：删掉两个不该给出去的字段（UI 层，用户明确要求）

用户要删的：

| 要删的 | 具体位置 |
|---|---|
| **可用范围** | `src/index.html:202` `<div class="field"><label data-i18n="f.apps">应用范围（可选）</label><input type="text" id="fApps" ...>` —— 编辑抽屉里的「应用范围」输入框 |
| **内部用编号排序** | `src/index.html:203` `<div class="field"><label data-i18n="f.order">轮盘位置（手动排序时生效）</label><input type="text" id="fOrder" ...>` —— 编辑抽屉里的「轮盘位置」编号输入框 |

用户原话：「**这个不需要被调整就不要给出去**」—— 意思是这是内部机制，不该裸露给用户，**UI 不显示即可**。

### 要求

- **从 UI 移除这两个字段**（抽屉不再显示）
- `src/js/views/prompts.js` 里对应的**读取/写入/监听**（`setV('#fApps', ...)`、`setV('#fOrder', ...)`、`payload` 里的 `app_scopes_json` / `inject_order`、`['fName','fContent','fTags','fApps','fOrder']` 的 dirty 监听数组）**全部清理干净**
- 同时清理 i18n 里不再使用的 key（`f.apps`、`f.appsPh`、`f.order` 等，中英双语都要删）
- **不要动后端/数据库/schema**：`app_scopes_json` 和 `inject_order` 字段**保留**（后端与其他流程可能仍依赖，尤其 `set_pin_order` 写的就是 pin order）。只是**前端不再让用户直接编辑它们**
- 注意：`src/js/store.js:92` 有 `inject_order` 的解析（`parseInt(p.inject_order, 10)`），那是内部排序逻辑，**保留**

---

# 硬约束

- 无 cargo，本机不编译。**CI 必须全绿**（cargo check/clippy/test + frontend static checks + playwright）
- Playwright：**新增拖拽排序的真实验证用例**（pointer 事件驱动，不是 `fill()` 那种假交互）。断言：按住→移动→松手后 DOM 顺序变化**且** `set_pin_order` 被调用
- 既有测试不得回归
- i18n 中英双语同步，删掉的 key 两侧一致
- 一个 commit `fix(ui): real pointer-based manual sort + hide internal fields from drawer`，push，PR `--base master`，**不合并**
- 遵守 LOOM 流程

---

## 最终交付

```
## 拖拽为什么拖不动
（先给逐条证据链：mousedown/draggable 时序问题、dataset attribute 名核实结果、以及任何你发现的其他断点。**先证明你找到了真根因再动手**）

## 实现方案
（为什么选 pointer 事件而非修 HTML5 原生拖拽；插入位置指示怎么做的；滚动冲突怎么解；非 manual 模式的处理策略及理由）

## 删除字段
（删了哪些 UI 元素 + 清理了哪些 JS/i18n；确认后端字段保留未被破坏）

## 测试
（CI 结果 + 新拖拽用例的断言细节 + 回归结果）

## 真机验证清单
（用户在 Windows 上按什么顺序试，每步预期）

## 未编译验证清单
```

```json
COMPLETION_NOTIFY
source: assistant
task: promptkey-manual-sort-fix
deliverables:
- root cause of "drag does not move" proven with evidence
- pointer-event based manual sort implemented and visually verified
- insertion indicator + scroll coexistence + click-suppression handled
- app scope & inject order fields removed from drawer UI, JS, and i18n
- backend fields preserved
- CI green incl. a real pointer-driven drag test
- branch fix/wheel-manual-drag-and-hide-fields + PR against master (not merged)
status: success
errors: none
```
