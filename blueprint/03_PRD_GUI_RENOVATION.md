# PRD: GUI Renovation — Phase 1 & 2


**版本**: 2.0  
**作者**: Architect  
**日期**: 2025-12-29  
**状态**: 待确认

---

## 1. Executive Summary (执行摘要)

在完成 PromptWheel 功能后，主控制面板 (`index.html`) 存在以下问题：
1. **过时的交互逻辑**: 提示词卡片的"选中"功能已被 Wheel 接管
2. **冗余代码**: 多个热键注册中只需保留轮盘热键 (Ctrl+Alt+Q)
3. **失效的 UI 元素**: "重启服务"按钮、"注入设置"占位区等

本 PRD 定义精确的改动清单，聚焦于 **GUI 层面的清理与适配**，不涉及 Service 核心架构变更。

---

## 2. 精确改动清单

### 2.1 前端改动 (HTML/CSS/JS)

| 改动项 | 位置 | 当前状态 | 目标状态 |
|:---|:---|:---|:---|
| **移除"重启服务"按钮** | `main_simple.js` L608-630 | JS 动态插入按钮 | 删除相关代码 |
| **移除"注入设置"占位** | `index.html` L70-78 | "敬请期待" div | 整个 section 删除 |
| **提示词卡片交互重构** | `main_simple.js` L977-1006 | 点击 → 选中 → `set_selected_prompt` | 移除点击选中逻辑 |
| **添加"复制"按钮** | `main_simple.js` L955-973 | 只有编辑/删除按钮 | 新增复制按钮 |
| **添加视图切换器** | `index.html` L36-41 | 无 | 卡片/列表切换按钮 |
| **热键录制功能修复** | `main_simple.js` L349-440 | 可能有 bug | 检查并修复 |

### 2.2 后端改动 (Rust)

| 改动项 | 位置 | 当前状态 | 目标状态 |
|:---|:---|:---|:---|
| **删除热键 ID 1** | `service/src/hotkey/mod.rs` L106-111 | 注册主注入热键 | 删除 |
| **删除热键 ID 3** | `service/src/hotkey/mod.rs` L113-116 | 注册选择器热键 `Ctrl+Shift+H` | 删除 |
| **保留热键 ID 4** | `service/src/hotkey/mod.rs` L118-123 | 轮盘热键 `Ctrl+Alt+Q` | 保留 + 改为可配置 |
| **清理热键处理逻辑** | `service/src/main.rs` L64-96 | 处理 ID 1/2/3/4 | 只处理 ID 4 |
| **更新设置保存** | `src/main.rs` apply_settings | 保存 hotkey + uiaMode | 只保存 hotkey（轮盘热键）|

### 2.3 CSS 改动

| 改动项 | 当前状态 | 目标状态 |
|:---|:---|:---|
| 添加 `.prompt-list.compact` | 无 | 紧凑列表视图样式 |
| 添加 `.copy-btn` | 无 | 复制按钮样式 |
| 移除 `.prompt-item.selected` | 存在但不再需要 | 删除 |

---

## 3. User Stories (精简版)

### US-001: 提示词卡片交互重构
**当前**: 点击卡片 → 添加 `.selected` → 调用 `set_selected_prompt`  
**目标**: 
- 移除点击选中逻辑
- 添加"复制"按钮，点击复制内容到剪贴板
- 显示 Toast 提示"已复制"

### US-002: 日志面板清理
**当前**: 工具栏有"刷新日志"、"重启服务"、"清空日志"  
**目标**: 移除"重启服务"按钮

### US-003: 设置页简化
**当前**: 热键设置 + 注入设置（敬请期待）+ 界面设置  
**目标**: 
- 保留热键设置（录制轮盘热键）
- 删除"注入设置" section
- 保留界面设置（主题切换）

### US-004: 热键系统精简
**当前**: 注册 3 个热键 (主注入/选择器/轮盘)  
**目标**: 只保留轮盘热键 `Ctrl+Alt+Q`，支持通过设置页修改

### US-005: 提示词视图切换
**目标**: 在提示词面板 header 添加切换器，支持卡片/列表两种视图

---

## 4. Non-Goals (明确不做)

| 排除项 | 原因 |
|:---|:---|
| Service 核心架构重构 | 保持稳定性 |
| 市场功能 | 需要后端 API |
| 日志导出/筛选 | 低优先级 |
| App Scopes 编辑器 | 复杂度高 |
| 深色主题切换到默认 | 用户选择保持白色 |

---

## 5. 文件影响范围

```
src/
├── index.html          # 删除"注入设置"section，添加视图切换器
├── main_simple.js      # 大量修改：移除选中逻辑、添加复制、移除重启服务
└── styles.css          # 添加 .compact 样式，删除 .selected

service/src/
├── hotkey/mod.rs       # 删除热键 ID 1/3，只保留 ID 4
└── main.rs             # 清理热键处理逻辑

src/main.rs (GUI Rust)
└── apply_settings      # 简化设置保存逻辑
```

---

## 6. 验收标准

- [ ] 点击提示词卡片不再有"选中"效果
- [ ] 每个卡片有"复制"按钮，点击后内容进入剪贴板
- [ ] 日志页面没有"重启服务"按钮
- [ ] 设置页面没有"注入设置"区域
- [ ] 热键录制可以修改轮盘触发键
- [ ] 保存设置后轮盘新热键立即生效
- [ ] 视图可在卡片/列表间切换

---

## 8. Phase 2: Prompt Wheel (提示词轮盘)

### 8.1 Design Philosophy (设计哲学)
*   **Aesthetics**: **iOS Jelly Glass** (果冻玻璃拟态).
    *   **Translucency**: High saturation blur (`backdrop-filter: blur(25px) saturate(180%)`).
    *   **Depth**: Layered shadows, glossy highlights (inner borders).
    *   **Motion**: Spring animations (bouncy, organic) for appearing and hovering.
*   **Interaction**:
    *   **Radial Layout**: 6 petals (hexagonal symmetry) + Center hub.
    *   **Quick Access**: Mouse gestures or Numpad/Digit keys (1-6).
    *   **Instant Injection**: Click or Keypress immediately injects and closes.

### 8.2 Functional Requirements
1.  **Data Source**: Load top 6 "Pinned" or "Most Used" prompts from backend.
2.  **Visuals**:
    *   Transparent background window.
    *   Petals expand/spring out on open.
    *   Hover effects: Scale up, brighten, show full text in center? (TBD).
3.  **Control**:
    *   `Esc` or Click outside: Close without injecting.
    *   Selection: Inject text into previously active window.

### 8.3 Technical Constraints
*   **Windowing**: Must use a transparent, borderless, always-on-top Tauri window.
*   **Performance**: Animation must be 60fps; use CSS GPU acceleration.


---

## 7. Sign-off

请用户确认后进入 RFC 阶段。
