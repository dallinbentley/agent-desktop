## 1. Fix CDP Port-to-App Routing

- [x] 1.1 Add `cdp_port_map: HashMap<i32, (u16, String)>` (pid → (port, app_name)) to DaemonState. When `open --with-cdp` succeeds, store the mapping. When detecting app kind in detector.rs, check this map by PID first before falling back to port scanning.
- [x] 1.2 Update `browser_bridge.rs` snapshot/click/fill/type calls: resolve the correct CDP port from DaemonState's cdp_port_map using the target app's PID, not from detector port scanning. Add `get_cdp_port_for_app(app_name) -> Option<u16>` helper.

## 2. Add --app Flag to All Commands

- [x] 2.1 Add `--app` flag to CLI subcommands: Fill, Type, Press, Scroll, Screenshot, Get, Wait (cli/main.rs). Add `app: Option<String>` field to FillArgs, TypeArgs, PressArgs, ScrollArgs, ScreenshotArgs, GetArgs in protocol.rs.
- [x] 2.2 Update daemon command handlers: for fill, type, press, scroll — when `app` is specified, resolve the app's PID and CDP port from state, use those for routing instead of the last-snapshot context. For press/scroll with `--app` on a CDP app, delegate to browser_bridge.

## 3. Add Wait Command

- [x] 3.1 Add WaitArgs (ref_or_ms: String, load: Option<String>, app: Option<String>) and WaitData to protocol.rs. Add Wait subcommand to CLI (cli/main.rs) with positional arg (ref or ms), --load flag, --app flag.
- [x] 3.2 Add `wait(session, cdp_port, args: &[&str])` to browser_bridge.rs — delegates to `agent-browser --session <s> --cdp <port> wait <args>`.
- [x] 3.3 Implement handle_wait in daemon main.rs: if arg is numeric → sleep. If arg is @ref → poll AX tree or use agent-browser wait. If --load → delegate to agent-browser wait --load <state>. Route based on --app context.

## 4. Fix Click Reliability for SPAs

- [x] 4.1 After CDP clicks via browser_bridge, add a 500ms post-click delay by default. Add `--no-wait` flag to click command to skip this delay when not needed.
- [x] 4.2 For link-type elements (role contains "link"), auto-chain a brief `agent-browser wait 500` after the click to let SPA routers update.

## 5. Fix open --with-cdp Relaunch

- [x] 5.1 Update `open_app_with_cdp` in app.rs: if app is already running, force quit via `osascript -e 'quit app "Name"'`, then loop-wait (100ms intervals, 5s timeout) until the old PID is gone from the process table. Only then launch with CDP flag.
- [x] 5.2 After launching, wait for new PID to appear (poll running apps, 100ms intervals, 10s timeout). Then probe CDP port until responsive (100ms intervals, 10s timeout). Store (new_pid, port, app_name) in DaemonState.cdp_port_map.

## 6. Fix Blank Screenshots

- [x] 6.1 In capture.rs, after capturing a screenshot, check if the resulting PNG file is < 1KB. If so, retry up to 3 times with 500ms delay between attempts. Log a warning on each retry. Return error if all retries fail.

## 7. Add Snapshot Scoping

- [x] 7.1 Add `--selector <css>` flag to Snapshot CLI subcommand. Add `selector: Option<String>` to SnapshotArgs in protocol.rs.
- [x] 7.2 In browser_bridge.rs snapshot method, if selector is Some, pass `-s "<selector>"` to agent-browser snapshot call.

## 8. Add get Delegation for Web Content

- [x] 8.1 Add `get(session, cdp_port, what, ab_ref)` to browser_bridge.rs. Delegates to `agent-browser --session <s> --cdp <port> get <what> @<ref>`.
- [x] 8.2 In handle_get in main.rs, when ref is CDP-sourced, delegate to browser_bridge.get() instead of AX attribute reading.

## 9. Integration Testing

- [x] 9.1 Test CDP routing: launch Spotify with CDP, launch Slack with CDP (different ports), verify snapshot --app Spotify returns Spotify content and snapshot --app Slack returns Slack content.
- [x] 9.2 Test full headless flow: open --with-cdp Spotify → snapshot -i --app Spotify → fill @e<search> "Luke Combs" --app Spotify → press enter --app Spotify → wait --load networkidle --app Spotify → snapshot -i --app Spotify → verify search results → click @e<play> --app Spotify.
- [x] 9.3 Test scoped snapshot: snapshot -i --app Spotify --selector "nav" → verify fewer refs than full snapshot.
