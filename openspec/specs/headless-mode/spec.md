# headless-mode Specification

## Purpose
TBD - created by archiving change agent-browser-bridge. Update Purpose after archive.
## Requirements
### Requirement: --app flag enables fully headless interaction
When `--app <name>` is specified, ALL interactions SHALL happen without stealing focus from the user's current foreground app. Native apps use AX headless actions. Electron/browser apps use agent-browser CDP.

#### Scenario: Click native app button headlessly
- **WHEN** `agent-desktop click @e3 --app "System Settings"` and System Settings is in background
- **THEN** AXPress is used, System Settings stays in background, user is not interrupted

#### Scenario: Click Electron app button headlessly
- **WHEN** `agent-desktop click @e7 --app Spotify` and Spotify is in background
- **THEN** agent-browser CDP click is used, Spotify stays in background, user is not interrupted

#### Scenario: Screenshot without bringing to front
- **WHEN** `agent-desktop screenshot --app Spotify` and Spotify is behind other windows
- **THEN** ScreenCaptureKit captures Spotify's window content without activating it

### Requirement: Coordinate clicks require --foreground flag
CGEvent coordinate-based clicks (click x y) SHALL only bring the app to foreground when `--foreground` flag is explicitly passed, or when the app is already frontmost.

#### Scenario: Coordinate click with foreground
- **WHEN** `agent-desktop click 500 300 --app Spotify --foreground`
- **THEN** Spotify is brought to front, then CGEvent click at (500, 300)

#### Scenario: Coordinate click without foreground
- **WHEN** `agent-desktop click 500 300 --app Spotify` without --foreground
- **THEN** error: "Coordinate clicks require --foreground flag or the app must be frontmost."

### Requirement: Default snapshot targets frontmost or --app
Without `--app`, snapshot SHALL target the frontmost app. With `--app`, it SHALL target the named app regardless of what's frontmost. Both modes SHALL work without changing focus.

#### Scenario: Snapshot frontmost
- **WHEN** `agent-desktop snapshot -i` with Finder frontmost
- **THEN** returns Finder's AX tree without changing focus

#### Scenario: Snapshot background app
- **WHEN** `agent-desktop snapshot -i --app Spotify` with Terminal frontmost
- **THEN** returns Spotify's snapshot (via agent-browser if CDP available) without switching to Spotify

