## ADDED Requirements

### Requirement: Socket path override
The daemon and CLI SHALL support an `AGENT_COMPUTER_SOCKET` environment variable to override the default socket path. This enables parallel test instances.

#### Scenario: Custom socket path via env var
- **WHEN** `AGENT_COMPUTER_SOCKET=/tmp/test-123.sock` is set
- **THEN** the daemon listens on `/tmp/test-123.sock` instead of `~/.agent-computer/daemon.sock`
- **THEN** the CLI connects to `/tmp/test-123.sock`

### Requirement: TestDaemon helper
A `TestDaemon` struct SHALL manage daemon lifecycle for integration tests.

#### Scenario: Start and stop
- **WHEN** `TestDaemon::start().await` is called
- **THEN** the daemon binary starts on a unique temp socket path
- **THEN** `send_request(Request)` returns the daemon's `Response`
- **THEN** the daemon is killed when `TestDaemon` is dropped

#### Scenario: Parallel test isolation
- **WHEN** two tests create separate `TestDaemon` instances
- **THEN** each uses a unique socket path
- **THEN** they do not interfere with each other

### Requirement: TestCli helper
A `TestCli` struct SHALL wrap `assert_cmd::Command` for testing the CLI binary.

#### Scenario: Run CLI command
- **WHEN** `TestCli::new(socket_path).run(&["snapshot", "-i"])` is called
- **THEN** it executes the CLI binary with `AGENT_COMPUTER_SOCKET` set
- **THEN** it returns stdout, stderr, and exit code for assertions
