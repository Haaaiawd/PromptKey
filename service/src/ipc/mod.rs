// IPC Client Module - Service → GUI Communication via Named Pipe
// T1-006: Quick Selection Panel IPC Layer

use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// IPC Client for sending messages to GUI via Named Pipe
pub struct IPCClient {
    pipe_name: String,
    last_send: Mutex<Option<Instant>>,
}

impl IPCClient {
    /// Create a new IPC client with the specified pipe name
    pub fn new(pipe_name: String) -> Self {
        IPCClient {
            pipe_name,
            last_send: Mutex::new(None),
        }
    }

    /// Default constructor using standard pipe name
    pub fn default() -> Self {
        Self::new("\\\\.\\pipe\\promptkey_selector".to_string())
    }

    /// Send "show selector" command to GUI
    /// Includes 500ms debounce to prevent spam
    pub fn send_show_selector(&self) -> Result<(), Box<dyn Error>> {
        // Debounce: check if 500ms has passed since last send
        {
            let mut last = self.last_send.lock().unwrap();
            if let Some(last_time) = *last {
                let elapsed = last_time.elapsed();
                if elapsed < Duration::from_millis(500) {
                    log::debug!("IPC send debounced ({}ms since last)", elapsed.as_millis());
                    return Ok(()); // Debounced, silently ignore
                }
            }
            *last = Some(Instant::now());
        }

        // Send message via Named Pipe
        match OpenOptions::new().write(true).open(&self.pipe_name) {
            Ok(mut pipe) => {
                let message = "SHOW_SELECTOR\n";
                pipe.write_all(message.as_bytes())?;
                log::info!("IPC: Sent SHOW_SELECTOR to GUI via {}", self.pipe_name);
                Ok(())
            }
            Err(e) => {
                // Non-critical: GUI might not be running or pipe not ready
                log::warn!("IPC: Failed to open named pipe '{}': {}", self.pipe_name, e);
                Err(Box::new(e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_debounce() {
        let client = IPCClient::new("\\\\.\\pipe\\test_pipe".to_string());

        // First send updates timestamp
        let _ = client.send_show_selector();

        // Immediate second send should be debounced
        thread::sleep(Duration::from_millis(100));
        let _ = client.send_show_selector();

        // After 500ms, should allow send
        thread::sleep(Duration::from_millis(450));
        let _ = client.send_show_selector();
    }
}
