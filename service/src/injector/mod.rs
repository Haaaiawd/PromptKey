use std::result::Result as StdResult;
use windows::{
    core::*,
    Win32::UI::Accessibility::*,
    Win32::System::Com::*,
    Win32::UI::Input::KeyboardAndMouse::*,
    Win32::UI::WindowsAndMessaging::*,
    Win32::System::DataExchange::*,
    Win32::System::Memory::*,
};
use crate::config::Config;
use std::time::Duration;

#[derive(Debug)]
pub struct InjectionContext {
    #[allow(dead_code)]
    pub target_text: String,
    pub app_name: String,
    pub window_title: String,
    #[allow(dead_code)]
    pub window_handle: windows::Win32::Foundation::HWND,
}

#[derive(Debug, Clone)]
pub enum InjectionStrategy {
    UIA,
    #[allow(dead_code)]
    Clipboard,
    #[allow(dead_code)]
    SendInput,
}

pub struct Injector {
    strategies: Vec<InjectionStrategy>,
    config: Config,
}

impl Injector {
    pub fn new(strategies: Vec<InjectionStrategy>, config: Config) -> Self {
        log::debug!("Creating injector with strategies: {:?}", strategies);
        Injector { strategies, config }
    }
    
    pub fn inject(&self, text: &str, context: &InjectionContext) -> StdResult<(), Box<dyn std::error::Error>> {
    log::info!("Attempting to inject text using available strategies");
        log::debug!("Text to inject: {}", text);
        log::debug!("Context: app_name={}, window_title={}", context.app_name, context.window_title);
        
    let effective = self.effective_strategies_for(&context.app_name);
    log::debug!("Effective strategy order: {:?}", effective);

    for strategy in &effective {
            log::debug!("Trying injection strategy: {:?}", strategy);
            let result = match strategy {
                InjectionStrategy::UIA => self.inject_via_uia(text, context),
                InjectionStrategy::Clipboard => self.inject_via_clipboard(text, context),
                InjectionStrategy::SendInput => self.inject_via_sendinput(text, context),
            };
            
            match result {
                Ok(_) => {
                    log::info!("Successfully injected text using {:?} strategy", strategy);
                    return Ok(());
                }
                Err(e) => {
                    log::warn!("Failed to inject using {:?} strategy: {}", strategy, e);
                    // 继续尝试下一个策略
                }
            }
        }
        
        log::error!("All injection strategies failed");
        Err("All injection strategies failed".into())
    }

    fn effective_strategies_for(&self, app_name: &str) -> Vec<InjectionStrategy> {
    // 移除应用名特例，纯能力驱动：默认使用配置顺序
    // 具体的能力判断与回退在各策略实现内部完成（例如 UIA 路径会在缺少 ValuePattern 时优先剪贴板回退）
    let _ = app_name; // 抑制未使用警告
    self.strategies.clone()
    }
    
    fn inject_via_uia(&self, text: &str, context: &InjectionContext) -> StdResult<(), Box<dyn std::error::Error>> {
        log::debug!("Attempting UIA injection");
        
        // 将目标窗口前置，提升跨应用稳定性
        unsafe {
            if !context.window_handle.0.is_null() {
                let _ = SetForegroundWindow(context.window_handle);
            }
        }
        
        // 初始化COM
        unsafe {
            let result = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            if result.is_err() {
                return Err(format!("COM initialization failed with HRESULT: {:?}", result).into());
            }
        }
        
        // 获取UI自动化对象
        let automation: IUIAutomation = unsafe {
            match CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) {
                Ok(instance) => instance,
                Err(e) => return Err(e.into()),
            }
        };
        
        // 获取焦点元素
        let focused_element = unsafe {
            match automation.GetFocusedElement() {
                Ok(element) => element,
                Err(e) => return Err(e.into()),
            }
        };

        // 在焦点元素下寻找真正可编辑的子元素（ValuePattern/TextPattern）
        let target_element = find_editable_element(&automation, &focused_element)
            .unwrap_or(focused_element.clone());

        // 将目标元素设为焦点，提升模式接口可用性
        unsafe { let _ = target_element.SetFocus(); }
        
        // 检查是否为密码框
        let is_password = unsafe {
            match focused_element.CurrentIsPassword() {
                Ok(result) => result,
                Err(e) => return Err(e.into()),
            }
        };
        
        if is_password.as_bool() {
            return Err("Cannot inject into password fields".into());
        }
        
        // 尝试使用ValuePattern
    let value_pattern_result = unsafe { target_element.GetCurrentPattern(UIA_ValuePatternId) };
        
        if let Ok(pattern) = value_pattern_result {
            let value_pattern: IUIAutomationValuePattern = pattern.cast()?;
            // 先判断是否只读
            let read_only = unsafe { value_pattern.CurrentIsReadOnly()? }.as_bool();
            if read_only {
                log::warn!("ValuePattern is read-only on target element; trying TextPattern/SendInput fallback");
            } else {
            
            // 根据配置选择覆盖或末尾追加
            let final_text = if self.config.injection.uia_value_pattern_mode == "append" {
                let current_value = unsafe { value_pattern.CurrentValue() };
                match current_value {
                    Ok(val) => format!("{}{}", val.to_string(), text),
                    Err(_) => text.to_string(),
                }
            } else {
                // overwrite
                text.to_string()
            };
            
            // 使用 SetValue 注入文本
            unsafe {
                value_pattern.SetValue(&final_text.into())?;
            }
            // 轻微等待后做读回校验，避免“看似成功但实际未变”的情况（如 VS Code）
            let mut ok = false;
            for _ in 0..3 {
                std::thread::sleep(Duration::from_millis(60));
                let verify_result = unsafe { value_pattern.CurrentValue() };
                if let Ok(v) = verify_result {
                    let v = v.to_string();
                    if self.config.injection.uia_value_pattern_mode == "append" {
                        if v.ends_with(text) { ok = true; break; }
                    } else {
                        // 覆盖模式下 final_text 等于 text
                        if v == text { ok = true; break; }
                    }
                }
            }
            if ok {
                return Ok(());
            } else {
                log::warn!("UIA SetValue verification failed; trying clipboard paste then SendInput");
                // 优先尝试剪贴板粘贴（更快且更稳定），最后再回退 SendInput
                if self.config.injection.allow_clipboard {
                    if let Err(e) = self.inject_via_clipboard(text, context) {
                        log::warn!("Clipboard paste fallback failed: {}. Using SendInput.", e);
                        self.type_text_via_sendinput(text)?;
                    }
                } else {
                    self.type_text_via_sendinput(text)?;
                }
                return Ok(());
            }
            }
        }
        
        // 如果没有 ValuePattern，则尝试 TextPattern + SendInput 回退
        log::debug!("Trying to get TextPattern");
    let text_pattern_result = unsafe { target_element.GetCurrentPattern(UIA_TextPatternId) };
        
        if let Ok(pattern) = text_pattern_result {
            log::debug!("TextPattern found - trying InsertText if supported, else SendInput");
            let text_pattern: IUIAutomationTextPattern = pattern.cast()?;
            
            // 尝试获取当前选区
            let selection_result = unsafe { text_pattern.GetSelection() };
            if let Ok(selection) = selection_result {
                let selection_length = unsafe { selection.Length()? };
                log::debug!("Selection length: {}", selection_length);
                
                if selection_length > 0 {
                    // 有选区，先删除选区内容，然后在当前位置插入新文本
                    log::debug!("Replacing selected text by deleting selection and inserting new text");
                    let text_range = unsafe { selection.GetElement(0)? };
                    unsafe {
                        // 选中范围并删除内容（通过将范围折叠到起点来删除选中内容）
                        text_range.Select()?;
                    }
                    log::debug!("Selection cleared, now inserting new text");
                } else {
                    // 无选区，在光标位置追加（移动到文档末尾）
                    log::debug!("No selection, appending at end of document");
                    let doc_range = unsafe { text_pattern.DocumentRange()? };
                    unsafe {
                        // 将光标移到末尾
                        let _ = doc_range.MoveEndpointByUnit(TextPatternRangeEndpoint_End, TextUnit_Character, 1_000_000);
                        let _ = doc_range.MoveEndpointByUnit(TextPatternRangeEndpoint_Start, TextUnit_Character, 1_000_000);
                        doc_range.Select()?;
                    }
                    log::debug!("Caret moved to end via TextPattern");
                }
            } else {
                // 无法获取选区信息，回退到文档末尾追加
                log::debug!("Could not get selection, falling back to end of document");
                let doc_range = unsafe { text_pattern.DocumentRange()? };
                unsafe {
                    // 将光标移到末尾
                    let _ = doc_range.MoveEndpointByUnit(TextPatternRangeEndpoint_End, TextUnit_Character, 1_000_000);
                    let _ = doc_range.MoveEndpointByUnit(TextPatternRangeEndpoint_Start, TextUnit_Character, 1_000_000);
                    doc_range.Select()?;
                }
                log::debug!("Caret moved to end via TextPattern");
            }

            // 在文本编辑控件（仅有 TextPattern 无 ValuePattern）上，优先尝试剪贴板粘贴（更稳定、跨控件适用）
            if self.config.injection.allow_clipboard {
                match self.inject_via_clipboard(text, context) {
                    Ok(_) => {
                        log::info!("Text injected via Clipboard after TextPattern caret placement");
                        return Ok(());
                    }
                    Err(e) => {
                        log::warn!("Clipboard paste after TextPattern failed: {}. Falling back to SendInput.", e);
                    }
                }
            }
            // 回退为 SendInput 模拟键入
            self.type_text_via_sendinput(text)?;
            log::info!("Text injected via TextPattern + SendInput fallback");
            return Ok(());
        } else {
            log::debug!("TextPattern not available");
        }
        
        // 返回错误或继续其他策略
        Err("No suitable pattern found for injection".into())
    }
    
    fn inject_via_clipboard(&self, text: &str, _context: &InjectionContext) -> StdResult<(), Box<dyn std::error::Error>> {
        log::debug!("Attempting clipboard injection");
        // 打开剪贴板，最多尝试 5 次
        let mut opened = false;
        for _ in 0..5 {
            unsafe {
                if OpenClipboard(HWND(0)).as_bool() {
                    opened = true; break;
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        if !opened {
            return Err("OpenClipboard failed".into());
        }

        // 读取现有剪贴板文本，用于注入后恢复
        let mut prev_text: Option<Vec<u16>> = None;
        unsafe {
            if IsClipboardFormatAvailable(CF_UNICODETEXT).as_bool() {
                let h = GetClipboardData(CF_UNICODETEXT);
                if !h.0.is_null() {
                    let ptr = GlobalLock(h);
                    if !ptr.is_null() {
                        // 复制出原文本（以 UTF-16 null 结尾）
                        let mut v = Vec::new();
                        let mut p = ptr as *const u16;
                        loop {
                            let ch = *p;
                            v.push(ch);
                            if ch == 0 { break; }
                            p = p.add(1);
                        }
                        prev_text = Some(v);
                        GlobalUnlock(h);
                    }
                }
            }
        }

        // 设置我们的文本到剪贴板
        unsafe {
            EmptyClipboard();
            // 分配全局内存并拷贝 UTF-16 文本（含结尾 0）
            let mut utf16: Vec<u16> = text.encode_utf16().collect();
            utf16.push(0);
            let bytes = (utf16.len() * std::mem::size_of::<u16>()) as usize;
            let hmem = GlobalAlloc(GMEM_MOVEABLE, bytes);
            if hmem.0.is_null() {
                CloseClipboard();
                return Err("GlobalAlloc failed".into());
            }
            let ptr = GlobalLock(hmem) as *mut u8;
            if ptr.is_null() {
                GlobalFree(hmem);
                CloseClipboard();
                return Err("GlobalLock failed".into());
            }
            std::ptr::copy_nonoverlapping(
                utf16.as_ptr() as *const u8,
                ptr,
                bytes,
            );
            GlobalUnlock(hmem);
            if SetClipboardData(CF_UNICODETEXT, hmem).0.is_null() {
                GlobalFree(hmem);
                CloseClipboard();
                return Err("SetClipboardData failed".into());
            }
            // 关闭剪贴板，让目标应用可读取
            CloseClipboard();
        }

            // 等待一下，确保热键修饰键已释放
            std::thread::sleep(Duration::from_millis(80));
            // 模拟 Ctrl+V 粘贴
        unsafe {
            let mut inputs = [
                // Ctrl down
                INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_CONTROL, wScan: VIRTUAL_KEY(0).0 as u16, dwFlags: KEYBD_EVENT_FLAGS(0), time: 0, dwExtraInfo: 0 } } },
                // V down
                INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(0x56), wScan: 0, dwFlags: KEYBD_EVENT_FLAGS(0), time: 0, dwExtraInfo: 0 } } },
                // V up
                INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(0x56), wScan: 0, dwFlags: KEYBD_EVENT_FLAGS(KEYEVENTF_KEYUP.0), time: 0, dwExtraInfo: 0 } } },
                // Ctrl up
                INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_CONTROL, wScan: VIRTUAL_KEY(0).0 as u16, dwFlags: KEYBD_EVENT_FLAGS(KEYEVENTF_KEYUP.0), time: 0, dwExtraInfo: 0 } } },
            ];
            if SendInput(&mut inputs, std::mem::size_of::<INPUT>() as i32) == 0 {
                return Err("SendInput Ctrl+V failed".into());
            }
        }

        // 粘贴后稍等再恢复剪贴板（避免覆盖目标应用读取）
        std::thread::sleep(Duration::from_millis(100));

        // 尝试恢复原剪贴板文本
        if let Some(v) = prev_text {
            unsafe {
                if OpenClipboard(HWND(0)).as_bool() {
                    EmptyClipboard();
                    let bytes = (v.len() * std::mem::size_of::<u16>()) as usize;
                    let hmem = GlobalAlloc(GMEM_MOVEABLE, bytes);
                    if !hmem.0.is_null() {
                        let ptr = GlobalLock(hmem) as *mut u8;
                        if !ptr.is_null() {
                            std::ptr::copy_nonoverlapping(v.as_ptr() as *const u8, ptr, bytes);
                            GlobalUnlock(hmem);
                            let _ = SetClipboardData(CF_UNICODETEXT, hmem);
                        } else {
                            GlobalFree(hmem);
                        }
                    }
                    CloseClipboard();
                }
            }
        }

        log::info!("Text injected via Clipboard paste");
        Ok(())
    }
    
    fn inject_via_sendinput(&self, _text: &str, _context: &InjectionContext) -> StdResult<(), Box<dyn std::error::Error>> {
        log::debug!("SendInput injection skipped in MVP");
        Err("SendInput injection not implemented in MVP".into())
    }

    fn type_text_via_sendinput(&self, text: &str) -> StdResult<(), Box<dyn std::error::Error>> {
    log::debug!("Using SendInput to simulate typing: '{}'", text);
    // 小延时，避免与热键修饰键冲突或焦点切换未完成
    std::thread::sleep(Duration::from_millis(80));
        unsafe {
            for ch in text.encode_utf16() {
                let mut inputs = [INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: ch,
                            dwFlags: KEYBD_EVENT_FLAGS(KEYEVENTF_UNICODE.0),
                            time: 0,
                            dwExtraInfo: 0,
                        }
                    }
                }, INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: ch,
                            dwFlags: KEYBD_EVENT_FLAGS(KEYEVENTF_UNICODE.0 | KEYEVENTF_KEYUP.0),
                            time: 0,
                            dwExtraInfo: 0,
                        }
                    }
                }];
                let sent = SendInput(&mut inputs, std::mem::size_of::<INPUT>() as i32);
                if sent == 0 {
                    let err = windows::Win32::Foundation::GetLastError();
                    log::error!("SendInput failed with error: {:?}", err);
                    return Err("SendInput failed".into());
                }
            }
        }
        Ok(())
    }
}

/// 在元素子树中查找可编辑的元素（优先 ValuePattern，其次 TextPattern），限制遍历节点数避免卡顿
fn find_editable_element(
    automation: &IUIAutomation,
    root: &IUIAutomationElement,
) -> Option<IUIAutomationElement> {
    // 1) 优先使用属性条件查找：ControlType=Edit 或 支持 Value/Text Pattern
    unsafe {
        let cond_edit = automation
            .CreatePropertyCondition(UIA_ControlTypePropertyId, &VARIANT::from(UIA_EditControlTypeId.0))
            .ok();
        let cond_val = automation
            .CreatePropertyCondition(UIA_IsValuePatternAvailablePropertyId, &VARIANT::from(true))
            .ok();
        let cond_txt = automation
            .CreatePropertyCondition(UIA_IsTextPatternAvailablePropertyId, &VARIANT::from(true))
            .ok();
        if let (Some(ce), Some(cv), Some(ct)) = (cond_edit, cond_val, cond_txt) {
            if let Ok(or1) = automation.CreateOrCondition(&ce, &cv) {
                if let Ok(or_all) = automation.CreateOrCondition(&or1, &ct) {
                    if let Ok(el) = root.FindFirst(TreeScope_Subtree, &or_all) {
                        log::debug!("Editable element found via PropertyCondition");
                        return Some(el);
                    }
                }
            }
        }
    }

    // 2) 回退：RawViewWalker BFS
    let walker = unsafe { automation.RawViewWalker().ok()? };

    // 简单 BFS，最多 128 节点
    let mut queue: std::collections::VecDeque<IUIAutomationElement> = std::collections::VecDeque::new();
    queue.push_back(root.clone());
    let mut visited = 0u32;

    while let Some(node) = queue.pop_front() {
        visited += 1;
        if visited > 128 { break; }

        // 命中条件：支持 ValuePattern 或 TextPattern
        let has_value = unsafe { node.GetCurrentPattern(UIA_ValuePatternId).is_ok() };
        let has_text = unsafe { node.GetCurrentPattern(UIA_TextPatternId).is_ok() };
        if has_value || has_text {
            return Some(node);
        }

        // 遍历子节点
        unsafe {
            if let Ok(mut child) = walker.GetFirstChildElement(&node) {
                loop {
                    queue.push_back(child.clone());
                    match walker.GetNextSiblingElement(&child) {
                        Ok(next) => { child = next; }
                        Err(_) => break,
                    }
                }
            }
        }
    }
    None
}