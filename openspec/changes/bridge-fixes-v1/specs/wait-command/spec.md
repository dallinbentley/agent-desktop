## ADDED Requirements

### Requirement: Wait for element by ref
The system SHALL support `wait @eN` which polls for the element to appear in the accessibility tree or CDP tree, with a default timeout of 10 seconds.

#### Scenario: Wait for element after navigation
- **WHEN** agent runs `wait @e5` after clicking a navigation link
- **THEN** system polls until element @e5 is found or timeout expires

### Requirement: Wait for fixed duration
The system SHALL support `wait <ms>` for a fixed delay in milliseconds.

#### Scenario: Wait 2 seconds
- **WHEN** agent runs `wait 2000`
- **THEN** system waits 2000ms before returning

### Requirement: Wait for page load state via CDP
The system SHALL support `wait --load <state>` where state is `networkidle`, `domcontentloaded`, or `load`. This SHALL delegate to agent-browser's wait command for CDP-sourced contexts.

#### Scenario: Wait for network idle
- **WHEN** agent runs `wait --load networkidle --app Spotify`
- **THEN** system calls `agent-browser --session spotify --cdp <port> wait --load networkidle`
