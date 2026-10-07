use super::{Hotkey, ParsedHotkey, Probe, MOD_NOREPEAT_BIT};
use std::result::Result as StdResult;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;
use windows::{Win32::UI::Input::KeyboardAndMouse::*, Win32::UI::WindowsAndMessaging::*};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;

/// Posted to the worker's thread queue to wake its blocking GetMessageW on
/// shutdown. WM_APP-based so it can't collide with real input messages.
const WM_HOTKEY_QUIT: u32 = WM_APP + 0x4B;

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

    /// 注册快捷键（在同一线程里由消息循环收发 WM_HOTKEY）
    pub fn register(&self, id: u32, parsed: &ParsedHotkey) -> StdResult<(), String> {
        // MOD_NOREPEAT: one WM_HOTKEY per press, not per auto-repeat tick.
        let modifiers = HOT_KEY_MODIFIERS(parsed.modifiers | MOD_NOREPEAT_BIT);
        unsafe {
            RegisterHotKey(None, id as i32, modifiers, parsed.vk)
                .map_err(|e| format!("RegisterHotKey failed for {}: {}", parsed.canonical, e))?;
        }
        Ok(())
    }
}

/// 热键服务，负责在一个独立的 Windows 消息循环中处理热键
pub struct WindowsHotkey {
    hotkey_manager: HotkeyManager,
    should_quit: Arc<AtomicBool>,
    /// The message-loop thread's Win32 thread id — published by the worker once
    /// its queue exists, so stop() can PostThreadMessageW a wake-up.
    worker_tid: Arc<AtomicU32>,
    hotkey: String,
    quick_hotkey: String,
    thread_handle: Option<JoinHandle<StdResult<(), Box<dyn std::error::Error + Send + 'static>>>>,
}

impl WindowsHotkey {
    /// `hotkey` opens the wheel (id 4); `quick_hotkey` injects the default prompt (id 5)
    pub fn new(hotkey: String, quick_hotkey: String) -> Self {
        WindowsHotkey {
            hotkey_manager: HotkeyManager::new(),
            should_quit: Arc::new(AtomicBool::new(false)),
            worker_tid: Arc::new(AtomicU32::new(0)),
            hotkey,
            quick_hotkey,
            thread_handle: None,
        }
    }

}

/// Try to grab `combo` on the calling thread and immediately release it —
/// a pure availability probe used by `check_hotkeys` (brief Task 3).
pub fn probe(vk: u32, modifiers: u32) -> Probe {
    const PROBE_ID: i32 = 0x4B00; // far from the real ids 4/5
    unsafe {
        // Ensure the calling thread has a message queue before a thread-scoped
        // RegisterHotKey — cheap no-op if one already exists.
        let mut m = MSG::default();
        let _ = PeekMessageW(&mut m, None, 0, 0, PM_NOREMOVE);
        match RegisterHotKey(None, PROBE_ID, HOT_KEY_MODIFIERS(modifiers | MOD_NOREPEAT_BIT), vk) {
            Ok(()) => {
                let _ = UnregisterHotKey(None, PROBE_ID);
                Probe::Free
            }
            Err(e) => {
                use windows::Win32::Foundation::{ERROR_HOTKEY_ALREADY_REGISTERED, WIN32_ERROR};
                if WIN32_ERROR::from_error(&e) == Some(ERROR_HOTKEY_ALREADY_REGISTERED) {
                    Probe::Busy
                } else {
                    Probe::Failed(e.to_string())
                }
            }
        }
    }
}

impl Hotkey for WindowsHotkey {
    fn start(&mut self) -> StdResult<(), Box<dyn std::error::Error + Send + 'static>> {
        // A stop() followed by start() must actually work — reset the flags the
        // previous worker left behind (review: restart-kills-hotkeys bug).
        self.should_quit.store(false, Ordering::SeqCst);
        self.worker_tid.store(0, Ordering::SeqCst);
        let should_quit = self.should_quit.clone();
        let worker_tid = self.worker_tid.clone();
        let hotkey_str = self.hotkey.clone();
        let quick_hotkey_str = self.quick_hotkey.clone();
        let tx = self.hotkey_manager.tx.clone();

        let handle = std::thread::spawn(
            move || -> StdResult<(), Box<dyn std::error::Error + Send + 'static>> {
                // Force the thread message queue to exist BEFORE publishing our
                // tid, so a stop() that posts WM_HOTKEY_QUIT can never race an
                // unborn queue (PostThreadMessage fails on queue-less threads).
                let mut msg = MSG::default();
                unsafe {
                    let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
                }
                worker_tid.store(unsafe { GetCurrentThreadId() }, Ordering::Release);

                let manager = HotkeyManager {
                    tx: tx.clone(),
                    rx: mpsc::channel().1,
                }; // dummy rx — the real receiver lives on the struct

                // ID 1 (Inject) and ID 3 (Selector) removed.

                // Register the wheel hotkey; record the outcome for check_hotkeys
                // so a failure is user-visible instead of a buried log line.
                let wheel_parsed = super::parse_hotkey(&hotkey_str);
                let wheel = wheel_parsed
                    .as_ref()
                    .map_err(|e| e.clone())
                    .and_then(|p| manager.register(4, p));
                match &wheel {
                    Ok(()) => println!("✅ [HOTKEY] 轮盘触发热键已注册: {}", hotkey_str),
                    Err(e) => log::error!("注册轮盘热键失败: {}", e),
                }
                super::record_registration(4, &hotkey_str, wheel.map_err(|e| e.to_string()));

                // Phase 2 N4: "default prompt direct inject" hotkey.
                // Compare PARSED combos — "ctrl+alt+q" and "Ctrl+Alt+Q" are the
                // same registration and would collide if compared as strings.
                if !quick_hotkey_str.is_empty() {
                    let quick_parsed = super::parse_hotkey(&quick_hotkey_str);
                    let same_as_wheel = matches!(
                        (&quick_parsed, &wheel_parsed),
                        (Ok(q), Ok(w)) if q.vk == w.vk && q.modifiers == w.modifiers
                    );
                    let quick = if same_as_wheel {
                        println!("ℹ️ [HOTKEY] 快捷热键与轮盘热键相同，id5 复用: {}", quick_hotkey_str);
                        Ok(())
                    } else {
                        quick_parsed.and_then(|p| manager.register(5, &p))
                    };
                    match &quick {
                        Ok(()) => println!("✅ [HOTKEY] 默认提示词热键已注册/复用: {}", quick_hotkey_str),
                        Err(e) => log::error!("注册快捷热键失败: {}", e),
                    }
                    super::record_registration(5, &quick_hotkey_str, quick.map_err(|e| e.to_string()));
                }

                // Blocking pump: WM_HOTKEY arrives on this thread's queue
                // (RegisterHotKey was called with hwnd=NULL). GetMessageW sleeps
                // the thread instead of polling — no 10ms granularity, no spin.
                // Shutdown comes from either the quit flag (seen between
                // messages) or the WM_HOTKEY_QUIT wake posted by stop().
                loop {
                    if should_quit.load(Ordering::Acquire) {
                        break;
                    }
                    let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
                    if ret.0 <= 0 {
                        break; // 0 = WM_QUIT, -1 = error
                    }
                    if msg.message == WM_HOTKEY {
                        let _ = tx.send(msg.wParam.0 as u32);
                    } else if msg.message == WM_HOTKEY_QUIT {
                        break;
                    }
                    unsafe {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
                // Review F01/F28/F50/F64: release registrations before exit so a
                // restarted engine can't fight this worker for the same hotkeys.
                unsafe {
                    let _ = UnregisterHotKey(None, 4);
                    let _ = UnregisterHotKey(None, 5);
                }
                super::clear_registrations();
                Ok(())
            },
        );

        self.thread_handle = Some(handle);
        Ok(())
    }

    fn stop(&mut self) {
        self.should_quit.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            // Wait (bounded) for the worker to publish its tid; the publish
            // happens after the message queue exists, so the post below always
            // lands on a live queue. If the worker died before publishing,
            // join() returns immediately on the finished handle.
            for _ in 0..400 {
                if self.worker_tid.load(Ordering::Acquire) != 0 || handle.is_finished() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            let tid = self.worker_tid.load(Ordering::Acquire);
            if tid != 0 {
                unsafe {
                    // If the worker is parked in GetMessageW this wakes it; if
                    // it already exited the post fails harmlessly.
                    let _ = PostThreadMessageW(tid, WM_HOTKEY_QUIT, WPARAM(0), LPARAM(0));
                }
            }
            // If the worker starts after our wait (never published tid) it still
            // exits: the flag check runs before the first blocking GetMessageW.
            let _ = handle.join();
            self.worker_tid.store(0, Ordering::SeqCst);
        }
    }

    fn wait_for_hotkey(&self) -> Option<u32> {
        self.hotkey_manager.wait_for_hotkey()
    }

    fn try_wait_for_hotkey(&self) -> Option<u32> {
        self.hotkey_manager.try_wait_for_hotkey()
    }
}

impl Drop for WindowsHotkey {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Windows smoke test (brief Task 3): a real RegisterHotKey round-trip on a
    /// dedicated message-queue thread, using an exotic combo nobody holds.
    /// Verifies the whole register → unregister path the fix relies on.
    #[test]
    fn probe_and_register_roundtrip() {
        std::thread::spawn(|| {
            // prime the thread's message queue
            unsafe {
                let mut m = MSG::default();
                let _ = PeekMessageW(&mut m, None, 0, 0, PM_NOREMOVE);
            }
            // Ctrl+Alt+Shift+Win+F24 — vanishingly unlikely to be taken.
            use crate::hotkey::{MOD_ALT_BIT, MOD_CONTROL_BIT, MOD_SHIFT_BIT, MOD_WIN_BIT};
            let mods = MOD_CONTROL_BIT | MOD_ALT_BIT | MOD_SHIFT_BIT | MOD_WIN_BIT;
            let vk = 0x87; // VK_F24
            match probe(vk, mods) {
                Probe::Free | Probe::Busy => {}
                other => panic!("probe returned {:?}", other),
            }
            let parsed = ParsedHotkey {
                vk,
                modifiers: mods,
                canonical: "Ctrl+Alt+Shift+Win+F24".into(),
            };
            let mgr = HotkeyManager::new();
            let res = mgr.register(77, &parsed);
            unsafe { let _ = UnregisterHotKey(None, 77); }
            match res {
                Ok(()) => {}
                Err(e) => panic!("RegisterHotKey failed on CI runner: {}", e),
            }
        })
        .join()
        .unwrap();
    }
}
