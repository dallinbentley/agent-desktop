## Why

agent-desktop has 61 unit tests but zero integration or E2E tests. The unit tests only cover isolated module internals (parsing, ref maps, detection). Nothing validates that the CLI→daemon IPC pipeline works end-to-end, that commands actually perform their actions on real apps, or that error paths produce correct output. We cannot confidently ship or refactor without integration coverage.

## What Changes

- **New integration test harness**: A shared test fixture that manages daemon lifecycle (start/stop), provides helpers for sending commands and asserting responses over the Unix socket.
- **Daemon IPC integration tests**: Validate the full CLI→daemon→response pipeline for every command (snapshot, click, fill, type, press, scroll, screenshot, open, get, wait, status).
- **Error handling tests**: Invalid @refs, non-existent apps, permission errors, timeout scenarios.
- **Bridge integration tests**: Validate agent-browser bridge commands work for Electron apps (snapshot, click, fill via CDP).
- **CLI argument parsing tests**: Verify clap parses all subcommands and flags correctly, including edge cases.
- **Output formatting tests**: Validate terminal output formatting matches expected patterns.
- **New unit tests**: Binary bundling detection order, JSON→ElementRef conversion, connection management.

## Capabilities

### New Capabilities

- `test-harness`: Shared integration test infrastructure — daemon lifecycle management, IPC helpers, assertion utilities, test app fixtures
- `integration-tests`: Integration tests covering CLI→daemon round-trips for all commands, error handling, and multi-app scenarios
- `bridge-tests`: Integration tests for agent-browser bridge — CDP snapshot, click, fill on Electron apps

### Modified Capabilities

## Impact

- **Files**: New `crates/daemon/tests/` directory with integration tests, new `crates/cli/tests/` for CLI tests, expanded unit tests in existing modules
- **Dependencies**: May need `assert_cmd` and `predicates` crates for CLI integration testing
- **CI**: Tests should be categorized — fast unit tests vs slower integration tests (need running daemon + apps)
