// Phase 2 Task8: platform boundary for global hotkeys.
//
// `Hotkey` is the cross-platform trait. `create()` returns the platform
// implementation as a trait object so call sites carry zero cfg attributes.
// The Windows implementation lives in `windows_impl.rs` (was `HotkeyService`);
// Linux/macOS implementations are Phase 3 work — `NullHotkey` reports honest
// errors and never fires.

use std::result::Result as StdResult;

pub trait Hotkey: Send {
    /// Start listening (typically spawns the platform message-loop thread).
    fn start(&mut self) -> StdResult<(), Box<dyn std::error::Error + Send + 'static>>;
    /// Stop listening; idempotent.
    fn stop(&mut self);
    /// Block until a hotkey id arrives (None on channel close).
    fn wait_for_hotkey(&self) -> Option<u32>;
    /// Non-blocking poll for a hotkey id.
    fn try_wait_for_hotkey(&self) -> Option<u32>;
}

#[cfg(windows)]
mod windows_impl;
#[cfg(windows)]
pub use windows_impl::WindowsHotkey;

/// Fallback for platforms whose hotkey backend is not implemented yet.
#[cfg(not(windows))]
pub struct NullHotkey;

#[cfg(not(windows))]
impl Hotkey for NullHotkey {
    fn start(&mut self) -> StdResult<(), Box<dyn std::error::Error + Send + 'static>> {
        Err("global hotkeys are not implemented on this platform".into())
    }
    fn stop(&mut self) {}
    fn wait_for_hotkey(&self) -> Option<u32> { None }
    fn try_wait_for_hotkey(&self) -> Option<u32> { None }
}

/// Construct the platform hotkey service.
/// `hotkey` opens the wheel (id 4); `quick_hotkey` injects the default prompt (id 5).
pub fn create(hotkey: String, quick_hotkey: String) -> Box<dyn Hotkey> {
    #[cfg(windows)]
    {
        Box::new(WindowsHotkey::new(hotkey, quick_hotkey))
    }
    #[cfg(not(windows))]
    {
        let _ = (hotkey, quick_hotkey);
        Box::new(NullHotkey)
    }
}
