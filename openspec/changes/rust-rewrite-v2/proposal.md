## Why

The Swift MVP proved the architecture works — accessibility tree snapshots with @refs, daemon IPC, input simulation, screenshots. But three problems emerged:

1. **Web content is a blind spot.** Electron apps (Slack, Spotify, Cursor, Discord) return empty or useless AX trees. We need a native CDP (Chrome DevTools Protocol) engine to get rich accessibility trees from Chromium-based apps. CDP is WebSocket + JSON — Rust's async/networking ecosystem (tokio, tungstenite) is ideal for this, and aligns with agent-browser's native Rust daemon.

2. **Cross-platform is blocked.** Swift locks us to macOS. Rust gives us Linux (AT-SPI + X11/Wayland) and Windows (UI Automation + SendInput) as future platform backends without a rewrite.

3. **Headless interaction is missing.** Research proved AXUIElementPerformAction works on background apps (100-500x faster than CGEvent, no focus stealing). The current Swift code doesn't use this. The rewrite is the right time to implement AX-first interaction.

The codebase is 3,030 lines — small enough that rewriting now is cheap. Waiting means rewriting 6K+ lines after adding CDP.

## What Changes

- **Complete rewrite** from Swift to Rust — same architecture (thin CLI → persistent daemon → native APIs), same command grammar, same @ref system
- **New CDP engine** — native Rust WebSocket client speaking Chrome DevTools Protocol for browser and Electron app interaction
- **App detection layer** — automatically classifies apps as native/browser/Electron and routes to AX engine or CDP engine
- **AX-first headless interaction** — click via AXPress, fill via kAXValueAttribute, before falling back to CGEvent
- **Unified RefMap** — transparent @e1, @e2 refs regardless of whether source is AX or CDP
- **Screenshot improvements** — window frame data for coordinate mapping, frontmost app detection hardening, correct frontmost window ordering
- **Coordinate-based fallback** — for apps where both AX and CDP fail (e.g., Figma blocks CDP)

## Capabilities

### New Capabilities
- `rust-core`: Rust project structure (Cargo workspace), CLI binary, daemon binary, shared library with all protocol types
- `ax-engine`: macOS accessibility tree traversal, ref map, snapshot formatting — ported from Swift with AX-first headless actions (AXPress, AXSetValue)
- `input-engine`: Mouse clicks (CGWarp + CGEvent), keyboard typing (Unicode API), key presses (keycodes + modifiers), scroll, fill (AX selection-replace) — ported from Swift with AX-first fallback chain
- `cdp-engine`: Native Rust CDP client over WebSocket. Connect to browsers and Electron apps. Get accessibility tree, assign @refs, click/type/fill via CDP commands. Aligned with agent-browser's snapshot format.
- `app-detector`: Classify running apps as native/browser/Electron/CEF. Detect by bundle ID (known browsers), Electron Framework presence, CEF detection. Probe CDP port availability. Route to correct engine.
- `screenshot-engine`: ScreenCaptureKit capture with app-targeting (background windows), window frame coordinates in response, frontmost app detection with 3-tier fallback (AX systemWide → NSWorkspace → CGWindowList)
- `unified-refmap`: Single RefMap that holds both AX-sourced and CDP-sourced refs. Transparent @e1 numbering. Source-aware dispatch for interactions. Merged snapshots for browser windows (AX chrome + CDP web content).
- `daemon-ipc`: Unix domain socket server, JSON protocol, auto-start on first CLI use, stale socket cleanup — ported from Swift
- `cli-interface`: Rust CLI via clap, same command grammar as Swift version, human-readable + JSON output, AI-friendly errors — ported from Swift

### Modified Capabilities

_None — this is a rewrite, not modifying existing specs._

## Impact

- **Complete replacement** of Sources/CLI/, Sources/Daemon/, Sources/Shared/ (Swift → Rust)
- **New Cargo workspace** replacing Package.swift
- **New dependencies**: `clap` (CLI), `serde`/`serde_json` (JSON), `tokio` (async runtime), `tungstenite` (WebSocket for CDP), `accessibility-sys` (AXUIElement), `core-graphics` (CGEvent), `screencapturekit-rs` (screenshots)
- **Platform**: macOS 14+ (same as before), with Rust enabling future Linux/Windows
- **Distribution**: Single static binary via `cargo build --release`, eventual `brew install agent-computer`
- **Spike code preserved** in Sources/Spikes/ for reference during port
