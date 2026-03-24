## ADDED Requirements

### Requirement: Classify apps by type
The app detector SHALL classify any running macOS app into: Native, Browser, Electron, CEF, or Unknown. Classification SHALL be based on bundle identifier and framework detection.

#### Scenario: Detect browser
- **WHEN** target app has bundle ID "com.google.Chrome"
- **THEN** classified as Browser

#### Scenario: Detect Electron
- **WHEN** target app's bundle contains Contents/Frameworks/Electron Framework.framework
- **THEN** classified as Electron

#### Scenario: Detect CEF (Spotify)
- **WHEN** target app's bundle contains Chromium Embedded Framework
- **THEN** classified as CEF

#### Scenario: Native app
- **WHEN** target app is Finder (com.apple.finder)
- **THEN** classified as Native

### Requirement: Known browser bundle ID list
The detector SHALL maintain a list of known browser bundle identifiers: com.apple.Safari, com.google.Chrome, org.mozilla.firefox, com.microsoft.edgemac, com.brave.Browser, company.thebrowser.Browser (Arc), and others.

#### Scenario: Arc detected as browser
- **WHEN** target app has bundle ID "company.thebrowser.Browser"
- **THEN** classified as Browser

### Requirement: CDP port probing
For Browser, Electron, and CEF apps, the detector SHALL probe for an active CDP port. It SHALL check the DevToolsActivePort file (for Chrome) and probe common ports (9222-9229) via HTTP.

#### Scenario: Chrome with CDP enabled
- **WHEN** Chrome is running with --remote-debugging-port=9222
- **THEN** detector returns Browser with cdp_port=9222

#### Scenario: Electron without CDP
- **WHEN** Slack is running without --remote-debugging-port
- **THEN** detector returns Electron with cdp_port=None

### Requirement: Route snapshot to correct engine
Based on app classification, the dispatcher SHALL route snapshot commands to: AX engine (Native), AX + CDP merged (Browser with CDP), CDP only (Electron/CEF with CDP), or screenshot fallback (no CDP, poor AX).

#### Scenario: Browser with CDP routes to merged snapshot
- **WHEN** snapshot targets Chrome with CDP available
- **THEN** AX engine handles browser chrome (address bar, tabs), CDP engine handles web content, results are merged

#### Scenario: Electron without CDP suggests relaunch
- **WHEN** snapshot targets Spotify without CDP
- **THEN** system returns warning suggesting `open --with-cdp Spotify` and falls back to screenshot
