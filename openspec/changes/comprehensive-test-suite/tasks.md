## 1. Socket Path Override (prerequisite for all integration tests)

- [ ] 1.1 Add `AGENT_COMPUTER_SOCKET` env var support to daemon `main.rs` — if set, listen on that path instead of `~/.agent-computer/daemon.sock`
- [ ] 1.2 Add `AGENT_COMPUTER_SOCKET` env var support to CLI `connection.rs` — if set, connect to that path instead of default
- [ ] 1.3 Verify: start daemon with custom socket, CLI connects to it, status command works

## 2. Test Harness (crates/daemon/tests/common/mod.rs)

- [ ] 2.1 Create `TestDaemon` struct: starts daemon binary on unique temp socket (`/tmp/agent-computer-test-{uuid}.sock`), waits for socket to exist, stores child process handle
- [ ] 2.2 Add `TestDaemon::send_request(Request) -> Response` helper: connects to socket, sends JSON line, reads JSON line response
- [ ] 2.3 Add `TestDaemon::send_raw(json_str) -> String` for testing malformed input handling
- [ ] 2.4 Implement `Drop` for `TestDaemon` — kills daemon process, removes socket file
- [ ] 2.5 Add `TestCli` struct wrapping `assert_cmd::Command` — sets `AGENT_COMPUTER_SOCKET`, runs CLI binary, returns output
- [ ] 2.6 Add `assert_cmd` and `predicates` to dev-dependencies in cli and daemon Cargo.toml

## 3. CLI Argument Parsing Tests (crates/cli/src/main.rs or crates/cli/tests/)

- [ ] 3.1 Test all subcommand variants parse correctly: snapshot (-i, -c, -d, --app, -s), click (@ref, coords, --double, --right, --foreground, --app), fill, type, press, scroll, screenshot, open (--with-cdp, --background), get (text, title, apps, windows), wait (ref, ms, --load), status, install-browser
- [ ] 3.2 Test global flags: --json, --timeout, --verbose
- [ ] 3.3 Test edge cases: coordinate click `100 200`, type with ref `@e3 "text"`, type without ref `"just text"`, scroll with/without amount

## 4. Daemon Lifecycle Integration Tests (crates/daemon/tests/lifecycle.rs)

- [ ] 4.1 Test daemon starts and responds to status command
- [ ] 4.2 Test daemon returns correct permission info in status response
- [ ] 4.3 Test daemon handles multiple sequential requests on same connection
- [ ] 4.4 Test daemon handles multiple concurrent connections
- [ ] 4.5 Test daemon graceful shutdown (send shutdown request, verify process exits)

## 5. Snapshot Integration Tests (crates/daemon/tests/snapshot.rs)

- [ ] 5.1 Test snapshot of Finder (always running) — verify response has text starting with `[Finder]`, ref_count >= 0
- [ ] 5.2 Test snapshot -i of Finder — verify interactive elements have @eN refs
- [ ] 5.3 Test snapshot with depth limit — verify tree doesn't exceed depth
- [ ] 5.4 Test snapshot of non-existent app — verify error response
- [ ] 5.5 Test snapshot without --app (frontmost app) — verify response succeeds

## 6. Action Integration Tests (crates/daemon/tests/actions.rs)

- [ ] 6.1 Test click with invalid ref @e9999 — verify error "ref not found"
- [ ] 6.2 Test press escape --app Finder — verify success response
- [ ] 6.3 Test press key combo cmd+shift+n — verify success response
- [ ] 6.4 Test scroll down 100 — verify success response with direction/amount
- [ ] 6.5 Test fill with invalid ref — verify error
- [ ] 6.6 Test type without ref (types into frontmost app) — verify success
- [ ] 6.7 Test click after snapshot (valid ref) — snapshot Finder, click first ref, verify success

## 7. Screenshot Integration Tests (crates/daemon/tests/screenshot.rs)

- [ ] 7.1 Test screenshot --app Finder — verify file path returned, file exists, file > 0 bytes
- [ ] 7.2 Test screenshot --full — verify full screen capture works
- [ ] 7.3 Test screenshot of non-existent app — verify error response

## 8. App Management Integration Tests (crates/daemon/tests/app_mgmt.rs)

- [ ] 8.1 Test get apps — verify response contains list, includes "Finder"
- [ ] 8.2 Test open Finder — verify success response
- [ ] 8.3 Test open non-existent app — verify error response
- [ ] 8.4 Test get windows --app Finder — verify returns window list

## 9. Error Handling Integration Tests (crates/daemon/tests/errors.rs)

- [ ] 9.1 Test malformed JSON request — verify daemon doesn't crash, returns error
- [ ] 9.2 Test request with unknown command — verify error response
- [ ] 9.3 Test fill @e1 with empty refmap (no prior snapshot) — verify "ref not found"
- [ ] 9.4 Test click @xyz (invalid ref format) — verify error about format

## 10. Bridge Integration Tests (crates/daemon/tests/bridge.rs) — #[ignore] by default

- [ ] 10.1 Test bridge snapshot of Electron app (Slack) — verify CDP-sourced refs with ab_ref/ab_session populated
- [ ] 10.2 Test bridge click on Electron app — snapshot then click first ref, verify success
- [ ] 10.3 Test bridge fallback when agent-browser unavailable — verify falls back to AX
- [ ] 10.4 Test bridge timeout handling — verify 10s timeout doesn't hang tests

## 11. CLI Output Integration Tests (crates/cli/tests/output.rs)

- [ ] 11.1 Test `agent-computer status` output format matches expected terminal output
- [ ] 11.2 Test `agent-computer snapshot -i --app Finder` output contains @refs
- [ ] 11.3 Test `agent-computer --json snapshot -i --app Finder` returns valid JSON
- [ ] 11.4 Test `agent-computer click @e9999` exits with non-zero code and error message

## 12. Expanded Unit Tests

- [ ] 12.1 Add unit tests for `detect_binary()` — verify bundled path checked first, PATH second, nvm third
- [ ] 12.2 Add unit tests for `download_binary()` platform detection — darwin-arm64, darwin-x64, linux-x64
- [ ] 12.3 Add unit tests for `output.rs` formatting functions (if any are pure functions)
- [ ] 12.4 Add unit tests for `connection.rs` socket path resolution with/without env var
