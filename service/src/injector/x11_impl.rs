// P2: text injection on Linux/X11.
//
// Strategy order mirrors Windows (Clipboard → character synthesis):
//   1. arboard set_text + enigo Ctrl+V        → "Clipboard"
//   2. enigo.text() character synthesis        → "Enigo"
//
// Known honest gaps (documented, not hidden):
//   * X11 exposes no input-type hint for focused controls, so the
//     "never inject into password fields" gate from Windows has no X11
//     equivalent — the log line below says so instead of pretending.
//   * Wayland sessions can't synthesize input at all; inject() returns a
//     clear error and the GUI banner explains the degradation.

use super::{InjectionContext, Injector};
use crate::config::Config;
use crate::platform::Session;
use std::result::Result as StdResult;
use std::time::Instant;

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, InputFocus};

type AnyErr = Box<dyn std::error::Error>;

fn err(msg: impl Into<String>) -> AnyErr {
    std::io::Error::other(msg.into()).into()
}

pub fn clipboard_text() -> Option<String> {
    Clipboard::new().ok()?.get().text().ok()
}

/// Give focus back to the window that was active when the wheel was shown —
/// our own panel steals X11 focus while open, like SetForegroundWindow on
/// Windows. Best-effort: a failure just means we paste where focus already is.
fn refocus_target(window_handle: u64) {
    if window_handle == 0 {
        return;
    }
    if let Ok((conn, _)) = x11rb::connect(None) {
        let _ = conn.set_input_focus(
            InputFocus::POINTER_ROOT,
            window_handle as u32,
            x11rb::CURRENT_TIME,
        );
        let _ = conn.flush();
        std::thread::sleep(std::time::Duration::from_millis(80));
    }
}

pub struct X11Injector {
    config: Config,
}

impl X11Injector {
    pub fn new(config: Config) -> Self {
        X11Injector { config }
    }
}

impl Injector for X11Injector {
    fn inject(
        &self,
        text: &str,
        context: &InjectionContext,
    ) -> StdResult<(String, u64), AnyErr> {
        if crate::platform::session() == Session::Wayland {
            return Err(
                "automatic injection unavailable on Wayland (no XTEST); text was not inserted — copy-paste manually or run under X11"
                    .into(),
            );
        }
        if self.config.injection.secure_gate {
            // Honest limitation: X11 has no "is this a password field"
            // introspection, so the secure gate can't be enforced here.
            log::debug!(
                "secure_gate configured but X11 exposes no input-type info — cannot detect password fields for {}",
                context.app_name
            );
        }
        log::info!(
            "Injecting {} chars into {} (X11)",
            text.len(),
            context.app_name
        );
        let start = Instant::now();
        refocus_target(context.window_handle);

        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| err(format!("enigo init failed (XTEST missing?): {}", e)))?;

        if self.config.injection.allow_clipboard {
            match inject_via_clipboard(
                &mut enigo,
                text,
                self.config.injection.restore_clipboard,
            ) {
                Ok(()) => {
                    let elapsed = start.elapsed().as_millis() as u64;
                    log::info!("injected via Clipboard in {}ms", elapsed);
                    return Ok(("Clipboard".to_string(), elapsed));
                }
                Err(e) => log::warn!("clipboard path failed: {} — trying synthesis", e),
            }
        }

        enigo
            .text(text)
            .map_err(|e| err(format!("character synthesis failed: {}", e)))?;
        let elapsed = start.elapsed().as_millis() as u64;
        log::info!("injected via Enigo in {}ms", elapsed);
        Ok(("Enigo".to_string(), elapsed))
    }
}

fn inject_via_clipboard(
    enigo: &mut Enigo,
    text: &str,
    restore: bool,
) -> StdResult<(), AnyErr> {
    let mut clip = Clipboard::new().map_err(|e| err(format!("clipboard open: {}", e)))?;

    let saved = if restore {
        clip.get().text().ok()
    } else {
        None
    };

    clip.set()
        .text(text)
        .map_err(|e| err(format!("clipboard set: {}", e)))?;

    // Let the X11 selection ownership settle before pasting.
    std::thread::sleep(std::time::Duration::from_millis(30));
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| err(format!("ctrl press: {}", e)))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| err(format!("v click: {}", e)))?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| err(format!("ctrl release: {}", e)))?;
    std::thread::sleep(std::time::Duration::from_millis(80));

    if let Some(prev) = saved {
        let _ = clip.set().text(prev);
    }
    Ok(())
}
