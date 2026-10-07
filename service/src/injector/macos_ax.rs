// P3: minimal Accessibility (AXUIElement) FFI helpers for macOS.
//
// Hand-written bindings instead of a crate — the surface we need is ~8 C
// functions, and keeping them here makes the trust/secure-field logic fully
// auditable. Everything returns Results; callers decide whether an AX error
// means "fallback" or "permission missing".

use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::string::{CFString, CFStringRef};
use std::ffi::c_void;
use std::result::Result as StdResult;

pub type AXUIElementRef = *mut c_void;
type AnyErr = Box<dyn std::error::Error>;

fn err(msg: impl Into<String>) -> AnyErr {
    std::io::Error::other(msg.into()).into()
}

// ApplicationServices exports the prompt option key as a CFStringRef global.
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    static kAXTrustedCheckOptionPrompt: CFStringRef;

    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> i32;
}

const AX_SUCCESS: i32 = 0; // kAXErrorSuccess

fn cfstr(s: &str) -> CFString {
    CFString::new(s)
}

/// True when the process holds Accessibility ("辅助功能") permission.
pub fn is_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Ask macOS to show the system permission dialog. Returns the CURRENT trust
/// state (granting still requires the user to flip the switch — the dialog
/// only deep-links them there).
pub fn request_trust() -> bool {
    unsafe {
        let key = CFString::wrap_under_create_rule(kAXTrustedCheckOptionPrompt);
        let pairs = [(key.as_CFType(), CFBoolean::true_value().as_CFType())];
        let dict = CFDictionary::from_CFType_pairs(&pairs);
        AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef())
    }
}

/// RAII wrapper so every CF object we created/copied is released once.
struct CfObject(*mut c_void);
impl CfObject {
    fn new(p: *mut c_void) -> StdResult<Self, AnyErr> {
        if p.is_null() {
            Err(err("null CF object"))
        } else {
            Ok(CfObject(p))
        }
    }
    fn as_ax(&self) -> AXUIElementRef {
        self.0
    }
}
impl Drop for CfObject {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0 as CFTypeRef) };
        }
    }
}

/// Copy a CFString-valued attribute off an AX element.
fn copy_string_attr(el: AXUIElementRef, attr: &str) -> Option<String> {
    unsafe {
        let mut v: CFTypeRef = std::ptr::null();
        if AXUIElementCopyAttributeValue(el, cfstr(attr).as_concrete_TypeRef(), &mut v)
            != AX_SUCCESS
            || v.is_null()
        {
            return None;
        }
        let s = CFString::wrap_under_create_rule(v as CFStringRef).to_string();
        Some(s)
    }
}

fn copy_element(el: AXUIElementRef, attr: &str) -> StdResult<CfObject, AnyErr> {
    unsafe {
        let mut v: CFTypeRef = std::ptr::null();
        let rc = AXUIElementCopyAttributeValue(el, cfstr(attr).as_concrete_TypeRef(), &mut v);
        if rc != AX_SUCCESS || v.is_null() {
            return Err(err(format!("AX copy {} failed (code {})", attr, rc)));
        }
        CfObject::new(v as AXUIElementRef)
    }
}

/// The system-wide focused UI element (whatever holds the text caret).
fn focused_element() -> StdResult<CfObject, AnyErr> {
    unsafe {
        let sys = CfObject::new(AXUIElementCreateSystemWide())?;
        copy_element(sys.as_ax(), "AXFocusedUIElement")
    }
}

/// True when the focused element is a password/secure field — the macOS
/// equivalent of the Windows ES_PASSWORD + UIA IsPassword gate.
pub fn is_secure_focused() -> bool {
    let Ok(focused) = focused_element() else { return false };
    match copy_string_attr(focused.as_ax(), "AXRole") {
        Some(role) if role.contains("Secure") => true,
        Some(role) if role == "AXTextField" || role == "AXTextArea" => {
            // Some apps mark secure fields only via subrole.
            copy_string_attr(focused.as_ax(), "AXSubrole")
                .map(|s| s.contains("Secure"))
                .unwrap_or(false)
        }
        _ => false,
    }
}

/// Insert text at the caret of the currently focused element by setting
/// AXSelectedText (replaces selection; inserts when selection is empty).
/// This is the AX-native write path — no clipboard touch. Fails (Err) on
/// elements that don't support the attribute; caller falls back to paste.
pub fn insert_text(text: &str) -> StdResult<(), AnyErr> {
    let focused = focused_element()?;
    unsafe {
        let value = cfstr(text);
        let rc = AXUIElementSetAttributeValue(
            focused.as_ax(),
            cfstr("AXSelectedText").as_concrete_TypeRef(),
            value.as_CFTypeRef(),
        );
        if rc == AX_SUCCESS {
            Ok(())
        } else {
            Err(err(format!("AX set AXSelectedText failed (code {})", rc)))
        }
    }
}

/// Title of the focused window of `pid` — used by MacosContext.
/// Needs Accessibility trust; returns Err otherwise.
pub fn focused_window_title(pid: i32) -> StdResult<String, AnyErr> {
    if !is_trusted() {
        return Err(err("accessibility permission not granted"));
    }
    unsafe {
        let app = CfObject::new(AXUIElementCreateApplication(pid))?;
        let win = copy_element(app.as_ax(), "AXFocusedWindow")?;
        copy_string_attr(win.as_ax(), "AXTitle")
            .ok_or_else(|| err("AXTitle unreadable"))
    }
}
