// IPC Listener Module - GUI Server for Service Communication
// T1-010: Implement IPC Listener in GUI

use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncReadExt;
use tokio::net::windows::named_pipe::ServerOptions;

const PIPE_NAME: &str = r"\\.\pipe\promptkey_selector";

pub fn start_ipc_listener(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        println!("[IPC] Starting listener on {}", PIPE_NAME);

        loop {
            // Create named pipe server
            // Note: We create a new instance for each connection
            let mut server = match ServerOptions::new()
                .first_pipe_instance(false)
                .create(PIPE_NAME)
            {
                Ok(s) => s,
                Err(e) => {
                    eprintln!(
                        "[IPC] Failed to create named pipe: {}. Retrying in 1s...",
                        e
                    );
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            // Wait for client to connect
            if let Err(e) = server.connect().await {
                eprintln!("[IPC] Failed to accept connection: {}", e);
                continue;
            }

            println!("[IPC] Client connected");

            // Handle connection
            let app_handle = app.clone();
            let mut buf = [0u8; 128];

            match server.read(&mut buf).await {
                Ok(n) if n > 0 => {
                    let msg = String::from_utf8_lossy(&buf[..n]);
                    let msg_clean = msg.trim();
                    println!("[IPC] Received: {}", msg_clean);

                    if msg_clean == "SHOW_WHEEL" {
                        // Phase 2: wheel window follows the cursor (positioned in main.rs)
                        if let Err(e) = crate::present_wheel(&app_handle) {
                            eprintln!("[IPC] Failed to show wheel: {}", e);
                        }
                    }
                }
                Ok(_) => { /* EOF or empty */ }
                Err(e) => eprintln!("[IPC] Error reading from pipe: {}", e),
            }

            // Disconnect happens when server is dropped or loop restarts
        }
    });
}
