## ADDED Requirements

### Requirement: Capture frontmost window screenshot
The system SHALL capture a screenshot of the frontmost window using SCScreenshotManager and save it as a PNG file, returning the file path in the response.

#### Scenario: Default screenshot
- **WHEN** user runs `agent-desktop screenshot`
- **THEN** system captures the frontmost window as PNG, saves to temp directory, and returns the file path

#### Scenario: Full screen screenshot
- **WHEN** user runs `agent-desktop screenshot --full`
- **THEN** system captures the entire screen as PNG

### Requirement: Capture at 1x resolution by default
The system SHALL capture at 1x logical resolution by default (not 2x Retina physical resolution) to reduce file size for AI agent consumption. Full resolution SHALL be available via flag.

#### Scenario: Default 1x capture
- **WHEN** user runs `agent-desktop screenshot` on a Retina display
- **THEN** screenshot is at logical resolution (e.g., 1728×1117) not physical (3456×2234)

### Requirement: Detect screen recording permission
The system SHALL check screen recording permission via `CGPreflightScreenCaptureAccess()` before attempting capture and return an actionable error if permission is not granted.

#### Scenario: Permission denied
- **WHEN** user runs `agent-desktop screenshot` without Screen Recording permission
- **THEN** system returns error with message explaining how to grant permission in System Settings
