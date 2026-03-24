## ADDED Requirements

### Requirement: Traverse AX tree with batch attribute fetching
The AX engine SHALL traverse the macOS accessibility tree for a given PID using AXUIElementCopyMultipleAttributeValues for batch fetching. It SHALL respect depth limits (default 10) and a 3-second timeout, returning partial results on timeout.

#### Scenario: Snapshot of native app
- **WHEN** AX engine traverses System Settings
- **THEN** returns interactive elements with roles, labels, frames, and actions within 500ms

#### Scenario: Depth limit respected
- **WHEN** depth limit is set to 5
- **THEN** traversal stops at depth 5 and returns elements found

### Requirement: AX-first headless click via AXPress
The AX engine SHALL attempt AXUIElementPerformAction with kAXPressAction before falling back to CGEvent. AXPress SHALL work on background (non-frontmost) apps without stealing focus.

#### Scenario: Headless button click
- **WHEN** agent clicks @e3 (a button in a background native app)
- **THEN** AXPress is attempted first; if successful, no focus change occurs

#### Scenario: AXPress fallback to CGEvent
- **WHEN** AXPress returns an error (e.g., web content in Safari)
- **THEN** system falls back to CGEvent click at element coordinates

### Requirement: AX-first headless text fill via kAXValueAttribute
The AX engine SHALL attempt kAXValueAttribute for fill commands as the primary method. If that silently fails (value doesn't change on read-back), it SHALL try selection-replace (kAXSelectedTextRangeAttribute + kAXSelectedTextAttribute). If both fail, fall back to CGEvent Cmd+A + type.

#### Scenario: Fill text field in background app
- **WHEN** agent fills @e4 with "hello" in a background native app
- **THEN** kAXValueAttribute is tried first, verified via read-back

#### Scenario: Silent fail detection
- **WHEN** kAXValueAttribute returns success but value doesn't change
- **THEN** system detects via read-back and falls to selection-replace

### Requirement: AX action verification
After performing any AX action, the engine SHALL verify the action took effect by reading back relevant attributes where possible.

#### Scenario: Click verification
- **WHEN** AXPress is performed on a checkbox
- **THEN** engine reads kAXValueAttribute to confirm state changed
