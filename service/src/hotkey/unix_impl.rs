// P2/P3: global hotkeys on Linux (X11) and macOS via the `global-hotkey`
// crate — the same backend Tauri's own globalShortcut plugin uses.
//
// Threading mirrors the Windows impl: `start()` spawns a worker that OWNS the
// backend manager for its whole life (create → register → event bridge →
// unregister on exit). On Linux the manager is !Send on some versions, so it
// can never leave the worker. On macOS it must additionally be created on the
// main thread (its Carbon event tap binds the main run loop) — we hop via
// libdispatch once, register there, and let the manager live in the worker.
//
// Failure semantics match Windows: start() only fails if the thread can't be
// spawned; per-combo outcomes are recorded into the shared registry so
// check_hotkeys shows "failed: <reason>" instead of silent no-ops.

use super::{Hotkey, ParsedHotkey};
use std::collections::HashMap;
use std::result::Result as StdResult;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;

use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};

/// Win32 VK (what the shared parser produces) → global-hotkey `Code`.
fn vk_to_code(vk: u32) -> StdResult<Code, String> {
    let code = match vk {
        0x41 => Code::KeyA, 0x42 => Code::KeyB, 0x43 => Code::KeyC,
        0x44 => Code::KeyD, 0x45 => Code::KeyE, 0x46 => Code::KeyF,
        0x47 => Code::KeyG, 0x48 => Code::KeyH, 0x49 => Code::KeyI,
        0x4A => Code::KeyJ, 0x4B => Code::KeyK, 0x4C => Code::KeyL,
        0x4D => Code::KeyM, 0x4E => Code::KeyN, 0x4F => Code::KeyO,
        0x50 => Code::KeyP, 0x51 => Code::KeyQ, 0x52 => Code::KeyR,
        0x53 => Code::KeyS, 0x54 => Code::KeyT, 0x55 => Code::KeyU,
        0x56 => Code::KeyV, 0x57 => Code::KeyW, 0x58 => Code::KeyX,
        0x59 => Code::KeyY, 0x5A => Code::KeyZ,
        0x30 => Code::Digit0, 0x31 => Code::Digit1, 0x32 => Code::Digit2,
        0x33 => Code::Digit3, 0x34 => Code::Digit4, 0x35 => Code::Digit5,
        0x36 => Code::Digit6, 0x37 => Code::Digit7, 0x38 => Code::Digit8,
        0x39 => Code::Digit9,
        // Named keys — mirror of the parser's named_key table.
        0x20 => Code::Space,
        0x0D => Code::Enter,
        0x09 => Code::Tab,
        0x1B => Code::Escape,
        0x08 => Code::Backspace,
        0x2E => Code::Delete,
        0x2D => Code::Insert,
        0x24 => Code::Home,
        0x23 => Code::End,
        0x21 => Code::PageUp,
        0x22 => Code::PageDown,
        0x26 => Code::ArrowUp,
        0x28 => Code::ArrowDown,
        0x25 => Code::ArrowLeft,
        0x27 => Code::ArrowRight,
        0x14 => Code::CapsLock,
        0x90 => Code::NumLock,
        0x91 => Code::ScrollLock,
        0x2C => Code::PrintScreen,
        0x13 => Code::Pause,
        // OEM punctuation (Win32 VKs, NOT ascii).
        0xBA => Code::Semicolon,
        0xBF => Code::Slash,
        0xC0 => Code::Backquote,
        0xDB => Code::BracketLeft,
        0xDC => Code::Backslash,
        0xDD => Code::BracketRight,
        0xDE => Code::Quote,
        0xBD => Code::Minus,
        0xBB => Code::Equal,
        0xBC => Code::Comma,
        0xBE => Code::Period,
        // Numpad
        0x60 => Code::Numpad0, 0x61 => Code::Numpad1, 0x62 => Code::Numpad2,
        0x63 => Code::Numpad3, 0x64 => Code::Numpad4, 0x65 => Code::Numpad5,
        0x66 => Code::Numpad6, 0x67 => Code::Numpad7, 0x68 => Code::Numpad8,
        0x69 => Code::Numpad9,
        0x6A => Code::NumpadMultiply,
        0x6B => Code::NumpadAdd,
        0x6D => Code::NumpadSubtract,
        0x6E => Code::NumpadDecimal,
        0x6F => Code::NumpadDivide,
        // F1..=F24
        0x70..=0x87 => {
            const F: [Code; 24] = [
                Code::F1, Code::F2, Code::F3, Code::F4, Code::F5, Code::F6,
                Code::F7, Code::F8, Code::F9, Code::F10, Code::F11, Code::F12,
                Code::F13, Code::F14, Code::F15, Code::F16, Code::F17, Code::F18,
                Code::F19, Code::F20, Code::F21, Code::F22, Code::F23, Code::F24,
            ];
            F[(vk - 0x70) as usize]
        }
        _ => return Err(format!("no physical-key mapping for VK 0x{:02X}", vk)),
    };
    Ok(code)
}

fn mods_to_global(modifiers: u32) -> Modifiers {
    let mut m = Modifiers::empty();
    if modifiers & super::MOD_CONTROL_BIT != 0 { m |= Modifiers::CONTROL; }
    if modifiers & super::MOD_ALT_BIT != 0 { m |= Modifiers::ALT; }
    if modifiers & super::MOD_SHIFT_BIT != 0 { m |= Modifiers::SHIFT; }
    // Win on Linux = Super; Cmd on macOS — same META slot in global-hotkey.
    if modifiers & super::MOD_WIN_BIT != 0 { m |= Modifiers::META; }
    m
}

#[cfg(target_os = "macos")]
mod main_thread {
    use std::ffi::c_void;

    extern "C" {
        fn dispatch_get_main_queue() -> *mut c_void;
        fn dispatch_sync_f(queue: *mut c_void, context: *mut c_void, work: extern "C" fn(*mut c_void));
    }

    extern "C" fn trampoline<F: FnMut()>(ctx: *mut c_void) {
        unsafe { (*(ctx as *mut F))() }
    }

    /// Run `f` synchronously on the main GCD queue. Safe from any worker
    /// thread while the Tauri/NSApplication run loop services the main queue.
    pub fn run<F: FnMut()>(mut f: F) {
        let ctx = &mut f as *mut F as *mut c_void;
        unsafe { dispatch_sync_f(dispatch_get_main_queue(), ctx, trampoline::<F>) }
    }
}

#[cfg(not(target_os = "macos"))]
mod main_thread {
    /// Non-macOS platforms have no main-thread requirement — run inline.
    pub fn run<F: FnMut()>(mut f: F) {
        f();
    }
}

/// Registration job for the worker: parse + build happen up front so errors
/// are recorded deterministically before any backend call.
struct Job {
    our_id: u32,
    combo: String,
    result: StdResult<HotKey, String>,
}

fn build_job(our_id: u32, combo: &str) -> Job {
    let result = super::parse_hotkey(combo).and_then(|p: ParsedHotkey| {
        vk_to_code(p.vk).map(|code| HotKey::new(Some(mods_to_global(p.modifiers)), code))
    });
    Job { our_id, combo: combo.to_string(), result }
}

pub struct GlobalHotkey {
    tx: mpsc::Sender<u32>,
    rx: mpsc::Receiver<u32>,
    should_quit: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
    hotkey: String,
    quick_hotkey: String,
}

impl GlobalHotkey {
    pub fn new(hotkey: String, quick_hotkey: String) -> Self {
        let (tx, rx) = mpsc::channel();
        GlobalHotkey {
            tx,
            rx,
            should_quit: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
            hotkey,
            quick_hotkey,
        }
    }
}

impl Hotkey for GlobalHotkey {
    fn start(&mut self) -> StdResult<(), Box<dyn std::error::Error + Send + 'static>> {
        self.should_quit.store(false, Ordering::SeqCst);

        let mut jobs = vec![build_job(4, &self.hotkey)];

        // Same-combo dedup on PARSED values — "ctrl+alt+q" and "Ctrl+Alt+Q"
        // are one registration (mirrors windows_impl).
        let same_combo = !self.quick_hotkey.is_empty()
            && matches!(
                (super::parse_hotkey(&self.quick_hotkey), super::parse_hotkey(&self.hotkey)),
                (Ok(q), Ok(w)) if q.vk == w.vk && q.modifiers == w.modifiers
            );
        if !self.quick_hotkey.is_empty() {
            if same_combo {
                // id5 shares id4's registration — the bridge can't tell them
                // apart, and the engine treats a shared press as wheel (4).
                // Record it as registered so check_hotkeys doesn't misreport.
                super::record_registration(5, &self.quick_hotkey, Ok(()));
            } else {
                jobs.push(build_job(5, &self.quick_hotkey));
            }
        }

        let tx = self.tx.clone();
        let should_quit = self.should_quit.clone();
        self.thread_handle = Some(std::thread::spawn(move || {
            // Manager creation: macOS requires the main thread; the manager
            // then lives inside this worker for its whole life.
            let mut manager: Option<GlobalHotKeyManager> = None;
            let mut create_err: Option<String> = None;
            main_thread::run(|| {
                match GlobalHotKeyManager::new() {
                    Ok(m) => manager = Some(m),
                    Err(e) => create_err = Some(e.to_string()),
                }
            });
            let Some(manager) = manager else {
                let e = create_err.unwrap_or_else(|| "unknown".into());
                for job in &jobs {
                    super::record_registration(
                        job.our_id,
                        &job.combo,
                        Err(format!("hotkey backend unavailable: {}", e)),
                    );
                }
                log::error!("global-hotkey backend unavailable: {}", e);
                return;
            };

            let mut id_map: HashMap<u32, u32> = HashMap::new();
            let mut registered: Vec<HotKey> = Vec::new();
            for job in jobs {
                match job.result {
                    Err(e) => {
                        super::record_registration(job.our_id, &job.combo, Err(e.clone()));
                        log::error!("hotkey {} not registered: {}", job.combo, e);
                    }
                    Ok(hk) => {
                        let gh_id = hk.id();
                        let mut reg = Ok(());
                        main_thread::run(|| {
                            reg = manager.register(hk).map_err(|e| e.to_string());
                        });
                        super::record_registration(job.our_id, &job.combo, reg.clone());
                        match reg {
                            Ok(()) => {
                                log::info!("registered {} (id {})", job.combo, job.our_id);
                                id_map.insert(gh_id, job.our_id);
                                registered.push(hk);
                            }
                            Err(e) => log::error!("register {} failed: {}", job.combo, e),
                        }
                    }
                }
            }

            // Event bridge: fired ids arrive on the global channel; remap to
            // our ids (4=wheel, 5=quick) into the engine's mpsc.
            let receiver = GlobalHotKeyEvent::receiver();
            while !should_quit.load(Ordering::Acquire) {
                match receiver.try_recv() {
                    Ok(ev) => {
                        if let Some(our_id) = id_map.get(&ev.id) {
                            let _ = tx.send(*our_id);
                        }
                    }
                    Err(_) => std::thread::sleep(std::time::Duration::from_millis(10)),
                }
            }

            // Unregister before exit so a restarted engine can't fight this
            // worker for the same grabs (review F01/F28/F50/F64 semantics).
            main_thread::run(|| {
                for hk in &registered {
                    let _ = manager.unregister(*hk);
                }
            });
            super::clear_registrations();
        }));
        Ok(())
    }

    fn stop(&mut self) {
        self.should_quit.store(true, Ordering::SeqCst);
        if let Some(h) = self.thread_handle.take() {
            let _ = h.join();
        }
    }

    fn wait_for_hotkey(&self) -> Option<u32> {
        self.rx.recv().ok()
    }

    fn try_wait_for_hotkey(&self) -> Option<u32> {
        self.rx.try_recv().ok()
    }
}

impl Drop for GlobalHotkey {
    fn drop(&mut self) {
        self.stop();
    }
}
