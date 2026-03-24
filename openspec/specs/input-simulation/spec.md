# input-simulation Specification

## Purpose
TBD - created by archiving change mvp-agent-computer. Update Purpose after archive.
## Requirements
### Requirement: Click element by ref
The system SHALL resolve a @ref to screen coordinates (center of element frame) and simulate a mouse click via CGEvent. It SHALL use `CGWarpMouseCursorPosition` for cursor positioning followed by mouseDown/mouseUp events.

#### Scenario: Click a button
- **WHEN** user runs `agent-computer click @e3` where @e3 is a button
- **THEN** system resolves @e3 to coordinates, warps cursor, and sends left click at that position

#### Scenario: Double click
- **WHEN** user runs `agent-computer click @e3 --double`
- **THEN** system sends a double-click event with mouseEventClickState set to 2

#### Scenario: Right click
- **WHEN** user runs `agent-computer click @e3 --right`
- **THEN** system sends a right-click (context menu) event at the element's position

#### Scenario: Click at coordinates
- **WHEN** user runs `agent-computer click 500 300`
- **THEN** system clicks at absolute screen coordinates (500, 300) without ref resolution

#### Scenario: Click stale ref
- **WHEN** user runs `agent-computer click @e3` but the element no longer exists at the stored path
- **THEN** system falls back to stored frame coordinates; if element cannot be located at all, returns error with suggestion to re-snapshot

### Requirement: Type text using Unicode API
The system SHALL type text into the focused element using `CGEvent.keyboardSetUnicodeString`, chunking strings at 20 UTF-16 units per event with brief delays between chunks.

#### Scenario: Type ASCII text
- **WHEN** user runs `agent-computer type @e4 "Hello World"`
- **THEN** system focuses @e4 and types "Hello World" character-by-character via Unicode events

#### Scenario: Type Unicode text
- **WHEN** user runs `agent-computer type @e4 "café"`
- **THEN** system correctly types all characters including the accented "é"

#### Scenario: Type into focused element
- **WHEN** user runs `agent-computer type "some text"` without a ref
- **THEN** system types into the currently focused element

### Requirement: Fill text field with replacement
The system SHALL implement `fill` by selecting all text in the target element (via AX `kAXSelectedTextRangeAttribute`) and replacing it (via `kAXSelectedTextAttribute`). It SHALL NOT use direct `kAXValueAttribute` setting.

#### Scenario: Fill replaces existing text
- **WHEN** user runs `agent-computer fill @e4 "new text"` where @e4 contains "old text"
- **THEN** system selects all text in @e4 and replaces with "new text"

### Requirement: Press keys and key combinations
The system SHALL simulate key presses using CGEvent with virtual keycodes and modifier flags. It SHALL support named keys (Enter, Tab, Escape, Space, Delete, arrow keys) and modifier combinations (cmd+c, cmd+shift+s).

#### Scenario: Press Enter
- **WHEN** user runs `agent-computer press Enter`
- **THEN** system sends keyDown/keyUp events for Return (keycode 36)

#### Scenario: Press modifier combo
- **WHEN** user runs `agent-computer press cmd+c`
- **THEN** system sends keyDown with Command flag set for key 'c', then keyUp

#### Scenario: Press complex combo
- **WHEN** user runs `agent-computer press cmd+shift+s`
- **THEN** system sends key event with both Command and Shift flags for key 's'

### Requirement: Scroll in direction
The system SHALL simulate scroll wheel events via CGEvent in the specified direction (up, down, left, right) with configurable pixel amount.

#### Scenario: Scroll down
- **WHEN** user runs `agent-computer scroll down 300`
- **THEN** system sends scroll wheel events equivalent to 300 pixels downward

