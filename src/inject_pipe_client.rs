// TW004: GUI IPC Client for Inject Pipe
// Sends INJECT_PROMPT:{id}\n messages to Service via Named Pipe

use std::fs::OpenOptions;
use std::io::Write;

const PIPE_NAME: &str = r"\\.\pipe\promptkey_inject";

/// Send inject request to Service
/// Returns Ok(()) if message sent successfully
pub fn send_inject_request(prompt_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    // Open named pipe as client
    let mut pipe = OpenOptions::new().write(true).open(PIPE_NAME)?;

    // Format message: INJECT_PROMPT:{id}\n
    let message = format!("INJECT_PROMPT:{}\n", prompt_id);

    // Write and flush
    pipe.write_all(message.as_bytes())?;
    pipe.flush()?;

    Ok(())
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
