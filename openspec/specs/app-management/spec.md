# app-management Specification

## Purpose
TBD - created by archiving change mvp-agent-computer. Update Purpose after archive.
## Requirements
### Requirement: Open and focus application by name
The system SHALL launch or activate an application by its display name using NSWorkspace. If the app is already running, it SHALL activate (bring to front). If not running, it SHALL launch it.

#### Scenario: Focus running app
- **WHEN** user runs `agent-computer open "Safari"` and Safari is running
- **THEN** system activates Safari, bringing it to the front

#### Scenario: Launch app not running
- **WHEN** user runs `agent-computer open "TextEdit"` and TextEdit is not running
- **THEN** system launches TextEdit and waits for it to become active

#### Scenario: App not found
- **WHEN** user runs `agent-computer open "Safarri"` (typo)
- **THEN** system returns error listing running apps and suggesting closest match

### Requirement: Report frontmost app and window info
The `status` command SHALL report the currently frontmost application name, PID, and frontmost window title.

#### Scenario: Status shows active app
- **WHEN** user runs `agent-computer status` with Finder in front
- **THEN** response includes `frontmost: { app: "Finder", pid: 456, window: "Documents" }`

### Requirement: List running applications
The system SHALL provide a list of running GUI applications with their names, PIDs, and active status.

#### Scenario: Get apps list
- **WHEN** user runs `agent-computer get apps`
- **THEN** system returns list of running apps with name, PID, and isActive flag

