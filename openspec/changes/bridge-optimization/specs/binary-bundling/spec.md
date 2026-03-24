## ADDED Requirements

### Requirement: Binary detection order
The bridge SHALL check for agent-browser in this order: (1) bundled at `~/.agent-computer/bin/agent-browser`, (2) system PATH via `which`, (3) common npm/nvm paths. First found wins.

#### Scenario: Bundled binary exists
- **WHEN** `~/.agent-computer/bin/agent-browser` exists and is executable
- **THEN** the bridge uses it without checking PATH

#### Scenario: No bundled binary, found in PATH
- **WHEN** the bundled path doesn't exist but `which agent-browser` succeeds
- **THEN** the bridge uses the PATH binary

### Requirement: Auto-download on first use
When no agent-browser binary is found, the bridge SHALL download the platform-appropriate binary from the agent-browser npm registry and cache it at `~/.agent-computer/bin/agent-browser`.

#### Scenario: First use with no agent-browser installed
- **WHEN** the bridge detects no binary anywhere
- **THEN** it downloads the correct platform binary (e.g., `agent-browser-darwin-arm64`)
- **THEN** it marks it executable and caches at `~/.agent-computer/bin/agent-browser`
- **THEN** subsequent uses find it via bundled path check

#### Scenario: Download fails
- **WHEN** the download fails (no network, 404, etc.)
- **THEN** the bridge logs a clear error with manual install instructions
- **THEN** Electron/browser features are disabled but native app features work

### Requirement: Version pinning
The bridge SHALL pin to a specific agent-browser version and only download that version.

#### Scenario: Version check
- **WHEN** a bundled binary exists
- **THEN** the bridge MAY check `agent-browser --version` on startup and warn if mismatched
