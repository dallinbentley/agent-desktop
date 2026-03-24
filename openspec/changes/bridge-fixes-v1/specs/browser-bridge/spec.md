## MODIFIED Requirements

### Requirement: Execute agent-browser commands via subprocess
The bridge SHALL execute agent-browser CLI commands via `std::process::Command`, passing flags like `--cdp <port>`, `--session <name>`, and command-specific arguments. It SHALL capture stdout, stderr, and exit code. It SHALL resolve the correct CDP port by looking up the target app's PID in the stored port mapping, NOT by scanning ports.

#### Scenario: Snapshot via agent-browser
- **WHEN** bridge calls `agent-browser --session spotify --cdp 9371 snapshot -i`
- **THEN** stdout contains accessibility tree text with [ref=eN] markers

#### Scenario: CDP port resolves to correct app
- **WHEN** Spotify is on port 9371 and Slack is on port 9229
- **THEN** `snapshot --app Spotify` uses port 9371, never 9229

#### Scenario: agent-browser not found
- **WHEN** agent-browser binary is not in PATH or at known locations
- **THEN** bridge returns a clear error with install instructions

## ADDED Requirements

### Requirement: Delegate wait command to agent-browser
The bridge SHALL support `wait(session, cdp_port, args)` which delegates to `agent-browser --session <s> --cdp <port> wait <args>`.

#### Scenario: Wait for network idle
- **WHEN** bridge.wait(session, port, ["--load", "networkidle"]) is called
- **THEN** agent-browser wait --load networkidle is executed

### Requirement: Delegate get command to agent-browser
The bridge SHALL support `get(session, cdp_port, what, selector)` for reading text, value, title from web elements.

#### Scenario: Get text from web element
- **WHEN** bridge.get(session, port, "text", "@e32") is called
- **THEN** agent-browser get text @e32 is executed and result returned
