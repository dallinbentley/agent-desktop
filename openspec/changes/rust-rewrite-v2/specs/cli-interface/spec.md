## ADDED Requirements

### Requirement: Same command grammar as Swift MVP
The CLI SHALL support the same commands: snapshot, click, fill, type, press, scroll, screenshot, open, get, status. Same @ref syntax, same flag names, same argument patterns. An AI agent's workflow SHALL NOT change between Swift and Rust versions.

#### Scenario: Snapshot command
- **WHEN** `agent-computer snapshot -i --app "System Settings"`
- **THEN** same output format as Swift version

### Requirement: New open --with-cdp flag
The CLI SHALL support `agent-computer open --with-cdp <app>` which relaunches the target app with --remote-debugging-port. It SHALL assign a deterministic port and track it in daemon state.

#### Scenario: Relaunch Electron app with CDP
- **WHEN** `agent-computer open --with-cdp Spotify`
- **THEN** Spotify is relaunched with CDP enabled, port is reported, daemon connects

### Requirement: Human-readable and JSON output
Default output SHALL be colored human-readable text. --json flag SHALL output raw JSON. Snapshot shows tree with @refs. Actions show brief confirmations. Errors show red text with yellow suggestions.

#### Scenario: JSON mode
- **WHEN** `agent-computer snapshot -i --json`
- **THEN** raw JSON response from daemon is printed

### Requirement: AI-friendly error messages
All errors SHALL include code, message, and actionable suggestion. Errors SHALL reference specific commands to resolve the issue.

#### Scenario: CDP not available
- **WHEN** snapshot targets an Electron app without CDP
- **THEN** error includes suggestion: "Run `agent-computer open --with-cdp <app>` to enable rich UI interaction."

#### Scenario: No refs available
- **WHEN** click @e3 without prior snapshot
- **THEN** error says: "No element references available. Run `snapshot -i` first."

### Requirement: Non-zero exit code on failure
Exit code 0 on success, 1 on failure.

#### Scenario: Failed command
- **WHEN** any command fails
- **THEN** exit code is 1
