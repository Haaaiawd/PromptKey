# Phase 1 产品架构研究汇总

综合来源：`.loom/design/MERGE_ARCHITECTURE.md`（合并信息架构、功能评级表、隐式耦合清单）、`service/src/db.rs` 字段审计（is_pinned/inject_order/variables_json 已备）、`service/src/hotkey/mod.rs` 死代码注释、`src/main_simple.js` 五导航现状。

专家立场：
- "pin 是提示词属性、轮盘是 pin 投影"——轮盘管理页删除，不做第二真相源。
- 新功能准入走评级表（必需/延后/不做），与用户钦定原则"需要的就新加"对齐。
- 隐式状态耦合（find_prompt_for_context 忽略上下文）要显式化为设置项。
