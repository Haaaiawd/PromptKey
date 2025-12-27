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

/// Maximum clipboard size to read (1M UTF-16 chars = 2MB)
/// Prevents potential unsafe memory overflow attacks.
const MAX_CLIPBOARD_SIZE: usize = 1_000_000;

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
        log::info!("Injecting text using simplified strategy (Clipboard → SendInput)");
        log::debug!(
            "Text length: {}, app: {}, window_title: {}",
            text.len(),
            context.app_name,
            context.window_title
        );

        let start = std::time::Instant::now();

        // Primary strategy: Clipboard (works in 99% of scenarios)
        match self.inject_via_clipboard(text, context) {
            Ok(_) => {
                let elapsed = start.elapsed().as_millis() as u64;
                log::info!("Successfully injected text via Clipboard in {}ms", elapsed);
                return Ok(("Clipboard".to_string(), elapsed));
            }
            Err(e) => {
                log::warn!(
                    "Clipboard injection failed: {}. Falling back to SendInput",
                    e
                );
            }
        }

        // Fallback strategy: SendInput (for apps that block paste)
        self.inject_via_sendinput(text, context)?;
        let elapsed = start.elapsed().as_millis() as u64;
        log::info!("Successfully injected text via SendInput in {}ms", elapsed);
        Ok(("SendInput".to_string(), elapsed))
    }

    // effective_strategies_for deleted (T0-003 - strategy now hardcoded)

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
                        let mut len = 0usize;
                        loop {
                            if len >= MAX_CLIPBOARD_SIZE {
                                log::warn!(
                                    "Clipboard backup exceeds max size ({}), truncating",
                                    MAX_CLIPBOARD_SIZE
                                );
                                break;
                            }
                            let ch = *p;
                            v.push(ch);
                            if ch == 0 {
                                break;
                            }
                            p = p.add(1);
                            len += 1;
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
