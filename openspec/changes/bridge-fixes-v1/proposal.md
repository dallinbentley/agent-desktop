## Why

The Spotify demo exposed 9 gaps — 3 critical blockers, 3 medium issues, and 3 minor. The tool works end-to-end but isn't reliable enough for unattended agent use. The biggest issues: CDP connects to the wrong app, clicks don't always navigate SPAs, and there's no way to wait for page transitions between actions.

## What Changes

- **Fix CDP port-to-app routing** — track which PID owns which CDP port, match by PID not just port scan
- **Add `--app` flag to ALL interaction commands** — fill, type, press, scroll (not just click)
- **Add `wait` command** — expose agent-browser's `wait` for page transitions, element appearance, and network idle
- **Fix SPA navigation clicks** — use agent-browser's click (which handles JS navigation) and add post-click wait
- **Fix `open --with-cdp` relaunch** — actually kill and relaunch, verify new PID, confirm CDP ready
- **Fix blank screenshots** — add retry with delay for freshly launched apps
- **Add snapshot scoping** — pass through agent-browser's `--selector` flag to reduce token count
- **Expose agent-browser `get` for reading state** — get text, value, title from web elements

## Capabilities

### New Capabilities
- `wait-command`: Wait for element appearance, page load, network idle, or fixed delay. Delegates to agent-browser's wait for web content.
- `snapshot-scoping`: Scope snapshots to a CSS selector region via `--selector` flag, reducing token count for large apps.

### Modified Capabilities
- `browser-bridge`: Fix CDP port-to-PID mapping, add wait delegation, add get delegation, fix click reliability
- `headless-mode`: Add `--app` flag to fill, type, press, scroll commands
- `bridge-lifecycle`: Fix open --with-cdp to actually relaunch with new PID, verify CDP ready
- `screenshot-capture`: Add retry logic for freshly launched apps

## Impact

- Modified files: browser_bridge.rs, main.rs, detector.rs, cli/main.rs, protocol.rs, app.rs, capture.rs
- New protocol types: WaitArgs, WaitData
- New CLI subcommand: wait
