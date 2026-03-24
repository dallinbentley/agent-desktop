## ADDED Requirements

### Requirement: Connect to CDP endpoint via WebSocket
The CDP engine SHALL connect to a Chrome DevTools Protocol endpoint via WebSocket (tungstenite). It SHALL support connecting by port number (ws://localhost:<port>) or full WebSocket URL.

#### Scenario: Connect to Chrome CDP
- **WHEN** Chrome is running with --remote-debugging-port=9222
- **THEN** CDP engine connects via ws://localhost:9222 and retrieves browser info

#### Scenario: Connect to Electron app
- **WHEN** Slack is running with --remote-debugging-port=9229
- **THEN** CDP engine connects and can interact with Slack's web content

### Requirement: Get page accessibility tree via CDP
The CDP engine SHALL use Accessibility.getFullAXTree (or equivalent) to retrieve the page's accessibility tree. It SHALL walk the tree, filter to interactive roles, and assign @refs using the same logic as the AX engine.

#### Scenario: Snapshot of web page
- **WHEN** CDP snapshot is taken of a GitHub page in Chrome
- **THEN** returns interactive elements (links, buttons, textboxes) with @refs and labels

#### Scenario: Snapshot of Electron app
- **WHEN** CDP snapshot is taken of Spotify
- **THEN** returns full UI tree (buttons, search box, playlists, player controls)

### Requirement: CDP snapshot format matches AX snapshot format
The CDP engine's snapshot text output SHALL use the same format as the AX engine: indented tree with `@eN role "label"` per element. An AI agent SHALL NOT be able to distinguish AX-sourced from CDP-sourced snapshots.

#### Scenario: Format consistency
- **WHEN** snapshot is taken of a browser tab via CDP
- **THEN** output format matches `@e5 link "Pull requests"` style

### Requirement: Interact via CDP commands
The CDP engine SHALL support: clicking elements (Input.dispatchMouseEvent or DOM.focus + Runtime.callFunctionOn to call .click()), typing text (Input.dispatchKeyEvent or Input.insertText), and JavaScript evaluation (Runtime.evaluate).

#### Scenario: Click web element
- **WHEN** agent clicks @e5 (a CDP-sourced link)
- **THEN** CDP engine dispatches click event on the corresponding DOM node

#### Scenario: Type into web input
- **WHEN** agent types "search query" into @e7 (a CDP-sourced textbox)
- **THEN** CDP engine focuses the element and dispatches key events

### Requirement: Discover active CDP page/tab
When connected to a browser with multiple tabs, the CDP engine SHALL discover available pages via /json/list endpoint and connect to the active/visible tab by default.

#### Scenario: Multi-tab browser
- **WHEN** Chrome has 5 tabs open
- **THEN** CDP engine connects to the active tab and snapshots its content

### Requirement: Probe CDP availability
The CDP engine SHALL be able to probe whether a CDP endpoint is accessible at a given port by making an HTTP request to localhost:<port>/json/version with a 500ms timeout.

#### Scenario: CDP available
- **WHEN** probing port 9222 and Chrome CDP is active
- **THEN** returns true with browser version info

#### Scenario: CDP not available
- **WHEN** probing port 9222 and nothing is listening
- **THEN** returns false within 500ms
