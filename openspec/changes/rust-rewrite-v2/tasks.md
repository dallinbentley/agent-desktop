## 1. Rust Project Skeleton & Shared Types

- [x] 1.1 Create Cargo workspace with three crates: `cli` (agent-computer binary), `daemon` (agent-computer-daemon binary), `shared` (agent-computer-shared library). Add dependencies: clap, serde, serde_json, tokio, tungstenite, accessibility-sys, core-graphics, screencapturekit-rs. Verify `cargo build` succeeds with placeholder mains.
- [x] 1.2 Port all protocol types from Swift Sources/Shared/Protocol.swift to shared/src/protocol.rs: Request, Response, CommandArgs (all variants), ResponseData (all variants), all data payload structs. Derive Serialize/Deserialize. Add unit test for JSON round-trip.
- [x] 1.3 Port types from Swift Sources/Shared/Types.swift to shared/src/types.rs: ElementRef (with RefSource enum: AX/CDP/Coordinate), PathSegment, Rect, Point, ErrorInfo, ErrorCode constants, interactive_roles set, key_name_to_code map, socket path constants.
- [x] 1.4 Port error helpers from Swift Sources/Shared/Errors.swift to shared/src/errors.rs: factory functions for all error codes (ref_not_found, ref_stale, no_ref_map, app_not_found, permission_denied, timeout, cdp_not_available, etc). Add permission check functions.

## 2. Daemon Socket Server

- [x] 2.1 Port daemon entry point from Swift Sources/Daemon/main.swift to daemon/src/main.rs: create Unix socket at ~/.agent-computer/daemon.sock, listen for connections, read newline-delimited JSON, decode command field, dispatch to handler stubs. Handle SIGTERM/SIGINT (remove socket, exit 0). Handle stale sockets.
- [x] 2.2 Add request ID correlation and timing (elapsed_ms). Handle malformed JSON with INVALID_COMMAND error. Support concurrent command dispatch via tokio async runtime.

## 3. CLI Client & Argument Parsing

- [x] 3.1 Port CLI from Swift Sources/CLI/AgentComputer.swift to cli/src/main.rs using clap: define all subcommands (snapshot, click, fill, type, press, scroll, screenshot, open, get, status). Parse @ref syntax, key+modifier combos, coordinate pairs. Add --json, --timeout, --verbose global flags. Add new --with-cdp flag on open subcommand.
- [x] 3.2 Port socket client from Swift Sources/CLI/Connection.swift to cli/src/connection.rs: connect to daemon socket, auto-start daemon if missing (spawn background process, poll 100ms/5s), send JSON line, read response.
- [x] 3.3 Port output formatting from Swift Sources/CLI/Output.swift to cli/src/output.rs: colored human-readable text (snapshot tree, action confirmations, error messages with suggestions), JSON mode, exit codes.

## 4. AX Engine (Accessibility Tree)

- [x] 4.1 Create daemon/src/ax_engine.rs: port AX tree traversal from Swift Sources/Daemon/Snapshot.swift using raw FFI to ApplicationServices. AXUIElementCreateApplication → get windows → recursive traverse with batch attribute fetching. Respect depth limit and 3s timeout. Return structured tree.
- [x] 4.2 Port interactive element filtering (19 roles from shared/types.rs). Port snapshot text formatter: [AppName — WindowTitle] header, indented tree, @eN role "label" per interactive element.
- [x] 4.3 Implement AX-first headless actions: ax_press(element) using AXUIElementPerformAction(kAXPressAction), ax_set_value(element, text) using kAXValueAttribute with read-back verification, ax_selection_replace(element, text) using kAXSelectedTextRangeAttribute + kAXSelectedTextAttribute.
- [x] 4.4 Implement frontmost app detection with 3-tier fallback: AX systemWide kAXFocusedApplicationAttribute → NSWorkspace.frontmostApplication (via objc2-app-kit) → CGWindowListCopyWindowInfo (first window at layer 0).

## 5. Input Engine (CGEvent Fallback)

- [x] 5.1 Create daemon/src/input.rs: port mouse click from Swift Sources/Daemon/Input.swift using core-graphics crate. CGWarpMouseCursorPosition + CGEvent mouseDown/mouseUp. Support left, right, double click.
- [x] 5.2 Port keyboard press: CGEvent with virtual keycodes + modifier flags. Use key_name_to_code from shared.
- [x] 5.3 Port string typing: CGEvent keyboardSetUnicodeString, chunk at 20 UTF-16 units, Return keycode for newlines.
- [x] 5.4 Port scroll: CGEvent scroll wheel with direction mapping.

## 6. CDP Engine (Chrome DevTools Protocol)

- [x] 6.1 Create daemon/src/cdp_engine.rs: implement CDP WebSocket client using tungstenite. Connect to ws://localhost:<port>/json/version to get WebSocket debugger URL. Upgrade to WebSocket. Send/receive JSON-RPC messages with incrementing IDs.
- [x] 6.2 Implement page discovery: HTTP GET localhost:<port>/json/list to enumerate tabs. Auto-select the active/visible tab. Connect to its webSocketDebuggerUrl.
- [x] 6.3 Implement CDP accessibility tree: call Accessibility.getFullAXTree (or DOM.getDocument + Accessibility.queryAXTree). Walk the tree, filter to interactive roles, assign @refs matching the AX engine format. Produce same snapshot text output.
- [x] 6.4 Implement CDP interactions: click (DOM.focus + Runtime.callFunctionOn to .click(), or Input.dispatchMouseEvent), type (Input.insertText or Input.dispatchKeyEvent), fill (focus + select all + insertText).
- [x] 6.5 Implement CDP port probing: HTTP GET localhost:<port>/json/version with 500ms timeout. Return bool + version info. Try ports 9222-9229 and app-specific deterministic ports.
- [x] 6.6 Implement CDP connection management in daemon state: track active connections by PID/port, reuse connections across commands, close on daemon shutdown.

## 7. App Detector & Router

- [x] 7.1 Create daemon/src/detector.rs: implement detect_app(pid) → AppKind enum (Native, Browser{port}, Electron{port}, CEF{port}, Unknown). Check bundle ID against known browsers list. Check bundle path for Electron Framework.framework and Chromium Embedded Framework.framework. Probe CDP port.
- [x] 7.2 Implement snapshot routing: based on AppKind, dispatch to AX engine (Native), merged AX+CDP (Browser with CDP), CDP only (Electron/CEF with CDP), or screenshot fallback (no CDP). For merged mode: AX snapshot stops at AXWebArea boundary, CDP handles web content, refs are unified.
- [x] 7.3 Implement interaction routing: resolve ref from unified RefMap, check source (AX/CDP/Coordinate), dispatch to correct engine.

## 8. Screenshot Engine

- [x] 8.1 Create daemon/src/capture.rs: port screenshot capture using CGWindowListCreateImage (legacy but fast ~8ms). Capture frontmost window or specific app's window. Save PNG to temp dir via ImageIO.
- [x] 8.2 Add window frame data to ScreenshotData response: windowOriginX, windowOriginY, appName. Enable coordinate-based clicking from screenshot coordinates.
- [x] 8.3 Implement coordinate-click translation: when click receives image-relative coordinates + app name, look up window frame, translate to screen coordinates (screen_x = origin_x + image_x at 1x), bring app to front, CGEvent click.

## 9. Unified RefMap

- [x] 9.1 Create daemon/src/refmap.rs: port RefMap from Swift Sources/Daemon/RefMap.swift with extended ElementRef supporting RefSource (AX, CDP, Coordinate). Assign sequential refs across both sources. Provide resolve(ref) → ElementRef with source-aware data.
- [x] 9.2 Implement merged ref building: given AX tree nodes and CDP tree nodes, assign @e1... continuously. AX refs first (browser chrome), then CDP refs (web content). Store source-specific data (axPath for AX, cdpNodeId for CDP).
- [x] 9.3 Implement source-aware dispatch: resolve ref → check source → route to ax_engine, cdp_engine, or input_engine accordingly.

## 10. App Management & Open --with-cdp

- [x] 10.1 Port app management from Swift Sources/Daemon/AppManager.swift: open/focus app (NSWorkspace via objc), get running apps list, get text from AX element, status command with permissions + frontmost app + ref map state.
- [x] 10.2 Implement `open --with-cdp <app>`: detect app kind, quit existing instance, relaunch with --remote-debugging-port=<deterministic_port>. For Electron: pass flag to app binary. For CEF (Spotify): launch via direct binary path. Wait for app + CDP to be ready. Store CDP connection in daemon state.
- [x] 10.3 Implement deterministic port assignment: hash app name → port in 9222-9399 range. Track assigned ports in daemon state. Avoid port conflicts.

## 11. Wire Everything Together

- [x] 11.1 Wire all command handlers in daemon dispatch: snapshot (detect → route → AX/CDP/merged → format → respond), click (resolve ref → dispatch to correct engine), fill, type, press, scroll, screenshot, open, get, status.
- [x] 11.2 Implement the full interaction fallback chain: for click → try AX action first → if fails try CGEvent → return error. For fill → try kAXValueAttribute → try selection-replace → try CGEvent Cmd+A+type → return error. For CDP refs → CDP commands (no fallback needed).

## 12. End-to-End Testing

- [x] 12.1 Test native app flow: `open "System Settings"` → `snapshot -i` → verify @refs → `click @e<About>` → verify navigation → `screenshot` → verify PNG.
- [x] 12.2 Test Electron app flow (if CDP available): `open --with-cdp Slack` → `snapshot -i` → verify rich CDP refs (labeled buttons, channels) → `click @e<channel>` → verify navigation.
- [x] 12.3 Test browser flow (if Chrome available): `open --with-cdp Chrome` → `snapshot -i` → verify merged output (AX chrome + CDP web content) → `click @e<web_link>` → verify navigation.
- [x] 12.4 Test fallback flow: `snapshot --app Spotify` (no CDP) → verify screenshot fallback with warning message. Test coordinate click.
- [x] 12.5 Build release binary, test `agent-computer --help`, verify daemon auto-start, verify clean shutdown, verify CDP connections close on shutdown.
