# Phase 1 设计系统研究汇总

综合来源：`.loom/design/UI_UX_REDESIGN.md`（Phase 1 全量产出：token 体系、动效规范、i18n、三轮盘原型对比、稳定性清单）、`main_simple.js` 现状审计（8 处 alert、调试 DOM 写入）、`prototypes/*.html`（可交互验证）。

专家立场：
- 桌面工具应用不引框架：状态复杂度低，vanilla ES modules + token 化 CSS 足够；构建链改造成本 > 收益。
- 明暗双主题用语义 token 双层，`prefers-color-scheme` + 手动覆盖 + 持久化；切换只 transition 背景/文字色避免闪烁。
- 动效只碰 transform/opacity、≤280ms、reduced-motion 必须降级；backdrop-blur 上限 ~24px。
- 错误反馈统一 toast；用户可见文案全部走 i18n key，中英同步给值。
