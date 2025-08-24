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
    #[allow(dead_code)]
    Scintilla,      // Notepad++
    #[allow(dead_code)]
    Electron,       // VS Code, Atom
    #[allow(dead_code)]
    WPF,           // Visual Studio
    #[allow(dead_code)]
    Swing,         // IntelliJ IDEA, Eclipse
    #[allow(dead_code)]
    Qt,            // Qt-based editors
}

#[derive(Debug, Clone)]
pub struct EditorDetection {
    #[allow(dead_code)]
    pub editor_type: EditorType,
    #[allow(dead_code)]
    pub class_name: String,
    #[allow(dead_code)]
    pub framework_id: String,
    #[allow(dead_code)]
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
        // 优先使用应用级策略；否则使用全局顺序
        let app_cfg = self.config.get_app_config(app_name);
        let mut order: Vec<String> = Vec::new();
        if !app_cfg.strategies.primary.is_empty() {
            order.push(app_cfg.strategies.primary.to_lowercase());
            for f in app_cfg.strategies.fallback { order.push(f.to_lowercase()); }
        } else {
            order = self.config.injection.order.iter().map(|s| s.to_lowercase()).collect();
        }

        // 去重并映射到枚举
        let mut seen = std::collections::HashSet::new();
        let mut mapped = Vec::new();
        for s in order {
            let key = s.as_str();
            let variant = match key {
                "uia" | "textpattern_enhanced" => Some(InjectionStrategy::UIA),
                "clipboard" => Some(InjectionStrategy::Clipboard),
                "sendinput" => Some(InjectionStrategy::SendInput),
                _ => { log::debug!("Unknown strategy '{}' ignored", s); None }
            };
            if let Some(v) = variant {
                let name = match v { InjectionStrategy::UIA => "uia", InjectionStrategy::Clipboard => "clipboard", InjectionStrategy::SendInput => "sendinput" };
                if seen.insert(name.to_string()) { mapped.push(v); }
            }
        }
        if mapped.is_empty() { mapped.push(InjectionStrategy::UIA); }
        mapped
    }

    fn inject_via_uia(&self, text: &str, context: &InjectionContext) -> StdResult<(), Box<dyn std::error::Error>> {
        log::debug!("inject_via_uia starting; mode={}", self.config.injection.uia_value_pattern_mode);

        // 将目标窗口置前，稍等焦点稳定
        unsafe { let _ = SetForegroundWindow(context.window_handle); }
        std::thread::sleep(Duration::from_millis(self.get_pre_inject_delay(&context.app_name)));

        // 初始化 COM 并创建 UIAutomation
        unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }
        let automation: IUIAutomation = unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)? };

        // 获取聚焦元素，必要时在其子树中寻找可编辑控件
        let focused_element = unsafe { automation.GetFocusedElement()? };
        let target_element = find_editable_element(&automation, &focused_element).unwrap_or(focused_element.clone());

        // 应用一些编辑器特定的焦点处理
        let detection = self.detect_editor_type(&target_element, &context.app_name).unwrap_or(EditorDetection{
            editor_type: EditorType::Generic,
            class_name: "Unknown".to_string(),
            framework_id: "Unknown".to_string(),
            process_name: context.app_name.clone()
        });
        let _ = self.apply_editor_specific_focus(&target_element, &detection);

        // 如果是密码或不可编辑，直接走粘贴/SendInput
        let is_password = unsafe { target_element.CurrentIsPassword().unwrap_or(BOOL(0)).as_bool() };
        if is_password { log::warn!("Target element is password field; bypassing ValuePattern"); }

        match self.config.injection.uia_value_pattern_mode.as_str() {
            "insert" => {
                // 保持当前光标/选区：如果能拿到 TextPattern，仅用于确认选区，不移动光标
                if let Ok(p) = unsafe { target_element.GetCurrentPattern(UIA_TextPatternId) } {
                    if let Ok(tp) = p.cast::<IUIAutomationTextPattern>() {
                        if let Ok(sel) = unsafe { tp.GetSelection() } {
                            let len = unsafe { sel.Length().unwrap_or(0) }; // 仅日志用途
                            log::debug!("Current selection count: {}", len);
                        }
                    }
                }

                // 执行插入：优先剪贴板，失败回退 SendInput
                if self.config.injection.allow_clipboard {
                    if let Err(e) = self.inject_via_clipboard(text, context) {
                        log::warn!("Clipboard insert failed: {}. Fallback to SendInput.", e);
                        self.type_text_via_sendinput(text)?;
                    }
                } else {
                    self.type_text_via_sendinput(text)?;
                }
                Ok(())
            }
            mode => {
                // 非 insert 模式：尝试 ValuePattern（append/overwrite），失败走 TextPattern + 粘贴/SendInput
                if !is_password {
                    if let Ok(p) = unsafe { target_element.GetCurrentPattern(UIA_ValuePatternId) } {
                        if let Ok(vp) = p.cast::<IUIAutomationValuePattern>() {
                            let read_only = unsafe { vp.CurrentIsReadOnly().unwrap_or(BOOL(1)).as_bool() };
                            if !read_only {
                                let final_text = if mode == "append" {
                                    match unsafe { vp.CurrentValue() } {
                                        Ok(val) => format!("{}{}", val.to_string(), text),
                                        Err(_) => text.to_string(),
                                    }
                                } else { text.to_string() };

                                unsafe { vp.SetValue(&final_text.into())?; }

                                // 简单校验
                                let mut ok = false;
                                for _ in 0..self.config.injection.max_retries.max(1) {
                                    std::thread::sleep(Duration::from_millis(60));
                                    if let Ok(v) = unsafe { vp.CurrentValue() } {
                                        let v = v.to_string();
                                        if mode == "append" { if v.ends_with(text) { ok = true; break; } }
                                        else { if v == text { ok = true; break; } }
                                    }
                                }
                                if ok { return Ok(()); }
                                log::warn!("UIA ValuePattern verification failed; falling back");
                            } else {
                                log::warn!("ValuePattern is read-only; falling back");
                            }
                        }
                    }
                }

                // TextPattern：定位末尾（仅 append），否则保持原位
                if let Ok(p) = unsafe { target_element.GetCurrentPattern(UIA_TextPatternId) } {
                    if let Ok(tp) = p.cast::<IUIAutomationTextPattern>() {
                        if mode == "append" {
                            if let Ok(doc) = unsafe { tp.DocumentRange() } {
                                unsafe {
                                    let _ = doc.MoveEndpointByUnit(TextPatternRangeEndpoint_End, TextUnit_Character, 1_000_000);
                                    let _ = doc.MoveEndpointByUnit(TextPatternRangeEndpoint_Start, TextUnit_Character, 1_000_000);
                                    let _ = doc.Select();
                                }
                            }
                        }
                    }
                }

                if self.config.injection.allow_clipboard {
                    if let Ok(_) = self.inject_via_clipboard(text, context) { return Ok(()); }
                }
                self.type_text_via_sendinput(text)
            }
        }
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
            // 使用if-else结构替代match表达式以避免不可达模式警告
            let priority = if control_type == UIA_DocumentControlTypeId {
                3  // 最高优先级：Document + Pattern
            } else if control_type == UIA_EditControlTypeId {
                2  // 中等优先级：Edit + Pattern
            } else {
                1  // 低优先级：其他 + Pattern
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