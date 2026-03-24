## ADDED Requirements

### Requirement: Pipe-based daemon startup signaling
The CLI process creates an OS pipe before spawning the daemon. The write end is passed to the daemon as an inherited file descriptor. After the daemon successfully binds the Unix socket, it writes a single byte (0x01) to the pipe and closes it. The CLI blocks on reading the pipe's read end, receiving instant notification that the socket is ready.

#### Scenario: Cold start with ready-pipe
- **WHEN** the CLI detects no daemon is running and spawns a new daemon process with an inherited pipe fd
- **THEN** the CLI blocks on the pipe read (not polling) and connects to the socket within 5ms of the daemon binding it

#### Scenario: Pipe fd not available (manual daemon start)
- **WHEN** the daemon is started manually without a pipe fd (e.g., `agent-computer-daemon` directly)
- **THEN** the daemon skips the pipe write and starts normally; the CLI falls back to socket polling at 10ms intervals

#### Scenario: Daemon fails to start
- **WHEN** the daemon process exits before writing to the pipe
- **THEN** the CLI detects EOF on the pipe read within 100ms and returns a startup error
