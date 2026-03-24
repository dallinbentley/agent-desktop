## ADDED Requirements

### Requirement: Daemon listens on Unix domain socket
The daemon SHALL create and listen on a Unix domain socket at `~/.agent-computer/daemon.sock`, accepting newline-delimited JSON commands and returning JSON responses.

#### Scenario: Daemon accepts connection
- **WHEN** daemon is running and CLI connects to the socket
- **THEN** daemon accepts the connection and reads a JSON command line

#### Scenario: Daemon returns JSON response
- **WHEN** daemon receives a valid command
- **THEN** daemon returns a JSON response with `id`, `success`, `data`/`error`, and `timing` fields

### Requirement: CLI auto-starts daemon on first use
The CLI SHALL check if the daemon socket exists and is connectable. If not, it SHALL spawn the daemon as a background process, poll for the socket (100ms intervals, 5s timeout), and then connect.

#### Scenario: Daemon not running
- **WHEN** user runs any `agent-computer` command and daemon is not running
- **THEN** CLI spawns daemon, waits for socket, connects, and executes the command

#### Scenario: Daemon already running
- **WHEN** user runs a command and daemon socket is already accepting connections
- **THEN** CLI connects directly without spawning a new daemon

#### Scenario: Stale socket cleanup
- **WHEN** CLI finds a socket file but cannot connect (daemon crashed)
- **THEN** CLI removes the stale socket file, spawns a new daemon, and connects

### Requirement: Daemon maintains ref map state
The daemon SHALL maintain the ref map in memory across multiple CLI invocations. The ref map SHALL persist until a new `snapshot` command invalidates it or the daemon shuts down.

#### Scenario: Refs persist across commands
- **WHEN** user runs `snapshot -i` then `click @e3` as separate CLI invocations
- **THEN** daemon resolves @e3 from the ref map created during the snapshot command

### Requirement: Clean daemon shutdown
The daemon SHALL handle SIGTERM/SIGINT by closing the socket, removing the socket file, and exiting cleanly.

#### Scenario: Shutdown via SIGTERM
- **WHEN** daemon receives SIGTERM
- **THEN** daemon closes all connections, removes socket file, and exits with code 0

### Requirement: JSON protocol with request/response correlation
Each request SHALL include an `id` field. The response SHALL echo the same `id` for correlation. Commands SHALL include `command` name and `args` object. Responses SHALL include `success` boolean and either `data` or `error`.

#### Scenario: Command round-trip
- **WHEN** CLI sends `{"id":"req_1","command":"status","args":{}}`
- **THEN** daemon returns `{"id":"req_1","success":true,"data":{...},"timing":{"elapsed_ms":...}}`
