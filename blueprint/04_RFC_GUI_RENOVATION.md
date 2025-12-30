# RFC: GUI Renovation — Phase 1 & 2
2

**Feature**: GUI Renovation  
**Author**: Architect  
**Status**: DRAFT  
**PRD Reference**: [03_PRD_GUI_RENOVATION.md](./03_PRD_GUI_RENOVATION.md)

---

## 1. Introduction

本 RFC 详述 GUI Renovation Phase 1 的技术实现细节。核心目标是清理过时的交互逻辑（旧的选中注入模式），适配新的 Wheel 优先架构，并提升 UI 的可用性（视图切换、热键录制修复）。

本方案涉及 **Rust 后端服务的热键逻辑裁剪** 和 **Tauri 前端交互的重构**。

---

## 2. Technical Design

### 2.1 Backend Architecture (Rust)

#### 2.1.1 Service / Hotkey Cleanup
**目标**: 移除所有非 Wheel 的热键监听，仅保留轮盘呼出热键。

*   **File**: `d:\PROJECTALL\Workflow\PromptKey\service\src\hotkey\mod.rs`
*   **Changes**:
    *   在 `HotkeyService::start` 中，**删除** ID 1 (Inject) and ID 3 (Selector) 的 `manager.register` 调用。
    *   保留 ID 4 (Wheel) 的注册，且**使其使用配置中的 `hotkey` 字段**（原 `ctrl+alt+q` 硬编码改为读取配置）。这意味着 `config.hotkey` 的语义从"注入热键"变为"轮盘热键"。

*   **File**: `d:\PROJECTALL\Workflow\PromptKey\service\src\main.rs`
*   **Changes**:
    *   在主事件循环 (`run_service` -> `match hotkey_id`) 中：
        *   移除 `1 | 2 =>` 分支 (自动注入)。
        *   移除 `3 =>` 分支 (搜索面板)。
        *   保留 `4 =>` 分支 (轮盘)，并确保它依然能够正确触发 `ipc_client.send_show_wheel()`。

#### 2.1.2 GUI Config Command
**目标**: 适配配置语义变更。

*   **File**: `d:\PROJECTALL\Workflow\PromptKey\src\main.rs`
*   **Command**: `apply_settings`
*   **Logic**:
    *   参数保持 `hotkey: Option<String>`。
    *   移除 `uia_mode` 参数处理（如果前端不再传递）。
    *   当接收到 `hotkey` 时，将其保存到 `config.toml`。此热键将重启 Service 后生效（或热更新），作为**轮盘呼出热键**。

---

### 2.2 Frontend Architecture (HTML/CSS/JS)

#### 2.2.1 Component: Prompt List
**目标**: 移除选中态，增加复制功能，支持视图切换。

*   **State**: 新增 `viewMode` 状态，值为 `'card'` (默认) 或 `'list'`。初始值从 `localStorage.getItem('promptViewMode')` 读取。
*   **View Switcher**:
    *   在 `#prompts-panel .panel-header` 中添加各 Radio Button 或 Toggle Button 组。
    *   切换时更新 `promptList.className` (添加/移除 `.compact`) 并保存状态。

*   **Render Logic (`main_simple.js`)**:
    *   **移除**: `<div class="prompt-item" onclick="...">` 中的选中逻辑 (`set_selected_prompt` 调用)。
    *   **新增**: 在 `.prompt-actions` 中添加 `<button class="copy-btn" onclick="copyPrompt(${id})">`。
    *   **Compact Mode**: 当容器有 `.compact` 类时，CSS 调整布局：
        *   `flex-direction: row`
        *   隐藏 `.prompt-content` (预览文本)
        *   只显示 Title, Tags, Actions

*   **Interaction**:
    *   `copyPrompt(id)`:通过 `prompts.find` 获取内容 -> `navigator.clipboard.writeText` -> `showNotification('已复制', 'success')`。

#### 2.2.2 Component: Logs Panel
**目标**: 移除引起歧义的重启按钮。

*   **File**: `main_simple.js`
*   **Changes**: 删除创建 `#restart-service-btn` 的 DOM 操作代码。仅保留刷新和清空按钮。

#### 2.2.3 Component: Settings Page
**目标**: 清理 UI。

*   **File**: `index.html`
*   **Changes**: 删除整个 "注入设置" (`.settings-section` 包含 "敬请期待")。
*   **Logic**: 修复热键录制 JS。确保录制的按键组合成字符串（如 "Ctrl+Alt+W"）并正确传给 `apply_settings`。

---

## 3. Implementation Steps

### Step 1: Backend Cleanup (Rust)
1.  修改 `service/src/hotkey/mod.rs`:
    *   修改 `start()`: 使用 `self.hotkey` 注册 ID 4。移除 ID 1 和 3 的注册。
2.  修改 `service/src/main.rs`:
    *   清理 `match hotkey_id`: 仅处理 ID 4。
3.  编译验证 Service 能否用配置的热键呼出轮盘。

### Step 2: GUI Interaction Refactor (JS/HTML)
1.  **Remove Legacy**:
    *   删除 `restart_service` 按钮代码。
    *   删除 `index.html` 中的注入设置 HTML。
    *   删除 `main_simple.js` 中的 `set_selected_prompt` 相关调用和 `.selected` 样式切换逻辑。
2.  **Add New Features**:
    *   实现 `copyPrompt` 函数。
    *   更新 `loadPrompts` 模版，加入复制按钮。
    *   实现 `toggleViewMode` 函数及 UI。
3.  **Refine Settings**:
    *   确保热键录制器 save 时调用 `apply_settings`。

### Step 3: Styling (CSS)
1.  定义 `.prompt-list.compact` 样式：
    *   Row layout, align items center.
    *   Hide description text.
2.  定义 `.copy-btn` 样式 (icon only or text "复制")。

---

## 4. Verification Plan

### 4.1 Automated Tests
*   **Unit Tests**: 由于主要涉及 UI 交互和 Service 内部逻辑裁剪，单元测试覆盖率可能有限。主要依靠手动验证。

### 4.2 Manual Verification
*   **V-001 热键验证**:
    *   Action: 修改设置为 `Ctrl+Alt+W`，保存。
    *   Check: 按下 `Ctrl+Alt+W` 是否呼出轮盘？按下原 `Ctrl+Alt+Space` (ID 1) 是否**无反应**？
*   **V-002 复制功能**:
    *   Action: 点击提示词卡片上的复制按钮。
    *   Check: 剪贴板中是否有正确内容？收到 Toast 提示？卡片没有变色（未选中）？
*   **V-003 视图切换**:
    *   Action: 点击列表视图按钮。
    *   Check: 列表变紧凑？内容预览消失？重启软件后是否记住视图状态？
*   **V-004 日志页**:
    *   Check: 是否还有"重启服务"按钮？

---

## 5. Security Considerations
*   无新增外部依赖或网络调用。
*   剪贴板写入操作属于用户主动触发，符合安全预期。

## 6. Definition of Done
*   所有 Rust 代码编译通过，无 Warning。
*   前端功能符合 Acceptance Criteria。
*   通过上述 V-001 至 V-004 手动验证。

---

## 7. Phase 2 Technical Design: Prompt Wheel

### 7.1 Frontend Structure (Wheel)

*   **File**: `src/wheel.html`
*   **Structure**: 
    *   Container: `.wheel-container` (Centered).
    *   Petals: `.petal` (Wrapper for rotation) -> `.petal-inner` (The visible "Jelly").
    *   Content: `.petal-content` (Rotated text/icon to remain upright).
    *   Center: `.center-overlay` (For branding or full text preview).

### 7.2 Styling Strategy (Jelly Glass)

*   **CSS Variables**:
    *   `--glass-bg`: `rgba(30, 35, 45, 0.45)`
    *   `--glass-border`: `rgba(255, 255, 255, 0.12)`
    *   `--glass-highlight`: `rgba(255, 255, 255, 0.2)`
*   **Key Properties**:
    ```css
    .petal-inner {
        backdrop-filter: blur(25px) saturate(180%);
        box-shadow: 
            inset 0 1px 1px 0 var(--glass-highlight), 
            inset 0 0 0 1px var(--glass-border),
            0 10px 20px rgba(0,0,0,0.2); /* Deep shadow */
    }
    ```
*   **Animation**:
    *   Entry: `springIn` keyframe (scale 0.8 -> 1.05 -> 1.0).
    *   Hover: Scale 1.03, brighter background.

### 7.3 Data Logic (`wheel.js`)

1.  **Init**: 
    *   Listen for `tauri://window-created` or just `DOMContentLoaded`.
    *   Invoke `get_pinned_prompts` (or `get_all_prompts` and slice top 6).
2.  **Rendering**:
    *   Map prompt data to 6 petals.
    *   Handle empty slots (grayed out or hidden).
3.  **Interaction**:
    *   `onclick` / `keypress (1-6)`:
        *   Invoke `inject_prompt(id)`.
        *   Invoke `hide_wheel_window`.
    *   `Escape`:
        *   Invoke `hide_wheel_window`.

### 7.4 Backend Integration

*   **IPC**: `ipc_client.send_show_wheel()` triggers the Tauri main window to:
    *   Show the `wheel` window.
    *   Focus it.
    *   (Optional) Move it to cursor position (or center screen).


---

## 7. Phase 2 Technical Design: Prompt Wheel

### 7.1 Frontend Structure (Wheel)

*   **File**: `src/wheel.html`
*   **Structure**: 
    *   Container: `.wheel-container` (Centered).
    *   Petals: `.petal` (Wrapper for rotation) -> `.petal-inner` (The visible "Jelly").
    *   Content: `.petal-content` (Rotated text/icon to remain upright).
    *   Center: `.center-overlay` (For branding or full text preview).

### 7.2 Styling Strategy (Jelly Glass)

*   **CSS Variables**:
    *   `--glass-bg`: `rgba(30, 35, 45, 0.45)`
    *   `--glass-border`: `rgba(255, 255, 255, 0.12)`
    *   `--glass-highlight`: `rgba(255, 255, 255, 0.2)`
*   **Key Properties**:
    ```css
    .petal-inner {
        backdrop-filter: blur(25px) saturate(180%);
        box-shadow: 
            inset 0 1px 1px 0 var(--glass-highlight), 
            inset 0 0 0 1px var(--glass-border),
            0 10px 20px rgba(0,0,0,0.2); /* Deep shadow */
    }
    ```
*   **Animation**:
    *   Entry: `springIn` keyframe (scale 0.8 -> 1.05 -> 1.0).
    *   Hover: Scale 1.03, brighter background.

### 7.3 Data Logic (`wheel.js`)

1.  **Init**: 
    *   Listen for `tauri://window-created` or just `DOMContentLoaded`.
    *   Invoke `get_pinned_prompts` (or `get_all_prompts` and slice top 6).
2.  **Rendering**:
    *   Map prompt data to 6 petals.
    *   Handle empty slots (grayed out or hidden).
3.  **Interaction**:
    *   `onclick` / `keypress (1-6)`:
        *   Invoke `inject_prompt(id)`.
        *   Invoke `hide_wheel_window`.
    *   `Escape`:
        *   Invoke `hide_wheel_window`.

### 7.4 Backend Integration

*   **IPC**: `ipc_client.send_show_wheel()` triggers the Tauri main window to:
    *   Show the `wheel` window.
    *   Focus it.
    *   (Optional) Move it to cursor position (or center screen).

