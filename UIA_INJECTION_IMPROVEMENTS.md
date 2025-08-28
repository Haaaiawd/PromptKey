# UIA文本注入改进方案

## 当前实现分析

经过对代码的深入分析，我们发现当前的UIA文本注入实现已经包含了大部分用户要求的功能：

1. **插入模式支持**：
   - 使用`TextPattern2.GetCaretRange`定位插入点
   - 使用`TextPattern`检测并折叠选区
   - 在折叠失败时发送`VK_RIGHT`键
   - 优先使用`SendInput`而不是剪贴板

2. **默认配置**：
   - 默认模式已设置为"insert"
   - SendInput实现已完成

## 存在的问题

尽管实现了相关功能，但用户反馈仍然存在问题，可能的原因包括：

### 1. TextPattern API兼容性问题
- `TextPattern2.GetCaretRange`在某些应用中可能不可用或不正确工作
- `TextPattern`的`CompareEndpoints`和`MoveEndpointByRange`方法在某些控件中可能行为异常

### 2. 时序问题
- 在某些应用中，折叠选区后需要更长的延迟时间
- 焦点切换和实际光标位置同步可能存在延迟

### 3. 特定应用处理不足
- 浏览器控件（如contenteditable）可能需要特殊处理
- 富文本控件可能有特定的行为模式

## 改进建议

### 1. 增强TextPattern处理逻辑

```rust
// 改进的选区折叠逻辑
match unsafe { range.CompareEndpoints(
    TextPatternRangeEndpoint_Start,
    &range,
    TextPatternRangeEndpoint_End,
) } {
    Ok(cmp) => {
        if cmp != 0 {
            selection_was_nonempty = true;
            // 尝试多种方式折叠选区
            let collapsed = try_collapse_range(&range);
            if !collapsed {
                collapse_failed = true;
            }
        }
    },
    Err(_) => {
        // 出错时保守处理
        selection_was_nonempty = true;
        collapse_failed = true;
    }
}
```

### 2. 增加延迟配置选项

```yaml
# 配置文件中增加延迟选项
applications:
  chrome.exe:
    display_name: "Google Chrome"
    settings:
      pre_inject_delay: 150
      post_collapse_delay: 50
      send_input_delay: 100
```

### 3. 浏览器专用处理策略

```rust
// 针对浏览器类应用的特殊处理
match app_name.to_lowercase().as_str() {
    "chrome.exe" | "firefox.exe" | "msedge.exe" => {
        // 浏览器应用特殊处理
        handle_browser_injection(text, context)?;
    },
    _ => {
        // 通用处理
        handle_generic_injection(text, context)?;
    }
}
```

## 针对特定场景的优化

### 1. 浏览器/富文本控件优化

对于浏览器和富文本控件，建议：

1. **强制使用SendInput**：避免剪贴板路径，防止站点的粘贴处理逻辑
2. **增加微小延迟**：在关键操作后添加10-30ms延迟
3. **添加配置开关**：为浏览器类控件提供强制禁用剪贴板的选项

### 2. VS Code/Cursor优化

对于VS Code等Electron应用：

1. **使用TextPattern增强模式**
2. **适当的焦点处理重试机制**
3. **SendInput作为主要注入方式**

## 测试验证方法

### 1. 基础功能测试

在以下应用中测试：
- 浏览器聊天框：输入几字并将光标放在中间，触发热键应在中间插入
- VS Code/Cursor：同样操作应为插入而非替换
- 记事本：验证基本功能

### 2. 特殊场景测试

- 全选状态下触发热键
- 快速连续触发多次热键
- 在不同类型的文本控件中测试

### 3. 配置验证

- 验证不同延迟配置的效果
- 测试剪贴板禁用/启用的回退机制
- 验证不同应用的特定配置

## 实施计划

1. **第一阶段**：修复TextPattern处理逻辑，增强错误处理
2. **第二阶段**：增加延迟配置选项和浏览器专用处理
3. **第三阶段**：完善测试用例，验证各种场景
4. **第四阶段**：优化性能，减少不必要的延迟

## 预期效果

通过以上改进，预期能够：

1. 在浏览器和富文本控件中实现真正的插入而非替换
2. 降低对剪贴板的依赖，避免站点粘贴处理导致的问题
3. 提高在各种应用中的兼容性和稳定性
4. 提供可配置的参数以适应不同应用的需求