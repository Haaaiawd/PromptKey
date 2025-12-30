# Tasks: GUI Renovation Phase 1 & 2


**RFC Reference**: [04_RFC_GUI_RENOVATION.md](./04_RFC_GUI_RENOVATION.md)

| Task ID | Component | Description | Done When |
|:---:|:---|:---|:---|
| T1-001 | Backend | 移除热键服务中 ID 1 (Inject) 和 ID 3 (Selector) 的注册逻辑 | 代码编译通过，相关热键失效 |
| T1-002 | Backend | 修改热键服务 ID 4 (Wheel) 的注册，使其读取配置中的 `hotkey` 字段 | 轮盘热键响应配置文件的修改 |
| T1-003 | Backend | 清理 `main.rs` 事件循环，仅保留 ID 4 的处理分支 | 代码编译通过 |
| T1-004 | Backend | 更新 `apply_settings` 命令，仅保存 `hotkey` 到配置文件 | 调用命令后 `config.toml` 更新正确 |
| T1-005 | Frontend | 删除 `main_simple.js` 中创建 `#restart-service-btn` 的 DOM 操作 | 日志页不再显示重启按钮 |
| T1-006 | Frontend | 删除 `index.html` 中 `#injection-settings` 相关的 HTML 结构 | 设置页不再显示"敬请期待" |
| T1-007 | Frontend | 实现 `copyPrompt(id)` 函数及 Toast 提示 | 调用函数能复制文本并弹提示 |
| T1-008 | Frontend | 更新 `loadPrompts` 模版：移除 `onclick` 选中逻辑，增加复制按钮 | 界面显示复制按钮，点击卡片无选中反应 |
| T1-009 | Frontend | 实现 `toggleViewMode` 逻辑及 Header 上的切换 UI | 点击切换按钮能切换 `.compact` 类并持久化 |
| T1-010 | Styling | 添加 `.prompt-list.compact` 及复制按钮的 CSS 样式 | 列表模式下布局变为紧凑行 |
| T1-011 | Frontend | 修复热键录制 JS，确保能生成正确的字符串并调用 `apply_settings` | 录制界面交互正常，保存后后端生效 |

## Phase 2: Prompt Wheel Implementation

| Task ID | Component | Description | Done When |
|:---:|:---|:---|:---|
| T2-001 | Frontend | 实现 `wheel.html` 结构 (6 Petals + Center) | ✅ 页面包含正确的 DOM 结构 |
| T2-002 | Styling | 实现 `wheel.css` (iOS Jelly Glass 风格) | ✅ 模糊、透明、阴影效果，动画流畅 |
| T2-003 | Frontend | 实现 `wheel.js` (数据加载与交互) | ✅ 后端加载提示词，点击/按键触发注入 |
| T2-004 | Backend | 集成后端 `show_wheel` IPC 与窗口管理 | ✅ 热键唤起轮盘，注入后自动隐藏 |
