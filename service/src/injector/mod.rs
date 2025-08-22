use std::result::Result as StdResult;
use windows::{
    core::*,
    Win32::UI::Accessibility::*,
    Win32::System::Com::*,
    Win32::UI::Input::KeyboardAndMouse::*,
    Win32::UI::WindowsAndMessaging::*,
    Win32::System::DataExchange::*,
    Win32::System::Memory::*,
    Win32::Foundation::*,
};
use crate::config::Config;
use std::time::Duration;

// windows 0.58 下方便使用的常量（CF_UNICODETEXT = 13）
const CF_UNICODETEXT_CONST: u32 = 13;

#[derive(Debug)]
pub struct InjectionContext {
    #[allow(dead_code)]
    pub target_text: String,
    pub app_name: String,
    pub window_title: String,
    #[allow(dead_code)]
    pub window_handle: windows::Win32::Foundation::HWND,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectionStrategy {
    UIA,
    #[allow(dead_code)]
    Clipboard,
    #[allow(dead_code)]
    SendInput,
}

#[derive(Debug, Clone)]
pub enum EditorType {
    Generic,
    Scintilla,      // Notepad++
    Electron,       // VS Code, Atom
    WPF,           // Visual Studio
    Swing,         // IntelliJ IDEA, Eclipse
    Qt,            // Qt-based editors
}

#[derive(Debug, Clone)]
pub struct EditorDetection {
    pub editor_type: EditorType,
    pub class_name: String,
    pub framework_id: String,
    pub process_name: String,
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
        // 获取应用特定配置
        let app_config = self.config.get_app_config(app_name);
        
        // 构建策略列表：primary + fallback
        let mut strategies = Vec::new();
        
        // 添加主要策略
        if let Some(primary_strategy) = self.parse_strategy(&app_config.strategies.primary) {
            strategies.push(primary_strategy);
        }
        
        // 添加回退策略
        for fallback in &app_config.strategies.fallback {
            if let Some(fallback_strategy) = self.parse_strategy(fallback) {
                if !strategies.contains(&fallback_strategy) {
                    strategies.push(fallback_strategy);
                }
            }
        }
        
        // 如果没有配置策略，使用默认顺序
        if strategies.is_empty() {
            strategies = self.strategies.clone();
        }
        
        log::debug!("Effective strategies for {}: {:?}", app_name, strategies);
        strategies
    }
    
    fn parse_strategy(&self, strategy_name: &str) -> Option<InjectionStrategy> {
        match strategy_name.to_lowercase().as_str() {
            "uia" | "textpattern_enhanced" => Some(InjectionStrategy::UIA),
            "clipboard" => Some(InjectionStrategy::Clipboard),
            "sendinput" => Some(InjectionStrategy::SendInput),
            _ => None,
        }
    }
    
    fn inject_via_uia(&self, text: &str, context: &InjectionContext) -> StdResult<(), Box<dyn std::error::Error>> {
        log::debug!("Attempting UIA injection");
        
        // 将目标窗口前置，提升跨应用稳定性
        unsafe {
            if !context.window_handle.0.is_null() {
                let _ = SetForegroundWindow(context.window_handle);
            }
        }
        
        // 根据应用类型添加延迟
        let pre_inject_delay = self.get_pre_inject_delay(&context.app_name);
        if pre_inject_delay > 0 {
            log::debug!("Pre-injection delay: {}ms for app: {}", pre_inject_delay, context.app_name);
            std::thread::sleep(Duration::from_millis(pre_inject_delay));
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
        
        // 检测编辑器类型并应用特殊处理
        let editor_detection = self.detect_editor_type(&focused_element, &context.app_name)?;
        log::debug!("Detected editor type: {:?} for app: {}", editor_detection.editor_type, context.app_name);

        // 在焦点元素下寻找真正可编辑的子元素（ValuePattern/TextPattern）
        let target_element = find_editable_element(&automation, &focused_element)
            .unwrap_or(focused_element.clone());

        // 根据编辑器类型应用特殊的焦点处理
        self.apply_editor_specific_focus(&target_element, &editor_detection)?;
        
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
                    // 根据配置决定插入位置：当前光标位置或文档末尾
                    if self.config.injection.uia_value_pattern_mode == "append" {
                        // 移动到文档末尾
                        log::debug!("Appending at end of document");
                        let doc_range = unsafe { text_pattern.DocumentRange()? };
                        unsafe {
                            // 将光标移到末尾
                            let _ = doc_range.MoveEndpointByUnit(TextPatternRangeEndpoint_End, TextUnit_Character, 1_000_000);
                            let _ = doc_range.MoveEndpointByUnit(TextPatternRangeEndpoint_Start, TextUnit_Character, 1_000_000);
                            doc_range.Select()?;
                        }
                        log::debug!("Caret moved to end via TextPattern");
                    } else {
                        // 在当前光标位置插入
                        log::debug!("Inserting at current cursor position");
                        // 当前光标位置已经正确，不需要额外移动
                    }
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

            // 根据编辑器类型选择最优注入策略
            let editor_detection = self.detect_editor_type(&target_element, &context.app_name)?;
            match editor_detection.editor_type {
                EditorType::Electron => {
                    // VS Code等Electron应用，SendInput通常更稳定
                    log::debug!("Using SendInput for Electron-based editor");
                    self.type_text_via_sendinput(text)?;
                }
                EditorType::Swing => {
                    // Java Swing应用，尽量使用剪贴板
                    if self.config.injection.allow_clipboard {
                        match self.inject_via_clipboard(text, context) {
                            Ok(_) => {
                                log::info!("Text injected via Clipboard for Swing editor");
                                return Ok(());
                            }
                            Err(e) => {
                                log::warn!("Clipboard paste failed for Swing: {}. Trying SendInput.", e);
                                self.type_text_via_sendinput(text)?;
                            }
                        }
                    } else {
                        self.type_text_via_sendinput(text)?;
                    }
                }
                _ => {
                    // 通用策略：优先剪贴板，回退SendInput
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
                }
            }
            log::info!("Text injected via TextPattern + fallback strategy");
            return Ok(());
        } else {
            log::debug!("TextPattern not available");
        }
        
        // 如果UIA完全不可用，根据编辑器类型选择最佳回退策略
        let editor_detection = self.detect_editor_type(&focused_element, &context.app_name).unwrap_or(EditorDetection {
            editor_type: EditorType::Generic,
            class_name: "Unknown".to_string(),
            framework_id: "Unknown".to_string(),
            process_name: context.app_name.clone(),
        });
        
        log::warn!("No UIA patterns available for {:?} editor, trying fallback strategies", editor_detection.editor_type);
        
        // 优先使用剪贴板（兼容性好）
        if self.config.injection.allow_clipboard {
            if let Ok(_) = self.inject_via_clipboard(text, context) {
                log::info!("Text injected via Clipboard fallback for {:?} editor", editor_detection.editor_type);
                return Ok(());
            }
        }
        
        // 最后使用SendInput
        self.type_text_via_sendinput(text)?;
        log::info!("Text injected via SendInput fallback for {:?} editor", editor_detection.editor_type);
        Ok(())
    }
    
    fn inject_via_clipboard(&self, text: &str, _context: &InjectionContext) -> StdResult<(), Box<dyn std::error::Error>> {
        log::debug!("Attempting clipboard injection");

        // 1) 打开剪贴板，最多尝试 5 次
    let mut opened = false;
        for _ in 0..5 {
            unsafe {
        if OpenClipboard(HWND(std::ptr::null_mut())).is_ok() {
                    opened = true;
                }
            }
            if opened { break; }
            std::thread::sleep(Duration::from_millis(10));
        }
        if !opened {
            return Err("OpenClipboard failed".into());
        }

        // 2) 读取现有剪贴板文本，用于注入后恢复
        let mut prev_text: Option<Vec<u16>> = None;
        unsafe {
            if IsClipboardFormatAvailable(CF_UNICODETEXT_CONST).is_ok() {
                if let Ok(h) = GetClipboardData(CF_UNICODETEXT_CONST) {
                    let hg = HGLOBAL(h.0);
                    let ptr = GlobalLock(hg) as *const u16;
                    if !ptr.is_null() {
                        let mut v = Vec::new();
                        let mut p = ptr;
                        loop {
                            let ch = *p;
                            v.push(ch);
                            if ch == 0 { break; }
                            p = p.add(1);
                        }
                        prev_text = Some(v);
                        let _ = GlobalUnlock(hg);
                    }
                }
            }
        }

        // 3) 设置我们的文本到剪贴板
        unsafe {
            let _ = EmptyClipboard();
            let mut utf16: Vec<u16> = text.encode_utf16().collect();
            utf16.push(0);
            let bytes = (utf16.len() * std::mem::size_of::<u16>()) as usize;
            let hmem = GlobalAlloc(GMEM_MOVEABLE, bytes).map_err(|_| "GlobalAlloc failed")?;
            let ptr = GlobalLock(hmem) as *mut u8;
            if ptr.is_null() {
                let _ = GlobalFree(hmem);
                let _ = CloseClipboard();
                return Err("GlobalLock failed".into());
            }
            std::ptr::copy_nonoverlapping(utf16.as_ptr() as *const u8, ptr, bytes);
            let _ = GlobalUnlock(hmem);
            if SetClipboardData(CF_UNICODETEXT_CONST, HANDLE(hmem.0)).is_err() {
                let _ = GlobalFree(hmem);
                let _ = CloseClipboard();
                return Err("SetClipboardData failed".into());
            }
            let _ = CloseClipboard();
        }

        // 4) 等待一下，确保热键修饰键已释放，然后模拟 Ctrl+V 粘贴
        std::thread::sleep(Duration::from_millis(80));
        unsafe {
            let mut inputs = [
                INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_CONTROL, wScan: VIRTUAL_KEY(0).0 as u16, dwFlags: KEYBD_EVENT_FLAGS(0), time: 0, dwExtraInfo: 0 } } },
                INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(0x56), wScan: 0, dwFlags: KEYBD_EVENT_FLAGS(0), time: 0, dwExtraInfo: 0 } } },
                INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(0x56), wScan: 0, dwFlags: KEYBD_EVENT_FLAGS(KEYEVENTF_KEYUP.0), time: 0, dwExtraInfo: 0 } } },
                INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_CONTROL, wScan: VIRTUAL_KEY(0).0 as u16, dwFlags: KEYBD_EVENT_FLAGS(KEYEVENTF_KEYUP.0), time: 0, dwExtraInfo: 0 } } },
            ];
            if SendInput(&mut inputs, std::mem::size_of::<INPUT>() as i32) == 0 {
                return Err("SendInput Ctrl+V failed".into());
            }
        }

        // 5) 粘贴后稍等再恢复剪贴板（避免覆盖目标应用读取）
        std::thread::sleep(Duration::from_millis(100));

        if let Some(v) = prev_text {
            unsafe {
                if OpenClipboard(HWND(std::ptr::null_mut())).is_ok() {
                    let _ = EmptyClipboard();
                    let bytes = (v.len() * std::mem::size_of::<u16>()) as usize;
                    let hmem = GlobalAlloc(GMEM_MOVEABLE, bytes).map_err(|_| "GlobalAlloc failed")?;
                    let ptr = GlobalLock(hmem) as *mut u8;
                    if !ptr.is_null() {
                        std::ptr::copy_nonoverlapping(v.as_ptr() as *const u8, ptr, bytes);
                        let _ = GlobalUnlock(hmem);
                        let _ = SetClipboardData(CF_UNICODETEXT_CONST, HANDLE(hmem.0));
                    } else {
                        let _ = GlobalFree(hmem);
                    }
                    let _ = CloseClipboard();
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

    fn get_pre_inject_delay(&self, app_name: &str) -> u64 {
        let app_config = self.config.get_app_config(app_name);
        app_config.settings.pre_inject_delay
    }
    
    fn detect_editor_type(&self, element: &IUIAutomationElement, app_name: &str) -> StdResult<EditorDetection, Box<dyn std::error::Error>> {
        let class_name = unsafe {
            element.CurrentClassName().unwrap_or_else(|_| "Unknown".into()).to_string()
        };
        
        let framework_id = unsafe {
            element.CurrentFrameworkId().unwrap_or_else(|_| "Unknown".into()).to_string()
        };
        
        let editor_type = match (class_name.as_str(), framework_id.as_str(), app_name.to_lowercase().as_str()) {
            ("Scintilla", _, _) => EditorType::Scintilla,  // Notepad++
            (_, "WPF", _) => EditorType::WPF,              // Visual Studio
            ("Chrome_WidgetWin_1", _, "code.exe") => EditorType::Electron, // VS Code
            (_, _, "idea64.exe") | (_, _, "idea.exe") => EditorType::Swing,    // IntelliJ IDEA
            _ => EditorType::Generic
        };
        
        log::debug!("Editor detection: class={}, framework={}, type={:?}", class_name, framework_id, editor_type);
        
        Ok(EditorDetection {
            editor_type,
            class_name,
            framework_id,
            process_name: app_name.to_string(),
        })
    }
    
    fn apply_editor_specific_focus(&self, element: &IUIAutomationElement, detection: &EditorDetection) -> StdResult<(), Box<dyn std::error::Error>> {
        let app_config = self.config.get_app_config(&detection.process_name);
        let retry_count = app_config.settings.focus_retry_count;
        
        match detection.editor_type {
            EditorType::Electron => {
                // VS Code等Electron应用需要多次焦点设置
                for i in 0..retry_count {
                    unsafe { 
                        let _ = element.SetFocus(); 
                    }
                    std::thread::sleep(Duration::from_millis(30));
                    log::debug!("Electron focus attempt {}/{}", i + 1, retry_count);
                }
            },
            EditorType::Swing => {
                // Java Swing应用需要特殊的焦点处理
                if app_config.settings.use_accessibility_api {
                    log::debug!("Using enhanced accessibility API for Swing editor");
                    std::thread::sleep(Duration::from_millis(150)); // 等待Java AWT事件队列
                }
                
                for i in 0..retry_count {
                    unsafe { let _ = element.SetFocus(); }
                    std::thread::sleep(Duration::from_millis(50));
                    log::debug!("Swing focus attempt {}/{}", i + 1, retry_count);
                }
                log::debug!("Applied Swing-specific focus handling");
            },
            EditorType::Scintilla => {
                // Scintilla控件需要确保编辑区域获得焦点
                for i in 0..retry_count {
                    unsafe { let _ = element.SetFocus(); }
                    std::thread::sleep(Duration::from_millis(60));
                    log::debug!("Scintilla focus attempt {}/{}", i + 1, retry_count);
                }
                log::debug!("Applied Scintilla-specific focus handling");
            },
            EditorType::WPF => {
                // WPF控件通常焦点设置较快
                unsafe { let _ = element.SetFocus(); }
                std::thread::sleep(Duration::from_millis(30));
                log::debug!("Applied WPF-specific focus handling");
            },
            _ => {
                // 通用焦点设置
                unsafe { let _ = element.SetFocus(); }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        Ok(())
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
        let cond_document = automation
            .CreatePropertyCondition(UIA_ControlTypePropertyId, &VARIANT::from(UIA_DocumentControlTypeId.0))
            .ok();
        let cond_val = automation
            .CreatePropertyCondition(UIA_IsValuePatternAvailablePropertyId, &VARIANT::from(true))
            .ok();
        let cond_txt = automation
            .CreatePropertyCondition(UIA_IsTextPatternAvailablePropertyId, &VARIANT::from(true))
            .ok();
        if let (Some(ce), Some(cd), Some(cv), Some(ct)) = (cond_edit, cond_document, cond_val, cond_txt) {
            // 创建复合条件：(Edit OR Document) AND (ValuePattern OR TextPattern)
            if let Ok(control_or) = automation.CreateOrCondition(&ce, &cd) {
                if let Ok(pattern_or) = automation.CreateOrCondition(&cv, &ct) {
                    if let Ok(final_cond) = automation.CreateAndCondition(&control_or, &pattern_or) {
                        if let Ok(el) = root.FindFirst(TreeScope_Subtree, &final_cond) {
                            log::debug!("Editable element found via enhanced PropertyCondition");
                            return Some(el);
                        }
                    }
                }
            }
        }
    }

    // 2) 回退：增强的BFS搜索，优先查找Document控件
    let walker = unsafe { automation.RawViewWalker().ok()? };

    // 两阶段搜索：先找Document控件，再找Edit控件
    let mut candidates = Vec::new();
    let mut queue: std::collections::VecDeque<IUIAutomationElement> = std::collections::VecDeque::new();
    queue.push_back(root.clone());
    let mut visited = 0u32;

    while let Some(node) = queue.pop_front() {
        visited += 1;
        if visited > 128 { break; }

        // 获取控件类型和模式支持
        let control_type = unsafe { 
            node.CurrentControlType().unwrap_or(UIA_CustomControlTypeId) 
        };
        let has_value = unsafe { node.GetCurrentPattern(UIA_ValuePatternId).is_ok() };
        let has_text = unsafe { node.GetCurrentPattern(UIA_TextPatternId).is_ok() };
        
        if has_value || has_text {
            let priority = match control_type {
                UIA_DocumentControlTypeId => 3,  // 最高优先级：Document + Pattern
                UIA_EditControlTypeId => 2,      // 中等优先级：Edit + Pattern  
                _ => 1,                           // 低优先级：其他 + Pattern
            };
            candidates.push((priority, node.clone()));
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
    
    // 返回优先级最高的候选元素
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    if let Some((priority, element)) = candidates.first() {
        log::debug!("Found editable element via BFS with priority: {}", priority);
        return Some(element.clone());
    }
    log::debug!("No editable element found in subtree");
    None
}