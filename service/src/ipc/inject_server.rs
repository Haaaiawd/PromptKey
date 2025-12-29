// TW001: Inject Pipe Server
// Listens on \\.\\pipe\\promptkey_inject for INJECT_PROMPT:{id}\n messages

use std::fs::OpenOptions;
use std::io::Read;
use std::sync::mpsc;
use std::thread;

const PIPE_NAME: &str = r"\\.\pipe\promptkey_inject";

/// Start the inject pipe server in a background thread
/// Returns a receiver channel that yields prompt IDs when messages arrive
pub fn start() -> mpsc::Receiver<i32> {
    let (tx, rx) = mpsc::channel::<i32>();

    thread::spawn(move || {
        loop {
            match listen_once(&tx) {
                Ok(_) => {}
                Err(e) => log::error!("[InjectServer] Error: {}", e),
            }
            // Always restart listener after client disconnects
        }
    });

    rx
}

fn listen_once(tx: &mpsc::Sender<i32>) -> Result<(), Box<dyn std::error::Error>> {
    log::info!(
        "[InjectServer] Waiting for client connection on {}",
        PIPE_NAME
    );

    // Open named pipe (blocking until client connects)
    let mut pipe = OpenOptions::new().read(true).open(PIPE_NAME)?;

    log::debug!("[InjectServer] Client connected");

    // Read message
    let mut buffer = vec![0u8; 256];
    let n = pipe.read(&mut buffer)?;
    let message = String::from_utf8_lossy(&buffer[..n]);

    log::debug!("[InjectServer] Received: {}", message.trim());

    // Parse INJECT_PROMPT:{id}\n
    if let Some(prompt_id) = parse_message(&message) {
        log::info!("[InjectServer] Parsed prompt_id={}", prompt_id);
        let _ = tx.send(prompt_id); // Send to main loop
    } else {
        log::warn!("[InjectServer] Invalid message format: {}", message.trim());
    }

    Ok(())
}

/// Parse "INJECT_PROMPT:123\n" -> Some(123)
fn parse_message(msg: &str) -> Option<i32> {
    let trimmed = msg.trim();
    if let Some(id_str) = trimmed.strip_prefix("INJECT_PROMPT:") {
        id_str.parse::<i32>().ok()
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
