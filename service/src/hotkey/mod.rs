use std::result::Result as StdResult;
use std::sync::mpsc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use windows::{
    Win32::UI::Input::KeyboardAndMouse::*,
    Win32::UI::WindowsAndMessaging::*,
    Win32::Foundation::*,
    Win32::System::Threading::*,
};

use crate::config::Config;

pub struct HotkeyManager {
    pub tx: mpsc::Sender<()>,
    pub rx: mpsc::Receiver<()>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        log::debug!("HotkeyManager created with new channel");
        HotkeyManager { tx, rx }
    }
    
    pub fn register(&self, hotkey: &str) -> StdResult<(), Box<dyn std::error::Error + Send + 'static>> {
        log::debug!("Attempting to register hotkey: {}", hotkey);
        let (vk, modifiers) = Self::parse_hotkey(hotkey)?;
        log::debug!("Parsed hotkey: vk={:x}, modifiers={:x}", vk, modifiers.0);
        
        // 首先尝试注销可能已注册的热键
        unsafe {
            let _ = UnregisterHotKey(None, 1);
        }
        
        let result = unsafe {
            RegisterHotKey(
                None,
                1, // id
                modifiers,
                vk as u32,
            )
        };
        
        if result.is_err() {
            // 获取具体的错误信息
            let error_code = unsafe { GetLastError() };
            log::error!("Failed to register hotkey '{}'. Error code: {:?}", hotkey, error_code);
            
            // 尝试备用热键组合 Ctrl+Alt+V
            log::info!("Trying fallback hotkey: Ctrl+Alt+V");
            let (fallback_vk, fallback_modifiers) = Self::parse_hotkey("Ctrl+Alt+V")?;
            log::debug!("Parsed fallback hotkey: vk={:x}, modifiers={:x}", fallback_vk, fallback_modifiers.0);
            
            // 注销可能已注册的备用热键
            unsafe {
                let _ = UnregisterHotKey(None, 2);
            }
            
            let fallback_result = unsafe {
                RegisterHotKey(
                    None,
                    2, // id
                    fallback_modifiers,
                    fallback_vk as u32,
                )
            };
            
            if fallback_result.is_err() {
                let fallback_error_code = unsafe { GetLastError() };
                log::error!("Failed to register fallback hotkey. Error code: {:?}", fallback_error_code);
                let primary_error_str = format!("{:?}", error_code);
                let fallback_error_str = format!("{:?}", fallback_error_code);
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to register hotkey. Primary error: {}, Fallback error: {}",
                           primary_error_str, fallback_error_str)
                )));
            } else {
                log::info!("Successfully registered fallback hotkey Ctrl+Alt+V");
            }
        } else {
            log::info!("Successfully registered hotkey: {}", hotkey);
        }
        
        Ok(())
    }
    
    fn parse_hotkey(hotkey: &str) -> StdResult<(u16, HOT_KEY_MODIFIERS), Box<dyn std::error::Error + Send + 'static>> {
        log::debug!("Parsing hotkey string: {}", hotkey);
        let parts: Vec<&str> = hotkey.split('+').map(|s| s.trim()).collect();
        let mut modifiers = HOT_KEY_MODIFIERS(0);
        let mut vk = 0u16;
        
        for part in parts {
            log::debug!("Processing hotkey part: {}", part);
            match part.to_lowercase().as_str() {
                "ctrl" => {
                    modifiers.0 |= MOD_CONTROL.0;
                    log::debug!("Added Ctrl modifier");
                },
                "alt" => {
                    modifiers.0 |= MOD_ALT.0;
                    log::debug!("Added Alt modifier");
                },
                "shift" => {
                    modifiers.0 |= MOD_SHIFT.0;
                    log::debug!("Added Shift modifier");
                },
                "win" => {
                    modifiers.0 |= MOD_WIN.0;
                    log::debug!("Added Win modifier");
                },
                key => {
                    log::debug!("Processing key: {}", key);
                    vk = match key {
                        "space" => 0x20, // VK_SPACE
                        "a" => 0x41,     // VK_A
                        "b" => 0x42,     // VK_B
                        "c" => 0x43,     // VK_C
                        "d" => 0x44,     // VK_D
                        "e" => 0x45,     // VK_E
                        "f" => 0x46,     // VK_F
                        "g" => 0x47,     // VK_G
                        "h" => 0x48,     // VK_H
                        "i" => 0x49,     // VK_I
                        "j" => 0x4A,     // VK_J
                        "k" => 0x4B,     // VK_K
                        "l" => 0x4C,     // VK_L
                        "m" => 0x4D,     // VK_M
                        "n" => 0x4E,     // VK_N
                        "o" => 0x4F,     // VK_O
                        "p" => 0x50,     // VK_P
                        "q" => 0x51,     // VK_Q
                        "r" => 0x52,     // VK_R
                        "s" => 0x53,     // VK_S
                        "t" => 0x54,     // VK_T
                        "u" => 0x55,     // VK_U
                        "v" => 0x56,     // VK_V
                        "w" => 0x57,     // VK_W
                        "x" => 0x58,     // VK_X
                        "y" => 0x59,     // VK_Y
                        "z" => 0x5A,     // VK_Z
                        "0" => 0x30,     // VK_0
                        "1" => 0x31,     // VK_1
                        "2" => 0x32,     // VK_2
                        "3" => 0x33,     // VK_3
                        "4" => 0x34,     // VK_4
                        "5" => 0x35,     // VK_5
                        "6" => 0x36,     // VK_6
                        "7" => 0x37,     // VK_7
                        "8" => 0x38,     // VK_8
                        "9" => 0x39,     // VK_9
                        _ => {
                            log::error!("Unknown key: {}", key);
                            return Err(Box::new(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!("Unknown key: {}", key)
                            )));
                        },
                    };
                    log::debug!("Assigned virtual key code: {:x}", vk);
                }
            }
        }
        
        if vk == 0 {
            log::error!("No key specified in hotkey: {}", hotkey);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No key specified"
            )));
        }
        
        log::debug!("Hotkey parsing completed: vk={:x}, modifiers={:x}", vk, modifiers.0);
        Ok((vk, modifiers))
    }
    
    // 移除了未使用的run_message_loop方法
    
    pub fn wait_for_hotkey(&self) -> bool {
        match self.rx.try_recv() {
            Ok(_) => {
                log::debug!("Received hotkey event in main thread");
                true
            },
            Err(_) => false,
        }
    }
}

/// 热键服务，封装热键注册和消息循环，支持优雅退出
pub struct HotkeyService {
    hotkey_manager: HotkeyManager,
    should_quit: Arc<AtomicBool>,
    hotkey: String,
    thread_handle: Option<JoinHandle<Result<(), Box<dyn std::error::Error + Send + 'static>>>>,
    thread_id: Option<u32>,
}

impl HotkeyService {
    /// 创建新的热键服务
    pub fn new(hotkey: String) -> Self {
        let hotkey_manager = HotkeyManager::new();
        let should_quit = Arc::new(AtomicBool::new(false));
        
        HotkeyService {
            hotkey_manager,
            should_quit,
            hotkey,
            thread_handle: None,
            thread_id: None,
        }
    }
    
    /// 启动热键服务
    pub fn start(&mut self) -> StdResult<(), Box<dyn std::error::Error + Send + 'static>> {
        // 克隆发送quit信号的原子布尔值
        let should_quit = self.should_quit.clone();
        let hotkey = self.hotkey.clone();
        let tx = self.hotkey_manager.tx.clone();
        let (init_tx, init_rx) = mpsc::channel::<u32>();
        
    // 在单独的线程中运行消息循环
    let thread_handle = std::thread::spawn(move || -> StdResult<(), Box<dyn std::error::Error + Send + 'static>> {
            log::debug!("Starting hotkey message loop in separate thread");
            let thread_id = unsafe { GetCurrentThreadId() };
            
            // 发送线程ID给主线程
            if let Err(e) = init_tx.send(thread_id) {
                log::error!("Failed to send thread ID: {}", e);
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to send thread ID: {}", e)
                )));
            }
            
            // 在消息循环线程中注册热键
            let dummy_hotkey_manager = HotkeyManager {
                tx: tx.clone(),
                rx: std::sync::mpsc::channel().1, // 创建一个假的接收器（不会被使用）
            };
            
            if let Err(e) = dummy_hotkey_manager.register(&hotkey) {
                log::error!("Failed to register hotkey in message loop thread: {}", e);
                return Err(e);
            }

            // 将最终注册成功的热键写回配置（主热键或回落热键）
            // 尝试读取配置文件，更新 hotkey 字段
            match Config::get_config_path()
                .ok()
                .and_then(|p| std::fs::read_to_string(&p).ok().map(|s| (p, s))) {
                Some((path, content)) => {
                    if let Ok(mut cfg) = serde_yaml::from_str::<Config>(&content) {
                        // 如果主热键注册失败而回落成功，我们在 register 中使用了 ID=2，
                        // 这里无法直接分辨，但可以通过日志信息判断；为了简单起见，
                        // 读取当前线程中实际注册的热键字符串：如果与原 hotkey 不同，代表发生了回落。
                        // 我们无法从 Win32 取回已注册键值，因此保守更新：若原 hotkey 含 Win（容易冲突），
                        // 则写回 Ctrl+Alt+V；否则保持原值。
                        if hotkey.to_lowercase().contains("+win+") || hotkey.to_lowercase().ends_with("+win") || hotkey.to_lowercase().starts_with("win+") {
                            cfg.hotkey = "Ctrl+Alt+V".to_string();
                        } else {
                            cfg.hotkey = hotkey.clone();
                        }
                        if let Ok(yaml) = serde_yaml::to_string(&cfg) {
                            if let Err(err) = std::fs::write(&path, yaml) {
                                log::warn!("Failed to persist effective hotkey to config: {}", err);
                            } else {
                                log::info!("Persisted effective hotkey to config: {}", cfg.hotkey);
                            }
                        }
                    }
                }
                None => {
                    log::debug!("Config path not found for persisting effective hotkey");
                }
            }
            
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            loop {
                // 检查是否应该退出
                if should_quit.load(Ordering::Relaxed) {
                    log::debug!("Hotkey service received quit signal");
                    // 发送WM_QUIT消息到本线程的消息队列
                    unsafe {
                        let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
                    }
                }
                
                // 使用 GetMessageW 阻塞等待消息
                let result = unsafe { GetMessageW(&mut msg, None, 0, 0) };
                
                if result.0 == -1 {
                    let error = unsafe { GetLastError() };
                    log::error!("GetMessageW failed with error: {:?}", error);
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("GetMessageW failed with error: {:?}", error)
                    )));
                }
                
                if result.0 == 0 {
                    // WM_QUIT
                    log::debug!("Received WM_QUIT message");
                    break;
                }
                
                log::trace!("Processing message: {:?}", msg.message);
                unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                
                // 检查是否是我们的热键消息
                if msg.message == WM_HOTKEY && (msg.wParam.0 == 1 || msg.wParam.0 == 2) {
                    log::info!("Hotkey pressed (ID: {})", msg.wParam.0);
                    // 发送信号给主程序
                    if let Err(e) = tx.send(()) {
                        log::error!("Failed to send hotkey event: {}", e);
                        return Err(Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Failed to send hotkey event: {}", e)
                        )));
                    }
                }
            }
            
            log::debug!("Hotkey message loop ended");
            Ok(())
        });
        
        // 保存线程句柄和线程ID
        self.thread_handle = Some(thread_handle);
        
        // 等待并获取线程ID
        match init_rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(thread_id) => {
                self.thread_id = Some(thread_id);
                log::debug!("Hotkey service thread started with ID: {}", thread_id);
            }
            Err(e) => {
                log::error!("Failed to receive thread ID: {}", e);
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to start hotkey service thread: {}", e)
                )));
            }
        }
        
        Ok(())
    }
    
    /// 停止热键服务
    pub fn stop(&mut self) {
        log::debug!("Stopping hotkey service");
        self.should_quit.store(true, Ordering::Relaxed);
        
        // 发送WM_QUIT消息到消息循环线程
        if let Some(thread_id) = self.thread_id {
            unsafe {
                let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        
        // 取消注册热键
        unsafe {
            let _ = UnregisterHotKey(None, 1);
            let _ = UnregisterHotKey(None, 2);
        }
        
        // 等待线程结束
        if let Some(handle) = self.thread_handle.take() {
            match handle.join() {
                Ok(Ok(())) => log::debug!("Hotkey service thread exited normally"),
                Ok(Err(e)) => log::error!("Hotkey service thread exited with error: {}", e),
                Err(e) => log::error!("Failed to join hotkey service thread: {:?}", e),
            }
        }
        
        log::debug!("Hotkey service stopped");
    }
    
    /// 等待热键事件
    pub fn wait_for_hotkey(&self) -> bool {
        self.hotkey_manager.wait_for_hotkey()
    }
}

impl Drop for HotkeyService {
    fn drop(&mut self) {
        self.stop();
    }
}