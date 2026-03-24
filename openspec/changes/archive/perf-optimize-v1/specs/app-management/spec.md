## MODIFIED Requirements

### Requirement: App listing uses native APIs
The `get apps` command uses `NSWorkspace.shared.runningApplications` via objc FFI instead of spawning an osascript subprocess. Returns the same data: app name, PID, frontmost status.

#### Scenario: List running GUI apps
- **WHEN** `get apps` is executed
- **THEN** results match current osascript output (all non-background processes with name, PID, frontmost flag)
- **THEN** daemon-side execution completes in under 5ms

### Requirement: Window title uses AX API
The `handle_status` and related functions use AXUIElement kAXFocusedWindowAttribute → kAXTitleAttribute instead of osascript for window title retrieval.

#### Scenario: Get frontmost window title
- **WHEN** the status command needs the frontmost window title
- **THEN** it queries the AX API directly (no subprocess spawn)
- **THEN** returns the same title string as the osascript approach
- **THEN** completes in under 2ms

### Requirement: Process existence check uses kill -0
Functions checking if an app is running use `kill(pid, 0)` (signal 0 existence check) or NSRunningApplication lookup instead of osascript process name queries.

#### Scenario: Check if app is running
- **WHEN** `open_app` checks if an app is already running
- **THEN** it uses `kill -0 <pid>` or process table lookup
- **THEN** completes in under 1ms

### Requirement: Graceful quit keeps osascript
The `tell application "X" to quit` osascript call is retained for graceful app termination since it's infrequent (only during `open --with-cdp` relaunch) and the AppleEvent approach is more reliable than NSRunningApplication.terminate().

#### Scenario: Quit app before relaunch
- **WHEN** `open --with-cdp` needs to quit an existing instance
- **THEN** osascript quit is used (acceptable since it's a one-time operation)
