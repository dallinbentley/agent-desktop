## ADDED Requirements

### Requirement: Mouse click via CGWarp + CGEvent
The input engine SHALL position the cursor using CGWarpMouseCursorPosition (NOT .mouseMoved events) and send CGEvent mouseDown/mouseUp. It SHALL support left click, right click, and double click (mouseEventClickState=2).

#### Scenario: Left click at coordinates
- **WHEN** click is requested at (500, 300)
- **THEN** cursor warps to (500,300) and left click events are posted

### Requirement: Type text via keyboardSetUnicodeString
The input engine SHALL type strings using CGEvent's keyboardSetUnicodeString API, chunking at 20 UTF-16 code units per event. Newlines SHALL be handled by sending Return keycode (36) separately.

#### Scenario: Type Unicode text
- **WHEN** typeString("café") is called
- **THEN** all characters including accented "é" are typed correctly

### Requirement: Press keys via virtual keycodes with modifiers
The input engine SHALL send key press events using CGEvent with virtual keycodes and modifier flags (.maskCommand, .maskShift, .maskAlternate, .maskControl).

#### Scenario: Press modifier combo
- **WHEN** press("cmd+shift+s") is requested
- **THEN** CGEvent is created with Command+Shift flags and keycode for 's'

### Requirement: Scroll via CGEvent scroll wheel
The input engine SHALL simulate scrolling via CGEvent scroll wheel events in up/down/left/right directions.

#### Scenario: Scroll down
- **WHEN** scroll(down, 300) is requested
- **THEN** scroll wheel events equivalent to 300 pixels downward are posted

### Requirement: Fill via AX selection-replace fallback
When the AX engine's kAXValueAttribute approach fails and selection-replace also fails, the input engine SHALL provide CGEvent-based fill: Cmd+A (select all) followed by typeString with the replacement text.

#### Scenario: CGEvent fill fallback
- **WHEN** AX fill methods fail for a text area
- **THEN** input engine sends Cmd+A then types replacement text via CGEvent
