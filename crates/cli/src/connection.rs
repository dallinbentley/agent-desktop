use agent_desktop_shared::protocol::{Request, Response};
use agent_desktop_shared::types::daemon_socket_path;

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
                "Could not find agent-desktop-daemon binary. Make sure it's in the same directory as agent-desktop or on your PATH."
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

    // Socket not available — start daemon with ready-pipe
    eprintln!("Starting agent-desktop daemon...");
    let read_fd = spawn_daemon(verbose)?;

    // If we got a ready-pipe read fd, block on it for instant wake
    if let Some(fd) = read_fd {
        if verbose {
            eprintln!("[verbose] Waiting on ready-pipe for daemon signal...");
        }

        let pipe_ready = wait_on_ready_pipe(fd, Duration::from_secs(2));

        // Close read fd
        unsafe { libc::close(fd); }

        if pipe_ready {
            // Daemon signaled ready — connect immediately
            if let Some(stream) = try_connect(socket_path) {
                return Ok(stream);
            }
            // Pipe signaled but socket not yet connectable — brief retry
            for _ in 0..10 {
                std::thread::sleep(Duration::from_millis(5));
                if let Some(stream) = try_connect(socket_path) {
                    return Ok(stream);
                }
            }
        }

        if verbose {
            eprintln!("[verbose] Ready-pipe failed or timed out, falling back to polling");
        }
    }

    // Fallback: poll for socket availability (10ms intervals, 5s timeout)
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if let Some(stream) = try_connect(socket_path) {
            return Ok(stream);
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    Err(ConnectionError::DaemonStartTimeout)
}

/// Wait on the read end of the ready-pipe with a timeout.
/// Returns true if a byte was received (daemon ready), false on timeout/error/EOF.
fn wait_on_ready_pipe(read_fd: i32, timeout: Duration) -> bool {
    // Use poll() to wait on the fd with a timeout
    let timeout_ms = timeout.as_millis() as i32;
    let mut pollfd = libc::pollfd {
        fd: read_fd,
        events: libc::POLLIN,
        revents: 0,
    };

    let ret = unsafe { libc::poll(&mut pollfd, 1, timeout_ms) };

    if ret <= 0 {
        // Timeout (0) or error (-1)
        return false;
    }

    // Data available — read 1 byte
    let mut buf = [0u8; 1];
    let n = unsafe { libc::read(read_fd, buf.as_mut_ptr() as *mut libc::c_void, 1) };
    n == 1
}

/// Try to connect to Unix socket. Returns stream on success, None on failure.
fn try_connect(socket_path: &PathBuf) -> Option<UnixStream> {
    if !socket_path.exists() {
        return None;
    }

    UnixStream::connect(socket_path).ok()
}

/// Spawn the daemon as a background process.
/// Returns the read fd of the ready-pipe (if pipe creation succeeded), or None.
fn spawn_daemon(verbose: bool) -> Result<Option<i32>, ConnectionError> {
    // Find the daemon binary relative to the CLI binary
    let cli_path = std::env::current_exe().ok();
    let daemon_path = cli_path
        .as_ref()
        .and_then(|p| p.parent())
        .map(|dir| dir.join("agent-desktop-daemon"));

    let final_path = if let Some(ref path) = daemon_path {
        if path.is_file() {
            path.clone()
        } else {
            find_in_path("agent-desktop-daemon")?
        }
    } else {
        find_in_path("agent-desktop-daemon")?
    };

    if verbose {
        eprintln!("[verbose] Spawning daemon: {}", final_path.display());
    }

    // Ensure socket directory exists
    let socket_path = agent_desktop_shared::types::daemon_socket_path();
    let socket_dir = socket_path.parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(agent_desktop_shared::types::daemon_socket_dir);
    let _ = std::fs::create_dir_all(&socket_dir);

    // Create ready-pipe: pipe_fds[0] = read end, pipe_fds[1] = write end
    let mut pipe_fds = [0i32; 2];
    let pipe_ok = unsafe { libc::pipe(pipe_fds.as_mut_ptr()) } == 0;

    let read_fd = if pipe_ok { Some(pipe_fds[0]) } else { None };
    let write_fd = if pipe_ok { Some(pipe_fds[1]) } else { None };

    // Spawn daemon as background process
    let mut cmd = std::process::Command::new(&final_path);
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    // Forward AGENT_COMPUTER_SOCKET to daemon so it listens on the same custom path
    if let Ok(custom_socket) = std::env::var("AGENT_COMPUTER_SOCKET") {
        cmd.env("AGENT_COMPUTER_SOCKET", custom_socket);
    }

    if let Some(wfd) = write_fd {
        // Pass write fd to daemon via env var
        cmd.env("AGENT_COMPUTER_READY_FD", wfd.to_string());

        // Ensure the write fd is not close-on-exec so the child inherits it
        unsafe {
            let flags = libc::fcntl(wfd, libc::F_GETFD);
            if flags >= 0 {
                libc::fcntl(wfd, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
            }
        }
    }

    cmd.spawn()
        .map_err(|e| {
            // Clean up pipe fds on spawn failure
            if let Some(rfd) = read_fd { unsafe { libc::close(rfd); } }
            if let Some(wfd) = write_fd { unsafe { libc::close(wfd); } }
            ConnectionError::ConnectionFailed(format!(
                "Failed to spawn daemon at {}: {}",
                final_path.display(),
                e
            ))
        })?;

    // Close write fd in CLI process — only daemon needs it
    if let Some(wfd) = write_fd {
        unsafe { libc::close(wfd); }
    }

    Ok(read_fd)
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
