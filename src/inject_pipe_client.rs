// TW004: GUI IPC Client for Inject endpoint
// Sends INJECT_PROMPT:{id}\n messages to Service.
// Windows: named pipe opened via OpenOptions. Unix: domain socket connect —
// same line protocol either way; endpoint from service::platform.

use std::io::Write;

fn open_writer() -> Result<Box<dyn Write>, Box<dyn std::error::Error>> {
    write_to(&service::platform::inject_endpoint())
}

#[cfg(windows)]
fn write_to(endpoint: &str) -> Result<Box<dyn Write>, Box<dyn std::error::Error>> {
    let pipe = std::fs::OpenOptions::new().write(true).open(endpoint)?;
    Ok(Box::new(pipe))
}

#[cfg(unix)]
fn write_to(endpoint: &str) -> Result<Box<dyn Write>, Box<dyn std::error::Error>> {
    let stream = std::os::unix::net::UnixStream::connect(endpoint)?;
    Ok(Box::new(stream))
}

fn send(message: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut w = open_writer()?;
    w.write_all(message.as_bytes())?;
    w.flush()?;
    Ok(())
}

/// Send inject request to Service
/// Returns Ok(()) if message sent successfully
pub fn send_inject_request(prompt_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    send(format!("INJECT_PROMPT:{}\n", prompt_id))
}

/// Phase 2 D5: send inject request with collected {{var}} values
/// Format: INJECT_PROMPT:{id}:VARS:{urlencoded_json}\n
pub fn send_inject_request_vars(prompt_id: i32, vars_json: String) -> Result<(), Box<dyn std::error::Error>> {
    // vars_json may contain ':' so it's delimited by the third ':'-prefix
    send(format!("INJECT_PROMPT:{}:VARS:{}\n", prompt_id, vars_json))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_message_format() {
        // Just verify the format logic (can't test actual pipe without server)
        let prompt_id = 123;
        let expected = "INJECT_PROMPT:123\n";
        let actual = format!("INJECT_PROMPT:{}\n", prompt_id);
        assert_eq!(actual, expected);
    }
}
