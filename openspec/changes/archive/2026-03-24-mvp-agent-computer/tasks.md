## 1. Project Skeleton & Shared Types

- [x] 1.1 Set up Package.swift with three targets: `agent-desktop` (CLI executable), `agent-desktop-daemon` (daemon executable), `AgentComputerShared` (library). Add swift-argument-parser dependency. Scaffold Sources/CLI, Sources/Daemon, Sources/Shared directories. Verify `swift build` succeeds.
- [x] 1.2 Define shared protocol types in Sources/Shared: `Command` enum (snapshot, click, fill, type, press, scroll, screenshot, open, get, status), `Response` struct (id, success, data, error, timing), `ElementRef` struct (id, role, label, frame, axPath, actions), `ErrorInfo` struct (code, message, suggestion). All Codable. Unit test for JSON round-trip.
- [x] 1.3 Define `SnapshotOptions`, `ClickTarget`, `TypeTarget`, `FillTarget`, `PressTarget`, `ScrollTarget`, `ScreenshotOptions`, `OpenTarget`, `GetTarget` arg types in Sources/Shared. All Codable.

## 2. Daemon Socket Server

- [x] 2.1 Create daemon entry point in Sources/Daemon/main.swift. Create Unix domain socket at `~/.agent-desktop/daemon.sock`. Listen for connections. Read newline-delimited JSON, decode as Command, dispatch to handler stubs that return mock responses. Handle SIGTERM/SIGINT for clean shutdown (remove socket file, exit 0).
- [x] 2.2 Add request/response correlation: each request has `id`, response echoes it. Add `timing` field measuring elapsed_ms per command. Handle malformed JSON gracefully with INVALID_COMMAND error response.
- [x] 2.3 Handle stale socket: on startup, if socket file exists, try connecting to it — if connection fails, remove stale file and proceed with creating new socket.

## 3. CLI Client & Argument Parsing

- [x] 3.1 Create CLI entry point in Sources/CLI/main.swift using Swift Argument Parser. Define subcommands: Snapshot, Click, Fill, Type, Press, Scroll, Screenshot, Open, Get, Status. Parse @ref syntax (strip `@` prefix, validate `e\d+` format). Parse key names and modifier combos for Press (cmd+c → key "c" with modifier .command).
- [x] 3.2 Implement CLI socket client: connect to `~/.agent-desktop/daemon.sock`, send JSON command, read JSON response. If socket doesn't exist or connection fails, spawn daemon as background process via `Process()`, poll for socket (100ms intervals, 5s timeout), then connect.
- [x] 3.3 Implement output formatting: human-readable colored text by default, raw JSON with `--json` flag. Format snapshot as indented tree, actions as brief confirmations, errors with recovery suggestions. Non-zero exit code on failure.

## 4. Accessibility Tree Snapshot

- [x] 4.1 Implement AX tree traversal in Sources/Daemon/Snapshot.swift: given a PID, create AXUIElementCreateApplication, get windows via kAXWindowsAttribute, recursively traverse children. Use `AXUIElementCopyMultipleAttributeValues` for batch fetching (role, title, description, frame, children, actions). Respect depth limit and 3s timeout. Return structured `[AXNode]` tree.
- [x] 4.2 Implement interactive element filtering: define `isInteractive(role:)` for the 19 interactive roles from spec. Walk tree and mark interactive elements.
- [x] 4.3 Implement RefMap: assign sequential refs (@e1, @e2...) to interactive elements in tree order. Store ElementRef with axPath (role+index chain), frame, role, label, actions. Provide `resolve(ref:) -> ElementRef?` and `resolveToCoordinates(ref:) -> CGPoint?`. Invalidate on new snapshot.
- [x] 4.4 Implement snapshot text formatter: produce compact output with window title header `[AppName — WindowTitle]`, indented tree structure, `@e1 role "label"` per interactive element. Include structural parents (toolbar, content) as unlabeled context.
- [x] 4.5 Wire snapshot command end-to-end: daemon receives snapshot command → traverses tree → builds ref map → formats text → returns in response. Test with TextEdit and Finder.

## 5. Input Simulation

- [x] 5.1 Implement mouse click in Sources/Daemon/Input.swift: `mouseClick(at:button:clickCount:)` using `CGWarpMouseCursorPosition` + CGEvent mouseDown/mouseUp. Support left, right (.rightMouseDown/Up), and double click (mouseEventClickState=2). Brief delay (10ms) between down/up.
- [x] 5.2 Implement keyboard press: `keyPress(key:modifiers:)` with virtual keycode mapping for named keys (Enter=36, Tab=48, Escape=53, Space=49, Delete=51, arrows). Support modifier flags (.maskCommand, .maskShift, .maskAlternate, .maskControl). Post via .cghidEventTap.
- [x] 5.3 Implement string typing: `typeString(_:)` using `CGEvent.keyboardSetUnicodeString`. Chunk at 20 UTF-16 units per event. Handle newlines by sending Return keycode separately. 20-30ms delay between chunks.
- [x] 5.4 Implement fill: `fillElement(ref:text:)` — resolve ref to AXUIElement via path re-traversal, set kAXSelectedTextRangeAttribute to select all, set kAXSelectedTextAttribute to replacement text. Fallback: Cmd+A via CGEvent then typeString.
- [x] 5.5 Implement scroll: `scroll(direction:amount:)` via CGEvent scroll wheel events. Map up/down/left/right to appropriate wheel delta values.
- [x] 5.6 Wire click command end-to-end: daemon receives click → resolves ref from RefMap → re-locates element (path re-traversal, frame fallback) → computes center point → calls mouseClick. Return confirmation with element info. Handle stale refs with AI-friendly error.
- [x] 5.7 Wire type, fill, press, scroll commands end-to-end. Test each against TextEdit.

## 6. Screenshot Capture

- [x] 6.1 Implement screenshot in Sources/Daemon/Capture.swift: use SCScreenshotManager to capture frontmost window. Save as PNG to temp directory. Return file path and dimensions. Capture at 1x by default. Check permission via `CGPreflightScreenCaptureAccess()` first — return actionable error if denied.
- [x] 6.2 Support `--full` flag for full screen capture. Wire screenshot command end-to-end in daemon.

## 7. App Management

- [x] 7.1 Implement open/focus app in Sources/Daemon/AppManager.swift: use `NSWorkspace.shared.runningApplications` to find by localizedName, call `.activate()` to bring to front. If not running, launch via `NSWorkspace.shared.open(URL)`. Handle app not found with fuzzy match suggestion.
- [x] 7.2 Implement `get apps` (list running GUI apps with name, PID, isActive) and `get text @ref` (read element's value/title from AX attributes).
- [x] 7.3 Implement status command: report daemon PID/uptime, check AXIsProcessTrusted() for accessibility permission, check CGPreflightScreenCaptureAccess() for screen recording, report frontmost app/window, ref map element count and age.

## 8. Error Handling & Polish

- [x] 8.1 Implement AI-friendly error messages for all error paths: REF_NOT_FOUND, REF_STALE, NO_REF_MAP, APP_NOT_FOUND, PERMISSION_DENIED, TIMEOUT, AX_ERROR, INPUT_ERROR, INVALID_COMMAND, DAEMON_ERROR. Each with code, message, and suggestion.
- [x] 8.2 Add element re-identification: when resolving a ref for action, first try re-traversing the stored axPath. If element not found at path, try coordinate fallback. If neither works, return stale ref error.
- [x] 8.3 Add permission-specific error handling: detect missing Accessibility permission (AXIsProcessTrusted), detect missing Screen Recording permission, return specific setup instructions for each.

## 9. End-to-End Integration Test

- [x] 9.1 Create a test script that exercises the full demo flow: `open "TextEdit"` → `snapshot -i` → verify output has @refs → `click @e<text_area>` → `type @e<ref> "Hello from agent-desktop!"` → `press cmd+a` → `screenshot` → verify PNG exists → `status` → verify all green. Script should run unattended and report pass/fail.
- [x] 9.2 Test against Finder: open → snapshot → click a folder → snapshot again to verify UI changed.
- [x] 9.3 Build release binary, test `agent-desktop --help`, verify daemon auto-start, verify clean shutdown.
