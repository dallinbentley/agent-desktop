## Context

We have a working Swift MVP (3,030 lines) that proves the core architecture: thin CLI → persistent daemon → macOS native APIs, with accessibility tree snapshots producing compact @ref text. Five technical spikes validated every macOS API. Follow-up research proved:

- **AX headless actions work** — AXPress clicks buttons without focus (0.05ms vs 50ms CGEvent), kAXValueAttribute sets text headlessly on native apps
- **CDP covers our blind spots** — Electron apps (Slack, Cursor, Spotify) have empty AX trees but rich CDP accessibility trees. Agent-browser's native Rust daemon already speaks CDP.
- **App detection is trivial** — Bundle ID for browsers, Electron Framework file check for Electron, CDP port probing

Key Rust crates validated: `accessibility-sys` (AXUIElement), `core-graphics` (CGEvent), `screencapturekit-rs` (screenshots), `tungstenite` (WebSocket/CDP), `clap` (CLI), `serde_json` (JSON).

## Goals / Non-Goals

**Goals:**
- Feature parity with Swift MVP + all researched improvements in one pass
- Native CDP engine for browser/Electron apps — single binary, no external dependencies
- AX-first headless interaction for native apps (100-500x faster, no focus stealing)
- Automatic app classification and engine routing
- Unified @ref experience — agent sees @e1 whether it's AX or CDP sourced
- Cross-platform ready architecture (platform-specific backends behind traits)

**Non-Goals:**
- Linux/Windows backends (architecture supports it, implementation is future)
- Safari WebDriver support (different protocol, Safari AX is decent anyway)
- Firefox Marionette protocol (future — Firefox users can use Chrome for now)
- MCP server mode
- Playwright integration (we speak CDP directly, not through Playwright)

## Decisions

### D1: Cargo workspace with 3 crates

```
agent-desktop/
├── Cargo.toml              (workspace)
├── crates/
│   ├── shared/             (agent-desktop-shared)
│   │   ├── src/
│   │   │   ├── protocol.rs  # Request, Response, CommandArgs, ResponseData
│   │   │   ├── types.rs     # ElementRef, RefMap types, error codes, key mappings
│   │   │   └── errors.rs    # AI-friendly error builders
│   │   └── Cargo.toml
│   ├── daemon/             (agent-desktop-daemon)
│   │   ├── src/
│   │   │   ├── main.rs      # Socket server, command dispatch
│   │   │   ├── ax_engine.rs # AXUIElement traversal, AX actions
│   │   │   ├── cdp_engine.rs # WebSocket CDP client, browser a11y tree
│   │   │   ├── detector.rs  # App classification (native/browser/Electron)
│   │   │   ├── refmap.rs    # Unified RefMap (AX + CDP sources)
│   │   │   ├── input.rs     # CGEvent mouse/keyboard (fallback)
│   │   │   ├── capture.rs   # ScreenCaptureKit screenshots
│   │   │   ├── app.rs       # App management (open, focus, list)
│   │   │   └── snapshot.rs  # Snapshot text formatting + merging
│   │   └── Cargo.toml
│   └── cli/                (agent-desktop CLI)
│       ├── src/
│       │   ├── main.rs      # Clap command definitions
│       │   ├── connection.rs # Unix socket client + daemon auto-start
│       │   └── output.rs    # Human-readable + JSON formatting
│       └── Cargo.toml
└── src/spikes/             (preserved Swift spikes for reference)
```

**Why workspace**: Clean separation of concerns. Shared crate for protocol types used by both CLI and daemon. Each crate compiles independently.

### D2: AX-first interaction with CGEvent fallback

The interaction chain for any command (click, fill, type):

```
1. Resolve @ref → check source
2. IF source == AX:
   a. Try AX action (AXPress, AXSetValue) → headless, fast
   b. Verify action took effect (read back attribute)
   c. IF failed → fall back to CGEvent (bring to front, click at coordinates)
3. IF source == CDP:
   a. Send CDP command (DOM.click, Input.dispatchKeyEvent)
   b. Already headless by nature
4. IF coordinate-based (click 500 300):
   a. Bring target app to front
   b. CGWarp + CGEvent click
```

**Why AX-first**: 100-500x faster (0.05ms vs 50ms), no focus stealing, works headlessly. CGEvent only needed when AX actions fail (web content in Safari, custom controls).

### D3: Native CDP client via tungstenite WebSocket

```
┌─────────────┐    WebSocket     ┌──────────────┐
│ cdp_engine   │ ──────────────▶ │ Browser/      │
│              │   JSON-RPC      │ Electron app  │
│ • connect()  │ ◀────────────── │ (CDP server)  │
│ • snapshot() │                 │               │
│ • click()    │                 └──────────────┘
│ • type()     │
│ • evaluate() │
└─────────────┘
```

CDP protocol: JSON-RPC over WebSocket. Key methods:
- `Accessibility.getFullAXTree` → get page accessibility tree
- `DOM.resolveNode` + `Runtime.callFunctionOn` → interact with elements
- `Input.dispatchMouseEvent` / `Input.dispatchKeyEvent` → input simulation
- `Page.captureScreenshot` → page screenshot (alternative to ScreenCaptureKit)

**Port agent-browser's snapshot logic**: Walk the CDP accessibility tree, filter to interactive roles (same set as AX), assign @refs, produce same text format.

**Why native CDP, not agent-browser subprocess**: Single binary requirement. CDP is just WebSocket + JSON — straightforward in Rust. No Node.js dependency, no version management, no output parsing fragility.

### D4: App detector classifies before every snapshot

```rust
enum AppKind {
    Native,                          // Use AX engine only
    Browser { cdp_port: Option<u16> }, // AX for chrome, CDP for web content
    Electron { cdp_port: Option<u16> }, // CDP preferred, AX fallback
    CEF { cdp_port: Option<u16> },     // CDP preferred (Spotify)
    Unknown,                          // Try AX, fall back to screenshot
}

fn detect_app(pid: pid_t) -> AppKind {
    let bundle_id = get_bundle_id(pid);
    let bundle_path = get_bundle_path(pid);
    
    if KNOWN_BROWSERS.contains(&bundle_id) {
        let port = probe_cdp_port(pid);
        return AppKind::Browser { cdp_port: port };
    }
    if has_electron_framework(&bundle_path) {
        let port = probe_cdp_port(pid);
        return AppKind::Electron { cdp_port: port };
    }
    if has_cef_framework(&bundle_path) {
        let port = probe_cdp_port(pid);
        return AppKind::CEF { cdp_port: port };
    }
    AppKind::Native
}
```

### D5: Unified RefMap with source tracking

```rust
struct ElementRef {
    id: String,              // "e1", "e2"...
    source: RefSource,
    // Common fields:
    role: String,
    label: Option<String>,
    frame: Option<Rect>,     // screen coordinates (for AX and screenshot overlay)
    // AX-specific:
    ax_path: Option<Vec<PathSegment>>,
    ax_actions: Option<Vec<String>>,
    ax_pid: Option<pid_t>,
    // CDP-specific:
    cdp_node_id: Option<i64>,
    cdp_backend_node_id: Option<i64>,
    cdp_port: Option<u16>,
}

enum RefSource { AX, CDP, Coordinate }
```

For browser windows, merged snapshot output:
```
[Chrome — GitHub]
  @e1 button "Back"              ← AX (browser chrome)
  @e2 button "Forward"           ← AX (browser chrome)
  @e3 textbox "Address bar"      ← AX (browser chrome)
  --- web content ---
  @e4 link "Pull requests"       ← CDP (page content)
  @e5 button "New pull request"  ← CDP (page content)
  @e6 textbox "Search"           ← CDP (page content)
```

### D6: Frontmost app detection — 3-tier fallback

```rust
fn get_frontmost_app() -> Option<(String, pid_t)> {
    // 1. AX system-wide (most reliable with permission)
    if let Some(app) = ax_get_focused_app() { return Some(app); }
    // 2. NSWorkspace (simpler, works from background)
    if let Some(app) = nsworkspace_frontmost() { return Some(app); }
    // 3. CGWindowList (ordered front-to-back, no special permission)
    if let Some(app) = cgwindowlist_frontmost() { return Some(app); }
    None
}
```

### D7: Screenshot with window frame for coordinate mapping

ScreenshotData response includes window origin for coordinate translation:
```rust
struct ScreenshotData {
    path: String,
    width: u32,
    height: u32,
    scale: u32,
    window_origin_x: Option<f64>,  // screen coordinates
    window_origin_y: Option<f64>,
    app_name: Option<String>,
}
```

Agent uses: `screen_x = window_origin_x + image_x` (at 1x capture, no scaling needed).

### D8: Electron/browser CDP setup UX

```
$ agent-desktop snapshot --app Spotify
⚠ Spotify is an Electron app but CDP is not available.
  To enable rich UI interaction, relaunch with:
    agent-desktop open --with-cdp Spotify
  
  Falling back to screenshot mode.
  [screenshot saved to /tmp/...]

$ agent-desktop open --with-cdp Spotify
Relaunching Spotify with CDP on port 9230...
Spotify ready with CDP.

$ agent-desktop snapshot --app Spotify
[Spotify — Search]
  @e1 button "Home"
  @e2 combobox "What do you want to play?"
  @e3 navigation "Main"
    @e4 heading "Your Library"
    @e5 option "Playlists"
    ...
```

## Risks / Trade-offs

**[Rust FFI ergonomics for AXUIElement]** → `accessibility-sys` provides raw bindings. We'll write thin safe wrappers (~200 lines). The C API is straightforward — our Swift code shows exactly which calls to make.

**[CDP protocol complexity]** → We only need ~10 CDP methods (getFullAXTree, resolveNode, dispatchMouseEvent, dispatchKeyEvent, evaluate, captureScreenshot). Not implementing full Playwright — just the subset agent-browser uses for snapshots + interaction.

**[Rewrite regression risk]** → Mitigated by: keeping Swift code as reference, same architecture, same test scenarios (TextEdit flow, System Settings flow, Finder flow). Run same E2E tests against Rust version.

**[Electron app relaunch requirement]** → CDP can't be injected into running processes. Users must relaunch Electron apps with `--remote-debugging-port`. The `open --with-cdp` command makes this painless. Some apps (Figma) block CDP entirely — fall back to screenshot+coordinates.

**[CDP port management]** → Multiple Electron apps need different ports. Use deterministic assignment: hash of app name → port in 9222-9399 range. Track active CDP connections in daemon state.

**[Browser must be launched with CDP flag]** → Chrome/Edge need `--remote-debugging-port`. Can auto-detect via DevToolsActivePort file. For users who haven't enabled it, provide `agent-desktop open --with-cdp Chrome` helper.
