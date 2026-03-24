## ADDED Requirements

### Requirement: Scope snapshot to CSS selector
The system SHALL support `snapshot --selector <css>` which passes through to agent-browser's `-s` flag for CDP-sourced snapshots, reducing the number of elements returned.

#### Scenario: Scoped Spotify snapshot
- **WHEN** agent runs `snapshot -i --app Spotify --selector "#main-content"`
- **THEN** agent-browser receives `-s "#main-content"` and returns only elements within that container
