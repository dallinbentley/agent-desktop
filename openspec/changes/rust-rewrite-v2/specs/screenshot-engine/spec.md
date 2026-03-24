## ADDED Requirements

### Requirement: Capture specific app window in background
The screenshot engine SHALL capture a specific app's window using ScreenCaptureKit's desktopIndependentWindow filter, which captures full content even when the window is behind other windows.

#### Scenario: Background window capture
- **WHEN** `screenshot --app Spotify` is run while another app is frontmost
- **THEN** Spotify's window is captured with full content, not the frontmost window

### Requirement: Return window frame coordinates
The screenshot response SHALL include the window's screen-space origin (windowOriginX, windowOriginY) and app name. This enables coordinate-based clicking by translating image coordinates to screen coordinates.

#### Scenario: Coordinate data in response
- **WHEN** screenshot of Spotify's window is captured
- **THEN** response includes windowOriginX, windowOriginY matching the window's screen position

### Requirement: Frontmost app detection with 3-tier fallback
The screenshot engine SHALL determine the frontmost app using: (1) AX system-wide kAXFocusedApplicationAttribute, (2) NSWorkspace.frontmostApplication, (3) CGWindowListCopyWindowInfo ordered front-to-back. It SHALL try each in order and use the first that succeeds.

#### Scenario: Frontmost detection from daemon
- **WHEN** daemon process queries frontmost app
- **THEN** at least one of the three methods returns the correct app

### Requirement: Correct frontmost window ordering
When capturing "frontmost window" without --app flag, the engine SHALL use CGWindowListCopyWindowInfo (which is ordered front-to-back) to identify the correct frontmost window ID, then match it in SCShareableContent.

#### Scenario: Default screenshot captures actual frontmost
- **WHEN** `screenshot` is run with no flags and TextEdit is frontmost
- **THEN** TextEdit's window is captured, not some other window

### Requirement: 1x resolution by default
Screenshots SHALL be captured at 1x logical resolution by default. At 1x, image pixels equal logical points, making coordinate translation trivial (screen_x = window_origin_x + image_x).

#### Scenario: 1x coordinate mapping
- **WHEN** screenshot at 1x shows a button at image pixel (150, 200)
- **THEN** screen coordinate for clicking is (window_origin_x + 150, window_origin_y + 200)
