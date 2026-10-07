// P3: text injection on macOS.
//
// Strategy order:
//   1. AXSelectedText on the focused element  → "AXUIElement"   (needs AX trust)
//   2. arboard set_text + enigo Cmd+V         → "Clipboard"     (needs AX trust for CGEvent)
//   3. enigo.text() CGEvent typing            → "CGEvent"       (needs AX trust)
//
// Without Accessibility permission, macOS blocks ALL synthetic input — so the
// honest degrade is: copy the text to the clipboard and report it, letting the
// user paste manually. The GUI mirrors this via the AX permission banner.

use super::macos_ax;
use super::{InjectionContext, Injector};
use crate::config::Config;
use std::result::Result as StdResult;
use std::time::Instant;

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use objc::{class, msg_send, sel, sel_impl};
use objc::runtime::Object;

type AnyErr = Box<dyn std::error::Error>;

fn err(msg: impl Into<String>) -> AnyErr {
    std::io::Error::other(msg.into()).into()
}

pub fn clipboard_text() -> Option<String> {
    Clipboard::new().ok()?.get().text().ok()
}

/// Accessibility trust probe — also exported through injector::ax_is_trusted.
pub fn is_trusted() -> bool {
    macos_ax::is_trusted()
}

/// Trigger the system "grant Accessibility" dialog. Safe to call repeatedly —
/// macOS shows the pane at most once and later calls are no-ops.
pub fn request_trust() -> bool {
    macos_ax::request_trust()
}

/// Re-activate the app that was foreground when the wheel appeared — our
//  Tauri windows steal key-focus while open (like SetForegroundWindow).
fn refocus_target(pid: u64) {
    if pid == 0 {
        return;
    }
    unsafe {
        let cls = class!(NSRunningApplication);
        let app: *mut Object =
            msg_send![cls, runningApplicationWithProcessIdentifier: pid as i32];
        if !app.is_null() {
            // NSApplicationActivateIgnoringOtherApps = 1 << 1
            let _: () = msg_send![app, activateWithOptions: 2u64];
            std::thread::sleep(std::time::Duration::from_millis(80));
        }
    }
}

pub struct MacosInjector {
    config: Config,
}

impl MacosInjector {
    pub fn new(config: Config) -> Self {
        MacosInjector { config }
    }
}

impl Injector for MacosInjector {
    fn inject(
        &self,
        text: &str,
        context: &InjectionContext,
    ) -> StdResult<(String, u64), AnyErr> {
        if !is_trusted() {
            // Degrade honestly: synthetic input is impossible, but handing the
            // text to the clipboard still saves the user a re-type.
            if let Ok(mut clip) = Clipboard::new() {
                let _ = clip.set().text(text);
            }
            log::warn!("injection degraded to clipboard: accessibility not granted");
            return Err(
                "accessibility-permission: text copied to clipboard — grant Accessibility in System Settings ▸ Privacy & Security ▸ Accessibility, or paste manually".into(),
            );
        }

        // macOS equivalent of the Windows ES_PASSWORD/IsPassword gate.
        if self.config.injection.secure_gate && macos_ax::is_secure_focused() {
            log::warn!("injection refused: focused control is a secure field (AX)");
            return Err("Refused: focused control is a secure field".into());
        }

        log::info!(
            "Injecting {} chars into {} (macOS)",
            text.len(),
            context.app_name
        );
        let start = Instant::now();
        refocus_target(context.window_handle);

        // Primary: AX native write — no clipboard disturbance.
        match macos_ax::insert_text(text) {
            Ok(()) => {
                let elapsed = start.elapsed().as_millis() as u64;
                log::info!("injected via AXUIElement in {}ms", elapsed);
                return Ok(("AXUIElement".to_string(), elapsed));
            }
            Err(e) => log::warn!("AX insertion failed: {} — trying clipboard paste", e),
        }

        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| err(format!("CGEvent init failed: {}", e)))?;

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
                Err(e) => log::warn!("clipboard path failed: {} — trying CGEvent typing", e),
            }
        }

        enigo
            .text(text)
            .map_err(|e| err(format!("CGEvent typing failed: {}", e)))?;
        let elapsed = start.elapsed().as_millis() as u64;
        log::info!("injected via CGEvent in {}ms", elapsed);
        Ok(("CGEvent".to_string(), elapsed))
    }
}

fn inject_via_clipboard(
    enigo: &mut Enigo,
    text: &str,
    restore: bool,
) -> StdResult<(), AnyErr> {
    let mut clip = Clipboard::new().map_err(|e| err(format!("clipboard open: {}", e)))?;

    // Best-effort textual restore only — unlike Windows we don't snapshot
    // files/images; documented in PLATFORMS.md.
    let saved = if restore { clip.get().text().ok() } else { None };

    clip.set()
        .text(text)
        .map_err(|e| err(format!("clipboard set: {}", e)))?;
    std::thread::sleep(std::time::Duration::from_millis(30));

    enigo
        .key(Key::Meta, Direction::Press)
        .map_err(|e| err(format!("cmd press: {}", e)))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| err(format!("v click: {}", e)))?;
    enigo
        .key(Key::Meta, Direction::Release)
        .map_err(|e| err(format!("cmd release: {}", e)))?;
    std::thread::sleep(std::time::Duration::from_millis(80));

    if let Some(prev) = saved {
        let _ = clip.set().text(prev);
    }
    Ok(())
}
