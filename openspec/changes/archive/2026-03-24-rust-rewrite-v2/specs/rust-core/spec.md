## ADDED Requirements

### Requirement: Cargo workspace with CLI, daemon, and shared crates
The project SHALL be structured as a Cargo workspace with three crates: `agent-desktop` (CLI binary), `agent-desktop-daemon` (daemon binary), and `agent-desktop-shared` (library). All protocol types, error codes, and constants SHALL live in the shared crate.

#### Scenario: Build produces two binaries
- **WHEN** `cargo build --release` is run
- **THEN** two binaries are produced: `agent-desktop` and `agent-desktop-daemon`

#### Scenario: Shared types used by both
- **WHEN** CLI sends a Request and daemon sends a Response
- **THEN** both use the same Codable types from agent-desktop-shared

### Requirement: All protocol types ported from Swift
The shared crate SHALL define: Request, Response, CommandArgs (snapshot, click, fill, type, press, scroll, screenshot, open, get, status), ResponseData variants, ElementRef, ErrorInfo, Timing, and all supporting types. All SHALL be serde Serializable/Deserializable.

#### Scenario: JSON round-trip
- **WHEN** a Request is serialized to JSON and deserialized back
- **THEN** all fields match the original

### Requirement: Key mapping and interactive roles ported
The shared crate SHALL include the virtual keycode mapping (key name → macOS keycode) and the set of interactive AX roles, matching the Swift implementation exactly.

#### Scenario: Key lookup
- **WHEN** looking up "enter"
- **THEN** returns keycode 36
