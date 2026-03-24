## ADDED Requirements

### Requirement: Parse command grammar
The CLI SHALL parse commands in the format `agent-computer <command> [args] [options]` using Swift Argument Parser. It SHALL support: snapshot, click, fill, type, press, scroll, screenshot, open, get, status.

#### Scenario: Valid snapshot command
- **WHEN** user runs `agent-computer snapshot -i -d 10`
- **THEN** CLI parses command as snapshot with interactive=true, depth=10

#### Scenario: Valid click with ref
- **WHEN** user runs `agent-computer click @e3`
- **THEN** CLI parses ref as "e3", strips the @ prefix

#### Scenario: Valid press with modifiers
- **WHEN** user runs `agent-computer press cmd+shift+s`
- **THEN** CLI parses key as "s" with modifiers [cmd, shift]

#### Scenario: Unknown command
- **WHEN** user runs `agent-computer frobnicate`
- **THEN** CLI prints usage help listing available commands

### Requirement: Human-readable output by default
The CLI SHALL format responses as colored, human-readable text by default. Snapshot output SHALL show the indented tree with @refs. Action results SHALL show a brief confirmation.

#### Scenario: Snapshot output
- **WHEN** user runs `agent-computer snapshot -i`
- **THEN** output is a readable text tree with @ref annotations

#### Scenario: Click confirmation
- **WHEN** user runs `agent-computer click @e3` successfully
- **THEN** output shows brief confirmation: "Clicked @e3 button 'Submit' at (450, 230)"

### Requirement: JSON output mode
The CLI SHALL support `--json` flag that outputs raw JSON response from the daemon.

#### Scenario: JSON mode
- **WHEN** user runs `agent-computer snapshot -i --json`
- **THEN** output is the JSON response object with success, data, and timing fields

### Requirement: AI-friendly error messages
All error messages SHALL include a concrete recovery suggestion. Error messages SHALL reference specific commands the user can run to resolve the issue.

#### Scenario: Stale ref error
- **WHEN** user clicks a ref that no longer exists
- **THEN** error message says: "Element @e3 not found. The UI may have changed. Run `snapshot` to refresh element references."

#### Scenario: No ref map error
- **WHEN** user runs `click @e3` without having taken a snapshot
- **THEN** error message says: "No element references available. Run `snapshot -i` first to discover interactive elements."

#### Scenario: Permission error
- **WHEN** command fails due to missing Accessibility permission
- **THEN** error message says: "Accessibility permission required. Grant access in System Settings → Privacy & Security → Accessibility."

### Requirement: Non-zero exit code on failure
The CLI SHALL exit with code 0 on success and non-zero on failure.

#### Scenario: Successful command
- **WHEN** any command succeeds
- **THEN** CLI exits with code 0

#### Scenario: Failed command
- **WHEN** any command fails
- **THEN** CLI exits with code 1

### Requirement: Status command shows system state
The `status` command SHALL report: daemon running state, accessibility permission, screen recording permission, frontmost app/window, display info, and ref map state.

#### Scenario: All permissions granted
- **WHEN** user runs `agent-computer status` with all permissions
- **THEN** output shows ✅ for each permission, daemon status, frontmost app, and ref map count
