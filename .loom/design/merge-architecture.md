# 提示词 + 轮盘合并架构

- Kind: product
- Status: done (Phase 1 设计完成)

## 正文位置

**`.loom/design/MERGE_ARCHITECTURE.md`**

## 结论摘要

合并 = 消灭独立"轮盘配置页"，pin 变提示词内联属性，轮盘降级为 pinned 子集的运行时投射视图。导航 5→4（提示词/模板库/记录/设置）。修复 `find_prompt_for_context` 隐式耦合（ID1 热键改为显式"默认快捷提示词"）。附完整功能必要性评级表（必需/延后/不做）。

## Related documents and capabilities

- 正文：`.loom/design/MERGE_ARCHITECTURE.md`
- Capability：`.loom/capabilities/product-architecture/capability.md`
- 原型：`prototypes/main-window.html`
