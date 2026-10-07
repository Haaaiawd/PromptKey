// Phase 2 Task8: platform boundary for text injection.
//
// `Injector` is the cross-platform trait. `create()` returns the platform
// implementation as a trait object so call sites carry zero cfg attributes.
// The Windows implementation lives in `windows_impl.rs` (was `Injector`);
// Linux/macOS implementations are Phase 3 work — until then `NullInjector`
// fails honestly.

use crate::config::Config;
use std::result::Result as StdResult;

#[derive(Debug)]
pub struct InjectionContext {
    pub app_name: String,
    pub window_title: String,
    /// Opaque native window handle — HWND bits on Windows, 0 if unknown.
    pub window_handle: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectionStrategy {
    Clipboard,
    SendInput,
    /// macOS AX write (AXSelectedText) — primary path there.
    AXUIElement,
    /// macOS CGEvent / Linux XTEST character synthesis via enigo.
    Synthesize,
}

/// Cross-platform injection contract: returns (strategy_used, elapsed_ms).
pub trait Injector: Send + Sync {
    fn inject(
        &self,
        text: &str,
        context: &InjectionContext,
    ) -> StdResult<(String, u64), Box<dyn std::error::Error>>;
}

/// Phase 2 D1: read the current text clipboard content for {{clipboard}}.
/// Returns None on platforms without an implementation or when unavailable.
pub fn clipboard_text() -> Option<String> {
    #[cfg(windows)]
    {
        windows_impl::clipboard_text()
    }
    #[cfg(target_os = "linux")]
    {
        x11_impl::clipboard_text()
    }
    #[cfg(target_os = "macos")]
    {
        macos_impl::clipboard_text()
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

/// Accessibility ("辅助功能") trust probe — meaningful on macOS, always true
/// elsewhere so callers don't need their own cfg.
pub fn ax_is_trusted() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos_impl::is_trusted()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Prompt the user for Accessibility permission (macOS only, no-op elsewhere).
#[cfg(target_os = "macos")]
pub fn request_ax_trust() -> bool {
    macos_impl::request_trust()
}

#[cfg(windows)]
mod windows_impl;
#[cfg(windows)]
pub use windows_impl::WindowsInjector;

#[cfg(target_os = "linux")]
mod x11_impl;
#[cfg(target_os = "linux")]
pub use x11_impl::X11Injector;

#[cfg(target_os = "macos")]
mod macos_ax;
#[cfg(target_os = "macos")]
mod macos_impl;
#[cfg(target_os = "macos")]
pub use macos_impl::MacosInjector;

/// Construct the platform injector.
pub fn create(config: Config) -> Box<dyn Injector> {
    #[cfg(windows)]
    {
        Box::new(WindowsInjector::new(vec![], config))
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(X11Injector::new(config))
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(MacosInjector::new(config))
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        let _ = config;
        compile_error!("no Injector implementation for this platform");
    }
}
