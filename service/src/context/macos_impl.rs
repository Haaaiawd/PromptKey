// P3: foreground-app context on macOS.
//
// process_name ← NSWorkspace.frontmostApplication.bundleIdentifier
//                (stable, matches how users scope "which app" — better than
//                localizedName which changes with language)
// window_title ← AX focused window's AXTitle on that app (needs
//                Accessibility permission; degrades to "" without it)
// window_handle ← the app's pid as u64. The AX injector uses it to build an
//                AXUIElement application reference on demand; a raw
//                AXUIElement pointer would not survive in the AppContext.

use super::{AppContext, Context};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::result::Result as StdResult;

use objc::{class, msg_send, sel, sel_impl};
use objc::runtime::Object;

use crate::injector::macos_ax;

type AnyErr = Box<dyn std::error::Error>;

fn err(msg: impl Into<String>) -> AnyErr {
    std::io::Error::other(msg.into()).into()
}

pub struct MacosContext;

impl Default for MacosContext {
    fn default() -> Self {
        Self::new()
    }
}

impl MacosContext {
    pub fn new() -> Self {
        MacosContext
    }

    /// NSString* → owned String via UTF8String.
    unsafe fn nsstring_to_string(ns: *mut Object) -> Option<String> {
        if ns.is_null() {
            return None;
        }
        let c: *const c_char = msg_send![ns, UTF8String];
        if c.is_null() {
            return None;
        }
        Some(CStr::from_ptr(c).to_string_lossy().into_owned())
    }
}

impl Context for MacosContext {
    fn get_foreground_context(&self) -> StdResult<AppContext, AnyErr> {
        unsafe {
            let workspace: *mut Object = msg_send![class!(NSWorkspace), sharedWorkspace];
            if workspace.is_null() {
                return Err(err("NSWorkspace.sharedWorkspace unavailable"));
            }
            let app: *mut Object = msg_send![workspace, frontmostApplication];
            if app.is_null() {
                return Err(err("no frontmost application"));
            }
            let pid: i32 = msg_send![app, processIdentifier];

            let bundle: *mut Object = msg_send![app, bundleIdentifier];
            let name = Self::nsstring_to_string(bundle)
                .or_else(|| {
                    let localized: *mut Object = msg_send![app, localizedName];
                    Self::nsstring_to_string(localized)
                })
                .unwrap_or_else(|| format!("pid:{}", pid));

            // Window title via AX — best-effort, needs Accessibility trust.
            let title = if crate::injector::ax_is_trusted() {
                macos_ax::focused_window_title(pid).unwrap_or_default()
            } else {
                String::new()
            };

            Ok(AppContext {
                process_name: name,
                window_title: title,
                window_handle: pid as u64,
            })
        }
    }
}
