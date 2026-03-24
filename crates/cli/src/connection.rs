use agent_computer_shared::protocol::{Request, Response};
use agent_computer_shared::types::daemon_socket_path;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

// MARK: - Connection Errors

#[derive(Debug)]
pub enum ConnectionError {
    DaemonNotFound,
    DaemonStartTimeout,
    ConnectionFailed(String),
    ConnectionClosed,
    WriteFailed(String),
    EncodingFailed(String),
    DecodingFailed(String),
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DaemonNotFound => write!(
                f,
                "Could not find agent-computer-daemon binary. Make sure it's in the same directory as agent-computer or on your PATH."
            ),
            Self::DaemonStartTimeout => write!(
                f,
                "Daemon did not start within 5 seconds. Check if another instance is running or if there are permission issues."
            ),
            Self::ConnectionFailed(e) => write!(f, "Failed to connect to daemon socket: {}", e),
            Self::ConnectionClosed => {
                write!(f, "Daemon closed the connection unexpectedly.")
            }
            Self::WriteFailed(e) => write!(f, "Failed to send data to daemon: {}", e),
            Self::EncodingFailed(e) => write!(f, "Failed to encode request as JSON: {}", e),
            Self::DecodingFailed(e) => {
                write!(f, "Failed to decode response from daemon: {}", e)
            }
        }
    }
}

impl std::error::Error for ConnectionError {}

// MARK: - Send

/// Send a request to the daemon and receive a response.
/// Auto-starts the daemon if it's not running.
pub fn send(request: &Request, verbose: bool) -> Result<Response, ConnectionError> {
    let socket_path = daemon_socket_path();

    // Try to connect, auto-start daemon if needed
    let stream = connect_or_start_daemon(&socket_path, verbose)?;
    let mut stream_write = stream;

    // Encode request as JSON line
    let json = serde_json::to_string(request).map_err(|e| ConnectionError::EncodingFailed(e.to_string()))?;
    let mut json_line = json.clone();
    json_line.push('\n');

    if verbose {
        eprintln!("[verbose] Sending: {}", json);
    }

    // Send
    stream_write
        .write_all(json_line.as_bytes())
        .map_err(|e| ConnectionError::WriteFailed(e.to_string()))?;
    stream_write
        .flush()
        .map_err(|e| ConnectionError::WriteFailed(e.to_string()))?;

    // Read response line
    let mut reader = BufReader::new(&stream_write);
    let mut response_line = String::new();
    reader
        .read_line(&mut response_line)
        .map_err(|e| ConnectionError::DecodingFailed(e.to_string()))?;

    if response_line.is_empty() {
        return Err(ConnectionError::ConnectionClosed);
    }

    if verbose {
        eprintln!("[verbose] Received: {}", response_line.trim());
    }

    // Decode response
    let response: Response = serde_json::from_str(response_line.trim())
        .map_err(|e| ConnectionError::DecodingFailed(e.to_string()))?;

    Ok(response)
}

// MARK: - Connect or Start Daemon

fn connect_or_start_daemon(
    socket_path: &PathBuf,
    verbose: bool,
) -> Result<UnixStream, ConnectionError> {
    // Try connecting first
    if let Some(stream) = try_connect(socket_path) {
        return Ok(stream);
    }

    // Socket not available — start daemon
    eprintln!("Starting agent-computer daemon...");
    spawn_daemon(verbose)?;

    // Poll for socket availability (100ms intervals, 5s timeout)
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if let Some(stream) = try_connect(socket_path) {
            return Ok(stream);
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    Err(ConnectionError::DaemonStartTimeout)
}

/// Try to connect to Unix socket. Returns stream on success, None on failure.
fn try_connect(socket_path: &PathBuf) -> Option<UnixStream> {
    if !socket_path.exists() {
        return None;
    }

    UnixStream::connect(socket_path).ok()
}

/// Spawn the daemon as a background process.
fn spawn_daemon(verbose: bool) -> Result<(), ConnectionError> {
    // Find the daemon binary relative to the CLI binary
    let cli_path = std::env::current_exe().ok();
    let daemon_path = cli_path
        .as_ref()
        .and_then(|p| p.parent())
        .map(|dir| dir.join("agent-computer-daemon"));

    let final_path = if let Some(ref path) = daemon_path {
        if path.is_file() {
            path.clone()
        } else {
            find_in_path("agent-computer-daemon")?
        }
    } else {
        find_in_path("agent-computer-daemon")?
    };

    if verbose {
        eprintln!("[verbose] Spawning daemon: {}", final_path.display());
    }

    // Ensure socket directory exists
    let socket_dir = agent_computer_shared::types::daemon_socket_dir();
    let _ = std::fs::create_dir_all(&socket_dir);

    // Spawn daemon as background process
    std::process::Command::new(&final_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| {
            ConnectionError::ConnectionFailed(format!(
                "Failed to spawn daemon at {}: {}",
                final_path.display(),
                e
            ))
        })?;

    Ok(())
}

/// Find a binary in PATH.
fn find_in_path(name: &str) -> Result<PathBuf, ConnectionError> {
    let output = std::process::Command::new("which")
        .arg(name)
        .output()
        .map_err(|_| ConnectionError::DaemonNotFound)?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    Err(ConnectionError::DaemonNotFound)
}
