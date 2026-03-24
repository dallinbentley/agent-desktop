## 1. Switch to --json output mode (browser_bridge.rs)

- [x] 1.1 Add serde JSON response structs: `AgentBrowserResponse { success, data, error }`, `SnapshotData { origin, refs, snapshot }`, `RefInfo { name, role }`. Derive Deserialize.
- [x] 1.2 Modify `execute()` to always pass `--json` flag and return deserialized `AgentBrowserResponse` instead of raw stdout string.
- [x] 1.3 Rewrite `snapshot()` to use JSON response: extract `refs` map directly (no regex parsing), return both structured refs and snapshot text from `data.snapshot`.
- [x] 1.4 Update `click()`, `fill()`, `type_text()`, `press()`, `scroll()` to check `response.success` and return `response.error` on failure.
- [x] 1.5 Update all call sites in `main.rs` that use bridge methods to work with the new return types.
- [x] 1.6 Remove `parse_snapshot_output()`, `parse_snapshot_line()`, and all regex parsing functions. Keep tests that validate the new JSON path.

## 2. Bundle agent-browser binary (browser_bridge.rs)

- [x] 2.1 Add `AGENT_BROWSER_VERSION` constant (pin to current version, e.g., "0.22.1").
- [x] 2.2 Update `detect_binary()` to check `~/.agent-computer/bin/agent-browser` first, before PATH and nvm paths.
- [x] 2.3 Add `download_binary()` function: detect platform (darwin-arm64, darwin-x64, linux-x64, etc.), download from npm registry (`https://registry.npmjs.org/agent-browser/-/agent-browser-{version}.tgz`), extract the platform binary, chmod +x, save to `~/.agent-computer/bin/agent-browser`.
- [x] 2.4 Call `download_binary()` from `detect_binary()` when no binary found. Log progress to stderr.
- [x] 2.5 After download, run `agent-browser install` to download Chrome for Testing (required by agent-browser on first use).
- [x] 2.6 Add `--install-browser` CLI command to agent-computer that triggers manual download/install of agent-browser + Chrome.

## 3. Pre-warm CDP sessions (main.rs, app.rs)

- [x] 3.1 After `open_app_with_cdp()` successfully launches an app, call `bridge.connect(session, cdp_port)` to pre-warm the agent-browser daemon.
- [x] 3.2 Add a brief delay (500ms) after app launch before connect to allow CDP port to be ready.
- [x] 3.3 Handle connect failure gracefully — log warning but don't fail the open command.

## 4. Async subprocess execution (browser_bridge.rs)

- [x] 4.1 Add `tokio` process feature to daemon's Cargo.toml. (Already had `full` features in workspace)
- [x] 4.2 Convert `execute()` to async: use `tokio::process::Command` with `.output().await`.
- [x] 4.3 Convert all bridge methods (`snapshot`, `click`, `fill`, etc.) to async.
- [x] 4.4 Update all call sites in `main.rs` to `.await` bridge calls. (Also updated app.rs call sites for compilation)
- [x] 4.5 Add 10-second timeout to bridge subprocess calls via `tokio::time::timeout`.

## 5. Testing & validation

- [x] 5.1 Update existing bridge unit tests for JSON response format.
- [x] 5.2 Test with Electron app (Slack or Spotify): snapshot → click → fill → verify output matches previous behavior.
- [x] 5.3 Test binary auto-download: remove bundled binary, verify it re-downloads on next use.
- [x] 5.4 Build release and verify all tests pass.
