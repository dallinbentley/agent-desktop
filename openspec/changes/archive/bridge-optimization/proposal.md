## Why

The agent-browser bridge currently has three problems: (1) it requires users to separately install agent-browser via npm, (2) it spawns a new subprocess per command losing ~160ms each time, and (3) it parses agent-browser's text output with regex when structured JSON is available via `--json`. These issues hurt usability and reliability.

## What Changes

- **Bundle agent-browser binary**: Ship platform-specific agent-browser binary alongside agent-desktop. Auto-download on first use if not bundled. Zero npm/Node dependency.
- **Use `--json` output mode**: Switch from text parsing to `--json` flag for structured refs + snapshot data. Eliminates regex parsing, more reliable.
- **Pre-warm CDP sessions**: Connect agent-browser session during `open --with-cdp` so first snapshot doesn't pay daemon startup cost.
- **Async subprocess execution**: Use `tokio::process::Command` for non-blocking bridge calls.

## Capabilities

### New Capabilities

- `binary-bundling`: Auto-detection and download of agent-browser binary for zero-dependency install

### Modified Capabilities

- `browser-bridge`: Switch to --json output parsing, async subprocess, pre-warm sessions

## Impact

- **Files**: `browser_bridge.rs` (JSON parsing, async, binary management), `main.rs` (pre-warm on open), `app.rs` (open_app_with_cdp pre-warm), `Cargo.toml` (tokio process, serde_json)
- **Dependencies**: `serde_json` (already present), `tokio` process feature
- **User experience**: `agent-desktop` works out of the box for Electron apps without npm install
