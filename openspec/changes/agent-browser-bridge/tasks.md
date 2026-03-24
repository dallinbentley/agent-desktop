## 1. Remove Native CDP Engine & Prep

- [x] 1.1 Remove `crates/daemon/src/cdp_engine.rs`. Remove `tungstenite` and `url` from `crates/daemon/Cargo.toml`. Remove `mod cdp_engine` from lib.rs/main.rs. Fix all compilation errors from removed module. Verify `cargo build` succeeds. *(Stripped cdp_engine.rs to port-probing only; removed tungstenite dep; replaced all CDP interaction code in main.rs with agent-browser bridge calls)*
- [x] 1.2 Update `crates/shared/src/types.rs`: keep `RefSource::CDP` variant but rename semantics — it now means "sourced from agent-browser" not "sourced from our CDP client". Add `ab_ref: Option<String>` field to ElementRef for storing the original agent-browser ref ID. Add `ab_session: Option<String>` for the session name.

## 2. Browser Bridge Core

- [x] 2.1 Create `crates/daemon/src/browser_bridge.rs`: implement `BrowserBridge` struct that detects agent-browser binary at construction (check PATH, common npm/brew paths). Cache binary path. Provide `is_available() -> bool`.
- [x] 2.2 Implement `BrowserBridge::execute(session, cdp_port, args) -> Result<String>`: run agent-browser as subprocess via `std::process::Command`. Pass `--session <session> --cdp <port>` plus command args. Capture stdout/stderr. Return stdout on exit code 0, error with stderr on non-zero.
- [x] 2.3 Implement `BrowserBridge::snapshot(session, cdp_port, interactive) -> Result<Vec<ParsedElement>>`: call `agent-browser --session <s> --cdp <port> snapshot -i`, parse the text output. Extract for each line: ref_id (regex `\[ref=(e\d+)\]`), role (first word after `- `), label (quoted string), indentation depth, attributes.
- [x] 2.4 Implement `BrowserBridge::click(session, cdp_port, ab_ref)`, `fill(session, cdp_port, ab_ref, text)`, `type_text(session, cdp_port, ab_ref, text)`, `press(session, cdp_port, key)`, `scroll(session, cdp_port, direction, amount)`: each delegates to the corresponding agent-browser CLI command using the original agent-browser ref.
- [x] 2.5 Implement `BrowserBridge::connect(session, cdp_port) -> Result<()>`: call `agent-browser --session <s> connect <port>` to establish persistent CDP connection for the session. Implement `close(session)`: call `agent-browser --session <s> close`.

## 3. Update Detector & RefMap

- [x] 3.1 Update `detector.rs`: change snapshot routing — when `SnapshotStrategy` is `CDPOnly` or `MergedAXAndCDP`, use `BrowserBridge` instead of `CdpEngine`. Remove all references to `cdp_engine` module.
- [x] 3.2 Update `refmap.rs`: when building refs from agent-browser output, store the original agent-browser ref ID in `ab_ref` field. The `route()` method SHALL return `InteractionRoute::AgentBrowser { session, cdp_port, ab_ref }` for CDP-sourced refs.
- [x] 3.3 Implement merged snapshot building: for browser apps, take AX snapshot (stop at AXWebArea), call `browser_bridge.snapshot()` for web content, concatenate with `--- web content ---` separator, assign continuous @e1... numbering across both.

## 4. Headless Mode

- [x] 4.1 Add `--foreground` flag to CLI click command. Update click handler: for AX refs, always use AX headless action (AXPress). For agent-browser refs, always use agent-browser (inherently headless). For coordinate clicks, require `--foreground` flag or error.
- [x] 4.2 Ensure `--app` flag works headlessly for all commands: snapshot (AX or agent-browser, no focus change), click (AX headless or agent-browser), fill/type (AX or agent-browser), screenshot (ScreenCaptureKit background capture). No command should call `open_app()`/`activate()` unless `--foreground` is specified.

## 5. Lifecycle & Session Management

- [x] 5.1 Add `BrowserBridge` to `DaemonState`. Initialize on daemon startup. Track active sessions as `HashMap<String, u16>` (session_name → cdp_port). *(Done by other agent in DaemonState::new())*
- [x] 5.2 When `open --with-cdp <app>` succeeds, auto-connect: call `browser_bridge.connect(session, port)`. Register session in state. *(Added to app.rs open_app_with_cdp)*
- [x] 5.3 On daemon SIGTERM shutdown, iterate active sessions and call `browser_bridge.close(session)` for each. *(Added to main.rs shutdown handler)*

## 6. Wire Into Command Dispatch

- [x] 6.1 Update `handle_snapshot` in main.rs: after detecting app kind and choosing strategy, if strategy involves agent-browser, use `browser_bridge.snapshot()` to get web content. Merge with AX if needed. Build unified RefMap with both AX and agent-browser refs.
- [x] 6.2 Update `handle_click`, `handle_fill`, `handle_type` in main.rs: when ref routes to `InteractionRoute::AgentBrowser`, delegate to `browser_bridge.click/fill/type_text` with the stored original ref and session/port.
- [x] 6.3 Update `handle_press` and `handle_scroll`: for web-focused apps (when last snapshot was CDP-sourced), delegate to agent-browser.

## 7. End-to-End Testing

- [x] 7.1 Test native app headless: `snapshot -i --app "System Settings"` → verify @refs → `click @e10` → verify no focus steal → `screenshot --app "System Settings"` → verify About page.
- [x] 7.2 Test Electron app with CDP: `open --with-cdp Spotify` → `snapshot -i --app Spotify` → verify rich agent-browser refs → `click @e<search>` → `type @e<search> "Luke Combs"` → `press enter` → verify all headless.
- [x] 7.3 Test fallback: `snapshot -i --app Spotify` (without CDP) → verify warning about agent-browser/CDP → fall back to screenshot.
- [x] 7.4 Test agent-browser not installed: temporarily rename binary → verify graceful error with install instructions → restore.
