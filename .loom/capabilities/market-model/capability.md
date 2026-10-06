# 市场机制（本地优先产品的内容分发形态）

## Field identity and boundary

本地优先桌面应用的内容分发/模板生态设计。涵盖：内置资源包、URL 源导入、格式设计、版权与审核责任边界。不涵盖：UI 呈现（→ design-system）、注入（→ injection-engine）。

## Project scenario

单人维护、本地优先、无后端的提示词管理器，用户为 AI 重度使用者。要解决的真实问题是"新用户空列表冷启动"和"获取高质量模板"，而不是"建一个社区"。

## Decision tree

### C1: 是否需要后端/账号体系

- entry_when: 评估任何"市场"提案的第一步
- options:
  - A: 需要（社区上传/评分/排行）→ 拒绝：与本地优先冲突、审核与版权责任无法承担 → leads_to: 毙掉该形态
  - B: 不需要（本地包/URL 拉取/文件导入）→ leads_to: C2
- decide_by: 是否需要服务器存储他人内容
- source: North_Star 边界"本地优先，云同步需显式开启"；MARKET_DECISION.md §2 成本表
- counterexample: 若未来 owner 决定投后端运营资源则重估——当前无此资源
- output: 形态白名单 {内置包, URL导入, 文件导入}

### C2: 内容来源可信度与更新机制

- entry_when: 每个入选形态的内容生命周期设计
- options:
  - A: 内置精选包（owner 策展，随版本发布）→ leads_to: 内容审核=发布前人工把关
  - B: 用户自选 URL 源 → leads_to: "来源自担"提示，产品侧不审核
  - C: 自动订阅/刷新 → 拒绝（隐性网络行为）
- decide_by: 是否产生"应用替用户信任第三方"的隐式背书
- source: espanso hub git-源模式；MARKET_DECISION.md §4
- counterexample: 内置包内容若直接搬运第三方商业库 → 版权风险，必须改写或授权
- output: 每形态的审核责任归属

### C3: 导入语义

- entry_when: 用户点击导入
- options:
  - A: 导入即拷贝进 prompts 表（无包管理）→ leads_to: DONE
  - B: 维护"已安装包"与更新关联 → 拒绝（espanso hub 复杂度，不回答"没有它会死吗"）
- decide_by: 用户是否需要"更新已导入包"——本地工具的答案是手动重导即可
- source: MARKET_DECISION.md §3
- counterexample: 若用户群形成分享生态后确有版本更新诉求再议——当前无社区
- output: 导入实现规格

## Stance and rejected defaults

做"模板库"不做"市场"。拒绝：社区后端、评分评论排行、包管理器、自动订阅。接受：内置精选 + URL/文件导入 + 导出。理由 = 真实需求是冷启动与模板质量，不是生态运营。

## Failure signals

- 设计里出现"服务器/API/账号/审核后台"字样 → 走偏成 B 案
- 内置包质量参差 → 损害的是产品颜面，宁少勿滥
- 默认开启远程拉取 → 违反"显式开启"边界

## Relationships without merger

- → product-architecture：模板库占一个导航位，导入落库复用 prompts 表
- → design-system：导入确认/来源提示文案归它管
