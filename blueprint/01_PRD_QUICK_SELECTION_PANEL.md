# Product Requirements Document (PRD)

**Title**: 快速选择面板 (Quick Selection Panel)  
**Status**: Draft  
**Version**: 1.0  
**Date**: 2025-12-27  
**Author**: Spec Writer (Blueprint Phase)  
**Related Scout Report**: `SCOUT_CONCEPT_MODEL.md`, `SCOUT_REPORT.md`  
**Dependencies**: Phase 0 (UIA Removal) must be completed first

---

## 1. Executive Summary

**Problem**: 当前用户需要通过主GUI窗口浏览和选择Prompts，流程繁琐（需要切换窗口、滚动列表、点击多次）。高频使用场景下效率低下，打断工作流。

**Solution**: 实现一个通过热键(`Ctrl+Shift+H`)唤起的轻量级浮动选择面板，支持模糊搜索、键盘导航、一键复制Prompts到剪贴板。面板失焦后自动隐藏，不干扰用户工作流。

**Value**: 
- **效率提升**: 从"切换窗口→浏览列表→点击选择"(~10秒) 缩短至 "热键→输入关键词→回车"(<3秒)
- **流畅体验**: 无需离开当前工作窗口，保持专注
- **数据洞察**: 显示使用统计（最常用/最近使用），引导用户快速访问高频Prompts

---

## 2. Goals & Non-Goals

### Goals

1. **快速访问**: 用户可在任意应用中通过热键(`Ctrl+Shift+H`)唤起选择面板，响应时间 < 100ms (p95)
2. **高效搜索**: 支持模糊搜索（匹配Prompt名称、标签、分类），返回结果 < 50ms (p99)
3. **键盘优先**: 100%键盘操作（无需鼠标），支持↑↓导航、Enter确认、ESC关闭
4. **无干扰**: 面板失焦后自动隐藏，不遮挡用户工作区域
5. **信息丰富**: 显示Prompt内容预览（前50字符）、分类标签、使用统计（最常用Top 3、最近使用）
6. **可靠性**: 选择后Prompt内容正确复制到剪贴板，成功率 ≥ 99%

### Non-Goals (Out of Scope)

- ❌ **不支持Prompt编辑**: 面板仅用于选择，编辑功能仍在主GUI窗口
- ❌ **不支持多选**: 一次仅能选择一个Prompt复制
- ❌ **不支持拖拽**: 无窗口位置调整或大小调整功能
- ❌ **不支持自定义热键**: MVP阶段热键固定为`Ctrl+Shift+H`
- ❌ **不支持离线缓存**: 每次打开面板都实时从数据库加载Prompts
- ❌ **不支持Prompt预览编辑高亮**: 预览仅为纯文本，无语法高亮或格式化
- ❌ **不支持分类筛选UI**: 搜索引擎会匹配分类，但无独立的分类下拉菜单

---

## 3. User Stories (The "What")

### US01: 快速唤起选择面板
**Story**: 作为用户，我希望在任意应用中通过热键快速打开选择面板，以便无需切换窗口即可访问Prompts。

**Acceptance Criteria (AC)**:
- [ ] 按下`Ctrl+Shift+H`后，选择面板在 **< 100ms (p95)** 显示在屏幕中央
- [ ] 面板自动获取焦点，搜索框处于激活状态（无需额外点击）
- [ ] 面板显示时，背景应用保持可见（半透明或小窗口，不全屏遮挡）
- [ ] 错误情况：
  - 如果热键被其他应用占用，记录警告日志（用户需手动解决冲突）
  - 如果数据库读取失败，显示错误提示："无法加载Prompts，请重启应用"
- [ ] 面板可重复唤起/隐藏（无状态残留）

**Priority**: P0 (最高优先级)

---

### US02: 模糊搜索Prompts
**Story**: 作为用户，我希望通过输入关键词快速过滤Prompts，以便在大量Prompts中快速定位目标。

**Acceptance Criteria (AC)**:
- [ ] 搜索框支持实时过滤（输入即搜，无需按回车）
- [ ] 搜索范围包括：Prompt名称、标签(tags)、分类(category)
- [ ] 搜索使用模糊匹配算法（fuse.js），匹配阈值 = 0.3（30%相似度即可匹配）
- [ ] 搜索结果排序逻辑：
  - 主排序：按相关性分数降序（最匹配的排在最前）
  - 次排序：相关性相同时，按最近使用时间降序（最近选择的优先）
  - 兜底排序：从未使用的按`id`升序
- [ ] 搜索延迟 **< 50ms (p99)** （输入到结果更新的延迟）
- [ ] 搜索框支持`Ctrl+V`粘贴（快速输入长查询词）
- [ ] 显示最多 **10个搜索结果**（避免列表过长）
- [ ] 边缘情况：
  - 无匹配结果时，显示空状态："未找到匹配的Prompts"
  - 搜索词为空时，显示全部Prompts（或按使用频率排序的Top 10）
  - 中文/英文搜索均支持
- [ ] 搜索高亮：匹配的关键词在结果中高亮显示（可选，建议Phase 2实现）

**Priority**: P0

---

### US03: 键盘导航选择
**Story**: 作为重度键盘用户，我希望完全通过键盘操作选择面板，以便保持手不离开键盘的工作流。

**Acceptance Criteria (AC)**:
- [ ] 键盘操作支持：
  - `↓` (或 `Ctrl+N`): 向下移动焦点到下一个Prompt
  - `↑` (或 `Ctrl+P`): 向上移动焦点到上一个Prompt
  - `Enter`: 选择当前焦点的Prompt并复制到剪贴板
  - `ESC`: 关闭面板（不选择任何Prompt）
- [ ] 默认焦点：面板打开时，第一个Prompt自动高亮
- [ ] 循环导航：在最后一个Prompt按`↓`时，跳转到第一个；反之亦然
- [ ] 视觉反馈：当前焦点的Prompt应有明显的高亮样式（如背景色变化、边框）
- [ ] 响应速度：按键响应延迟 **< 16ms** (60fps)

**Priority**: P0

---

### US04: 一键复制到剪贴板
**Story**: 作为用户，我希望选择Prompt后立即复制到剪贴板，以便快速粘贴到目标应用。

**Acceptance Criteria (AC)**:
- [ ] 按下`Enter`后，当前焦点的Prompt内容复制到系统剪贴板
- [ ] 复制操作成功率 **≥ 99%**
- [ ] 复制后面板自动关闭（返回到之前的应用）
- [ ] 复制延迟 **< 50ms** (从按Enter到剪贴板可用)
- [ ] 错误处理：
  - 如果复制失败（如剪贴板被占用），显示Toast通知："复制失败，请重试"
  - 面板不自动关闭（允许用户再次尝试）
- [ ] 兼容性：支持Windows 10/11系统剪贴板API

**Priority**: P0

---

### US05: 失焦自动隐藏
**Story**: 作为用户，我希望点击面板外部区域或切换到其他窗口时，选择面板自动隐藏，以便不干扰我的工作流。

**Acceptance Criteria (AC)**:
- [ ] 触发条件：
  - 用户点击面板外部区域
  - 用户通过`Alt+Tab`或其他方式切换到其他应用
  - 用户按下`ESC`键
- [ ] 隐藏行为：面板窗口隐藏（非销毁），状态重置（搜索框清空、焦点回到第一项）
- [ ] 隐藏延迟：**< 100ms** (从失焦事件到窗口隐藏)
- [ ] 边缘情况：
  - 用户在搜索框输入时失焦 → 正常隐藏，输入内容不保存
  - 用户在结果列表悬停时失焦 → 正常隐藏

**Priority**: P0

---

### US06: 显示Prompt详细信息
**Story**: 作为用户，我希望在选择面板中看到Prompt的预览和元数据，以便快速识别目标Prompt。

**Acceptance Criteria (AC)**:
- [ ] 每个列表项显示：
  - **Prompt名称** (主标题，加粗)
  - **内容预览** (前50字符，灰色小字)
  - **分类标签** (如 `[API]`, `[Database]`，彩色徽章)
- [ ] 内容预览截断规则：
  - 如果Prompt内容 > 50字符，显示前50字符 + "..."
  - 如果Prompt内容 ≤ 50字符，完整显示
  - 保留换行符为空格（单行显示）
- [ ] 重复名称处理：如果多个Prompts名称相同，按`id`升序排列（创建时间早的优先）
- [ ] 分类标签颜色：
  - 每个分类固定颜色（如 API=蓝色, Database=绿色, Snippet=橙色）
  - 如果无分类，显示 `[Uncategorized]` (灰色)
- [ ] 布局规范：
  - 每个列表项高度 = 60px
  - 可见区域最多显示 7个列表项 (420px)
  - 超过7个时支持滚动

**Priority**: P0

---

### US07: 显示使用统计
**Story**: 作为用户，我希望在面板底部看到最常用和最近使用的Prompts，以便快速访问高频Prompts。

**Acceptance Criteria (AC)**:
- [ ] 面板底部显示统计栏（20px高度）
- [ ] 统计内容：
  - "🔥 Hot: [Prompt Name 1] (23次) | [Prompt Name 2] (18次)" (显示使用次数最多的Top 2)
  - 如果使用次数相同，按Prompt创建时间排序（新的优先）
- [ ] 数据来源：读取`usage_logs`表，统计`action='selector_select'`的记录
- [ ] 更新频率：每次打开面板时实时查询（无缓存）
- [ ] 边缘情况：
  - 如果无使用记录，显示："📊 Hot: 暂无数据"
  - 如果仅1个Prompt有记录，显示该Prompt（不显示Top 2）
- [ ] 统计栏不可交互（仅展示，不支持点击）

**Priority**: P1 (重要但非阻塞MVP)

---

### US08: 暗色模式支持
**Story**: 作为用户，我希望选择面板支持暗色主题，以便在夜间或低光环境下使用时保护眼睛。

**Acceptance Criteria (AC)**:
- [ ] 面板主题自动跟随Windows系统主题（暗色/亮色）
- [ ] 主题检测方式：通过`window.matchMedia('(prefers-color-scheme: dark)')` API
- [ ] 暗色主题配色方案：
  - 背景色: `#1e1e1e` (深灰)
  - 文本色: `#e0e0e0` (浅灰)
  - 高亮背景色: `#2d2d2d` (中灰)
  - 边框色: `#3e3e3e`
- [ ] 亮色主题配色方案：
  - 背景色: `#ffffff` (白色)
  - 文本色: `#333333` (深灰)
  - 高亮背景色: `#f0f0f0` (浅灰)
  - 边框色: `#e0e0e0`
- [ ] 主题切换无闪烁（CSS变量切换）
- [ ] 边缘情况：如果系统不支持主题检测，默认使用亮色主题

**Priority**: P2 (Nice to have, 如果实现简单)

---

### US09: 窗口预创建优化
**Story**: 作为开发者，我希望选择面板窗口在应用启动时预创建（隐藏状态），以便首次唤起时响应更快。

**Acceptance Criteria (AC)**:
- [ ] 应用启动时，创建Tauri窗口（label: `"selector-panel"`）但处于隐藏状态
- [ ] 窗口配置：
  - 宽度: 700px
  - 高度: 500px
  - 无边框 (`decorations: false`)
  - 背景透明 (`transparent: true`)
  - 始终置顶 (`alwaysOnTop: true`)
  - 不在任务栏显示 (`skipTaskbar: true`)
- [ ] 首次唤起延迟：**< 100ms** (p95)
- [ ] 内存占用：窗口预创建后，应用额外内存增加 **< 20MB**
- [ ] 测试：使用`cargo build --release`构建，测量启动时间增量 **< 200ms**

**Priority**: P0 (性能关键)

---

### US10: IPC通信（Service → GUI）
**Story**: 作为系统架构师，我希望Service进程能通知GUI进程显示选择面板，以便热键监听和UI展示解耦。

**Acceptance Criteria (AC)**:
- [ ] Service进程监听`Ctrl+Shift+H`热键
- [ ] 热键触发时，通过**Named Pipe**发送事件到GUI进程
- [ ] Named Pipe配置：
  - 名称: `\\.\pipe\promptkey_selector`
  - 消息格式: JSON `{"event": "show_selector"}`
  - 超时时间: 100ms (如果GUI无响应，记录警告)
- [ ] GUI进程接收到事件后，调用`show_selector_window()`显示面板
- [ ] 错误处理：
  - 如果Named Pipe创建失败，记录错误日志：`"Failed to create IPC pipe"`
  - 如果发送消息失败，重试1次；仍失败则放弃并记录
- [ ] 测试：模拟Service发送1000次事件，成功率 **≥ 99.9%**

**Priority**: P0

---

### US11: 记录选择行为日志
**Story**: 作为产品经理，我希望记录用户通过选择面板选择Prompts的行为，以便分析使用模式和优化产品。

**Acceptance Criteria (AC)**:
- [ ] 用户按`Enter`选择Prompt后，插入一条记录到`usage_logs`表：
  ```sql
  INSERT INTO usage_logs (prompt_id, action, timestamp, query)
  VALUES (?, 'selector_select', ?, ?)
  ```
  - `prompt_id`: 选择的Prompt ID
  - `action`: 固定为 `'selector_select'`
  - `timestamp`: 当前时间 (ISO 8601格式)
  - `query`: 用户输入的搜索关键词（如为空则记录为`NULL`）
- [ ] 日志记录不应阻塞UI（异步操作）
- [ ] 如果数据库写入失败，记录警告日志但不影响复制功能
- [ ] 数据保留：无自动清理机制（由用户或未来功能管理）

**Priority**: P1

---

## 4. Constraint Analysis

**基于Scout Report的约束**:

### 4.1 技术约束
- **Tauri版本**: v2.x（与现有GUI保持一致）
- **Rust版本**: 2021 Edition
- **数据库**: 复用现有SQLite数据库（`promptmgr.db`），Schema已存在
- **Windows API**: 需要Named Pipe支持（`CreateNamedPipe`, `ConnectNamedPipe`）
- **前端技术**: HTML/CSS/JS（复用现有技术栈，避免引入React/Vue）

### 4.2 性能约束
- **窗口唤起延迟**: < 100ms (p95)
- **搜索响应时间**: < 50ms (p99, 基于fuse.js性能)
- **键盘响应延迟**: < 16ms (60fps, 用户感知流畅)
- **复制操作延迟**: < 50ms
- **内存占用**: 窗口预创建后额外内存 < 20MB
- **最大支持Prompts数量**: 1000+ (fuse.js可处理, 但UI仅显示Top 10结果)

### 4.3 兼容性约束
- **操作系统**: Windows 10/11 (与现有Service保持一致)
- **剪贴板**: 使用Windows Clipboard API (已在Phase 0中使用)
- **热键系统**: 需确保`Ctrl+Shift+H`不与常见应用冲突
  - 已知冲突：无（大部分应用未占用此组合键）
  - 备选方案：如占用，记录警告但不提供fallback热键

### 4.4 数据约束
- **Schema扩展**: 需在`usage_logs`表添加`query`字段（已在Scout报告中规划）
  ```sql
  ALTER TABLE usage_logs ADD COLUMN query VARCHAR(255);
  ```
- **向后兼容**: 如果`query`字段不存在，应能gracefully降级（记录`NULL`）

### 4.5 隐性约束（从Scout继承）
- **Invariant #18**: GUI和Service双写数据库（依赖WAL模式避免冲突）
- **Invariant #20**: Service进程无健康检查（需在IPC失败时记录警告）
- **焦点管理**: 面板显示后需自动获取焦点（避免用户手动点击）

---

## 5. Definition of Done

### 功能完整性
- [ ] 所有9个User Stories的Acceptance Criteria通过
- [ ] 手动测试覆盖以下场景：
  - [ ] 从VSCode中唤起面板，搜索"API"，选择Prompt，粘贴到编辑器
  - [ ] 从Chrome中唤起面板，ESC关闭，再次唤起（状态重置）
  - [ ] 面板显示时切换到其他窗口（自动隐藏）
  - [ ] 数据库中有100+个Prompts时的搜索性能
  - [ ] 无网络连接时的行为（应正常工作，仅本地数据库）

### 性能指标
- [ ] 窗口唤起延迟 < 100ms (p95, 测量10次取平均)
- [ ] 搜索响应时间 < 50ms (p99, 输入10个不同查询)
- [ ] 键盘导航流畅（目测无卡顿）
- [ ] 内存占用 < 20MB (使用Task Manager验证)

### 质量保证
- [ ] 无编译警告或错误
- [ ] `cargo clippy` 通过（无警告）
- [ ] 代码复杂度：所有新增函数CCN ≤ 10
- [ ] 无已知的安全漏洞（无unsafe代码或已充分验证）

### 文档
- [ ] 更新`README.md`：
  - [ ] 说明选择面板功能和热键
  - [ ] 添加使用截图（可选）
- [ ] 更新`config.yaml.example`：
  - [ ] 添加`selector`配置节（热键、窗口大小等）

### 部署与验证
- [ ] `cargo build --release` 成功
- [ ] 应用启动无崩溃（运行10分钟，唤起面板50+次）
- [ ] 数据库Schema迁移成功（添加`query`字段）

---

## 6. Risks & Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| **热键冲突** | Low | High | 记录警告日志；在文档中说明如何解决冲突 |
| **Named Pipe通信失败** | Low | High | 重试机制（1次）；如仍失败则记录错误但不崩溃 |
| **fuse.js性能不足** (大量Prompts) | Low | Medium | 限制搜索结果为Top 10；如仍慢则考虑索引优化 |
| **窗口焦点管理失败** | Medium | Medium | 调用`window.focus()` + `SetForegroundWindow` 双重保险 |
| **剪贴板复制失败** | Low | High | 使用Phase 0的健壮clipboard逻辑；添加错误提示 |
| **Tauri多窗口Bug** | Very Low | High | 使用Tauri v2稳定版本；充分测试窗口生命周期 |
| **统计数据查询慢** | Low | Low | 限制查询为Top 3；添加索引（如需要）|

---

## 7. Success Metrics

### 定量指标
- **使用频率**: 用户每天平均使用选择面板 **≥ 5次** (通过`usage_logs`分析)
- **搜索成功率**: 用户输入搜索词后选择Prompt的比例 **≥ 80%** (即不是频繁打开后关闭)
- **性能达标率**: 95%的唤起操作在100ms内完成
- **错误率**: IPC通信失败率 **< 1%**

### 定性指标
- **用户反馈**: "选择面板大幅提升了工作效率" (通过用户访谈)
- **流畅度**: 键盘导航无明显延迟或卡顿（目测+用户反馈）
- **可靠性**: 无因选择面板导致的应用崩溃

---

## 8. Resolved Questions (User Decisions)

1. **Q**: 如果用户在面板显示时关闭了主GUI窗口，面板应如何表现？  
   **A**: ✅ **已决定** - 主GUI在后台运行，面板正常工作（不关闭）

2. **Q**: 是否需要支持`Ctrl+V`直接粘贴到面板的搜索框？  
   **A**: ✅ **已决定** - 支持（已融入US02）

3. **Q**: 统计栏的"最常用"是否应该排除最近7天未使用的Prompts？  
   **A**: ✅ **已决定** - 不排除，但搜索结果排序优先最近使用的（已融入US02）

4. **Q**: 是否需要支持暗色模式？  
   **A**: ✅ **已决定** - 支持，跟随系统主题（如果实现简单）（已添加US08）

5. **Q**: 如果数据库中有重复名称的Prompts，搜索结果如何排序？  
   **A**: ✅ **已决定** - 按`id`升序（创建时间早的优先）（已融入US06）

---

## 9. Assumptions

1. **假设**: fuse.js在1000个Prompts的数据集上搜索延迟 < 50ms (基于官方性能测试)
2. **假设**: Windows Named Pipe的IPC延迟 < 10ms (基于系统性能)
3. **假设**: Tauri多窗口功能在v2版本中稳定（无已知的critical bug）
4. **假设**: 用户的Prompts平均长度 < 500字符（内容预览截断为50字符足够）
5. **假设**: 用户不会在1秒内快速唤起/关闭面板 > 10次（无需防抖逻辑）
6. **假设**: 应用启动时数据库连接已建立（选择面板可直接查询）

---

## 10. Dependencies & Prerequisites

**外部依赖**:
- ✅ **Phase 0 (UIA Removal)**: 必须先完成，确保clipboard逻辑健壮
- ✅ **fuse.js**: 通过CDN引入或本地打包（决策在RFC阶段）
- ✅ **SQLite**: 复用现有数据库连接

**内部依赖**:
- ✅ `usage_logs`表已存在（需扩展`query`字段）
- ✅ `prompts`表已存在（包含`name`, `content`, `tags`, `category`字段）
- ✅ Service进程的`HotkeyService`模块（需扩展支持多热键）

**开发环境**:
- Rust 2021 + Cargo
- Tauri CLI v2.x
- Windows 10/11开发机

---

## 11. Approval & Sign-Off

| Role | Name | Status | Date |
|------|------|--------|------|
| **Product Owner** | [User] | ⏳ Pending | - |
| **Spec Writer** | Blueprint AI | ✅ Approved | 2025-12-27 |
| **System Architect** | TBD | ⏳ Pending | - |
| **Complexity Guard** | TBD | ⏳ Pending | - |

---

**End of PRD**

**Next Step**: 进入RFC阶段，设计技术架构和API签名
