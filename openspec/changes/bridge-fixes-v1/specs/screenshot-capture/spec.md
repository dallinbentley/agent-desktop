## MODIFIED Requirements

### Requirement: Capture specific app window in background
The screenshot engine SHALL capture a specific app's window using ScreenCaptureKit/CGWindowList. If the captured image is blank or suspiciously small (< 1KB PNG), it SHALL retry up to 3 times with 500ms delays before returning an error.

#### Scenario: Screenshot of freshly launched app
- **WHEN** `screenshot --app Spotify` is called 1 second after Spotify relaunches
- **THEN** system retries if first capture is blank, returns valid screenshot within ~2s
