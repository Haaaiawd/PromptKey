// P2/P3: Inject server on Unix domain sockets — same wire protocol as the
// Windows named-pipe version:
//   INJECT_PROMPT:{id}\n
//   INJECT_PROMPT:{id}:VARS:{json}\n
//
// std-only (no tokio): the socket is a plain file accept loop on a dedicated
// thread, nonblocking accept + stop-flag polling, mirroring the pipe version's
// shutdown semantics (review F01/F28/F50/F64).
//
// The socket lives in platform::ipc_runtime_dir() — $XDG_RUNTIME_DIR or a
// uid-scoped /tmp dir — bound 0700/0600 so another local user can't trigger
// injections.

use std::io::Read;
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Same framing bound as the Windows server — read until '\n' or EOF.
const MAX_MESSAGE_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone)]
pub struct InjectionRequest {
    pub prompt_id: i32,
    /// Optional JSON object of {"var_name": "value"} pairs
    pub vars_json: Option<String>,
}

/// Start the inject socket server in a background thread.
/// `stop` is shared with the engine loop; when set, the accept loop exits and
/// the socket file is removed so a restarted engine can rebind cleanly.
pub fn start(stop: Arc<AtomicBool>) -> mpsc::Receiver<InjectionRequest> {
    let (tx, rx) = mpsc::channel::<InjectionRequest>();

    thread::spawn(move || {
        let path = crate::platform::inject_endpoint();
        log::info!("[InjectServer] Binding unix socket: {}", path);

        // A stale socket file (previous crash/kill) blocks bind — remove it.
        // Only our own file can sit at this path (dir is 0700, uid-scoped).
        let _ = std::fs::remove_file(&path);
        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(e) => {
                log::error!("[InjectServer] bind {} failed: {}", path, e);
                return;
            }
        };
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        if let Err(e) = listener.set_nonblocking(true) {
            log::error!("[InjectServer] set_nonblocking failed: {}", e);
            return;
        }

        while !stop.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut stream, _)) => serve(&mut stream, &tx),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(20));
                }
                Err(e) => {
                    log::error!("[InjectServer] accept failed: {}", e);
                    thread::sleep(Duration::from_millis(200));
                }
            }
        }

        let _ = std::fs::remove_file(&path);
        log::info!("[InjectServer] Stop requested; socket removed");
    });

    rx
}

/// Read one framed line (bounded), parse it, validate VARS json, forward.
fn serve(stream: &mut UnixStream, tx: &mpsc::Sender<InjectionRequest>) {
    // Blocking reads are fine: the client writes a full line then closes.
    // read_timeout guards against a client that connects and stalls forever.
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));

    let mut buf: Vec<u8> = Vec::with_capacity(8192);
    let mut chunk = [0u8; 8192];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.len() > MAX_MESSAGE_BYTES {
                    log::warn!("[InjectServer] Message exceeds {} bytes; dropped", MAX_MESSAGE_BYTES);
                    return;
                }
                if buf.contains(&b'\n') {
                    break;
                }
            }
            Err(e) => {
                log::warn!("[InjectServer] read failed: {}", e);
                return;
            }
        }
    }

    if buf.is_empty() {
        return;
    }
    let message = String::from_utf8_lossy(&buf);
    log::debug!("[InjectServer] Received: {}", message.trim());

    // Wire format must stay in lockstep with the Windows parse_message in
    // inject_server.rs — same prefixes, same VARS validation.
    if let Some(req) = parse_message(&message) {
        let bad_vars = req
            .vars_json
            .as_deref()
            .map(|vj| serde_json::from_str::<serde_json::Value>(vj).is_err())
            .unwrap_or(false);
        if bad_vars {
            log::warn!("[InjectServer] Rejected malformed VARS json for prompt {}", req.prompt_id);
            return;
        }
        log::info!("[InjectServer] Valid prompt_id received: {}", req.prompt_id);
        let _ = tx.send(req);
    } else {
        log::warn!("[InjectServer] Invalid message format: {}", message.trim());
    }
}

fn parse_message(msg: &str) -> Option<InjectionRequest> {
    let trimmed = msg.trim();
    let rest = trimmed.strip_prefix("INJECT_PROMPT:")?;
    let (id_str, vars_json) = match rest.find(":VARS:") {
        Some(idx) => (&rest[..idx], Some(rest[idx + 6..].to_string())),
        None => (rest, None),
    };
    let prompt_id = id_str.parse::<i32>().ok()?;
    Some(InjectionRequest { prompt_id, vars_json })
}
