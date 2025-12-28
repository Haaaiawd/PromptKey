# Product Requirements Document (PRD)

**Title**: UIA注入策略删除 (UIA Injection Strategy Removal)  
**Status**: Draft  
**Version**: 1.0  
**Date**: 2025-12-27  
**Author**: Blueprint Architect  
**Related Scout Report**: `SCOUT_REPORT.md`

---

## 1. Executive Summary

**Problem**: 当前 `injector/mod.rs` 实现的 UI Automation (UIA) 注入策略复杂度极高 (CCN 72, 技术债得分 648)，包含 2 个严重安全隐患（破坏性剪贴板探测、unsafe 越界风险），占据 500+ LOC，且维护成本高。

**Solution**: 删除整个 UIA 策略实现，简化注入机制为两种策略：**Clipboard (主策略) + SendInput (备选)**。预期减少 ~500 LOC，降低 CCN 至 ~10，消除 14/23 个隐性约束。

**Value**: 提升代码可维护性 10x，消除安全隐患，为后续快速选择面板功能提供干净的代码库。

---

## 2. Goals & Non-Goals

### Goals
1. **删除 UIA 策略**: 完全移除 `inject_via_uia()` 函数及其所有依赖代码 (~500 LOC)
2. **简化注入逻辑**: 保留并优化 `inject_via_clipboard()` 和 `inject_via_sendinput()`
3. **消除安全隐患**: 
   - 删除破坏性剪贴板探测 (Invariant #8)
   - 修复/删除 unsafe 越界风险代码 (Invariant #13)
4. **降低复杂度**: 
   - CCN: 72 → ≤ 10
   - Debt Score: 648 → ≤ 100
   - LOC: 839 → ~300
5. **保持功能性**: 确保现有的 Prompt 注入功能在主流应用中仍可用（如 VSCode, Notepad++, Word, Chrome）

### Non-Goals (Out of Scope)
- ❌ **不添加新功能**: 本次 PR 仅删除代码，不实现新的注入策略
- ❌ **不改动数据库 Schema**: 保持现有 Prompts 表结构不变
- ❌ **不优化配置系统**: 仅删除 UIA 相关配置项，不重构 config 模块
- ❌ **不实现自动化测试**: 虽然建议添加，但不在本次重构范围内
- ❌ **不支持所有边缘应用**: 接受某些特殊应用（如禁用粘贴的终端）可能无法工作

---

## 3. User Stories (The "What")

### US01: 删除 UIA 策略代码
**Story**: 作为开发者，我希望移除所有 UIA 相关代码，以降低代码复杂度并消除安全隐患。

**Acceptance Criteria (AC)**:
- [ ] `inject_via_uia()` 函数已完全删除
- [ ] UIA 辅助函数已删除：
  - [ ] `probe_selection_via_clipboard()`
  - [ ] `try_collapse_via_textpattern2()`
  - [ ] `try_collapse_via_textpattern()`
  - [ ] `try_insert_via_valuepattern()`
  - [ ] 所有 `get_focused_element()` 相关逻辑
- [ ] 编辑器特定焦点处理代码已删除 (~80 LOC)
- [ ] 所有 UIA-related Windows API 调用已移除：
  - [ ] `CoInitializeEx` / `CoUninitialize`
  - [ ] `IUIAutomation::GetFocusedElement`
  - [ ] `ITextPattern`, `ITextPattern2`, `IValuePattern`
- [ ] 代码编译通过，无编译错误或警告
- [ ] `injector/mod.rs` 的 LOC 减少至 ~300 行

**Priority**: P0 (最高优先级)

---

### US02: 简化注入策略选择逻辑
**Story**: 作为开发者，我希望简化 `inject()` 函数的策略选择逻辑，只保留 Clipboard 和 SendInput 两种策略。

**Acceptance Criteria (AC)**:
- [ ] `inject()` 函数的策略选择逻辑已简化：
  ```rust
  fn inject(&self, context: &AppContext, content: &str) -> Result<(), String> {
      // 主策略: Clipboard (适用于 99% 场景)
      match self.inject_via_clipboard(context, content) {
          Ok(_) => return Ok(()),
          Err(e) => log::warn!("Clipboard inject failed: {}, trying SendInput", e),
      }
      
      // 备选策略: SendInput (应对禁用粘贴的场景)
      self.inject_via_sendinput(context, content)
  }
  ```
- [ ] 配置文件中的 `strategy` 字段行为已更新：
  - [ ] 移除 `"uia"` 选项
  - [ ] 支持 `"clipboard"` (默认) 和 `"sendinput"` (手动指定)
  - [ ] 如果配置了无效策略，记录警告并使用默认值
- [ ] 应用级配置（如 VSCode, Chrome）中的 `strategy` 不再支持 `uia`

**Priority**: P0

---

### US03: 清理配置文件中的 UIA 相关项
**Story**: 作为用户，我希望配置文件中不再包含已废弃的 UIA 相关选项，避免混淆。

**Acceptance Criteria (AC)**:
- [ ] `service/src/config/mod.rs` 中移除 UIA 相关字段：
  - [ ] 删除 `uia_mode` 字段（如存在）
  - [ ] 删除任何 UIA 特定的应用配置
- [ ] `config.yaml` 示例文件已更新：
  - [ ] 移除 `strategy: "uia"` 示例
  - [ ] 添加注释说明仅支持 `clipboard` 和 `sendinput`
- [ ] 确保向后兼容：旧配置文件中的 `strategy: "uia"` 会被静默忽略并回退到 `clipboard`

**Priority**: P1

---

### US04: 修复剪贴板操作安全性
**Story**: 作为用户，我希望剪贴板操作是安全的，不会因越界读取导致崩溃或内存错误。

**Acceptance Criteria (AC)**:
- [ ] `inject_via_clipboard()` 和相关函数中的 unsafe 剪贴板读取已修复：
  - [ ] 添加 `MAX_CLIPBOARD_SIZE` 常量（如 1MB）
  - [ ] 在读取 UTF-16 数据时添加边界检查：
    ```rust
    const MAX_CLIPBOARD_SIZE: usize = 1_000_000;
    let mut len = 0;
    loop {
        if len >= MAX_CLIPBOARD_SIZE {
            log::warn!("Clipboard data exceeds max size");
            break;
        }
        let ch = *p;
        if ch == 0 { break; }
        v.push(ch);
        p = p.add(1);
        len += 1;
    }
    ```
- [ ] 边缘情况测试通过：
  - [ ] 剪贴板为空
  - [ ] 剪贴板包含非常大的文本 (>1MB)
  - [ ] 剪贴板包含非 UTF-16 数据（错误处理）

**Priority**: P0 (安全问题)

---

### US05: 验证注入功能在主流应用中可用
**Story**: 作为产品经理，我希望确认删除 UIA 后，Clipboard 策略在主流应用中仍可正常工作。

**Acceptance Criteria (AC)**:
- [ ] 手动测试通过以下应用的注入功能：
  - [ ] VSCode (Electron)
  - [ ] Notepad++ (Scintilla)
  - [ ] Microsoft Word (Office)
  - [ ] Chrome 浏览器
  - [ ] Windows 记事本 (Notepad)
  - [ ] Slack (Electron)
- [ ] 测试场景：
  - [ ] 光标在空白位置（插入模式）
  - [ ] 有选中文本（替换模式）
  - [ ] 大文本注入（>10KB）
- [ ] 记录任何失败的应用（预期：某些终端应用可能不支持）

**Priority**: P0

---

### US06: 清理 Cargo.toml 依赖
**Story**: 作为开发者，我希望移除不再使用的 UIA 相关依赖，减少编译时间和二进制大小。

**Acceptance Criteria (AC)**:
- [ ] 检查 `service/Cargo.toml` 中是否有 UIA 专用依赖（如 `windows` crate 的 UIA 特性）
- [ ] 移除不再使用的依赖或特性标志
- [ ] `cargo build --release` 成功，二进制大小有明显减少（预期 -50KB ~ -200KB）
- [ ] 编译时间减少（可选测量）

**Priority**: P2

---

## 4. Constraint Analysis

**基于 Scout Report 的约束**:

### 4.1 技术约束
- **Windows API 依赖**: 剪贴板和 SendInput 仍需依赖 Windows API (`user32.dll`)
- **Unsafe 代码**: `inject_via_clipboard()` 和 `inject_via_sendinput()` 仍包含 unsafe blocks，需谨慎处理
- **线程安全**: 剪贴板操作需确保在正确的线程和窗口上下文执行

### 4.2 性能约束
- **注入延迟**: 删除 UIA 后，注入速度应保持或提升（UIA 的多次重试和延迟 60-150ms 将被移除）
  - **目标**: Clipboard 注入 < 100ms (p99)
  - **SendInput 注入**: < 50ms (p99)

### 4.3 兼容性约束
- **现有用户配置**: 必须向后兼容旧的 `config.yaml`，即使其中指定了 `strategy: "uia"`
- **数据库**: 不修改数据库 Schema 或数据
- **Service 进程**: 不改变 Service 的启动和生命周期管理

### 4.4 隐性约束（从 Scout 继承）
- **Invariant #1**: 剪贴板恢复竞态（~180ms 窗口）—— 保持现有备份恢复机制
- **Invariant #15**: `SetForegroundWindow` 返回值检查 —— 建议添加，但不强制（可在后续优化）
- **Invariant #18**: GUI 和 Service 双写数据库 —— 保持 WAL 模式依赖

---

## 5. Definition of Done

### 代码变更
- [ ] `service/src/injector/mod.rs` 已重构完成
- [ ] 所有 UIA 相关代码已删除（~500 LOC）
- [ ] 剪贴板 unsafe 代码已加固（边界检查）
- [ ] 配置模块已更新（移除 UIA 选项）
- [ ] 无新增编译警告或错误

### 质量保证
- [ ] 手动测试通过（US05 的所有应用）
- [ ] 代码复杂度指标验证：
  - [ ] `injector/mod.rs` 的 CCN ≤ 10
  - [ ] LOC 减少至 ~300 行
- [ ] 技术债得分降低至 ≤ 100

### 文档
- [ ] 更新 `README.md`：
  - [ ] 说明当前支持的注入策略（Clipboard + SendInput）
  - [ ] 明确 UIA 已被移除
- [ ] 更新 `config.yaml.example`：
  - [ ] 移除 `uia` 相关示例
  - [ ] 添加策略选择说明

### 部署与验证
- [ ] 构建成功：`cargo build --release`
- [ ] 二进制大小减少（可选）
- [ ] 运行时无崩溃（运行 10 分钟，注入 50+ 次）

---

## 6. Risks & Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| **删除 UIA 后某些应用不可用** | Medium | High | 接受限制，在文档中标注不支持的应用（如某些终端） |
| **剪贴板策略失败率增加** | Low | Medium | 保留 SendInput 作为 fallback；记录失败日志供后续分析 |
| **Unsafe 代码引入新的内存错误** | Low | High | 添加边界检查和防护逻辑（US04） |
| **用户配置文件兼容性问题** | Low | Low | 静默回退到默认策略 + 警告日志 |
| **回归到旧行为** | Low | Medium | 手动测试主流应用（US05） |

---

## 7. Success Metrics

### 定量指标
- **代码量**: 减少 ~500 LOC (从 839 → ~300)
- **循环复杂度**: 从 72 降至 ≤ 10
- **技术债**: 从 648 降至 ≤ 100
- **编译时间**: 减少 5-10% (可选测量)
- **二进制大小**: 减少 50KB - 200KB

### 定性指标
- **可维护性**: 开发者能在 10 分钟内理解注入逻辑（vs. 之前需要 1 小时）
- **安全性**: 消除 2 个 Critical 安全问题 + 12 个 UIA 相关隐性约束
- **稳定性**: 主流应用注入成功率 ≥ 95% (与删除前持平或提升)

---

## 8. Open Questions (To Be Resolved)

1. **Q**: 是否需要在 GUI 中添加提示，告知用户 UIA 已被移除？  
   **A**: [待决定] —— 可选，可在 Changelog 中说明

2. **Q**: 是否需要添加自动化测试（单元测试或集成测试）？  
   **A**: [待决定] —— 建议添加，但不在本次 DoD 中强制

3. **Q**: 如果 Clipboard 和 SendInput 都失败，是否需要新的错误处理逻辑？  
   **A**: [待决定] —— 当前行为是返回 Error，可保持

---

## 9. Assumptions

1. **假设**: Scout Report 中识别的 14 个 UIA 相关隐性约束是准确的
2. **假设**: 用户可以接受某些极端边缘应用（如禁用粘贴的终端）不支持注入
3. **假设**: Clipboard 策略在 Windows 10/11 系统上有 ≥95% 的成功率
4. **假设**: 现有的剪贴板备份恢复机制（~180ms 窗口）是可接受的权衡

---

## 10. Approval & Sign-Off

| Role | Name | Status | Date |
|------|------|--------|------|
| **Product Owner** | [User] | ⏳ Pending | - |
| **Architect** | Blueprint AI | ✅ Approved | 2025-12-27 |
| **Developer** | TBD | ⏳ Pending | - |

---

**End of PRD**
