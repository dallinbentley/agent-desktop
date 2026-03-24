## ADDED Requirements

### Requirement: Detect agent-browser binary at daemon startup
The daemon SHALL check for agent-browser in: (1) PATH lookup, (2) common npm global paths, (3) brew paths. The result SHALL be cached for the daemon's lifetime.

#### Scenario: agent-browser found
- **WHEN** daemon starts and agent-browser is in PATH
- **THEN** bridge is enabled, binary path is cached

#### Scenario: agent-browser not found
- **WHEN** daemon starts and agent-browser is not available
- **THEN** bridge is disabled, web/Electron apps fall back to AX-only with warning

### Requirement: Use --session per app for isolation
Each Electron/browser app SHALL get its own agent-browser session via `--session <app-name>`. This isolates CDP connections so multiple apps can be controlled simultaneously.

#### Scenario: Simultaneous Spotify and Chrome
- **WHEN** agent takes snapshots of both Spotify (CDP 9371) and Chrome (CDP 9222)
- **THEN** bridge uses `--session spotify --cdp 9371` and `--session chrome --cdp 9222` respectively

### Requirement: CDP port management for Electron apps
The `open --with-cdp` command SHALL detect Electron apps, quit the running instance, relaunch with `--remote-debugging-port=<port>`, and wait for CDP to be ready. The assigned port SHALL be tracked in daemon state and passed to agent-browser.

#### Scenario: Launch Spotify with CDP
- **WHEN** `agent-computer open --with-cdp Spotify`
- **THEN** Spotify relaunches with CDP on deterministic port, daemon records port, subsequent snapshots use agent-browser

### Requirement: Clean shutdown closes agent-browser sessions
On daemon shutdown (SIGTERM), the daemon SHALL call `agent-browser close` for each active session to clean up agent-browser's daemon connections.

#### Scenario: Daemon shutdown
- **WHEN** daemon receives SIGTERM with 2 active agent-browser sessions
- **THEN** calls `agent-browser --session spotify close` and `agent-browser --session chrome close` before exiting
