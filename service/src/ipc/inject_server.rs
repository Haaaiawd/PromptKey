// TW001: Inject Pipe Server (Robust Tokio Implementation)
// Listens on \\.\pipe\promptkey_inject for messages:
//   INJECT_PROMPT:{id}\n
//   INJECT_PROMPT:{id}:VARS:{json}\n   (Phase 2 D5 — collected {{var}} values)

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use tokio::io::AsyncReadExt;
use tokio::net::windows::named_pipe::ServerOptions;
use tokio::runtime::Runtime;

const PIPE_NAME: &str = r"\\.\pipe\promptkey_inject";
/// Messages are framed by a trailing '\n'; a client write may still arrive in
/// several chunks on a byte-mode pipe, so read until the newline or EOF.
/// (review F10/F33/F45/F52/F68 — a single 8192-byte read truncated var JSON)
const MAX_MESSAGE_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone)]
pub struct InjectionRequest {
    pub prompt_id: i32,
    /// Optional JSON object of {"var_name": "value"} pairs
    pub vars_json: Option<String>,
}

/// Start the inject pipe server in a background thread.
/// `stop` is shared with the engine loop; when set, the accept loop exits and
/// the pipe listener is torn down so a restarted engine can rebind it.
pub fn start(stop: Arc<AtomicBool>) -> mpsc::Receiver<InjectionRequest> {
    let (tx, rx) = mpsc::channel::<InjectionRequest>();

    thread::spawn(move || {
        log::info!("[InjectServer] Background thread started");

        // Create a local tokio runtime for this thread
        let rt = match Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                log::error!("[InjectServer] Failed to create tokio runtime: {}", e);
                return;
            }
        };

        rt.block_on(async move {
            loop {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let watch = watch_stop(stop.clone());
                tokio::select! {
                    r = listen_once(&tx) => {
                        if let Err(e) = r {
                            log::error!("[InjectServer] Loop error: {}", e);
                            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                        }
                    }
                    _ = watch => {
                        log::info!("[InjectServer] Stop requested; closing listener");
                        break;
                    }
                }
            }
        });
    });

    rx
}

/// Resolves once the engine shutdown flag is set.
async fn watch_stop(stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Relaxed) {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

async fn listen_once(tx: &mpsc::Sender<InjectionRequest>) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("[InjectServer] Creating named pipe: {}", PIPE_NAME);

    // Create a new pipe instance
    let mut server = ServerOptions::new()
        .first_pipe_instance(true)
        .create(PIPE_NAME)?;

    log::info!("[InjectServer] Waiting for client connection...");
    server.connect().await?;

    log::info!("[InjectServer] Client connected, reading message...");

    // Frame is one line. A byte-mode pipe can deliver the client's write in
    // several chunks, so read until the trailing '\n' or EOF (bounded).
    let mut buf: Vec<u8> = Vec::with_capacity(8192);
    let mut chunk = [0u8; 8192];
    loop {
        let n = server.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() > MAX_MESSAGE_BYTES {
            log::warn!("[InjectServer] Message exceeds {} bytes; dropped", MAX_MESSAGE_BYTES);
            return Ok(());
        }
        if buf.contains(&b'\n') {
            break;
        }
    }

    if !buf.is_empty() {
        let message = String::from_utf8_lossy(&buf);
        log::debug!("[InjectServer] Received: {}", message.trim());

        if let Some(req) = parse_message(&message) {
            // Reject malformed var JSON up front instead of silently injecting
            // unresolved placeholders (review F10/F33/F45/F52/F68).
            if let Some(vj) = req.vars_json.as_deref() {
                if serde_json::from_str::<serde_json::Value>(vj).is_err() {
                    log::warn!("[InjectServer] Rejected malformed VARS json for prompt {}", req.prompt_id);
                    return Ok(());
                }
            }
            log::info!("[InjectServer] Valid prompt_id received: {}", req.prompt_id);
            let _ = tx.send(req);
        } else {
            log::warn!("[InjectServer] Invalid message format: {}", message.trim());
        }
    }

    Ok(())
}

fn parse_message(msg: &str) -> Option<InjectionRequest> {
    let trimmed = msg.trim();
    let rest = trimmed.strip_prefix("INJECT_PROMPT:")?;
    // Optional ":VARS:{json}" suffix
    let (id_str, vars_json) = match rest.find(":VARS:") {
        Some(idx) => (&rest[..idx], Some(rest[idx + 6..].to_string())),
        None => (rest, None),
    };
    let prompt_id = id_str.parse::<i32>().ok()?;
    Some(InjectionRequest { prompt_id, vars_json })
}
