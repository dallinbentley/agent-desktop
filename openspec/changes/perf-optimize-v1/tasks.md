## 1. Replace osascript with native APIs (app.rs)

- [x] 1.1 Add NSWorkspace FFI module: create `crates/daemon/src/ns_workspace.rs` with `get_running_gui_apps() -> Vec<AppInfo>` using objc FFI to call `NSWorkspace.shared.runningApplications`. Return localizedName, processIdentifier, isActive for each app where `activationPolicy == .regular`.
- [x] 1.2 Replace `get_running_gui_apps()` in app.rs: swap osascript implementation with ns_workspace call. Verify same output format.
- [x] 1.3 Replace `get_running_app_names()` in app.rs: use ns_workspace or simplified NSRunningApplication lookup instead of osascript.
- [x] 1.4 Replace `get_frontmost_window_title()` in app.rs: use AXUIElement for the app PID → kAXFocusedWindowAttribute → kAXTitleAttribute. Add helper to ax_engine.rs if needed.
- [x] 1.5 Replace process-running check in `open_app()`: use `kill(pid, 0)` via libc or NSRunningApplication lookup instead of osascript "get name of every process".
- [x] 1.6 Replace `get_app_pid_by_name()` in app.rs if it uses osascript: use NSWorkspace lookup by localizedName.

## 2. Reduce input simulation delays (input.rs)

- [x] 2.1 Reduce key_press() delays: change key-down→key-up sleep from 20ms to 2ms, post-keystroke sleep from 10ms to 1ms.
- [x] 2.2 Validate with typing test: run `type_string("The quick brown fox")` and verify all characters arrive correctly. Test against Terminal, Ghostty, and a text field in a native app.
- [x] 2.3 Check mouse_click timing: verify mouse delays are unchanged and still reliable.

## 3. Ready-pipe for cold start (connection.rs + daemon main.rs)

- [x] 3.1 CLI side (connection.rs): in `spawn_daemon()`, create an OS pipe (`pipe()` syscall). Pass write fd to daemon as env var `AGENT_COMPUTER_READY_FD`. Block on read fd after spawn.
- [x] 3.2 Daemon side (main.rs): after `UnixListener::bind()`, check for `AGENT_COMPUTER_READY_FD` env var. If present, write 1 byte to that fd and close it.
- [x] 3.3 Fallback: if pipe read fails or times out (2s), fall back to socket polling at 10ms intervals (reduced from 100ms).
- [x] 3.4 Reduce fallback poll interval: change the existing 100ms poll to 10ms in `connect_or_start_daemon()`.

## 4. Snapshot performance instrumentation (ax_engine.rs)

- [x] 4.1 Add timing per depth level: wrap each depth iteration in `Instant::now()` / `elapsed()`. Collect into a `Vec<(depth, duration, element_count)>`.
- [x] 4.2 Count total AX attribute queries: increment a counter in attribute-fetching functions.
- [x] 4.3 Add `--verbose` flag to snapshot CLI command. Pass through to daemon. When set, log profiling data to stderr.
- [x] 4.4 Run profiling against 3+ apps (Finder, Ghostty, System Settings) and document results in spikes/S6_snapshot_profiling.md.

## 5. Benchmarking & validation

- [x] 5.1 Create benchmark script: measures cold start, status, get apps, press, click, fill, snapshot. Runs 5 iterations each, reports avg/min/max.
- [x] 5.2 Run benchmark before changes (baseline saved to spikes/S6_bench_before.txt). After results pending integration.
- [x] 5.3 Run Spotify headless flow: open, snapshot, press cmd+l, status — all commands pass. Note: CDP routing shows wrong app content (pre-existing issue, not a regression).
