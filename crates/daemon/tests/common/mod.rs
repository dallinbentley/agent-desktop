use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use agent_desktop_shared::protocol::{Request, Response};

/// A test daemon instance running on a unique socket path.
///
/// Automatically kills the daemon process and cleans up the socket file on drop.
pub struct TestDaemon {
    pub socket_path: PathBuf,
    child: Child,
}

impl TestDaemon {
    /// Start a new daemon instance on a unique temporary socket.
    ///
    /// Polls for the socket file to appear (up to 5 seconds) before returning.
    /// Panics if the daemon fails to start or the socket doesn't appear in time.
    pub async fn start() -> Self {
        let socket_path = PathBuf::from(format!(
            "/tmp/agent-desktop-test-{}.sock",
            uuid::Uuid::new_v4()
        ));

        // Build the daemon binary path from the cargo target directory
        let daemon_bin = Self::daemon_binary_path();

        let child = Command::new(&daemon_bin)
            .env("AGENT_COMPUTER_SOCKET", &socket_path)
            .spawn()
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to start daemon at {}: {}",
                    daemon_bin.display(),
                    e
                )
            });

        // Poll for socket file to exist (up to 5 seconds)
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            if socket_path.exists() {
                // Brief extra delay to ensure the listener is ready
                tokio::time::sleep(Duration::from_millis(50)).await;
                return Self { socket_path, child };
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        panic!(
            "Daemon socket did not appear at {} within 5 seconds",
            socket_path.display()
        );
    }

    /// Send a typed request to the daemon and parse the response.
    /// Times out after 30 seconds to prevent test hangs.
    pub async fn send_request(&self, request: &Request) -> Response {
        tokio::time::timeout(Duration::from_secs(30), self.send_request_inner(request))
            .await
            .expect("send_request timed out after 30 seconds")
    }

    async fn send_request_inner(&self, request: &Request) -> Response {
        let stream = tokio::net::UnixStream::connect(&self.socket_path)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to connect to daemon at {}: {}",
                    self.socket_path.display(),
                    e
                )
            });

        let mut json = serde_json::to_string(request).expect("Failed to serialize request");
        json.push('\n');

        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let (reader, mut writer) = stream.into_split();
        writer
            .write_all(json.as_bytes())
            .await
            .expect("Failed to write request");
        writer.flush().await.expect("Failed to flush");

        let mut buf_reader = BufReader::new(reader);
        let mut response_line = String::new();
        buf_reader
            .read_line(&mut response_line)
            .await
            .expect("Failed to read response");

        serde_json::from_str(response_line.trim()).expect("Failed to parse response JSON")
    }

    /// Send a raw JSON string to the daemon and return the raw response string.
    /// Useful for testing malformed input handling.
    pub async fn send_raw(&self, json_str: &str) -> String {
        let stream = tokio::net::UnixStream::connect(&self.socket_path)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to connect to daemon at {}: {}",
                    self.socket_path.display(),
                    e
                )
            });

        let mut data = json_str.to_string();
        if !data.ends_with('\n') {
            data.push('\n');
        }

        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let (reader, mut writer) = stream.into_split();
        writer
            .write_all(data.as_bytes())
            .await
            .expect("Failed to write raw data");
        writer.flush().await.expect("Failed to flush");

        let mut buf_reader = BufReader::new(reader);
        let mut response_line = String::new();
        buf_reader
            .read_line(&mut response_line)
            .await
            .expect("Failed to read response");

        response_line.trim().to_string()
    }

    /// Resolve the daemon binary path.
    /// Uses CARGO_BIN_EXE_agent-desktop-daemon if available (set by cargo test),
    /// otherwise searches CARGO_TARGET_DIR, the shared cargo target, and workspace target/debug.
    fn daemon_binary_path() -> PathBuf {
        // CARGO_BIN_EXE_agent-desktop-daemon is set by cargo when running integration tests
        if let Ok(path) = std::env::var("CARGO_BIN_EXE_agent-desktop-daemon") {
            return PathBuf::from(path);
        }

        let binary_name = "agent-desktop-daemon";

        // Check CARGO_TARGET_DIR env var
        if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
            let bin_path = PathBuf::from(&target_dir).join("debug").join(binary_name);
            if bin_path.exists() {
                return bin_path;
            }
        }

        // Check shared cargo target (~/.cargo-shared/target)
        if let Some(home) = dirs::home_dir() {
            let bin_path = home
                .join(".cargo-shared")
                .join("target")
                .join("debug")
                .join(binary_name);
            if bin_path.exists() {
                return bin_path;
            }
        }

        // Fallback: look in workspace target/debug
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let workspace_root = PathBuf::from(manifest_dir)
            .parent()
            .and_then(|p| p.parent())
            .expect("Could not find workspace root")
            .to_path_buf();
        let bin_path = workspace_root
            .join("target")
            .join("debug")
            .join(binary_name);
        if bin_path.exists() {
            return bin_path;
        }

        panic!(
            "Could not find {} binary. Run `cargo build -p agent-desktop-daemon` first.",
            binary_name
        );
    }
}

impl Drop for TestDaemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
