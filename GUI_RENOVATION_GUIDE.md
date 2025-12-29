# PromptKey GUI Renovation & Evolution Guide

## 1. 演进总结 (Evolution Summary)

我们从一个基础的 **"CRUD 管理后台"** 进化为了一个 **"现代化桌面生态系统"**。

| 维度 | V1 初始状态 | V2 当前目标状态 | 核心差异 |
| :--- | :--- | :--- | :--- |
| **设计语言** | 通用 Admin 风格 (Light Theme, 朴素) | **Zinc/Slate 深色工业风** (匹配 PromptWheel) | 需要引入 `wheel.css` 中的 Zinc 色板变量，保证视觉一致性。 |
| **数据结构** | 简单的 `name`, `content` | **复杂元数据** (`tags` 数组, `app_scopes`, `version`) | 前端表单需要支持 JSON 序列化和标签输入组件。 |
| **交互模式** | 基础表单提交 | **富交互** (Tag 芯片, 应用范围选择器) | 需要更高级的 UI 组件来处理复杂字段。 |
| **定位** | 简单的数据库编辑器 | **Prompt 工程控制台** | 界面需要体现"配置"和"规则"的专业感。 |

---

## 2. 视觉重构规范 (Visual Renovation)

主窗口 (`index.html`) 不需要像轮盘那样完全透明异形，但必须共享 **"PromptKey Design System"**。

### 2.1 共享色板 (Shared Palette)
请在 `styles.css` 中定义以下核心变量（提取自 `wheel.css`）：

```css
:root {
    /* Zinc Scale (Dark Mode Base) */
    --pk-bg-base: #09090b;       /* Zinc 950 */
    --pk-bg-surface: #18181b;    /* Zinc 900 */
    --pk-border: #27272a;        /* Zinc 800 */
    --pk-text-primary: #fafafa;  /* Zinc 50 */
    --pk-text-secondary: #a1a1aa;/* Zinc 400 */
    
    /* Brand Colors */
    --pk-accent: #3b82f6;        /* Modern Blue */
    --pk-danger: #ef4444;
}

body {
    background-color: var(--pk-bg-base);
    color: var(--pk-text-primary);
    font-family: 'Inter', system-ui, sans-serif;
}
```

### 2.2 组件风格
*   **卡片/面板**：使用 subtle border (`1px solid var(--pk-border)`) 代替沉重的阴影。
*   **按钮**：扁平化，Hover 时轻微背景色变化，避免 3D/拟物感。
*   **输入框**：深色背景 (`--pk-bg-surface`)，去边框或仅保留底部边框。

---

## 3. 功能模块改造 (Feature Updates)

根据 `service/src/db.rs` 的最新结构，**"管理提示词" (Prompts Panel)** 需要重大升级。

### 3.1 提示词数据结构变更
后端 `Prompt` 结构已更新，前端必须同步适配：

| 字段名 | 旧版处理 | **新版需求 (V2)** | 前端改造点 |
| :--- | :--- | :--- | :--- |
| `tags` | 文本字符串 | **`Vec<String>` (JSON)** | **Tags Input 组件**：输入回车生成 Tag 芯片，提交时序列化为 `["tag1", "tag2"]`。 |
| `app_scopes_json` | 无 | **`Option<String>` (JSON)** | **应用范围选择器**：允许用户输入目标 App 名称（如 `notepad.exe`），支持多选。 |
| `inject_order` | 无 | **`Option<String>`** | **排序权重输入**：允许设置权重 (如 "10")。 |
| `version` | 无 | **`i32`** | 只读展示，每次编辑自动 +1。 |

### 3.2 新增/编辑模态框 (Modal Renovation)
现有的简单表单必须重写，布局建议：

*   **Header**: 标题 + 快捷操作。
*   **Body (双栏布局)**:
    *   **左栏 (Content)**: 大面积的 `textarea`，支持语法高亮（未来），用于编辑 Prompt 内容。
    *   **右栏 (Metadata)**:
        *   **Name**: 提示词名称。
        *   **Tags**: 交互式 Tag 输入。
        *   **App Scopes**: "生效应用" (如 `chrome.exe`)。
        *   **Variables**: 变量定义 (JSON)。
*   **Footer**: 取消/保存按钮。

---

## 4. 前端助手任务清单 (Tasks for Frontend Assistant)

请将此清单交给前端助手执行：

1.  **样式同步 (`styles.css`)**: 
    - [ ] 废弃旧的 `theme-light`，全面切换到 **Zinc Dark Theme**。
    - [ ] 引入 `wheel.css` 中的变量，确保主窗口和轮盘视觉一致。
2.  **JS 逻辑升级 (`main_simple.js`)**:
    - [ ] 更新 `fetchPrompts` 和 `savePrompt` 逻辑，处理 Rust 后端新的 JSON 字段 (`tags`, `app_scopes_json`)。
    - [ ] 如果 `tags` 从后端拿到是 JSON string，前端需要 `JSON.parse()` 转为数组显示。
3.  **UI 组件开发**:
    - [ ] 实现一个轻量级的 **Tags Input** (输入文本 -> 回车 -> 生成可删除的 `<span>` 标签)。
    - [ ] 优化列表页展示：在列表中显示 Tags 芯片，而不仅仅是文本。

---

**核心口号**：让管理后台配得上那个漂亮的轮盘。
