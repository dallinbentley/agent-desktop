## MODIFIED Requirements

### Requirement: JSON output mode
The bridge SHALL pass `--json` to all agent-browser subprocess calls and parse the JSON response with serde instead of regex text parsing.

#### Scenario: Snapshot via JSON
- **WHEN** the bridge calls `agent-browser --json snapshot -i`
- **THEN** it deserializes the JSON response containing `refs` map and `snapshot` text
- **THEN** ElementRefs are built from the structured `refs` data (name, role per ref)
- **THEN** the snapshot text from the `data.snapshot` field is used directly

#### Scenario: Action commands via JSON
- **WHEN** the bridge calls click, fill, type, press, scroll via `--json`
- **THEN** it checks `success` field for pass/fail
- **THEN** it extracts error messages from the `error` field on failure

### Requirement: Pre-warm CDP sessions
The bridge SHALL connect agent-browser sessions during `open --with-cdp` to hide daemon startup latency.

#### Scenario: App opened with CDP
- **WHEN** `open --with-cdp` successfully launches an Electron app
- **THEN** the bridge immediately runs `agent-browser --session <app> connect <cdp_port>`
- **THEN** the first snapshot command does not pay daemon cold-start cost

### Requirement: Async subprocess execution
Bridge subprocess calls SHALL use async process spawning so the daemon event loop is not blocked.

#### Scenario: Concurrent native and Electron commands
- **WHEN** an Electron snapshot is in flight (~160ms)
- **THEN** native app commands (click, status) can be processed concurrently
