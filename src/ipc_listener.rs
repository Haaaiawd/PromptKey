// IPC Listener Module - GUI Server for Service Communication
// T1-010: Implement IPC Listener in GUI
//
// Same line-framed protocol on every OS; only the transport differs:
//   Windows — named pipe (tokio::net::windows::named_pipe)
//   Unix    — unix domain socket (tokio::net::UnixListener)
// The endpoint itself comes from service::platform::selector_endpoint().

use serde::Serialize;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;
use tokio::io::AsyncReadExt;

/// Commands are one line; read until '\n' or EOF, bounded so a hostile or
/// confused client cannot grow the buffer forever.
const MAX_MESSAGE_BYTES: usize = 8192;

/// Liveness + last-activity record for the `diagnose_hotkey_pipeline` command.
/// println!/eprintln! are invisible in a windows_subsystem release build —
/// this state is the only place a broken link leaves a trace.
#[derive(Debug, Clone, Serialize)]
pub struct PipeListenerState {
    /// Task is inside its accept loop (a live or retrying listener).
    pub alive: bool,
    /// Named-pipe instances created since app start.
    pub instances_created: u64,
    /// Client connections served since app start.
    pub connects_served: u64,
    /// Non-empty messages received since app start.
    pub messages_handled: u64,
    pub last_message: Option<String>,
    /// Unix epoch ms.
    pub last_message_at_ms: Option<u64>,
    pub last_error: Option<String>,
    /// Unix epoch ms of last_error.
    pub last_error_at_ms: Option<u64>,
}

impl PipeListenerState {
    const fn new() -> Self {
        PipeListenerState {
            alive: false,
            instances_created: 0,
            connects_served: 0,
            messages_handled: 0,
            last_message: None,
            last_message_at_ms: None,
            last_error: None,
            last_error_at_ms: None,
        }
    }
}

static STATE: Mutex<PipeListenerState> = Mutex::new(PipeListenerState::new());

fn epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn update(f: impl FnOnce(&mut PipeListenerState)) {
    if let Ok(mut g) = STATE.lock() {
        f(&mut g);
    }
}

fn record_error(e: impl std::fmt::Display) {
    let s = e.to_string();
    eprintln!("[IPC] {}", s);
    update(|st| {
        st.last_error = Some(s);
        st.last_error_at_ms = Some(epoch_ms());
    });
}

/// Snapshot for `diagnose_hotkey_pipeline`.
pub fn listener_state() -> PipeListenerState {
    STATE.lock().map(|g| g.clone()).unwrap_or(PipeListenerState {
        alive: false,
        instances_created: 0,
        connects_served: 0,
        messages_handled: 0,
        last_message: None,
        last_message_at_ms: None,
        last_error: Some("listener state lock poisoned".into()),
        last_error_at_ms: None,
    })
}

/// One inbound command line → action. Shared by both transports.
fn handle_message(msg: &str, app: &AppHandle) {
    if msg.is_empty() {
        return;
    }
    println!("[IPC] Received: {}", msg);
    update(|st| {
        st.messages_handled += 1;
        st.last_message = Some(msg.to_string());
        st.last_message_at_ms = Some(epoch_ms());
    });
    if msg == "SHOW_WHEEL" {
        // Phase 2: wheel window follows the cursor (positioned in
        // main.rs). present_wheel records its own outcome.
        if let Err(e) = crate::present_wheel(app) {
            record_error(format_args!("present_wheel failed: {}", e));
        }
    }
}

pub fn start_ipc_listener(app: AppHandle) {
    let endpoint = service::platform::selector_endpoint();
    tauri::async_runtime::spawn(async move {
        println!("[IPC] Starting listener on {}", endpoint);
        run(app, &endpoint).await;
    });
}

/* ---------------- Windows: named-pipe server ---------------- */

#[cfg(windows)]
async fn run(app: AppHandle, endpoint: &str) {
    use tokio::net::windows::named_pipe::ServerOptions;

    loop {
        // Create a fresh pipe instance for the next connection.
        // first_pipe_instance(false) is REQUIRED here: true sets
        // FILE_FLAG_FIRST_PIPE_INSTANCE, which fails on every create
        // after the first — this loop creates one instance per
        // connection, so only the non-first variant can work.
        let mut server = match ServerOptions::new()
            .first_pipe_instance(false)
            .create(endpoint)
        {
            Ok(s) => s,
            Err(e) => {
                // Retrying forever is fine — but only via STATE is a
                // persistently-failing create() visible to the user.
                record_error(format_args!("create {} failed: {}", endpoint, e));
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };
        update(|st| {
            st.alive = true;
            st.instances_created += 1;
        });

        // Wait for client to connect
        if let Err(e) = server.connect().await {
            record_error(format_args!("connect failed: {}", e));
            continue;
        }
        update(|st| st.connects_served += 1);
        println!("[IPC] Client connected");

        // Read the command line: until '\n' or EOF, bounded. A byte-mode
        // pipe can split a short write; a single read() is not a frame.
        let mut buf: Vec<u8> = Vec::with_capacity(128);
        let mut chunk = [0u8; 512];
        loop {
            match server.read(&mut chunk).await {
                Ok(0) => break, // client closed
                Ok(n) => {
                    buf.extend_from_slice(&chunk[..n]);
                    if buf.contains(&b'\n') || buf.len() >= MAX_MESSAGE_BYTES {
                        break;
                    }
                }
                Err(e) => {
                    record_error(format_args!("read failed: {}", e));
                    break;
                }
            }
        }

        let msg = String::from_utf8_lossy(&buf).trim().to_string();
        handle_message(&msg, &app);
        // The served instance is dropped here; the next loop iteration
        // creates the next listening instance.
    }
}

/* ---------------- Unix: domain-socket server ---------------- */

#[cfg(unix)]
async fn run(app: AppHandle, endpoint: &str) {
    use tokio::net::UnixListener;

    // A stale socket file blocks bind; only our own uid can sit here
    // (the runtime dir is created 0700 by service::platform).
    let _ = std::fs::remove_file(endpoint);
    let listener = match UnixListener::bind(endpoint) {
        Ok(l) => l,
        Err(e) => {
            record_error(format_args!("bind {} failed: {}", endpoint, e));
            return;
        }
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(endpoint, std::fs::Permissions::from_mode(0o600));
    }
    update(|st| st.alive = true);
    st_instances_tick();

    loop {
        let (mut stream, _) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                record_error(format_args!("accept failed: {}", e));
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };
        update(|st| st.connects_served += 1);

        let mut buf: Vec<u8> = Vec::with_capacity(128);
        let mut chunk = [0u8; 512];
        loop {
            match stream.read(&mut chunk).await {
                Ok(0) => break,
                Ok(n) => {
                    buf.extend_from_slice(&chunk[..n]);
                    if buf.contains(&b'\n') || buf.len() >= MAX_MESSAGE_BYTES {
                        break;
                    }
                }
                Err(e) => {
                    record_error(format_args!("read failed: {}", e));
                    break;
                }
            }
        }

        let msg = String::from_utf8_lossy(&buf).trim().to_string();
        handle_message(&msg, &app);
    }
}

/// Keep `instances_created` meaningful on unix (one listener, many accepts).
#[cfg(unix)]
fn st_instances_tick() {
    update(|st| st.instances_created += 1);
}
