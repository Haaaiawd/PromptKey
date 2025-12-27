use crate::config::Config;
use std::result::Result as StdResult;
use std::time::Duration;
use windows::{
    Win32::Foundation::*, Win32::System::Com::*, Win32::System::DataExchange::*,
    Win32::System::Memory::*, Win32::UI::Accessibility::*, Win32::UI::Input::KeyboardAndMouse::*,
    Win32::UI::WindowsAndMessaging::*, core::*,
};

// windows 0.58 下方便使用的常量（CF_UNICODETEXT = 13）
const CF_UNICODETEXT_CONST: u32 = 13;

#[derive(Debug)]
pub struct InjectionContext {
    pub app_name: String,
    pub window_title: String,
    pub window_handle: HWND,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectionStrategy {
    UIA,
    Clipboard,
    SendInput,
}

#[derive(Debug, Clone)]
pub enum EditorType {
    Generic,
    Scintilla, // Notepad++
    Electron,  // VS Code, Atom
    WPF,       // Visual Studio
    Swing,     // IntelliJ IDEA, Eclipse
}

#[derive(Debug, Clone)]
pub struct EditorDetection {
    pub editor_type: EditorType,
    pub process_name: String,
}

pub struct Injector {
    config: Config,
}

// describe_element deleted (T0-002 Step 1.2)

impl Injector {
    pub fn new(_strategies: Vec<InjectionStrategy>, config: Config) -> Self {
        log::debug!("Creating injector with config-driven strategies");
        Injector { config }
    }

    pub fn inject(
        &self,
        text: &str,
        context: &InjectionContext,
    ) -> StdResult<(String, u64), Box<dyn std::error::Error>> {
        log::info!("Attempting to inject text using available strategies");
        log::debug!("Text to inject: {}", text);
        log::debug!(
            "Context: app_name={}, window_title={}",
            context.app_name,
            context.window_title
        );
        log::debug!(
            "Config: order={:?}, allow_clipboard={}, mode={}",
            self.config.injection.order,
            self.config.injection.allow_clipboard,
            self.config.injection.uia_value_pattern_mode
        );
        let effective = self.effective_strategies_for(&context.app_name);
        log::debug!("Effective strategy order: {:?}", effective);

        for strategy in &effective {
            log::debug!("Trying injection strategy: {:?}", strategy);

            // 测量实际注入操作的时间
            let start = std::time::Instant::now();
            let result = match strategy {
                InjectionStrategy::UIA => self.inject_via_uia(text, context),
                InjectionStrategy::Clipboard => self.inject_via_clipboard(text, context),
                InjectionStrategy::SendInput => self.inject_via_sendinput(text, context),
            };
            let injection_time = start.elapsed().as_millis() as u64;

            match result {
                Ok(_) => {
                    let strategy_name = match strategy {
                        InjectionStrategy::UIA => "UIA",
                        InjectionStrategy::Clipboard => "Clipboard",
                        InjectionStrategy::SendInput => "SendInput",
                    };
                    log::info!(
                        "Successfully injected text using {:?} strategy in {}ms",
                        strategy,
                        injection_time
                    );
                    return Ok((strategy_name.to_string(), injection_time));
                }
                Err(e) => {
                    log::warn!(
                        "Failed to inject using {:?} strategy in {}ms: {}",
                        strategy,
                        injection_time,
                        e
                    );
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
            for f in app_cfg.strategies.fallback {
                order.push(f.to_lowercase());
            }
        } else {
            order = self
                .config
                .injection
                .order
                .iter()
                .map(|s| s.to_lowercase())
                .collect();
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
                _ => {
                    log::debug!("Unknown strategy '{}' ignored", s);
                    None
                }
            };
            if let Some(v) = variant {
                let name = match v {
                    InjectionStrategy::UIA => "uia",
                    InjectionStrategy::Clipboard => "clipboard",
                    InjectionStrategy::SendInput => "sendinput",
                };
                if seen.insert(name.to_string()) {
                    mapped.push(v);
                }
            }
        }
        if mapped.is_empty() {
            mapped.push(InjectionStrategy::UIA);
        }
        mapped
    }


    fn inject_via_clipboard(
        &self,
        text: &str,
        _context: &InjectionContext,
    ) -> StdResult<(), Box<dyn std::error::Error>> {
        log::debug!("Attempting clipboard injection");

        // 1) 打开剪贴板，最多尝试 5 次
        let mut opened = false;
        for _ in 0..5 {
            unsafe {
                if OpenClipboard(HWND(std::ptr::null_mut())).is_ok() {
                    opened = true;
                }
            }
            if opened {
                break;
            }
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
                            if ch == 0 {
                                break;
                            }
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
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_CONTROL,
                            wScan: VIRTUAL_KEY(0).0 as u16,
                            dwFlags: KEYBD_EVENT_FLAGS(0),
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0x56),
                            wScan: 0,
                            dwFlags: KEYBD_EVENT_FLAGS(0),
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0x56),
                            wScan: 0,
                            dwFlags: KEYBD_EVENT_FLAGS(KEYEVENTF_KEYUP.0),
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_CONTROL,
                            wScan: VIRTUAL_KEY(0).0 as u16,
                            dwFlags: KEYBD_EVENT_FLAGS(KEYEVENTF_KEYUP.0),
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
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
                    let hmem =
                        GlobalAlloc(GMEM_MOVEABLE, bytes).map_err(|_| "GlobalAlloc failed")?;
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

    fn inject_via_sendinput(
        &self,
        text: &str,
        context: &InjectionContext,
    ) -> StdResult<(), Box<dyn std::error::Error>> {
        log::debug!("Attempting SendInput injection");

        // 将目标窗口置前
        unsafe {
            let _ = SetForegroundWindow(context.window_handle);
        }

        // 等待焦点稳定
        std::thread::sleep(Duration::from_millis(
            self.get_pre_inject_delay(&context.app_name),
        ));

        // 直接使用 SendInput 模拟输入
        self.type_text_via_sendinput(text)
    }

    fn get_pre_inject_delay(&self, app_name: &str) -> u64 {
        let app_config = self.config.get_app_config(app_name);
        app_config.settings.pre_inject_delay
    }



    fn type_text_via_sendinput(&self, text: &str) -> StdResult<(), Box<dyn std::error::Error>> {
        log::debug!("Using SendInput to simulate typing: '{}'", text);
        // 小延时，避免与热键修饰键冲突或焦点切换未完成
        std::thread::sleep(Duration::from_millis(80));
        unsafe {
            for ch in text.encode_utf16() {
                let mut inputs = [
                    INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wVk: VIRTUAL_KEY(0),
                                wScan: ch,
                                dwFlags: KEYBD_EVENT_FLAGS(KEYEVENTF_UNICODE.0),
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    },
                    INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wVk: VIRTUAL_KEY(0),
                                wScan: ch,
                                dwFlags: KEYBD_EVENT_FLAGS(KEYEVENTF_UNICODE.0 | KEYEVENTF_KEYUP.0),
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    },
                ];
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

// find_editable_element deleted (T0-002)
