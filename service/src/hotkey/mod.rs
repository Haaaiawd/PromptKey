use std::result::Result as StdResult;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;
use windows::{
    Win32::Foundation::*, Win32::System::Threading::*, Win32::UI::Input::KeyboardAndMouse::*,
    Win32::UI::WindowsAndMessaging::*,
};

/// 热键管理器
pub struct HotkeyManager {
    pub tx: mpsc::Sender<u32>,
    pub rx: mpsc::Receiver<u32>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        HotkeyManager { tx, rx }
    }

    /// 等待热键事件（阻塞模式）
    pub fn wait_for_hotkey(&self) -> Option<u32> {
        self.rx.recv().ok()
    }

    /// 非阻塞检查热键事件
    pub fn try_wait_for_hotkey(&self) -> Option<u32> {
        match self.rx.try_recv() {
            Ok(id) => Some(id),
            _ => None,
        }
    }

    /// 注册快捷键
    pub fn register(&self, id: u32, hotkey_str: &str) -> StdResult<(), String> {
        let (vk, modifiers) = self.parse_hotkey(hotkey_str)?;
        unsafe {
            RegisterHotKey(None, id as i32, modifiers, vk.0 as u32)
                .map_err(|e| format!("无法注册热键 {}: {}", hotkey_str, e))?;
        }
        Ok(())
    }

    /// 解析热键字符串
    fn parse_hotkey(
        &self,
        hotkey_str: &str,
    ) -> StdResult<(VIRTUAL_KEY, HOT_KEY_MODIFIERS), String> {
        let parts: Vec<&str> = hotkey_str.split('+').map(|s| s.trim()).collect();
        let mut modifiers = HOT_KEY_MODIFIERS(0);
        let mut vk = VIRTUAL_KEY(0);

        for part in parts {
            match part.to_uppercase().as_str() {
                "CTRL" => modifiers |= MOD_CONTROL,
                "ALT" => modifiers |= MOD_ALT,
                "SHIFT" => modifiers |= MOD_SHIFT,
                "WIN" => modifiers |= MOD_WIN,
                "SPACE" => vk = VK_SPACE,
                "ENTER" => vk = VK_RETURN,
                "Q" => vk = VIRTUAL_KEY(b'Q' as u16),
                "W" => vk = VIRTUAL_KEY(b'W' as u16),
                "H" => vk = VIRTUAL_KEY(b'H' as u16),
                s if s.len() == 1 => {
                    vk = VIRTUAL_KEY(s.as_bytes()[0] as u16);
                }
                _ => return Err(format!("不受支持的按键零件: {}", part)),
            }
        }
        Ok((vk, modifiers))
    }
}

/// 热键服务，负责在一个独立的 Windows 消息循环中处理热键
pub struct HotkeyService {
    hotkey_manager: HotkeyManager,
    should_quit: Arc<AtomicBool>,
    hotkey: String,
    thread_handle: Option<JoinHandle<StdResult<(), Box<dyn std::error::Error + Send + 'static>>>>,
}

impl HotkeyService {
    pub fn new(hotkey: String) -> Self {
        HotkeyService {
            hotkey_manager: HotkeyManager::new(),
            should_quit: Arc::new(AtomicBool::new(false)),
            hotkey,
            thread_handle: None,
        }
    }

    pub fn start(&mut self) -> StdResult<(), Box<dyn std::error::Error + Send + 'static>> {
        let should_quit = self.should_quit.clone();
        let hotkey_str = self.hotkey.clone();
        let tx = self.hotkey_manager.tx.clone();

        let handle = std::thread::spawn(
            move || -> StdResult<(), Box<dyn std::error::Error + Send + 'static>> {
                let manager = HotkeyManager {
                    tx: tx.clone(),
                    rx: mpsc::channel().1,
                }; // dummy rx

                // 注册主注入热键
                if let Err(e) = manager.register(1, &hotkey_str) {
                    log::error!("注册主热键失败: {}", e);
                } else {
                    log::info!("✅ 主注入热键注册成功: {}", hotkey_str);
                }

                // 注册选择器热键 (Ctrl+Shift+H)
                if let Err(e) = manager.register(3, "Ctrl+Shift+H") {
                    log::error!("注册选择器热键失败: {}", e);
                }

                // 注册轮盘热键 (Ctrl+Alt+Q)
                if let Err(e) = manager.register(4, "Ctrl+Alt+Q") {
                    log::error!("注册轮盘热键失败: {}", e);
                } else {
                    println!("✅ [HOTKEY] 轮盘触发热键已注册: Ctrl+Alt+Q");
                }

                let mut msg = MSG::default();
                while !should_quit.load(Ordering::Relaxed) {
                    unsafe {
                        if PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                            if msg.message == WM_HOTKEY {
                                let _ = tx.send(msg.wParam.0 as u32);
                            }
                            TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Ok(())
            },
        );

        self.thread_handle = Some(handle);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.should_quit.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn wait_for_hotkey(&self) -> Option<u32> {
        self.hotkey_manager.wait_for_hotkey()
    }

    pub fn try_wait_for_hotkey(&self) -> Option<u32> {
        self.hotkey_manager.try_wait_for_hotkey()
    }
}

impl Drop for HotkeyService {
    fn drop(&mut self) {
        self.stop();
    }
}
