use std::path::PathBuf;

/// Test helper for running CLI commands against a custom daemon socket.
pub struct TestCli {
    pub socket_path: PathBuf,
}

impl TestCli {
    /// Create a new TestCli that targets the given socket path.
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    /// Run the CLI binary with the given arguments, pointing at the test daemon socket.
    /// Returns an `assert_cmd::assert::Assert` for chaining assertions.
    pub fn run(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        assert_cmd::Command::cargo_bin("agent-computer")
            .expect("Failed to find agent-computer binary")
            .args(args)
            .env("AGENT_COMPUTER_SOCKET", &self.socket_path)
            .timeout(std::time::Duration::from_secs(10))
            .assert()
    }
}
