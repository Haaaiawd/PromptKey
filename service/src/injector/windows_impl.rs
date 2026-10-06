use super::{InjectionContext, InjectionStrategy, Injector};
use crate::config::Config;
use std::result::Result as StdResult;
use std::time::Duration;
use windows::{
    Win32::Foundation::*, Win32::System::Com::*, Win32::System::DataExchange::*,
    Win32::System::Memory::*, Win32::UI::Accessibility::*,
    Win32::UI::Input::KeyboardAndMouse::*, Win32::UI::WindowsAndMessaging::*,
};

// windows 0.58 下方便使用的常量（CF_UNICODETEXT = 13）
const CF_UNICODETEXT_CONST: u32 = 13;

/// Maximum clipboard size to read (1M UTF-16 chars = 2MB)
/// Prevents potential unsafe memory overflow attacks.
const MAX_CLIPBOARD_SIZE: usize = 1_000_000;

/// Phase 2 Task6: cap for per-format clipboard backup payload
const MAX_BACKUP_FORMAT_BYTES: usize = 16 * 1024 * 1024;
/// Total cap across all formats
const MAX_BACKUP_TOTAL_BYTES: usize = 64 * 1024 * 1024;
/// ES_PASSWORD window style for classic EDIT controls
const ES_PASSWORD: usize = 0x0020;



// EditorType and EditorDetection removed (UIA-specific, no longer used)

/// Phase 2 D1: read current CF_UNICODETEXT clipboard content for {{clipboard}}.
/// Best-effort: returns None if the clipboard can't be opened or has no text.
pub fn clipboard_text() -> Option<String> {
    unsafe {
        if OpenClipboard(HWND(std::ptr::null_mut())).is_err() {
            return None;
        }
        let mut result = None;
        if IsClipboardFormatAvailable(CF_UNICODETEXT_CONST).is_ok() {
            if let Ok(h) = GetClipboardData(CF_UNICODETEXT_CONST) {
                let hg = HGLOBAL(h.0);
                let ptr = GlobalLock(hg) as *const u16;
                if !ptr.is_null() {
                    let mut buf: Vec<u16> = Vec::new();
                    let mut p = ptr;
                    for _ in 0..MAX_CLIPBOARD_SIZE {
                        let ch = *p;
                        if ch == 0 {
                            break;
                        }
                        buf.push(ch);
                        p = p.add(1);
                    }
                    let _ = GlobalUnlock(hg);
                    result = Some(String::from_utf16_lossy(&buf));
                }
            }
        }
        let _ = CloseClipboard();
        result
    }
}

pub struct WindowsInjector {
    config: Config,
}

// describe_element deleted (T0-002 Step 1.2)

/// Phase 2 Task6: snapshot every clipboard format whose data lives in a GMEM
/// block (readable via GlobalLock). Returns `(items, complete)` — `complete`
/// is false when any format had to be skipped (non-GMEM handles like HBITMAP,
/// oversized blocks, owner-display/DSP formats), meaning restoration would
/// permanently lose data. Review F41/F51/F63: callers must treat an
/// incomplete snapshot as a reason not to overwrite the clipboard at all.
fn backup_clipboard_all() -> (Vec<(u32, Vec<u8>)>, bool) {
    let mut out: Vec<(u32, Vec<u8>)> = Vec::new();
    let mut complete = true;
    let mut total = 0usize;
    unsafe {
        if OpenClipboard(HWND(std::ptr::null_mut())).is_err() {
            return (out, false);
        }
        let mut fmt = 0u32;
        loop {
            fmt = EnumClipboardFormats(fmt);
            if fmt == 0 {
                break;
            }
            // owner-display / DSP formats are rendered on demand — cannot snapshot
            if (0x0080..=0x008F).contains(&fmt) {
                complete = false;
                continue;
            }
            match GetClipboardData(fmt) {
                Ok(h) => {
                    let hg = HGLOBAL(h.0);
                    let size = GlobalSize(hg);
                    if size > MAX_BACKUP_FORMAT_BYTES || total + size > MAX_BACKUP_TOTAL_BYTES {
                        complete = false;
                        continue;
                    }
                    if size == 0 {
                        // zero-length GMEM block — snapshot is trivially complete
                        out.push((fmt, Vec::new()));
                        continue;
                    }
                    let ptr = GlobalLock(hg) as *const u8;
                    if ptr.is_null() {
                        complete = false; // not a GMEM block (HBITMAP etc.)
                        continue;
                    }
                    let mut buf = vec![0u8; size];
                    std::ptr::copy_nonoverlapping(ptr, buf.as_mut_ptr(), size);
                    let _ = GlobalUnlock(hg);
                    total += size;
                    out.push((fmt, buf));
                }
                Err(_) => {
                    complete = false;
                }
            }
        }
        let _ = CloseClipboard();
    }
    (out, complete)
}

/// Phase 2 Task6: restore a full backup (as produced by backup_clipboard_all).
fn restore_clipboard_all(items: &[(u32, Vec<u8>)]) {
    unsafe {
        if OpenClipboard(HWND(std::ptr::null_mut())).is_err() {
            return;
        }
        let _ = EmptyClipboard();
        for (fmt, data) in items {
            if let Ok(hmem) = GlobalAlloc(GMEM_MOVEABLE, data.len().max(1)) {
                let ptr = GlobalLock(hmem) as *mut u8;
                if ptr.is_null() {
                    let _ = GlobalFree(hmem);
                    continue;
                }
                std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
                let _ = GlobalUnlock(hmem);
                // SetClipboardData takes ownership of the handle on success
                if SetClipboardData(*fmt, HANDLE(hmem.0)).is_err() {
                    let _ = GlobalFree(hmem);
                }
            }
        }
        let _ = CloseClipboard();
    }
}

/// Phase 2 Task6: is the currently focused control a password/secure field?
/// Two probes, both read-only:
///  1) classic EDIT control with ES_PASSWORD style (GetGUIThreadInfo focus hwnd)
///  2) UIA IsPassword on the focused element (no injection — detection only)
/// Fail-open=false: on probe errors we return false but callers should treat
/// "cannot determine" carefully — we only block on a positive signal.

/// Pairs a successful CoInitialize with CoUninitialize on all exit paths
/// (review F42/F67 — repeated injections leaked apartment refs).
struct ComGuard(bool);
impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

pub fn is_secure_input(target_handle: u64) -> bool {
    unsafe {
        // Find the real focused control within the foreground window's thread
        let target_hwnd = HWND(target_handle as usize as *mut _);
        let tid = GetWindowThreadProcessId(target_hwnd, None);
        let mut gti = GUITHREADINFO::default();
        gti.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
        let focus = if tid != 0 && GetGUIThreadInfo(tid, &mut gti).is_ok() && !gti.hwndFocus.0.is_null() {
            gti.hwndFocus
        } else {
            target_hwnd
        };

        // Probe 1: window class + ES_PASSWORD
        let mut cls = [0u16; 64];
        let n = GetClassNameW(focus, &mut cls);
        if n > 0 {
            let class = String::from_utf16_lossy(&cls[..n as usize]).to_lowercase();
            if class.contains("password") {
                return true;
            }
            if class.contains("edit") || class.contains("input") {
                let style = GetWindowLongPtrW(focus, GWL_STYLE) as usize;
                if style & ES_PASSWORD != 0 {
                    return true;
                }
            }
        }

        // Probe 2: UIA IsPassword (read-only detection; injection path stays
        // clipboard/SendInput — UIA write path is not resurrected).
        // CoInitialize is paired with CoUninitialize via `com_guard`.
        let _com_guard = ComGuard(CoInitialize(None).is_ok());
        if let Ok(uia) = CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_ALL) {
            // Review F44: ask UIA for the *focused element* — browser password
            // inputs have no HWND of their own, so ElementFromHandle(focus)
            // only sees the parent window and misses IsPassword.
            let el = uia
                .GetFocusedElement()
                .or_else(|_| uia.ElementFromHandle(focus));
            if let Ok(el) = el {
                if let Ok(is_pwd) = el.CurrentIsPassword() {
                    if is_pwd.as_bool() {
                        return true;
                    }
                }
            }
        }
        false
    }
}

impl WindowsInjector {
    pub fn new(_strategies: Vec<InjectionStrategy>, config: Config) -> Self {
        log::debug!("Creating injector with config-driven strategies");
        WindowsInjector { config }
    }
}

impl Injector for WindowsInjector {
    fn inject(
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

        // Task6: never inject into password/secure fields (hard boundary)
        if self.config.injection.secure_gate && is_secure_input(context.window_handle) {
            log::warn!("🔒 Injection refused: focused control is a secure/password field");
            return Err("Refused: focused control is a secure field".into());
        }

        let start = std::time::Instant::now();

        // Primary strategy: Clipboard (works in 99% of scenarios)
        if self.config.injection.allow_clipboard {
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
        } else {
            log::info!("Clipboard strategy disabled by config (injection.allow_clipboard=false)");
        }

        // Fallback strategy: SendInput (for apps that block paste)
        self.inject_via_sendinput(text, context)?;
        let elapsed = start.elapsed().as_millis() as u64;
        log::info!("Successfully injected text via SendInput in {}ms", elapsed);
        Ok(("SendInput".to_string(), elapsed))
    }
}

impl WindowsInjector {
    // effective_strategies_for deleted (T0-003 - strategy now hardcoded)

    fn inject_via_clipboard(
        &self,
        text: &str,
        context: &InjectionContext,
    ) -> StdResult<(), Box<dyn std::error::Error>> {
        log::debug!("Attempting clipboard injection");

        // 0) 先将目标窗口置前，确保焦点正确
        unsafe {
            let _ = SetForegroundWindow(HWND(context.window_handle as usize as *mut _));
        }
        std::thread::sleep(Duration::from_millis(
            self.get_pre_inject_delay(&context.app_name),
        ));

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
        unsafe {
            let _ = CloseClipboard();
        }

        // 2) Task6: 备份全部剪贴板格式（不再只是 CF_UNICODETEXT）
        // Review F41/F51/F63: if any format can't be snapshotted (bitmap,
        // oversized, owner-display), restoring would destroy it — refuse the
        // clipboard strategy so the caller falls back to SendInput, which
        // leaves the user's clipboard untouched.
        let (backup, backup_complete) = backup_clipboard_all();
        if self.config.injection.restore_clipboard && !backup_complete {
            log::warn!("Clipboard contains unsnapshotable formats; skipping clipboard strategy");
            return Err("clipboard backup incomplete".into());
        }

        // 3) 设置我们的文本到剪贴板
        unsafe {
            if OpenClipboard(HWND(std::ptr::null_mut())).is_err() {
                return Err("OpenClipboard failed (write)".into());
            }
            let _ = EmptyClipboard();
            let mut utf16: Vec<u16> = text.encode_utf16().collect();
            utf16.push(0);
            let bytes = (utf16.len() * std::mem::size_of::<u16>()) as usize;
            let hmem = match GlobalAlloc(GMEM_MOVEABLE, bytes) {
                Ok(h) => h,
                Err(_) => {
                    let _ = CloseClipboard();
                    return Err("GlobalAlloc failed".into());
                }
            };
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
        //    (Task6: 150ms，缩短剪贴板污染窗口)
        std::thread::sleep(Duration::from_millis(150));
        let pasted = unsafe {
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
            SendInput(&mut inputs, std::mem::size_of::<INPUT>() as i32) != 0
        };

        // 5) 粘贴后稍等再恢复剪贴板（80ms，确保目标应用已读取）
        std::thread::sleep(Duration::from_millis(80));

        // Task6: 恢复全部已备份格式（受 injection.restore_clipboard 开关控制）
        // Review F40/F62: restore on EVERY path after we overwrote the
        // clipboard — including SendInput failure. An originally empty
        // clipboard restores as empty (clears our prompt text).
        if self.config.injection.restore_clipboard {
            restore_clipboard_all(&backup);
        }

        // Review F40/F62: report paste failure only after the user's
        // clipboard has been restored.
        if !pasted {
            return Err("SendInput Ctrl+V failed".into());
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
            let _ = SetForegroundWindow(HWND(context.window_handle as usize as *mut _));
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
