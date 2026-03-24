## ADDED Requirements

### Requirement: Traverse accessibility tree for frontmost app
The system SHALL traverse the macOS accessibility tree (AXUIElement) for the frontmost application's frontmost window, extracting element role, title, description, frame, and available actions.

#### Scenario: Snapshot of a simple app
- **WHEN** user runs `agent-computer snapshot -i` with TextEdit as frontmost app
- **THEN** system traverses the accessibility tree and returns interactive elements with roles and labels

#### Scenario: Snapshot with app targeting
- **WHEN** user runs `agent-computer snapshot -i --app "Finder"`
- **THEN** system traverses Finder's accessibility tree regardless of which app is frontmost

#### Scenario: Snapshot respects depth limit
- **WHEN** user runs `agent-computer snapshot -i -d 5`
- **THEN** system traverses no deeper than 5 levels and returns elements found within that depth

#### Scenario: Snapshot times out gracefully
- **WHEN** accessibility tree traversal exceeds 3 seconds
- **THEN** system returns partial results with a warning indicating the timeout and element count

### Requirement: Filter to interactive elements only
The system SHALL filter the accessibility tree to include only interactive element roles: AXButton, AXTextField, AXTextArea, AXCheckBox, AXRadioButton, AXPopUpButton, AXComboBox, AXSlider, AXLink, AXMenuItem, AXMenuButton, AXTab, AXTabGroup, AXScrollArea, AXTable, AXOutline, AXSwitch, AXSearchField, AXIncrementor.

#### Scenario: Non-interactive elements excluded
- **WHEN** user runs `agent-computer snapshot -i` on an app with static text, images, and buttons
- **THEN** only elements with interactive roles receive @refs; static text and images are excluded from ref assignment

### Requirement: Assign sequential @refs to interactive elements
The system SHALL assign sequential references (`@e1`, `@e2`, `@e3`, ...) to each interactive element discovered during traversal.

#### Scenario: Refs assigned in tree order
- **WHEN** snapshot is taken of an app with 5 interactive elements
- **THEN** elements are assigned @e1 through @e5 in document/tree order (top-to-bottom, left-to-right)

### Requirement: Build and store ref map
The system SHALL store a ref map in daemon memory mapping each ref ID to: element path (role+index chain), role, label, frame (CGRect), and available actions. The ref map SHALL be invalidated when a new snapshot is taken.

#### Scenario: Ref map persists between commands
- **WHEN** user takes a snapshot then runs `click @e3`
- **THEN** daemon resolves @e3 from the stored ref map

#### Scenario: Ref map invalidated on re-snapshot
- **WHEN** user takes a new snapshot
- **THEN** previous ref map is discarded and new refs are assigned starting from @e1

### Requirement: Format snapshot as compact text
The system SHALL output the snapshot as an indented text tree with window title header, structural context (toolbar, content area), and one line per interactive element showing `@ref role "label"`.

#### Scenario: Typical snapshot output
- **WHEN** snapshot is taken of TextEdit with a document open
- **THEN** output includes window title header, @refs for buttons/menus/text areas, and is under 500 tokens

### Requirement: Batch attribute fetching for performance
The system SHALL use `AXUIElementCopyMultipleAttributeValues` to fetch multiple attributes per element in a single call rather than individual attribute queries.

#### Scenario: Performance on typical app
- **WHEN** snapshot is taken of a typical app (< 500 elements)
- **THEN** traversal completes in under 500ms
