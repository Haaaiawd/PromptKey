// IPC Client Module - Service → GUI Communication via Named Pipe
// T1-006: Quick Selection Panel IPC Layer

#[cfg(windows)]
pub mod inject_server; // TW001: Inject pipe server

/// Phase 2 Task8: named pipes are Windows-only. The stub keeps the service
/// compiling on other platforms and simply never yields a request.
#[cfg(not(windows))]
pub mod inject_server {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc;

    #[derive(Debug, Clone)]
    pub struct InjectionRequest {
        pub prompt_id: i32,
        pub vars_json: Option<String>,
    }

    pub fn start(_stop: Arc<AtomicBool>) -> mpsc::Receiver<InjectionRequest> {
        let (_tx, rx) = mpsc::channel::<InjectionRequest>();
        rx
    }
}

use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Snapshot of the last service→GUI wheel-send attempt. The service is a
/// linked lib inside the GUI process, so `diagnose_hotkey_pipeline` reads this
/// directly — in a release build (windows_subsystem) log lines go nowhere and
/// this record is the only trace the send leaves.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SendRecord {
    /// Unix epoch ms of the attempt.
    pub at_ms: u64,
    /// "sent" | "debounced" | "open_failed" | "write_failed"
    pub kind: String,
    /// Error text for *_failed kinds.
    pub detail: Option<String>,
}

static LAST_WHEEL_SEND: Mutex<Option<SendRecord>> = Mutex::new(None);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn record_wheel_send(kind: &str, detail: Option<String>) {
    if let Ok(mut g) = LAST_WHEEL_SEND.lock() {
        *g = Some(SendRecord {
            at_ms: now_ms(),
            kind: kind.into(),
            detail,
        });
    }
}

/// Read by `diagnose_hotkey_pipeline` in the GUI.
pub fn last_wheel_send() -> Option<SendRecord> {
    LAST_WHEEL_SEND.lock().ok().and_then(|g| g.clone())
}

/// Debounce window for repeat hotkey presses.
const DEBOUNCE: Duration = Duration::from_millis(500);
/// How long `open` retries while the GUI listener is between pipe instances.
const OPEN_BUDGET: Duration = Duration::from_millis(900);
const OPEN_STEP: Duration = Duration::from_millis(40);
/// Win32 codes that mean "pipe exists conceptually, try again":
///   2   ERROR_FILE_NOT_FOUND — zero instances (listener mid-recreate)
///   231 ERROR_PIPE_BUSY     — every instance is connected to a client
/// Anything else (access denied, invalid name) fails immediately.
const RETRYABLE_OPEN_CODES: [i32; 2] = [2, 231];

/// Open the pipe for writing, retrying the transient "no listening instance"
/// window. The GUI listener drops each served instance and creates the next
/// one inside the same loop iteration — a press landing in that gap used to
/// die on ERROR_FILE_NOT_FOUND after a single attempt.
fn open_pipe_write(pipe_name: &str, budget: Duration) -> std::io::Result<std::fs::File> {
    let deadline = Instant::now() + budget;
    loop {
        match OpenOptions::new().write(true).open(pipe_name) {
            Ok(f) => return Ok(f),
            Err(e) => {
                let retryable =
                    matches!(e.raw_os_error(), Some(c) if RETRYABLE_OPEN_CODES.contains(&c));
                if !retryable || Instant::now() >= deadline {
                    return Err(e);
                }
                std::thread::sleep(OPEN_STEP);
            }
        }
    }
}

/// IPC Client for sending messages to GUI via Named Pipe
pub struct IPCClient {
    pipe_name: String,
    last_send: Mutex<Option<Instant>>,
    open_budget: Duration,
}

impl IPCClient {
    /// Create a new IPC client with the specified pipe name
    pub fn new(pipe_name: String) -> Self {
        IPCClient {
            pipe_name,
            last_send: Mutex::new(None),
            open_budget: OPEN_BUDGET,
        }
    }

    /// Default constructor using standard pipe name
    pub fn default() -> Self {
        Self::new("\\\\.\\pipe\\promptkey_selector".to_string())
    }

    /// Debounced send. Returns Ok(()) when debounced (the previous send still
    /// counts as the effective one). The debounce latch is only set after a
    /// successful write — a failed send must not swallow the user's next press.
    fn send_line(&self, message: &str) -> Result<(), Box<dyn Error>> {
        {
            let last = self.last_send.lock().unwrap();
            if let Some(t) = *last {
                let elapsed = t.elapsed();
                if elapsed < DEBOUNCE {
                    log::debug!("IPC send debounced ({}ms since last)", elapsed.as_millis());
                    return Ok(());
                }
            }
        }
        let mut pipe = open_pipe_write(&self.pipe_name, self.open_budget)?;
        pipe.write_all(message.as_bytes())?;
        pipe.flush()?;
        *self.last_send.lock().unwrap() = Some(Instant::now());
        Ok(())
    }

    /// Send "show selector" command to GUI (legacy — selector panel is gone,
    /// kept for the shared send path).
    pub fn send_show_selector(&self) -> Result<(), Box<dyn Error>> {
        match self.send_line("SHOW_SELECTOR\n") {
            Ok(()) => {
                log::info!("IPC: Sent SHOW_SELECTOR to GUI via {}", self.pipe_name);
                Ok(())
            }
            Err(e) => {
                log::warn!("IPC: Failed to reach '{}': {}", self.pipe_name, e);
                Err(e)
            }
        }
    }

    /// TW013: Send "show wheel" command to GUI.
    /// Every attempt is recorded in `last_wheel_send()` — `let _ =` on the
    /// caller side used to make a broken pipe read as "hotkey does nothing".
    pub fn send_show_wheel(&self) -> Result<(), Box<dyn Error>> {
        {
            let last = self.last_send.lock().unwrap();
            if let Some(t) = *last {
                if t.elapsed() < DEBOUNCE {
                    record_wheel_send("debounced", None);
                    return Ok(());
                }
            }
        }
        match open_pipe_write(&self.pipe_name, self.open_budget) {
            Err(e) => {
                record_wheel_send("open_failed", Some(e.to_string()));
                log::warn!("IPC: Failed to open named pipe '{}': {}", self.pipe_name, e);
                Err(Box::new(e))
            }
            Ok(mut pipe) => match pipe
                .write_all(b"SHOW_WHEEL\n")
                .and_then(|()| pipe.flush())
            {
                Ok(()) => {
                    *self.last_send.lock().unwrap() = Some(Instant::now());
                    record_wheel_send("sent", None);
                    log::info!("IPC: Sent SHOW_WHEEL to GUI via {}", self.pipe_name);
                    Ok(())
                }
                Err(e) => {
                    record_wheel_send("write_failed", Some(e.to_string()));
                    log::warn!("IPC: write to '{}' failed: {}", self.pipe_name, e);
                    Err(Box::new(e))
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A failed send must NOT latch the debounce window: two presses in a row
    /// on a dead pipe must each attempt the open (and each return Err), never
    /// a silent Ok. Regression guard for the "press → nothing" symptom.
    #[test]
    fn failed_send_does_not_debounce() {
        let mut client = IPCClient::new("\\\\.\\pipe\\promptkey_test_no_listener".to_string());
        client.open_budget = Duration::ZERO; // tests must not wait 900ms
        let first = client.send_show_wheel();
        let second = client.send_show_wheel();
        assert!(first.is_err(), "first send to a dead pipe must fail");
        assert!(
            second.is_err(),
            "second send must retry, not be debounce-swallowed"
        );
        let rec = last_wheel_send().expect("attempt recorded");
        assert_eq!(rec.kind, "open_failed");
        assert!(rec.at_ms > 0);
    }

    /// Debounce still suppresses real repeat sends: after a successful send the
    /// next press within the window returns Ok without touching the pipe.
    /// (No live server exists to accept, so emulate via a recorded timestamp.)
    #[test]
    fn debounce_latch_only_on_success() {
        let client = IPCClient {
            pipe_name: "\\\\.\\pipe\\promptkey_test_no_listener".to_string(),
            last_send: Mutex::new(Some(Instant::now())),
            open_budget: Duration::ZERO,
        };
        // last_send just set → this press is debounced even though the pipe is dead.
        assert!(client.send_show_wheel().is_ok());
    }
}
