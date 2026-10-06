# Devin Brief: 修 D4 quickCreate 的 content=name 缺陷（+ 回归测试）

## 项目
`/home/haa/sites/promptkey`，分支从当前 `master` 新建。master HEAD 见 `git log --oneline -1`。

---

# 背景：我在 Linux 上实测发现一个真 UX 缺陷

Phase 2 已合并到 master（6 个 PR 全 MERGED，review 修复 `d229811` 也在）。我拉 master 代码在 Linux 上用 Playwright + mock `__TAURI__`（结构与真实 Rust 后端对齐）跑了完整 E2E，**26/29 通过**。大头全绿：明暗×中英四组合、搜索三语法（`关键词`/`#tag`/`c:全文`）、pin 星标内联、抽屉、轮盘 280px、品牌 logo、数字键直注、打字过滤、变量填充 varsJson 传递、F18 quickCreate 非空。

**但发现一个真 bug：D4 快速创建把「名称」当「内容」用。**

## 缺陷定位（已实测复现）

`src/js/wheel.js` 的 `quickCreate()`：

```js
const name = (query || '').trim() || t('wh.newName');
...
await invokeRaw('create_prompt', {
  prompt: { id: null, name, content: name, tags: [], ... }
});
await invokeRaw('toggle_prompt_pin', { id });
```

**`content: name`** —— 名称和内容被设成同一个字符串。

**复现路径**：轮盘弹出 → 敲「会议纪要」→ 右键中心 → 创建出 `name="会议纪要"` / `content="会议纪要"` 并自动 pin 到轮盘。

**后果**：
1. 用户选中这条花瓣注入，注入进去的就是两个字「会议纪要」，不是可用提示词 —— **D4 完全失去意义**
2. 它会占掉轮盘一个花瓣位（轮盘只有 6 格）
3. 主抽屉的保存路径有"内容不能为空"的校验，quickCreate 绕过了它，制造出半成品记录

**Devin Review 的 F18 当时已经提过**："Collect content before pinning, or open an editing flow for the newly created draft and omit drafts from the injectable wheel until populated." 上一轮只修了"空 content"（现在 content=name 所以非空），**没修 content=name 这个根本问题**。

---

# 任务

## Task 1：修 quickCreate 的语义

按 UX 最优解，你来定具体形态，但必须满足：

1. **创建出的提示词必须是"可注入即有意义"的**，不能 content=name
2. **不能把半成品塞进轮盘占位**（轮盘只有 6 格，很宝贵）
3. **不能增加用户的往返步骤** —— D4 的立身之本是"少回一次主窗口"，如果修完变成"快速创建后还得回主窗口填"，那这个功能就不该存在，你可以选择**删掉 D4** 并在设置里说明，只要理由充分
4. 中文案要说清楚这条记录处于什么状态

### 候选方向（供参考，不限定）

- A. 右键中心 → 弹一个小表单（名称 + 内容两栏，Enter 提交）→ 创建后 pin。比主抽屉轻，但仍是两步
- B. 创建为 draft，pin 到轮盘但选中时**弹变量表单让用户补内容**，注入的是用户填的值
- C. content 用带变量的模板骨架（如 `{{input}}`），名称用 query，用户注入前在变量表单里填实际内容 —— 这利用了轮盘已有的 var-fill 机制，**可能最优雅**
- D. 直接删掉 D4，理由：轮盘 6 格 + 输入成本，快速创建不如回主窗口
- E. 其他你更优的方案

**选哪个由你判断并在交付里论证**（从"热键到完成注入"的步数、心智负担、轮盘格子价值三方面）

## Task 2：回归测试

用 Playwright + mock `__TAURI__` 写测试并**实际跑通**，至少覆盖：

1. 右键中心（无 query）的行为
2. 敲字过滤后右键中心的行为（**这是暴露 bug 的场景**）
3. 长按中心（550ms）的行为 —— 与右键是否一致
4. 创建出的记录 content ≠ name，或明确处于 draft 不被注入
5. 主抽屉的新建/保存路径没被破坏（内容非空校验仍在）
6. `suppressCenterClick` 逻辑仍正确（长按/右键后不该触发 click）

**测试要真跑，把输出贴进交付。** 这次我发现 Devin Review 的 F18 修复只做了一半（修了空 content，没修 content=name），所以你要**专门验证 content 的语义**，不要只验证"非空"。

## Task 3：检查同类模式

搜一遍项目里还有没有类似"用 A 字段充当 B 字段"的取巧写法（不只是 quickCreate）。重点：
- `create_prompt` / `update_prompt` 的所有调用点，字段是否都语义正确
- pack 导入路径有没有把 meta 字段映射错
- 其他 `xxx: name` 这种同字段复用

发现了就一并修（每个单独说明）。

---

# 环境与约束

- 本机**没有 cargo**，不要 `cargo build`/`cargo check`。Rust 改动（如需）要核对 API 签名并列入未编译验证清单
- 前端可跑：Playwright 在 `/home/haa/.hermes/hermes-agent/node_modules/playwright`，`require` 它
- **保持零 npm 依赖、零构建**（vanilla ES modules）
- 不引框架
- 遵守已有设计系统与 i18n（新文案中英双语都要加，key 加进 `src/js/i18n/zh-CN.js` 和 `en-US.js`）
- 遵守图标规则：只用 vendored 的 Lucide 图标，不手画

## 分支与提交

- 分支：`fix/wheel-quickcreate-content`
- 一个 commit：`fix(wheel): quickCreate must not reuse name as content (#NN)`
- push，开 PR，**`--base master`**，不合并
- PR 描述写清：问题、方案对比、为什么选这个、实测输出

---

## 最终交付

```
## 方案选择与论证
（A-E 里选了哪个，或你的方案，从步数/心智/轮盘格子三方面论证）

## 改动
（文件清单 + diff 摘要 + 新 i18n key）

## 实测输出
（Playwright 6 项覆盖的真实输出，不省略）

## 同类模式扫描结果
（Task 3 的发现，修了哪些/为什么没修）

## 未编译验证项
（如有 Rust 改动）
```

```json
COMPLETION_NOTIFY
source: assistant
task: promptkey-fix-quickcreate
deliverables:
- quickCreate content=name defect fixed with justified approach
- Playwright regression tests written AND executed (6 scenarios)
- same-pattern sweep across create/update/import paths
- branch fix/wheel-quickcreate-content + PR against master (not merged)
status: success
errors: none
```
