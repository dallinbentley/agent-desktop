## MODIFIED Requirements

### Requirement: Cold start daemon discovery
The CLI uses a ready-pipe instead of polling when spawning the daemon. Polling interval reduced from 100ms to 10ms as fallback.

#### Scenario: CLI spawns daemon (pipe available)
- **WHEN** the CLI spawns the daemon process
- **THEN** it creates a pipe, passes write fd to daemon, and blocks on read fd until ready byte arrives
- **THEN** total cold start time (spawn + signal + connect + first command) is under 50ms

#### Scenario: Fallback polling
- **WHEN** the pipe mechanism fails or daemon was started externally
- **THEN** the CLI polls at 10ms intervals (down from 100ms) with a 5s timeout
