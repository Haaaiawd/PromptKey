# 设计系统（Zinc 工业风·明暗双套·动效·i18n）

## Field identity and boundary

桌面应用视觉与交互系统设计：token 体系（色/字/距/圆角/影/动效）、主题架构、组件规范、i18n 文案层、可访问性、前端稳定性。不涵盖：功能取舍（→ product-architecture）、注入（→ injection-engine）。

## Project scenario

Tauri 桌面应用，vanilla 技术栈（决策：不引框架），延续 GUI_RENOVATION_GUIDE.md 的 Zinc 深色系扩展到明暗双套 + 中英双版 + 动效规范。三个窗口：主窗（1000×700）、透明轮盘窗、（废弃 selector）。

## Decision tree

### C1: 技术栈/库引入

- entry_when: 任何"要不要引框架/库"的提案
- options:
  - A: Vanilla + ES modules 结构化 → 默认答案
  - B: 构建链框架 → 仅当证明 vanilla 无法维护且接受 beforeBuildCommand 改造 → leads_to: 重估（当前否决）
- decide_by: 打包体积增量、构建链复杂度、动画是否需框架级状态同步（本应用不需要）
- source: UI_UX_REDESIGN.md §1 选型表
- counterexample: 若未来状态复杂度爆炸（多端实时同步）可启用 Preact+htm 无构建退路
- output: 技术栈决策

### C2: 动效准入

- entry_when: 每个新动效
- options:
  - A: 只碰 transform/opacity、时长 ≤280ms、reduced-motion 降级 → 放行
  - B: 碰 layout 属性/超长动画 → 拒绝
- decide_by: GPU 合成层规则 + prefers-reduced-motion 尊重
- source: UI_UX_REDESIGN.md §2.3 性能红线
- counterexample: 轮盘 backdrop-blur >24px 在集成显卡掉帧 → 设上限
- output: 动效 token

### C3: 主题/语言运行时切换

- entry_when: 用户切换主题或语言
- options:
  - A: CSS 变量 + data-i18n 属性批量替换 + IPC 广播三窗 → 方案
  - B: 整页重载 → 拒绝（状态丢失）
- decide_by: 切换后是否需要重渲且能否保留窗口状态
- source: UI_UX_REDESIGN.md §5-6
- counterexample: 轮盘是独立窗口——切换需跨窗 IPC 通知，不能只改主窗
- output: 切换机制

### C4: 稳定性防护

- entry_when: 评审每个前端改动
- options:
  - A: alert/静默失败/裸 invoke → 拦截，转 toast + store 包装
  - B: 正规错误边界 → 放行
- decide_by: UI_UX_REDESIGN.md §8 加固清单
- source: main_simple.js:564/655 alert 现状
- counterexample: 无——防护清单必须全做
- output: 合并闸口

## Stance and rejected defaults

延续 Zinc 工业风；vanilla 不引框架；动效"有而克制"；轮盘推荐 Mini 径向 A 案。拒绝：React/Tailwind 迁移、全元素主题过渡动画、>500px 轮盘、alert() 兜底、ignore 的 prefers-reduced-motion。

## Failure signals

- 新组件裸写 hex 色值不走 token → 主题切换必然漏
- 轮盘在亮色桌面壁纸下文字不可读 → 玻璃层/描边没按规范
- 某交互耗时 >300ms 动画 → 违反 token
- 切换语言后仍有中文硬编码 → i18n key 抽取不全
- 出现新的 alert() → 加固清单没执行

## Relationships without merger

- → product-architecture：它定"有没有这个页"，本档案定"长什么样"
- → injection-engine：注入反馈 toast/错误呈现共用 toast 组件
