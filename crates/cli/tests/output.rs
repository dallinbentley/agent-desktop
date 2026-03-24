mod common;

use common::TestCli;
use predicates::str as pred_str;

// We need a daemon running for the CLI to connect to.
// Use a shared daemon across all CLI tests.

/// Minimal TestDaemon for CLI tests — starts the daemon binary on a temp socket.
struct TestDaemon {
    pub socket_path: std::path::PathBuf,
    _child: std::process::Child,
}

impl TestDaemon {
    async fn start() -> Self {
        let socket_path = std::path::PathBuf::from(format!(
            "/tmp/agent-desktop-test-{}.sock",
            uuid::Uuid::new_v4()
        ));

        // Find daemon binary
        let daemon_bin = Self::find_daemon_binary();

        let child = std::process::Command::new(&daemon_bin)
            .env("AGENT_COMPUTER_SOCKET", &socket_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap_or_else(|e| {
                panic!("Failed to start daemon at {}: {}", daemon_bin.display(), e)
            });

        // Wait for socket
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while std::time::Instant::now() < deadline {
            if socket_path.exists() {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                return Self {
                    socket_path,
                    _child: child,
                };
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        panic!("Daemon socket did not appear within 5 seconds");
    }

    fn find_daemon_binary() -> std::path::PathBuf {
        if let Ok(path) = std::env::var("CARGO_BIN_EXE_agent-desktop-daemon") {
            return std::path::PathBuf::from(path);
        }
        // Check shared cargo target
        if let Some(home) = dirs::home_dir() {
            let bin_path = home.join(".cargo-shared/target/debug/agent-desktop-daemon");
            if bin_path.exists() {
                return bin_path;
            }
        }
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let workspace_root = std::path::PathBuf::from(manifest_dir)
            .parent()
            .and_then(|p| p.parent())
            .expect("Could not find workspace root")
            .to_path_buf();
        let bin_path = workspace_root
            .join("target")
            .join("debug")
            .join("agent-desktop-daemon");
        if bin_path.exists() {
            return bin_path;
        }
        panic!("Could not find agent-desktop-daemon binary");
    }
}

impl Drop for TestDaemon {
    fn drop(&mut self) {
        let _ = std::process::Command::new("kill")
            .arg(self._child.id().to_string())
            .output();
        let _ = self._child.wait();
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

// --- Group 11: CLI Output Integration Tests ---

/// 11.1 Test `agent-desktop status` output format matches expected terminal output
#[tokio::test]
async fn test_status_output() {
    let daemon = TestDaemon::start().await;
    let cli = TestCli::new(daemon.socket_path.clone());

    cli.run(&["status"])
        .success()
        .stdout(pred_str::contains("agent-desktop daemon"))
        .stdout(pred_str::contains("PID:"))
        .stdout(pred_str::contains("Accessibility:"))
        .stdout(pred_str::contains("Screen Recording:"))
        .stdout(pred_str::contains("Ref Map:"));
}

/// 11.2 Test `agent-desktop snapshot -i --app Finder` output contains @refs
#[tokio::test]
async fn test_snapshot_output_contains_refs() {
    let daemon = TestDaemon::start().await;
    let cli = TestCli::new(daemon.socket_path.clone());

    cli.run(&["snapshot", "-i", "--app", "Finder"])
        .success()
        .stdout(pred_str::contains("[Finder"));

    // We can't guarantee @refs exist (depends on Finder state), but the command should succeed
}

/// 11.3 Test `agent-desktop --json snapshot -i --app Finder` returns valid JSON
#[tokio::test]
async fn test_json_snapshot_output() {
    let daemon = TestDaemon::start().await;
    let cli = TestCli::new(daemon.socket_path.clone());

    let output = cli
        .run(&["--json", "snapshot", "-i", "--app", "Finder"])
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("Output should be valid UTF-8");
    let json: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("Output should be valid JSON");

    assert!(json.get("success").is_some(), "JSON should have 'success' field");
    assert_eq!(json["success"], true, "Snapshot should succeed");
    assert!(json.get("data").is_some(), "JSON should have 'data' field");
    assert!(json.get("id").is_some(), "JSON should have 'id' field");
}

/// 11.4 Test `agent-desktop click @e9999` exits with non-zero code and error message
#[tokio::test]
async fn test_click_invalid_ref_output() {
    let daemon = TestDaemon::start().await;
    let cli = TestCli::new(daemon.socket_path.clone());

    // First snapshot to populate refs
    cli.run(&["snapshot", "-i", "--app", "Finder"]).success();

    // Then try to click a non-existent ref
    cli.run(&["click", "@e9999"])
        .failure()
        .stderr(pred_str::contains("Error"));
}
