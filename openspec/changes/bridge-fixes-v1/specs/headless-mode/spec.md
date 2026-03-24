## MODIFIED Requirements

### Requirement: --app flag enables fully headless interaction
When `--app <name>` is specified on ANY interaction command (snapshot, click, fill, type, press, scroll, screenshot, get, wait), ALL interactions SHALL happen without stealing focus. Native apps use AX headless actions. Electron/browser apps use agent-browser CDP.

#### Scenario: Fill in background Electron app
- **WHEN** `agent-desktop fill @e7 "Luke Combs" --app Spotify` and Spotify is in background
- **THEN** agent-browser fill is used headlessly, Spotify stays in background

#### Scenario: Press key in background Electron app
- **WHEN** `agent-desktop press enter --app Spotify`
- **THEN** agent-browser press Enter is used for the Spotify CDP session

#### Scenario: Type in background native app
- **WHEN** `agent-desktop type @e4 "hello" --app "System Settings"`
- **THEN** AX headless type is used, System Settings stays in background
