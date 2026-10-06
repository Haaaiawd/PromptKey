// TW001: Inject Pipe Server (Robust Tokio Implementation)
// Listens on \\.\pipe\promptkey_inject for messages:
//   INJECT_PROMPT:{id}\n
//   INJECT_PROMPT:{id}:VARS:{json}\n   (Phase 2 D5 — collected {{var}} values)

use std::sync::mpsc;
use std::thread;
use tokio::io::AsyncReadExt;
use tokio::net::windows::named_pipe::ServerOptions;
use tokio::runtime::Runtime;

const PIPE_NAME: &str = r"\\.\pipe\promptkey_inject";

#[derive(Debug, Clone)]
pub struct InjectionRequest {
    pub prompt_id: i32,
    /// Optional JSON object of {"var_name": "value"} pairs
    pub vars_json: Option<String>,
}

/// Start the inject pipe server in a background thread
pub fn start() -> mpsc::Receiver<InjectionRequest> {
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

        rt.block_on(async {
            loop {
                match listen_once(&tx).await {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("[InjectServer] Loop error: {}", e);
                        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    }
                }
            }
        });
    });

    rx
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

    let mut buffer = [0u8; 8192];
    let n = server.read(&mut buffer).await?;

    if n > 0 {
        let message = String::from_utf8_lossy(&buffer[..n]);
        log::debug!("[InjectServer] Received: {}", message.trim());

        if let Some(req) = parse_message(&message) {
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
