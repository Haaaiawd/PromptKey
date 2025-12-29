// TW001: Inject Pipe Server
// Listens on \\.\pipe\promptkey_inject for INJECT_PROMPT:{id}\n messages

use tokio::io::AsyncReadExt;
use tokio::net::windows::named_pipe::ServerOptions;
use tokio::sync::mpsc;

const PIPE_NAME: &str = r"\\.\pipe\promptkey_inject";

/// Start the inject pipe server in a background task
/// Returns a receiver channel that yields prompt IDs when messages arrive
pub fn start() -> mpsc::Receiver<i64> {
    let (tx, rx) = mpsc::channel::<i64>(32);

    tokio::spawn(async move {
        loop {
            match listen_once(&tx).await {
                Ok(_) => {}
                Err(e) => log::error!("[InjectServer] Error: {}", e),
            }
            // Always restart listener after client disconnects
        }
    });

    rx
}

async fn listen_once(tx: &mpsc::Sender<i64>) -> Result<(), Box<dyn std::error::Error>> {
    // Create named pipe server
    let mut server = ServerOptions::new()
        .first_pipe_instance(false)
        .create(PIPE_NAME)?;

    log::info!("[InjectServer] Listening on {}", PIPE_NAME);

    // Wait for client connection
    server.connect().await?;
    log::debug!("[InjectServer] Client connected");

    // Read message
    let mut buffer = vec![0u8; 256];
    let n = server.read(&mut buffer).await?;
    let message = String::from_utf8_lossy(&buffer[..n]);

    log::debug!("[InjectServer] Received: {}", message.trim());

    // Parse INJECT_PROMPT:{id}\n
    if let Some(prompt_id) = parse_message(&message) {
        log::info!("[InjectServer] Parsed prompt_id={}", prompt_id);
        let _ = tx.send(prompt_id).await; // Send to main loop
    } else {
        log::warn!("[InjectServer] Invalid message format: {}", message.trim());
    }

    Ok(())
}

/// Parse "INJECT_PROMPT:123\n" -> Some(123)
fn parse_message(msg: &str) -> Option<i64> {
    let trimmed = msg.trim();
    if let Some(id_str) = trimmed.strip_prefix("INJECT_PROMPT:") {
        id_str.parse::<i64>().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_message() {
        assert_eq!(parse_message("INJECT_PROMPT:123\n"), Some(123));
        assert_eq!(parse_message("INJECT_PROMPT:456"), Some(456));
        assert_eq!(parse_message("INVALID"), None);
        assert_eq!(parse_message("INJECT_PROMPT:abc"), None);
    }
}
