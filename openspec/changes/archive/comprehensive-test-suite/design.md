## Context

agent-desktop is a CLI tool backed by a daemon process. The CLI sends JSON-line requests over a Unix socket (`~/.agent-desktop/daemon.sock`), the daemon processes them (AX APIs, CGEvent, ScreenCaptureKit, agent-browser bridge), and returns JSON-line responses. Current test coverage is purely unit-level — isolated function tests within each module. There is no test that exercises the actual IPC pipeline.

## Goals / Non-Goals

**Goals:**
- Integration test harness that manages daemon lifecycle automatically
- Full round-trip coverage for every CLI command through the daemon
- Error path coverage (bad refs, missing apps, timeouts)
- Bridge integration tests for Electron/browser apps via agent-browser
- CLI argument parsing validation for all subcommands
- Tests categorized by speed: `unit` (no daemon), `integration` (daemon required), `e2e` (daemon + real apps)

**Non-Goals:**
- Visual regression testing (screenshots compared pixel-by-pixel)
- Performance benchmarking in tests (separate concern)
- Testing on Linux/Windows (macOS-only for now)
- Testing accessibility permissions (can't be automated — requires System Preferences)

## Decisions

### D1: Test Harness Architecture

**Approach**: A `TestDaemon` helper struct in `crates/daemon/tests/common/mod.rs` that:
1. Starts the daemon binary on a unique socket path (`/tmp/agent-desktop-test-{uuid}.sock`)
2. Provides `send_request(Request) -> Response` helper over async Unix socket
3. Auto-kills daemon on Drop
4. Configurable startup timeout (default 2s)

This lets tests run in parallel with isolated daemon instances. The real daemon binary is used (not mocked) — we're testing the actual code path.

### D2: CLI Tests via assert_cmd

Use `assert_cmd` crate to test the CLI binary directly:
- Verify argument parsing (all subcommands, flags, edge cases)
- Verify output formatting (terminal output matches expected patterns)
- Verify exit codes (0 for success, non-zero for errors)

CLI tests spawn the CLI binary which connects to the daemon. The `TestDaemon` provides the socket, CLI needs a way to use a custom socket path → add `--socket` flag or `AGENT_COMPUTER_SOCKET` env var.

### D3: Bridge Tests

Bridge tests require agent-browser to be installed and a target app (e.g., Chrome on example.com). These are tagged `#[ignore]` by default and run explicitly with `cargo test -- --ignored` in CI or manual testing.

### D4: Test Categories

```
cargo test                           # unit tests only (fast, no daemon)
cargo test --test integration        # integration tests (starts daemon)
cargo test --test integration -- --ignored  # + bridge/e2e tests (needs apps)
```

### D5: Socket Path Override

Add `AGENT_COMPUTER_SOCKET` env var support to both daemon and CLI. The daemon listens on it, the CLI connects to it. Default remains `~/.agent-desktop/daemon.sock`. This enables parallel test instances.

## Risks / Trade-offs

- **Integration tests are slower**: Starting a daemon per test (or test group) adds ~350ms. Mitigate by sharing daemon across tests in the same module via `once_cell::sync::Lazy`.
- **Flaky AX tests**: AX tree content varies by app state. Tests should assert structure (has refs, correct format) not exact content.
- **Bridge tests need agent-browser + Chrome**: Mark as `#[ignore]` by default. CI can opt in.
- **Socket path env var**: Small code change but critical for test isolation. Without it, tests would conflict with a user's running daemon.
