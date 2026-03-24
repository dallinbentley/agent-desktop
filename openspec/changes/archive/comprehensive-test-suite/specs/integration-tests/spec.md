## ADDED Requirements

### Requirement: Daemon lifecycle tests
Integration tests SHALL verify daemon start, status, and shutdown.

#### Scenario: Status command
- **WHEN** the daemon is running
- **THEN** `status` returns accessibility and screen recording permission info
- **THEN** response is `Ok` with `ResponseData::Status`

#### Scenario: Daemon auto-start
- **WHEN** the CLI sends a command with no daemon running
- **THEN** the daemon starts automatically
- **THEN** the command succeeds

### Requirement: Snapshot round-trip tests
Integration tests SHALL verify snapshot commands produce valid output.

#### Scenario: Native app snapshot
- **WHEN** `snapshot -i --app Finder` is sent
- **THEN** response contains `ResponseData::Snapshot` with non-empty text
- **THEN** `ref_count >= 0`
- **THEN** snapshot text starts with `[Finder]`

#### Scenario: Snapshot with depth limit
- **WHEN** `snapshot -i -d 3 --app Finder` is sent
- **THEN** response succeeds
- **THEN** the tree depth does not exceed 3 levels of indentation

### Requirement: Action round-trip tests
Integration tests SHALL verify click, fill, type, press, scroll commands.

#### Scenario: Click invalid ref
- **WHEN** `click @e9999` is sent (ref doesn't exist)
- **THEN** response is an error mentioning the ref not found

#### Scenario: Press key
- **WHEN** `press escape` is sent with `--app Finder`
- **THEN** response is `Ok` confirming the key was pressed

#### Scenario: Scroll
- **WHEN** `scroll down 100` is sent
- **THEN** response is `Ok` confirming scroll direction and amount

### Requirement: Screenshot round-trip tests
Integration tests SHALL verify screenshot capture works.

#### Scenario: Window screenshot
- **WHEN** `screenshot --app Finder` is sent
- **THEN** response contains a file path to a PNG image
- **THEN** the file exists and is non-empty

#### Scenario: Full screen screenshot
- **WHEN** `screenshot --full` is sent
- **THEN** response contains a file path to a PNG image

### Requirement: App management tests
Integration tests SHALL verify open, get apps, get windows.

#### Scenario: Get running apps
- **WHEN** `get apps` is sent
- **THEN** response contains a list of running GUI app names
- **THEN** list includes at least "Finder" (always running on macOS)

#### Scenario: Open app
- **WHEN** `open Finder` is sent
- **THEN** response succeeds
- **THEN** Finder is the frontmost app (or was already running)

### Requirement: Error handling tests
Integration tests SHALL verify error responses for invalid inputs.

#### Scenario: Non-existent app
- **WHEN** `snapshot --app NonExistentApp12345` is sent
- **THEN** response is an error indicating app not found

#### Scenario: Invalid ref format
- **WHEN** `click @xyz` is sent
- **THEN** response is an error about invalid ref format

#### Scenario: Fill without prior snapshot
- **WHEN** `fill @e1 "text"` is sent with no prior snapshot (empty refmap)
- **THEN** response is an error about ref not found

### Requirement: CLI argument parsing tests
Unit tests SHALL verify all subcommands parse correctly.

#### Scenario: All subcommands parse
- **WHEN** each subcommand is parsed (snapshot, click, fill, type, press, scroll, screenshot, open, get, wait, status, install-browser)
- **THEN** clap returns the correct `Commands` variant with correct field values

#### Scenario: Global flags
- **WHEN** `--json`, `--timeout 5000`, `--verbose` are passed
- **THEN** `Cli` struct has `json=true`, `timeout=Some(5000)`, `verbose=true`

#### Scenario: Edge cases
- **WHEN** `click 100 200` is sent (coordinate click)
- **THEN** `ref_or_x="100"`, `y=Some(200.0)`
- **WHEN** `type @e3 "hello world"` is sent
- **THEN** `ref_or_text="@e3"`, `text=Some("hello world")`
