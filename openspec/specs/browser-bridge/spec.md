# browser-bridge Specification

## Purpose
TBD - created by archiving change agent-browser-bridge. Update Purpose after archive.
## Requirements
### Requirement: Execute agent-browser commands via subprocess
The bridge SHALL execute agent-browser CLI commands via `std::process::Command`, passing flags like `--cdp <port>`, `--session <name>`, and command-specific arguments. It SHALL capture stdout, stderr, and exit code.

#### Scenario: Snapshot via agent-browser
- **WHEN** bridge calls `agent-browser --session spotify --cdp 9371 snapshot -i`
- **THEN** stdout contains accessibility tree text with [ref=eN] markers

#### Scenario: Click via agent-browser
- **WHEN** bridge calls `agent-browser --session spotify --cdp 9371 click @e32`
- **THEN** agent-browser clicks the element headlessly via CDP and exits with code 0

#### Scenario: agent-browser not found
- **WHEN** agent-browser binary is not in PATH or at known locations
- **THEN** bridge returns a clear error with install instructions

### Requirement: Parse agent-browser snapshot output into ElementRefs
The bridge SHALL parse agent-browser's text snapshot output, extracting for each element: ref ID (from `[ref=eN]`), role (first token), label (quoted string), and attributes (disabled, expanded, etc). Each parsed element SHALL be mapped to an ElementRef with `source: CDP`.

#### Scenario: Parse Spotify snapshot
- **WHEN** agent-browser returns `- button "Home" [ref=e14]`
- **THEN** bridge creates ElementRef { id: "e14", role: "button", label: "Home", source: CDP }

#### Scenario: Parse nested elements
- **WHEN** agent-browser returns indented elements (2-space indentation per level)
- **THEN** bridge preserves hierarchy in the output text while extracting flat ref list

### Requirement: Map agent-browser refs to unified RefMap
The bridge SHALL store each parsed agent-browser ref in the unified RefMap with a new sequential ID (@e1, @e2...) and store the original agent-browser ref ID for delegation. When an interaction targets a web-sourced ref, the bridge SHALL use the stored original ref to call agent-browser.

#### Scenario: Ref mapping for click
- **WHEN** unified RefMap assigns @e7 to an element whose agent-browser ref is "e32"
- **THEN** `click @e7` dispatches `agent-browser --cdp <port> click @e32`

### Requirement: Merged snapshot for browser windows
For browser/Electron apps with CDP, the bridge SHALL: (1) take AX snapshot of browser chrome (stopping at AXWebArea boundary), (2) call agent-browser for web content, (3) merge into single output with continuous ref numbering.

#### Scenario: Chrome merged snapshot
- **WHEN** snapshot targets Chrome with CDP on port 9222
- **THEN** output shows AX refs for Back/Forward/Address bar, then `--- web content ---`, then agent-browser refs for page elements

### Requirement: Support all interaction commands via bridge
The bridge SHALL support: click, fill, type, press, scroll, screenshot, and get text — all delegated to agent-browser for web-sourced refs.

#### Scenario: Fill web input
- **WHEN** agent fills @e9 (web-sourced, original ref "e43") with "search query"
- **THEN** bridge calls `agent-browser --session <app> --cdp <port> fill @e43 "search query"`

#### Scenario: Press key in web context
- **WHEN** agent presses Enter while web content is focused
- **THEN** bridge calls `agent-browser --session <app> --cdp <port> press Enter`

