## MODIFIED Requirements

### Requirement: CDP port management for Electron apps
The `open --with-cdp` command SHALL: (1) detect Electron/CEF app, (2) if already running, force quit via osascript and wait for PID to exit, (3) relaunch with `--remote-debugging-port=<port>`, (4) wait for new PID to appear, (5) probe CDP port until ready (up to 10s), (6) store (new_pid, port, app_name) in daemon state. It SHALL NOT reuse an existing instance without relaunching.

#### Scenario: Relaunch already-running Spotify
- **WHEN** `open --with-cdp Spotify` and Spotify is running with PID 9745
- **THEN** Spotify is quit, relaunched with CDP, new PID assigned, CDP confirmed ready

#### Scenario: CDP port stored by PID
- **WHEN** Spotify relaunches with PID 10234 on port 9371
- **THEN** daemon stores mapping (10234, 9371, "Spotify") for accurate routing
