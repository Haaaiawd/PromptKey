// TW001: Inject Pipe Server (Robust Tokio Implementation)
// Listens on \\.\pipe\promptkey_inject for INJECT_PROMPT:{id}\n messages

use std::sync::mpsc;
use std::thread;
use tokio::io::AsyncReadExt;
use tokio::net::windows::named_pipe::ServerOptions;
use tokio::runtime::Runtime;

const PIPE_NAME: &str = r"\\.\pipe\promptkey_inject";

/// Start the inject pipe server in a background thread
pub fn start() -> mpsc::Receiver<i32> {
    let (tx, rx) = mpsc::channel::<i32>();

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

async fn listen_once(tx: &mpsc::Sender<i32>) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("[InjectServer] Creating named pipe: {}", PIPE_NAME);

    // Create a new pipe instance
    let mut server = ServerOptions::new()
        .first_pipe_instance(true)
        .create(PIPE_NAME)?;

    log::info!("[InjectServer] Waiting for client connection...");
    server.connect().await?;

    log::info!("[InjectServer] Client connected, reading message...");

    let mut buffer = [0u8; 256];
    let n = server.read(&mut buffer).await?;

    if n > 0 {
        let message = String::from_utf8_lossy(&buffer[..n]);
        log::debug!("[InjectServer] Received: {}", message.trim());

        if let Some(prompt_id) = parse_message(&message) {
            log::info!("[InjectServer] Valid prompt_id received: {}", prompt_id);
            let _ = tx.send(prompt_id);
        } else {
            log::warn!("[InjectServer] Invalid message format: {}", message.trim());
        }
    }

    Ok(())
}

fn parse_message(msg: &str) -> Option<i32> {
    let trimmed = msg.trim();
    if let Some(id_str) = trimmed.strip_prefix("INJECT_PROMPT:") {
        id_str.parse::<i32>().ok()
    } else {
        None
    }
}
