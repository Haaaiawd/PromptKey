// Phase 2 Task8: platform boundary for foreground-app context capture.
//
// `Context` is the cross-platform trait. `create()` returns the platform
// implementation as a trait object so call sites carry zero cfg attributes.
// The Windows implementation lives in `windows.rs` (was `ContextManager`);
// Linux/macOS implementations are Phase 3 work — until then `NullContext`
// returns honest errors instead of silently doing the wrong thing.

use std::result::Result as StdResult;

/// Platform-agnostic description of the foreground app/window.
#[derive(Debug, Clone)]
pub struct AppContext {
    pub process_name: String,
    pub window_title: String,
    /// Opaque native window handle — HWND bit pattern on Windows.
    /// Kept as u64 so the service boundary carries no platform types.
    pub window_handle: u64,
}

pub trait Context: Send + Sync {
    fn get_foreground_context(&self) -> StdResult<AppContext, Box<dyn std::error::Error>>;
}

#[cfg(windows)]
mod windows_impl;
#[cfg(windows)]
pub use windows_impl::WindowsContext;

#[cfg(target_os = "linux")]
mod x11_impl;
#[cfg(target_os = "linux")]
pub use x11_impl::X11Context;

#[cfg(target_os = "macos")]
mod macos_impl;
#[cfg(target_os = "macos")]
pub use macos_impl::MacosContext;

/// Construct the platform context provider.
pub fn create() -> Box<dyn Context> {
    #[cfg(windows)]
    {
        Box::new(WindowsContext::new())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(X11Context::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(MacosContext::new())
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        compile_error!("no Context implementation for this platform");
    }
}
