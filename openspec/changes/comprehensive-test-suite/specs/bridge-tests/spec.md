## ADDED Requirements

### Requirement: Bridge snapshot test
Integration tests SHALL verify agent-browser bridge snapshot works for Electron apps.

#### Scenario: Electron app snapshot via CDP
- **WHEN** `snapshot -i --app Slack` is sent (Slack is an Electron app)
- **THEN** response contains `ResponseData::Snapshot` with refs from CDP
- **THEN** snapshot text contains web UI elements (buttons, links, etc.)
- **THEN** refs have `ab_ref` and `ab_session` fields populated

### Requirement: Bridge action test
Integration tests SHALL verify bridge actions work on Electron apps.

#### Scenario: Click via bridge
- **WHEN** a snapshot of an Electron app is taken (populating refmap)
- **THEN** `click @e1` is sent (first interactive element)
- **THEN** response is `Ok` confirming the click

#### Scenario: Fill via bridge
- **WHEN** a text input ref exists in the Electron app snapshot
- **THEN** `fill @eN "test text"` is sent
- **THEN** response is `Ok`

### Requirement: Bridge error handling
Integration tests SHALL verify bridge error paths.

#### Scenario: Bridge unavailable
- **WHEN** agent-browser binary is not found
- **THEN** Electron app snapshot falls back to AX tree
- **THEN** a warning is logged but the command does not crash

#### Scenario: CDP port not available
- **WHEN** an Electron app is targeted but CDP is not enabled
- **THEN** snapshot falls back to AX tree
- **THEN** response includes AX-sourced data (not CDP)

### Requirement: Test categorization
Bridge tests SHALL be marked `#[ignore]` by default since they require agent-browser + a running Electron app.

#### Scenario: Default test run skips bridge tests
- **WHEN** `cargo test` is run without flags
- **THEN** bridge tests are skipped
- **WHEN** `cargo test -- --ignored` is run
- **THEN** bridge tests execute
