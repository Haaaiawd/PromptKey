// P2/P3: runtime platform & session detection + per-OS directories.
//
// Everything in this module is std-only — no platform crates — so it compiles
// everywhere. It is the single source of truth for:
//   - where config/db live on each OS (was: hardcoded %APPDATA%, Windows-only)
//   - where the IPC endpoints live (named pipe name on Windows, unix socket
//     path on Linux/macOS)
//   - which desktop session we are in (X11 vs Wayland vs native), so degraded
//     capabilities are reported honestly instead of discovered by failure.

use std::path::PathBuf;

/// Operating system label for reports/UI.
pub fn os() -> &'static str {
    #[cfg(target_os = "windows")]
    { "windows" }
    #[cfg(target_os = "macos")]
    { "macos" }
    #[cfg(target_os = "linux")]
    { "linux" }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    { "other" }
}

/// Which display/input session the desktop is running. Only meaningful on
/// Linux/BSD; Windows and macOS are always `Native`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Session {
    /// Windows / macOS — no X/Wayland split exists.
    Native,
    /// X11 (or XWayland-visible) session — full injection/hotkey support.
    X11,
    /// Wayland session — X11 paths may still work for XWayland clients only.
    Wayland,
    /// Could not determine (headless, missing env).
    Unknown,
}

pub fn session() -> Session {
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // XDG_SESSION_TYPE is the canonical signal; WAYLAND_DISPLAY catches
        // sessions that don't set it (older compositors).
        let st = std::env::var("XDG_SESSION_TYPE").unwrap_or_default().to_lowercase();
        if st == "wayland" {
            return Session::Wayland;
        }
        if st == "x11" {
            return Session::X11;
        }
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            return Session::Wayland;
        }
        if std::env::var_os("DISPLAY").is_some() {
            return Session::X11;
        }
        Session::Unknown
    }
    #[cfg(not(all(unix, not(target_os = "macos"))))]
    {
        Session::Native
    }
}

/// Human-readable summary of what works on this machine right now.
/// Consumed by the `get_platform_status` Tauri command — the settings page
/// renders it verbatim, so keep the `notes` machine-keyed and localize in JS.
pub fn status_json() -> serde_json::Value {
    let os = os();
    let session = session();
    let mut notes: Vec<String> = Vec::new();
    let mut injection = "ok";
    let mut hotkeys = "ok";
    let mut context = "ok";

    match (os, session) {
        ("linux", Session::Wayland) => {
            // X11 impls can still reach XWayland clients, but Wayland-native
            // windows are unreachable by design. That is a platform decision,
            // not a bug — we surface it instead of pretending full support.
            injection = "xwayland-only";
            hotkeys = "xwayland-only";
            context = "xwayland-only";
            notes.push("wayland_degraded".into());
        }
        ("linux", Session::Unknown) => {
            injection = "unavailable";
            hotkeys = "unavailable";
            context = "unavailable";
            notes.push("no_display".into());
        }
        ("macos", _) if !crate::injector::ax_is_trusted() => {
            injection = "needs-permission";
            context = "partial";
            notes.push("ax_permission_missing".into());
        }
        _ => {}
    }

    serde_json::json!({
        "os": os,
        "session": match session {
            Session::Native => "native",
            Session::X11 => "x11",
            Session::Wayland => "wayland",
            Session::Unknown => "unknown",
        },
        "injection": injection,
        "hotkeys": hotkeys,
        "context": context,
        "notes": notes,
    })
}

/* ---------- per-OS directories ---------- */

/// Application data/config directory, created on demand.
///   Windows : %APPDATA%\PromptKey
///   macOS   : ~/Library/Application Support/PromptKey
///   Linux   : ${XDG_CONFIG_HOME:-~/.config}/promptkey
pub fn app_config_dir() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        let appdata = std::env::var("APPDATA")
            .map_err(|e| format!("读取 APPDATA 失败: {}", e))?;
        return Ok(PathBuf::from(appdata).join("PromptKey"));
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").map_err(|e| format!("读取 HOME 失败: {}", e))?;
        return Ok(PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("PromptKey"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // XDG Base Directory: $XDG_CONFIG_HOME or ~/.config
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
            && !xdg.is_empty()
        {
            return Ok(PathBuf::from(xdg).join("promptkey"));
        }
        let home = std::env::var("HOME").map_err(|e| format!("读取 HOME 失败: {}", e))?;
        Ok(PathBuf::from(home).join(".config").join("promptkey"))
    }
}

/// Directory the IPC sockets live in (unix only).
/// Prefers $XDG_RUNTIME_DIR (per-user, 0700); falls back to a uid-scoped dir
/// under TMPDIR//tmp so multi-user systems don't collide.
#[cfg(unix)]
pub fn ipc_runtime_dir() -> Result<PathBuf, String> {
    let base = std::env::var("XDG_RUNTIME_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let tmp = std::env::var("TMPDIR")
                .ok()
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/tmp"));
            let uid = unsafe { libc::geteuid() };
            tmp.join(format!("promptkey-{}", uid))
        });
    std::fs::create_dir_all(&base).map_err(|e| format!("创建 IPC 目录失败: {}", e))?;
    // Runtime sockets must not be world-accessible — another local user
    // could otherwise trigger injections.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o700));
    }
    Ok(base)
}

/// Endpoint the service uses to tell the GUI "show the wheel".
/// Windows: named pipe. Unix: filesystem socket in the runtime dir.
pub fn selector_endpoint() -> String {
    #[cfg(windows)]
    {
        r"\\.\pipe\promptkey_selector".to_string()
    }
    #[cfg(unix)]
    {
        ipc_runtime_dir()
            .unwrap_or_else(|_| PathBuf::from("/tmp"))
            .join("promptkey_selector.sock")
            .to_string_lossy()
            .into_owned()
    }
}

/// Endpoint the GUI uses to send injection requests to the service.
pub fn inject_endpoint() -> String {
    #[cfg(windows)]
    {
        r"\\.\pipe\promptkey_inject".to_string()
    }
    #[cfg(unix)]
    {
        ipc_runtime_dir()
            .unwrap_or_else(|_| PathBuf::from("/tmp"))
            .join("promptkey_inject.sock")
            .to_string_lossy()
            .into_owned()
    }
}

/// Default SQLite location — next to config.yaml on every platform.
pub fn default_database_path() -> String {
    app_config_dir()
        .map(|d| d.join("promptmgr.db").to_string_lossy().into_owned())
        .unwrap_or_else(|_| "promptmgr.db".to_string())
}
