## Why

Our native Rust CDP engine reinvents what agent-browser already does perfectly. agent-browser (v0.22.1, installed on this machine) produces rich accessibility tree snapshots with @refs for Electron apps and browsers — exactly what we need. Testing confirmed:

- Spotify via agent-browser: Full player UI with labeled buttons, search, playlists, navigation
- Slack via agent-browser: Hundreds of labeled elements including channels, messages, toolbar  
- Our native CDP engine: Connected to wrong app, no headless interaction, incomplete snapshot logic

Meanwhile, CGEvent coordinate clicks require bringing apps to the foreground, disrupting the user. agent-browser's CDP interactions are fully headless — the user keeps working while the agent controls Electron/browser apps in the background.

The right approach: **rip out our native CDP engine and bridge to agent-browser as a subprocess.** Use its proven CLI for all web/Electron interactions. Our tool becomes the intelligent router — AX engine for native macOS apps, agent-browser for everything web-based.

## What Changes

- **Remove** `crates/daemon/src/cdp_engine.rs` — replaced by agent-browser bridge
- **Add** `crates/daemon/src/browser_bridge.rs` — type-safe bridge that shells out to `agent-browser` CLI, parses its output, and maps its @refs into our unified RefMap
- **Modify** `crates/daemon/src/detector.rs` — route Browser/Electron/CEF apps to browser bridge instead of native CDP
- **Modify** `crates/daemon/src/refmap.rs` — web-sourced refs store the original agent-browser ref ID for delegation
- **Modify** `crates/daemon/src/main.rs` — wire bridge into command dispatch, manage agent-browser sessions
- **Add** `--app` flag behavior — when targeting a specific app, ALL interactions happen in the background without stealing focus (AX headless for native, agent-browser CDP for web)
- **Add** agent-browser binary bundling support — detect installed agent-browser, or bundle it alongside our binary

## Capabilities

### New Capabilities
- `browser-bridge`: Type-safe Rust bridge to agent-browser CLI. Manages CDP connections, translates between our protocol and agent-browser commands, parses snapshot output into our RefMap format.
- `headless-mode`: When using `--app`, interactions are fully headless. Native apps use AX actions (no focus stealing). Electron/browser apps use agent-browser CDP (no focus stealing). Coordinate fallback only used as last resort with explicit `--foreground` flag.
- `bridge-lifecycle`: Manage agent-browser daemon lifecycle — start/stop alongside our daemon, session management, CDP connection tracking per Electron/browser app.

### Modified Capabilities
_None — this replaces the cdp-engine capability which was just added and never shipped._

## Impact

- **Removes**: `crates/daemon/src/cdp_engine.rs` (~500 lines), `tungstenite` and `url` dependencies
- **Adds**: `crates/daemon/src/browser_bridge.rs` — subprocess management, output parsing
- **Runtime dependency**: agent-browser must be available (installed via npm/brew/cargo, or bundled)
- **Modified files**: detector.rs, refmap.rs, main.rs, Cargo.toml
