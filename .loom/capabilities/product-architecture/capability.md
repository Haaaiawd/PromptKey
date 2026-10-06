# 产品架构（提示词+轮盘合并与功能取舍）

## Field identity and boundary

桌面生产力工具的信息架构与功能必要性裁决。核心方法：每个功能必须回答"没有它会死吗"；从真实工作流（找得快→选得准→注得稳→留得住）倒推而非功能清单堆叠。不涵盖：视觉/动效规范（→ design-system）、注入实现（→ injection-engine）。

## Project scenario

单用户桌面工具的信息架构合并：两个一级对象收敛为属性+投影模型，隐式耦合显式化为设置项

## Decision tree

### C1: 某概念该是一级对象还是属性

- entry_when: 评估"轮盘/标签/收藏/默认提示词"等概念的信息层级
- options:
  - A: 独立页面对象 → 仅当其生命周期操作集 ≥3 且与提示词不同（轮盘：pin 是唯一操作 → 不合格）
  - B: 内联属性/筛选维度 → leads_to: DONE
- decide_by: 该概念的独立操作集大小；轮盘 = prompts 表 + pinned 标志，pin/unpin 一个动词
- source: phase1-synthesis.md（原始依据：MERGE_ARCHITECTURE.md §1-2；`db.rs:42-59` 字段即证据）
- counterexample: "记录"页——操作集小但读路径独立且为排障必需 → 保留独立 tab 但末位
- output: 信息架构层级

### C2: 新功能准入

- entry_when: 任何功能提案
- options:
  - A: 必需（链路断点/安全边界/数据主权）→ 本期做
  - B: 有价值但链路不断 → 延后，记入 backlog
  - C: 不回答"没有它会死吗" → 不做
- decide_by: 缺失它时"找-选-注-留"链路是否断
- source: phase1-synthesis.md（原始依据：用户钦定原则"需要的功能就新加，不需要的就不要"；MERGE_ARCHITECTURE.md §4 评级表）
- counterexample: 变量填充是高价值但可延后（字段已备 `variables_json`），与"必需"的区别是链路可用占位变量直通
- output: 功能评级

### C3: 隐式状态耦合清除

- entry_when: 发现"看似 A 实际 B"的行为（如 find_prompt_for_context 忽略上下文只读 selected_prompt，`db.rs:401-414`）
- options:
  - A: 保留隐式 → 拒绝（用户无法预测 ID1 热键注入目标）
  - B: 显式化（"默认快捷提示词"设置）或语义化（最近使用 #1）→ leads_to: DONE
- decide_by: 用户能否在界面上看到并预测该状态
- source: phase1-synthesis.md（原始依据：MERGE_ARCHITECTURE.md §5.2）
- counterexample: 纯内部缓存类隐式状态无用户可感知差异 → 不必管
- output: 显式状态/设置项

### C4: 死代码处置

- entry_when: 发现无触发路径的 UI/IPC（selector 面板热键已移除、wheel-panel 适应规则假卡片、config.applications 死配置）
- options:
  - A: 删除 → leads_to: DONE
  - B: 保留"以后可能用" → 仅当复活成本低且需求明确时保留数据字段（app_scopes_json 属此类），UI 必删
- decide_by: 是否存在真实触发路径 + 复活价值
- source: phase1-synthesis.md（原始依据：`service/src/hotkey/mod.rs:103` 注释、main.rs:64-77 只处理 id=4）
- counterexample: 数据字段保留（迁移成本低）vs UI 死代码删除（维护成本持续）
- output: 删除清单

## Stance and rejected defaults

合并 = pin 内联化 + 轮盘降级为渲染视图；导航 4 项；拒绝多轮盘/场景轮盘/版本对比/模板继承/云同步；日志页保留但改名"记录"居末位。

## Failure signals

- 导航 ≥5 项 → 合并没做干净
- 用户问"轮盘在哪配" → pin 内联可见性失败（用预览按钮+设置示意补）
- 新功能评审说不出"没有它会死" → 回到 C2 重审
- find_prompt_for_context 类隐式行为残留 → C3 未执行

## Relationships without merger

- → injection-engine：注入策略栈的能力探测为本树提供"目标控件能力"事实
- → design-system：IA 落地为导航/布局/空态归它管
- → market-model：模板库页归本树定位为导航第 2 位
