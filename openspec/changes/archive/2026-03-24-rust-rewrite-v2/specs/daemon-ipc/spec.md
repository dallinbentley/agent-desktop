## ADDED Requirements

### Requirement: Unix domain socket server with JSON protocol
The daemon SHALL listen on ~/.agent-desktop/daemon.sock, accepting newline-delimited JSON requests and returning JSON responses. Each request/response SHALL have an id field for correlation and responses SHALL include timing.elapsed_ms.

#### Scenario: Command round-trip
- **WHEN** CLI sends {"id":"r1","command":"status","args":{}}
- **THEN** daemon returns {"id":"r1","success":true,"data":{...},"timing":{"elapsed_ms":...}}

### Requirement: CLI auto-starts daemon
The CLI SHALL check for daemon socket, spawn daemon as background process if missing, poll for socket (100ms intervals, 5s timeout), then connect. Stale socket files SHALL be detected and removed.

#### Scenario: First use auto-start
- **WHEN** user runs any command and daemon is not running
- **THEN** CLI prints "Starting agent-desktop daemon..." and spawns daemon

### Requirement: Daemon maintains state across invocations
The daemon SHALL maintain: unified RefMap, active CDP connections, app classification cache, and last snapshot metadata. State persists across CLI invocations until daemon shutdown.

#### Scenario: CDP connection persists
- **WHEN** user runs snapshot on Chrome (CDP connects) then runs click @e5
- **THEN** daemon reuses the existing CDP WebSocket connection

### Requirement: Clean shutdown on SIGTERM
The daemon SHALL handle SIGTERM/SIGINT by: closing all CDP WebSocket connections, closing the Unix socket, removing the socket file, and exiting with code 0.

#### Scenario: Graceful shutdown
- **WHEN** daemon receives SIGTERM
- **THEN** all connections closed, socket file removed, exit 0
