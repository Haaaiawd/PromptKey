// P2: foreground-window context on Linux/X11 via EWMH.
//
// Reads _NET_ACTIVE_WINDOW off the root window, then _NET_WM_NAME /
// _NET_WM_PID off the active window. Pure-Rust x11rb — no xprop/xdotool
// runtime dependency. On a Wayland session this still answers, but only sees
// XWayland clients; platform::status_json() reports that degradation.

use super::{AppContext, Context};
use std::result::Result as StdResult;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};

type AnyErr = Box<dyn std::error::Error>;

fn err(msg: impl Into<String>) -> AnyErr {
    std::io::Error::other(msg.into()).into()
}

pub struct X11Context;

impl Default for X11Context {
    fn default() -> Self {
        Self::new()
    }
}

impl X11Context {
    pub fn new() -> Self {
        X11Context
    }

    fn atom(conn: &impl ConnectionExt, name: &[u8]) -> StdResult<u32, AnyErr> {
        Ok(conn
            .intern_atom(false, name)?
            .reply()?
            .atom)
    }

    fn string_property(
        conn: &impl ConnectionExt,
        win: u32,
        prop: u32,
    ) -> StdResult<Option<String>, AnyErr> {
        let reply = conn
            .get_property(false, win, prop, AtomEnum::ANY, 0, 1024)?
            .reply()?;
        if reply.value.is_empty() {
            return Ok(None);
        }
        Ok(Some(String::from_utf8_lossy(&reply.value).into_owned()))
    }

    fn u32_property(
        conn: &impl ConnectionExt,
        win: u32,
        prop: u32,
    ) -> StdResult<Option<u32>, AnyErr> {
        let reply = conn
            .get_property(false, win, prop, AtomEnum::CARDINAL, 0, 1)?
            .reply()?;
        let v = reply
            .value32()
            .and_then(|mut it| it.next());
        Ok(v)
    }

    /// `/proc/<pid>/comm` — the process's short name, matching what Windows
    /// reports as `process_name` (executable filename).
    fn process_name(pid: u32) -> String {
        std::fs::read_to_string(format!("/proc/{}/comm", pid))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| format!("pid:{}", pid))
    }
}

impl Context for X11Context {
    fn get_foreground_context(&self) -> StdResult<AppContext, AnyErr> {
        let (conn, screen_num) = x11rb::connect(None)
            .map_err(|e| err(format!("cannot connect to X display: {}", e)))?;
        let root = conn.setup().roots[screen_num].root;

        let net_active = Self::atom(&conn, b"_NET_ACTIVE_WINDOW")?;
        let net_wm_name = Self::atom(&conn, b"_NET_WM_NAME")?;
        let net_wm_pid = Self::atom(&conn, b"_NET_WM_PID")?;

        let active = conn
            .get_property(false, root, net_active, AtomEnum::WINDOW, 0, 1)?
            .reply()?;
        let win = active
            .value32()
            .and_then(|mut it| it.next())
            .filter(|w| *w != 0)
            .ok_or_else(|| err("no active X11 window (Wayland-native focus?)"))?;

        // Prefer _NET_WM_NAME (UTF-8); fall back to legacy WM_NAME.
        let title = Self::string_property(&conn, win, net_wm_name)?
            .or_else(|| {
                Self::string_property(&conn, win, AtomEnum::WM_NAME.into())
                    .ok()
                    .flatten()
            })
            .unwrap_or_default();

        let pid = Self::u32_property(&conn, win, net_wm_pid)?.unwrap_or(0);
        let process_name = if pid != 0 {
            Self::process_name(pid)
        } else {
            String::new()
        };

        Ok(AppContext {
            process_name,
            window_title: title,
            // XID fits in u64 — the injector never dereferences it on X11,
            // it only needs a stable identifier for logging/scoping.
            window_handle: win as u64,
        })
    }
}
