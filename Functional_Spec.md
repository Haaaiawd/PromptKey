# Functional Spec — Prompt Manager (Windows MVP)

## 1. 功能范围（In-Scope）
- 全局热键唤起与注入：用户按下热键 → 触发注入流程。
- 注入策略与回退：优先 UI Automation（UIA），失败降级至剪贴板快照粘贴，再降级 SendInput。
- 模板/提示词管理（最小化）：
  - 本地 SQLite 存储：`prompts` 表（name/tags/content/type/variables/app_scopes/version 等）。
  - 模板变量：字符串、布尔、枚举（MVP 可先仅字符串+默认值）。
  - 最近使用/收藏（可记录在 `usage_logs`）。
- 按应用上下文路由：匹配前台进程名/窗口标题正则，决定默认模板与注入策略优先级。
- 配置：`%APPDATA%/PromptManager/config.yaml`（热键、数据库路径、注入顺序、是否允许剪贴板等）。
- 日志与可观测：注入结果/耗时/失败原因本地记录。

## 2. 非功能性需求（NFR）
- 性能：见 North_Star KPI。
- 资源：后台驻留内存 ≤ 60MB。
- 稳定性：热键循环不可阻塞；注入失败必须可回滚。
- 安全：禁止注入密码框/安全输入控件；不持久化用户粘贴内容。

## 3. 交互流程（MVP）
- 基本流：
  1) 用户按下全局热键（默认 Ctrl+Alt+Space）；
  2) 系统获取前台窗口信息（进程/标题）；
  3) 解析默认模板（后续可扩为选择面板）；
  4) 变量自动填充（可从选中文本/剪贴板获取，MVP 简化为固定文本）；
  5) 执行注入流水线（UIA→剪贴板→SendInput），直至成功或失败；
  6) 写入使用日志。
- 失败流：
  - 权限不足/高完整性：提示并放弃；
  - 非文本控件/不支持 UIA：剪贴板或 SendInput 降级；
  - 剪贴板争用：重试+超时回滚。

## 4. 数据模型（简化）
- prompts(id, name, tags, content, content_type, variables_json, app_scopes_json, inject_order, version, updated_at)
- usage_logs(id, prompt_id, target_app, result, created_at)

## 5. 配置键位（示例）
```yaml
hotkey: "Ctrl+Alt+Space"
database_path: C:\\Users\\<you>\\AppData\\Roaming\\PromptManager\\promptmgr.db
injection:
  order: ["uia", "clipboard", "sendinput"]
  allow_clipboard: true
```

## 6. Out-of-Scope（MVP 不包含）
- 团队协作/云同步（导入导出即可）。
- 完整变量交互表单与高级校验。
- 跨平台（macOS/Linux）。
- 安全输入控件内注入。

## 7. 验收标准（UAT）
- 能注册默认热键，按下后在记事本中成功插入占位文本（至少通过一种策略）。
- 注入失败时不破坏用户剪贴板，且自动回滚。
- SQLite 自动迁移成功，能记录一条使用日志。
