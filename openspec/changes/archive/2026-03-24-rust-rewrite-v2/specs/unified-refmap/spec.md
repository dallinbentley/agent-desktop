## ADDED Requirements

### Requirement: Transparent refs regardless of source
The unified RefMap SHALL assign sequential @e1, @e2, @e3 refs regardless of whether the element came from the AX engine or CDP engine. The AI agent SHALL NOT need to know the source.

#### Scenario: Mixed AX and CDP refs
- **WHEN** snapshot of Chrome produces 4 AX elements (browser chrome) and 6 CDP elements (web content)
- **THEN** refs are @e1 through @e10, continuously numbered

### Requirement: Source-aware dispatch for interactions
When an agent interacts with a ref, the RefMap SHALL resolve the ref's source and dispatch to the correct engine: AX actions for AX-sourced refs, CDP commands for CDP-sourced refs, CGEvent for coordinate-sourced refs.

#### Scenario: Click AX-sourced ref
- **WHEN** agent clicks @e2 (AX source: browser Back button)
- **THEN** dispatched to AX engine (AXPress)

#### Scenario: Click CDP-sourced ref
- **WHEN** agent clicks @e7 (CDP source: web page link)
- **THEN** dispatched to CDP engine (DOM click)

### Requirement: Merged snapshot for browser windows
For browser/Electron apps with CDP available, the snapshot SHALL merge AX elements (browser chrome: address bar, tabs, toolbar) with CDP elements (page content). The AXWebArea boundary SHALL be used to determine where AX stops and CDP begins.

#### Scenario: Chrome merged snapshot
- **WHEN** snapshot of Chrome with CDP on a GitHub page
- **THEN** output shows AX refs for Back/Forward/Address bar, then "--- web content ---" marker, then CDP refs for page links/buttons/inputs

### Requirement: Ref invalidation across sources
When a new snapshot is taken, ALL refs (both AX and CDP sourced) SHALL be invalidated. The entire RefMap is rebuilt from scratch.

#### Scenario: Re-snapshot clears all refs
- **WHEN** new snapshot is taken after previous had 10 AX + 5 CDP refs
- **THEN** all 15 previous refs are invalid, new refs start from @e1

### Requirement: Element re-identification for AX refs
AX-sourced refs SHALL support re-identification via stored axPath (role+index chain). If path re-traversal fails, fall back to stored frame coordinates.

#### Scenario: AX ref still valid after minor UI change
- **WHEN** agent clicks @e3 and the element still exists at its stored path
- **THEN** re-traversal finds it and returns current frame coordinates
