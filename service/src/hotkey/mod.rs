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

/* =================== shared combo parser ===================
 *
 * Platform-agnostic: returns raw virtual-key code + modifier bits so it can be
 * unit-tested everywhere. Windows impl maps these into windows-rs types.
 * Modifier bit values match Win32 MOD_* so the impl can OR them verbatim.
 */

pub const MOD_ALT_BIT: u32 = 0x0001;
pub const MOD_CONTROL_BIT: u32 = 0x0002;
pub const MOD_SHIFT_BIT: u32 = 0x0004;
pub const MOD_WIN_BIT: u32 = 0x0008;
/// Win32 MOD_NOREPEAT — suppresses auto-repeat while the chord is held.
pub const MOD_NOREPEAT_BIT: u32 = 0x4000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedHotkey {
    /// Virtual-key code (Win32 VK_* value).
    pub vk: u32,
    /// Modifier bitmask using MOD_*_BIT values (NOREPEAT excluded — added by the platform impl).
    pub modifiers: u32,
    /// Canonical display form, e.g. "Ctrl+Alt+Space" — what the UI should show back.
    pub canonical: String,
}

/// Named (non-printable or OEM) keys → (aliases, VK code, canonical name).
/// VK codes are Win32 constants; the table is data, not platform API.
fn named_key(name: &str) -> Option<(u32, &'static str)> {
    let key = match name {
        "SPACE" => (0x20, "Space"),
        "ENTER" | "RETURN" => (0x0D, "Enter"),
        "TAB" => (0x09, "Tab"),
        "ESC" | "ESCAPE" => (0x1B, "Esc"),
        "BACKSPACE" | "BACK" => (0x08, "Backspace"),
        "DELETE" | "DEL" => (0x2E, "Delete"),
        "INSERT" | "INS" => (0x2D, "Insert"),
        "HOME" => (0x24, "Home"),
        "END" => (0x23, "End"),
        "PAGEUP" | "PGUP" => (0x21, "PageUp"),
        "PAGEDOWN" | "PGDN" => (0x22, "PageDown"),
        "UP" => (0x26, "Up"),
        "DOWN" => (0x28, "Down"),
        "LEFT" => (0x25, "Left"),
        "RIGHT" => (0x27, "Right"),
        "CAPSLOCK" | "CAPS" => (0x14, "CapsLock"),
        "NUMLOCK" => (0x90, "NumLock"),
        "SCROLLLOCK" => (0x91, "ScrollLock"),
        "PRINTSCREEN" | "PRTSC" | "SNAPSHOT" => (0x2C, "PrintScreen"),
        "PAUSE" => (0x13, "Pause"),
        // OEM punctuation — NOTE: these are NOT their ASCII codes.
        ";" | "SEMICOLON" | "OEM1" => (0xBA, ";"),
        "/" | "SLASH" | "OEM2" => (0xBF, "/"),
        "`" | "GRAVE" | "BACKQUOTE" | "OEM3" => (0xC0, "`"),
        "[" | "LBRACKET" | "OEM4" => (0xDB, "["),
        "\\" | "BACKSLASH" | "OEM5" => (0xDC, "\\"),
        "]" | "RBRACKET" | "OEM6" => (0xDD, "]"),
        "'" | "QUOTE" | "OEM7" => (0xDE, "'"),
        "-" | "MINUS" | "DASH" | "OEM_MINUS" => (0xBD, "-"),
        "=" | "EQUALS" | "EQUAL" | "OEM_PLUS" => (0xBB, "="),
        "," | "COMMA" | "OEM_COMMA" => (0xBC, ","),
        "." | "PERIOD" | "DOT" | "OEM_PERIOD" => (0xBE, "."),
        // Numpad
        "NUMPAD0" | "NUM0" => (0x60, "Num0"),
        "NUMPAD1" | "NUM1" => (0x61, "Num1"),
        "NUMPAD2" | "NUM2" => (0x62, "Num2"),
        "NUMPAD3" | "NUM3" => (0x63, "Num3"),
        "NUMPAD4" | "NUM4" => (0x64, "Num4"),
        "NUMPAD5" | "NUM5" => (0x65, "Num5"),
        "NUMPAD6" | "NUM6" => (0x66, "Num6"),
        "NUMPAD7" | "NUM7" => (0x67, "Num7"),
        "NUMPAD8" | "NUM8" => (0x68, "Num8"),
        "NUMPAD9" | "NUM9" => (0x69, "Num9"),
        // Canonical names must re-parse — the UI saves what it displays.
        // "Num+" can NEVER round-trip: '+' is the combo separator, so the
        // canonical name for VK_ADD is "NumAdd" (and bare "NUM+" is dropped).
        "NUMPAD*" | "NUM*" | "NUMMULTIPLY" | "MULTIPLY" => (0x6A, "Num*"),
        // "NUMPAD+" is unreachable — '+' is the combo separator. Use NUMADD.
        "NUMADD" | "ADD" | "NUMPADPLUS" => (0x6B, "NumAdd"),
        "NUMPAD-" | "NUM-" | "NUMSUBTRACT" | "SUBTRACT" => (0x6D, "Num-"),
        "NUMPAD." | "NUM." | "NUMDECIMAL" | "DECIMAL" => (0x6E, "Num."),
        "NUMPAD/" | "NUM/" | "NUMDIVIDE" | "DIVIDE" => (0x6F, "Num/"),
        _ => return None,
    };
    Some(key)
}

/// F1..=F24 → VK_F1(0x70)..VK_F24(0x87).
fn function_key(name: &str) -> Option<(u32, String)> {
    let n = name.strip_prefix('F')?.parse::<u32>().ok()?;
    if (1..=24).contains(&n) {
        Some((0x70 + n - 1, format!("F{}", n)))
    } else {
        None
    }
}

/// Parse "Ctrl+Alt+Space" → ParsedHotkey. Unknown parts are a hard error —
/// nothing is silently dropped or substituted.
pub fn parse_hotkey(hotkey_str: &str) -> StdResult<ParsedHotkey, String> {
    let parts: Vec<String> = hotkey_str
        .split('+')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        return Err("hotkey is empty".into());
    }

    let mut modifiers = 0u32;
    let mut vk: Option<(u32, String)> = None;

    for part in parts {
        // A "+" literal written as the last empty part was filtered out; to
        // express the '+' key itself users type "OEM_PLUS"/"=".
        match part.to_uppercase().as_str() {
            "CTRL" | "CONTROL" => modifiers |= MOD_CONTROL_BIT,
            "ALT" | "OPTION" => modifiers |= MOD_ALT_BIT,
            "SHIFT" => modifiers |= MOD_SHIFT_BIT,
            "WIN" | "META" | "SUPER" | "CMD" | "COMMAND" => modifiers |= MOD_WIN_BIT,
            other => {
                let resolved: Option<(u32, String)> = named_key(other)
                    .map(|(vk, name)| (vk, name.to_string()))
                    .or_else(|| function_key(other))
                    .or_else(|| {
                        // A-Z / 0-9 single chars: VK code == ASCII code.
                        if other.len() == 1
                            && other.as_bytes()[0].is_ascii_alphanumeric()
                        {
                            let c = other.as_bytes()[0] as u32;
                            Some((c, char::from_u32(c).unwrap().to_string()))
                        } else {
                            None
                        }
                    });
                match resolved {
                    Some(k) => {
                        if vk.is_some() {
                            return Err(format!(
                                "hotkey has more than one main key: {}",
                                part
                            ));
                        }
                        vk = Some(k);
                    }
                    None => return Err(format!("unsupported key part: {}", part)),
                }
            }
        }
    }

    let (vk, key_name) = vk.ok_or_else(|| "hotkey needs a non-modifier key".to_string())?;
    let mut names: Vec<&str> = Vec::new();
    if modifiers & MOD_CONTROL_BIT != 0 { names.push("Ctrl"); }
    if modifiers & MOD_ALT_BIT != 0 { names.push("Alt"); }
    if modifiers & MOD_SHIFT_BIT != 0 { names.push("Shift"); }
    if modifiers & MOD_WIN_BIT != 0 { names.push("Win"); }
    let canonical = format!("{}{}", names.join("+"), if names.is_empty() { key_name.clone() } else { format!("+{}", key_name) });

    Ok(ParsedHotkey { vk, modifiers, canonical })
}

/* ============== registration status registry ==============
 *
 * The platform worker records the outcome of every RegisterHotKey attempt so
 * the GUI (`check_hotkeys`) can surface what actually happened instead of a
 * silent log line. Cleared when the worker unregisters on shutdown.
 */

use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct RegOutcome {
    pub combo: String,
    pub ok: bool,
    pub error: Option<String>,
}

static REG_OUTCOMES: Mutex<Vec<(u32, RegOutcome)>> = Mutex::new(Vec::new());

/// Called by the platform worker right after each RegisterHotKey attempt.
pub fn record_registration(id: u32, combo: &str, result: StdResult<(), String>) {
    let outcome = RegOutcome {
        combo: combo.to_string(),
        ok: result.is_ok(),
        error: result.err(),
    };
    if let Ok(mut v) = REG_OUTCOMES.lock() {
        v.retain(|(i, _)| *i != id);
        v.push((id, outcome));
    }
}

/// Called when the worker tears down — stale "ok" records must not survive it.
pub fn clear_registrations() {
    if let Ok(mut v) = REG_OUTCOMES.lock() {
        v.clear();
    }
}

/// Snapshot of the worker's recorded outcomes (id → outcome).
pub fn outcomes() -> Vec<(u32, RegOutcome)> {
    REG_OUTCOMES
        .lock()
        .map(|v| v.clone())
        .unwrap_or_default()
}

/// What a live probe of the combo says about who holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Probe {
    /// RegisterHotKey succeeded → nobody holds this combo right now.
    Free,
    /// ERROR_HOTKEY_ALREADY_REGISTERED → someone (us or another app) holds it.
    Busy,
    /// RegisterHotKey rejected the combo itself (e.g. invalid vk).
    Failed(String),
    /// Probe could not run (non-Windows) → no information.
    Unavailable,
}

#[cfg(windows)]
fn probe_combo(vk: u32, modifiers: u32) -> Probe {
    windows_impl::probe(vk, modifiers)
}
#[cfg(not(windows))]
fn probe_combo(_vk: u32, _modifiers: u32) -> Probe {
    Probe::Unavailable
}

/// One row of the `check_hotkeys` report.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HotkeyCheck {
    pub id: u32,
    /// Combo as currently configured ("" = feature disabled).
    pub combo: String,
    /// Canonical form actually registered (what the UI should display).
    pub canonical: Option<String>,
    /// "ok" | "conflict" | "unsupported" | "failed" | "lost" | "not_registered" | "disabled"
    pub status: String,
    pub detail: Option<String>,
}

/// Combine config intent + the worker's recorded outcome + a live probe into a
/// per-hotkey verdict. `want` is [(id, configured_combo)].
pub fn check(want: &[(u32, String)]) -> Vec<HotkeyCheck> {
    let recorded: Vec<(u32, RegOutcome)> = REG_OUTCOMES
        .lock()
        .map(|v| v.clone())
        .unwrap_or_default();

    want.iter()
        .map(|(id, combo)| {
            let combo = combo.trim().to_string();
            if combo.is_empty() {
                return HotkeyCheck {
                    id: *id,
                    combo,
                    canonical: None,
                    status: "disabled".into(),
                    detail: None,
                };
            }
            let parsed = match parse_hotkey(&combo) {
                Ok(p) => p,
                Err(e) => {
                    return HotkeyCheck {
                        id: *id,
                        combo,
                        canonical: None,
                        status: "unsupported".into(),
                        detail: Some(e),
                    };
                }
            };
            // Only trust a recorded outcome for THIS combo — a pending edit
            // would otherwise inherit the previous combo's verdict.
            let rec = recorded
                .iter()
                .find(|(i, r)| *i == *id && r.combo == combo)
                .map(|(_, r)| r.clone());
            let probe = probe_combo(parsed.vk, parsed.modifiers);
            let (status, detail) = match (rec, probe) {
                (Some(r), _) if !r.ok => (
                    "failed",
                    r.error.or_else(|| Some("registration failed".into())),
                ),
                (Some(_), Probe::Free) => (
                    "lost",
                    Some("combo is free but our registration is missing".into()),
                ),
                (Some(_), _) => ("ok", None),
                (None, Probe::Busy) => (
                    "conflict",
                    Some("combo is held by another application".into()),
                ),
                (None, Probe::Failed(e)) => ("failed", Some(e)),
                (None, _) => (
                    "not_registered",
                    Some("service is not holding this hotkey".into()),
                ),
            };
            HotkeyCheck {
                id: *id,
                combo,
                canonical: Some(parsed.canonical),
                status: status.into(),
                detail,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> StdResult<ParsedHotkey, String> {
        parse_hotkey(s)
    }

    #[test]
    fn defaults_parse() {
        let p = parse("Ctrl+Alt+Space").unwrap();
        assert_eq!(p.vk, 0x20);
        assert_eq!(p.modifiers, MOD_CONTROL_BIT | MOD_ALT_BIT);
        assert_eq!(p.canonical, "Ctrl+Alt+Space");
    }

    #[test]
    fn function_and_special_keys() {
        assert_eq!(parse("Ctrl+F5").unwrap().vk, 0x74);
        assert_eq!(parse("Ctrl+Alt+F12").unwrap().vk, 0x7B);
        assert_eq!(parse("Ctrl+Shift+Home").unwrap().vk, 0x24);
        assert_eq!(parse("Win+Alt+Delete").unwrap().vk, 0x2E);
        assert_eq!(parse("Ctrl+Alt+ArrowUp").unwrap_err().to_string(), "unsupported key part: ArrowUp");
        assert_eq!(parse("Ctrl+Up").unwrap().vk, 0x26);
    }

    #[test]
    fn oem_symbols_use_real_vk_codes() {
        // The old parser wrote ASCII ';' (0x3B) — VK_OEM_1 is 0xBA.
        assert_eq!(parse("Ctrl+Alt+;").unwrap().vk, 0xBA);
        assert_eq!(parse("Ctrl+Alt+,").unwrap().vk, 0xBC);
        assert_eq!(parse("Ctrl+Alt+.").unwrap().vk, 0xBE);
        assert_eq!(parse("Ctrl+Shift+/").unwrap().vk, 0xBF);
        assert_eq!(parse("Ctrl+-").unwrap().vk, 0xBD);
        assert_eq!(parse("Ctrl+=").unwrap().vk, 0xBB);
    }

    #[test]
    fn numpad_keys() {
        assert_eq!(parse("Ctrl+Numpad5").unwrap().vk, 0x65);
        // A trailing '+' splits away — "NumAdd" is the reachable spelling.
        assert_eq!(parse("Ctrl+Alt+NumAdd").unwrap().vk, 0x6B);
        assert_eq!(parse("Ctrl+Alt+NumPad.").unwrap().vk, 0x6E);
    }

    #[test]
    fn errors_are_explicit() {
        assert!(parse("").is_err());
        assert!(parse("Ctrl+Alt").is_err());           // no main key
        assert!(parse("Ctrl+Alt+").is_err());          // trailing + — empty main key
        assert!(parse("Ctrl+Alt+Space+Q").is_err());   // two main keys
        assert!(parse("Ctrl+F25").is_err());           // out of range
        assert!(parse("Ctrl+Alt+空格").is_err());       // unsupported glyph
    }

    #[test]
    fn canonical_orders_modifiers() {
        assert_eq!(parse("shift+ctrl+f9").unwrap().canonical, "Ctrl+Shift+F9");
        assert_eq!(parse("Win+q").unwrap().canonical, "Win+Q");
    }

    /// Every canonical name the parser emits (what the UI shows/saves) must
    /// parse back to the same vk+modifiers — the recorder writes canonical
    /// strings, so a non-round-tripping name would brick on next load.
    #[test]
    fn canonical_names_round_trip() {
        for combo in [
            "Ctrl+Alt+Space", "Ctrl+Enter", "Alt+Tab", "Ctrl+Shift+Esc",
            "Ctrl+Backspace", "Alt+Delete", "Ctrl+Insert", "Win+Home",
            "Ctrl+End", "Alt+PageUp", "Ctrl+PageDown", "Ctrl+Up", "Alt+Down",
            "Ctrl+Left", "Shift+Right", "Win+CapsLock", "Ctrl+NumLock",
            "Alt+ScrollLock", "Ctrl+PrintScreen", "Ctrl+Pause",
            "Ctrl+;", "Alt+/", "Ctrl+`", "Shift+[", "Ctrl+\\", "Alt+]",
            "Ctrl+'", "Win+-", "Ctrl+=", "Alt+,", "Ctrl+.",
            "Ctrl+Num0", "Alt+Num5", "Ctrl+Num9", "Shift+Num*",
            "Ctrl+NumAdd", "Alt+Num-", "Ctrl+Num.", "Win+Num/",
            "Ctrl+F1", "Alt+F12", "Ctrl+Shift+F24", "Ctrl+A", "Alt+9",
        ] {
            let p = parse_hotkey(combo)
                .unwrap_or_else(|e| panic!("canonical combo {} failed to re-parse: {}", combo, e));
            assert_eq!(p.canonical, combo, "round-trip mismatch for {}", combo);
        }
    }
}
