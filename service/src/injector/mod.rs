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
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
mod windows_impl;
#[cfg(windows)]
pub use windows_impl::WindowsInjector;

/// Fallback for platforms whose injector is not implemented yet.
#[cfg(not(windows))]
pub struct NullInjector;

#[cfg(not(windows))]
impl Injector for NullInjector {
    fn inject(
        &self,
        _text: &str,
        _context: &InjectionContext,
    ) -> StdResult<(String, u64), Box<dyn std::error::Error>> {
        Err("text injection is not implemented on this platform".into())
    }
}

/// Construct the platform injector.
pub fn create(config: Config) -> Box<dyn Injector> {
    #[cfg(windows)]
    {
        Box::new(WindowsInjector::new(vec![], config))
    }
    #[cfg(not(windows))]
    {
        let _ = config;
        Box::new(NullInjector)
    }
}
