# agent-desktop — Project Roadmap

> A CLI tool for AI agents to control macOS desktops efficiently, inspired by [agent-browser](https://github.com/anthropics/agent-browser).

---

## 1. Project Vision

**agent-desktop** is to the desktop what agent-browser is to the web — a lightweight, text-first CLI tool that lets AI agents observe and interact with any macOS application through compact accessibility tree snapshots and simple atomic commands.

### The Core Insight

agent-browser proved that AI agents work best when they can:
1. **Observe** the UI via a compact text snapshot (not screenshots) — ~10x fewer tokens
2. **Reference** elements with short deterministic IDs (`@e1`, `@e2`, ...)
3. **Act** via simple atomic commands (`click @e3`, `type @e5 "hello"`)

macOS provides accessibility APIs (AXUIElement) that expose the **same kind of structured UI tree** as browser accessibility APIs — making this approach directly portable to desktop automation.

### Why Not Just Use Screenshots + Vision?

| Approach | Tokens per "observation" | Deterministic | Reliable |
|----------|------------------------|---------------|----------|
| Screenshot (1080p) | ~1,500–3,000 | No | Fragile |
| Full DOM / a11y tree | ~3,000–5,000 | Yes | Yes |
| **Filtered interactive snapshot** | **~200–400** | **Yes** | **Yes** |

The snapshot/ref approach is **10-100x more token-efficient** than screenshot-based approaches and produces deterministic, actionable references.

---

## 2. Core Principles

Derived from agent-browser's design strengths:

1. **Text-first, screenshots-as-fallback** — Accessibility tree snapshots are the primary observation method. Screenshots exist for debugging/verification and for apps with poor accessibility.

2. **Compact by default** — Only show interactive elements. Every token the agent reads should be actionable. Support depth limits, window scoping, and compact modes for additional reduction.

3. **Simple atomic commands** — Each command does one thing. `click @e3`. `type @e5 "hello"`. No complex scripting needed.

4. **Persistent daemon, thin CLI** — The CLI is stateless; a background daemon maintains state (ref maps, element cache, app connections). This eliminates startup overhead between commands.

5. **Refs are ephemeral** — Snapshot first, then act on refs. Refs are invalidated on re-snapshot or significant state changes. This keeps the mental model simple.

6. **AI-friendly errors** — Error messages include recovery suggestions. "Element @e3 no longer exists. Run `snapshot` to refresh."

7. **Progressive detail** — Start with `snapshot -i` (interactive only), drill down with `snapshot -i -d 15` (deeper), fall back to `screenshot` when needed.

---

## 3. Architecture Design

### 3.1 Three-Tier Architecture (Mirroring agent-browser)

```
┌──────────────┐     IPC (Unix Socket)     ┌──────────────────┐     Native APIs     ┌──────────┐
│  Rust/Swift  │ ─────────────────────────▶ │  Daemon          │ ──────────────────▶ │  macOS   │
│  CLI (thin)  │   JSON commands            │  (persistent)    │   AXUIElement       │  Desktop │
│              │ ◀───────────────────────── │                  │   CGEvent           │          │
│              │   JSON responses            │  • Ref map mgmt  │   ScreenCaptureKit  │          │
└──────────────┘                            │  • Element cache │                     └──────────┘
                                            │  • Tree traversal│
                                            └──────────────────┘
```

### 3.2 Concept Mapping: agent-browser → agent-desktop

| agent-browser | agent-desktop | Implementation |
|---------------|----------------|----------------|
| Browser DOM/Accessibility tree | macOS Accessibility tree | AXUIElement API |
| `open <url>` | `open <app>` / `focus <window>` | NSWorkspace / AX API |
| `snapshot -i` | `snapshot -i` | AXUIElement tree traversal |
| `@e1, @e2` refs | `@e1, @e2` refs | Same ephemeral ref system |
| `click @ref` | `click @ref` | Resolve to coordinates → CGEvent |
| `fill @ref <text>` | `fill @ref <text>` | AXSetValue or focus + CGEvent keys |
| `type @ref <text>` | `type @ref <text>` | Focus + CGEvent key events |
| `press <key>` | `press <key>` | CGEvent keyboard event |
| `scroll <dir>` | `scroll <dir>` | CGEvent scroll wheel |
| `screenshot` | `screenshot` | ScreenCaptureKit |
| `get text @ref` | `get text @ref` | AXUIElement attributes |
| `wait @ref\|ms` | `wait @ref\|ms` | Poll accessibility tree |
| Tab management | Window management | AX window enumeration |
| Playwright locators | AXUIElement path + coordinates | Element re-identification |

### 3.3 The Snapshot/Ref System

**This is the heart of the tool.** The snapshot system maps the macOS accessibility tree to a compact text representation.

**Input:** Raw AXUIElement tree from a target application

**Processing:**
1. Identify target app (by name, PID, or frontmost)
2. Get app's windows via `kAXWindowsAttribute`
3. Recursively traverse accessibility tree with depth limits and timeouts
4. Filter to interactive element roles (see below)
5. Assign sequential refs: `@e1`, `@e2`, `@e3`, ...
6. Build ref map: `{ "@e1": { pid, axPath, role, label, frame, actions } }`
7. Format as compact text tree

**Interactive Roles (get @refs):**
`AXButton`, `AXTextField`, `AXTextArea`, `AXCheckBox`, `AXRadioButton`, `AXPopUpButton`, `AXComboBox`, `AXSlider`, `AXLink`, `AXMenuItem`, `AXMenuButton`, `AXTab`, `AXTabGroup`, `AXScrollArea`, `AXTable`, `AXOutline`, `AXSwitch`, `AXSearchField`, `AXIncrementor`

**Example output:**
```
[Finder — ~/Documents — 5 items]
  @e1 toolbar_button "Back"
  @e2 toolbar_button "Forward"
  @e3 search_field "Search"
  @e4 menu_button "View"
  @e5 button "Sort By"
  --- content ---
  @e6 icon "Project Proposal.pdf" (selectable)
  @e7 icon "Budget.xlsx" (selectable)
  @e8 icon "Notes" (folder, selectable)
  @e9 scroll_area (scrollable, 5 of 23 items visible)
```

**Ref resolution for actions:**
- **Primary:** Use stored AXUIElement path (role + index chain) to re-locate element
- **Fallback:** Use stored frame coordinates for CGEvent targeting
- **Last resort:** Use stored label for AX search

### 3.4 IPC Protocol Contract (CLI ↔ Daemon)

Communication is over a Unix domain socket at `~/.agent-desktop/daemon.sock`. Messages are **newline-delimited JSON** (one JSON object per line).

#### Request Format (CLI → Daemon)
```json
{
  "id": "req_001",
  "command": "snapshot",
  "args": {
    "interactive": true,
    "compact": false,
    "depth": 10,
    "app": null
  },
  "options": {
    "timeout": 5000,
    "json": false,
    "verbose": false
  }
}
```

#### Command-specific `args` shapes:
```
snapshot:    { interactive: bool, compact: bool, depth: int, app: string?, allWindows: bool }
click:      { ref: string?, x: int?, y: int?, double: bool, right: bool }
fill:       { ref: string, text: string }
type:       { ref: string?, text: string }
press:      { key: string, modifiers: [string]? }
scroll:     { direction: "up"|"down"|"left"|"right", amount: int?, ref: string? }
screenshot: { full: bool, window: bool, app: string? }
open:       { target: string }
status:     {}
get:        { what: "text"|"title"|"apps"|"windows", ref: string?, app: string? }
```

#### Response Format (Daemon → CLI)
```json
{
  "id": "req_001",
  "success": true,
  "data": { ... },
  "error": null,
  "timing": { "elapsed_ms": 145 }
}
```

#### Success `data` shapes by command:
```
snapshot:    { text: string, refCount: int, app: string, window: string }
click:      { ref: string, coordinates: {x, y}, element: {role, label} }
fill:       { ref: string, text: string, previousValue: string? }
type:       { ref: string, text: string }
press:      { key: string, modifiers: [string] }
scroll:     { direction: string, amount: int }
screenshot: { path: string, dimensions: {width, height}, scale: int }
open:       { app: string, pid: int, wasRunning: bool }
status:     { daemon: {pid, uptime}, permissions: {accessibility, screenRecording}, frontmost: {app, pid, window}, display: {width, height, scale}, refMap: {count, age_ms} }
get text:   { ref: string?, text: string }
get apps:   { apps: [{name, pid, isActive}] }
get windows:{ windows: [{title, app, pid, frame, isKey}] }
```

#### Error Response:
```json
{
  "id": "req_001",
  "success": false,
  "data": null,
  "error": {
    "code": "REF_NOT_FOUND",
    "message": "Element @e3 not found. The UI may have changed.",
    "suggestion": "Run `snapshot` to refresh element references."
  }
}
```

#### Error Codes:
```
REF_NOT_FOUND       — element ref doesn't exist in current ref map
REF_STALE           — element existed but can't be re-located in live tree
NO_REF_MAP          — no snapshot taken yet
APP_NOT_FOUND       — target app not running / not installed
WINDOW_NOT_FOUND    — target window doesn't exist
PERMISSION_DENIED   — missing Accessibility or Screen Recording permission
TIMEOUT             — command exceeded timeout (partial results may be in data)
AX_ERROR            — accessibility API returned an error
INPUT_ERROR         — CGEvent failed to post
INVALID_COMMAND     — malformed command or args
DAEMON_ERROR        — internal daemon error
```

### 3.5 Performance Strategy

Full accessibility tree traversal is known to be slow (1-30s for complex apps). Mitigation:

| Strategy | Details |
|----------|---------|
| **Per-window caching** | 1.5s TTL, invalidated on snapshot command |
| **Depth limiting** | Default depth 10, configurable via `-d` flag |
| **Timeout** | 3s hard timeout per traversal, return partial results |
| **Batch attribute fetch** | Use `AXUIElementCopyMultipleAttributeValues` |
| **Interactive-only filter** | Skip non-interactive branches early |
| **Window scoping** | Default to frontmost window only |
| **Lazy children** | Don't expand collapsed sections / off-screen content |

---

## 4. Command Design

### Grammar: `agent-desktop <command> [args] [options]`

### Full Command Set

#### Observation Commands
| Command | Description |
|---------|-------------|
| `snapshot [-i] [-c] [-d N]` | Get accessibility tree snapshot. `-i` interactive only (default), `-c` compact, `-d` depth limit |
| `snapshot --app <name>` | Snapshot specific app (default: frontmost) |
| `snapshot --all-windows` | Include all windows, not just frontmost |
| `screenshot [--full] [--window]` | Capture screen/window. Returns image path |
| `screenshot --app <name>` | Capture specific app's window |
| `get text [@ref]` | Get element's text content |
| `get title` | Get frontmost window title |
| `get apps` | List running applications |
| `get windows [--app <name>]` | List windows |
| `diff snapshot` | Compare current vs previous snapshot |

#### Interaction Commands
| Command | Description |
|---------|-------------|
| `click @ref` | Click element (resolves ref → coordinates → CGEvent) |
| `click @ref --double` | Double-click |
| `click @ref --right` | Right-click / context menu |
| `click <x> <y>` | Click at absolute screen coordinates (fallback) |
| `fill @ref <text>` | Clear field and type text |
| `type @ref <text>` | Type text without clearing (append) |
| `type <text>` | Type text into focused element |
| `press <key>` | Press key (Enter, Tab, Escape, Space, etc.) |
| `press <modifier+key>` | Key combo (cmd+c, cmd+shift+s, etc.) |
| `scroll <dir> [px]` | Scroll up/down/left/right |
| `scroll @ref <dir> [px]` | Scroll within specific element |
| `select @ref <value>` | Select dropdown/popup item |
| `drag @ref1 @ref2` | Drag from element to element |
| `drag <x1> <y1> <x2> <y2>` | Drag between coordinates |

#### App/Window Management
| Command | Description |
|---------|-------------|
| `open <app>` | Launch or focus application |
| `open <file>` | Open file with default app |
| `focus @ref` | Focus specific element |
| `focus --window <title>` | Focus specific window |
| `close` | Close frontmost window |
| `close --app <name>` | Quit application |

#### Session Management
| Command | Description |
|---------|-------------|
| `wait @ref` | Wait for element to appear (poll a11y tree) |
| `wait <ms>` | Wait milliseconds |
| `status` | Show daemon status, permissions, active app |

#### Global Options
| Option | Description |
|--------|-------------|
| `--json` | JSON output (machine-readable) |
| `--timeout <ms>` | Command timeout |
| `--session <name>` | Named session (isolated state) |
| `--max-output <chars>` | Truncate output |
| `--verbose` | Include debug info |

---

## 5. Tech Stack Recommendation

### Primary: Swift CLI with Embedded Daemon

```
agent-desktop/
├── Sources/
│   ├── CLI/              # Swift CLI (thin client)
│   │   ├── main.swift    # Entry point, argument parsing
│   │   ├── Commands.swift # Command parsing → JSON
│   │   └── Connection.swift # Unix socket IPC
│   │
│   ├── Daemon/           # Persistent background process
│   │   ├── Server.swift  # Unix socket server
│   │   ├── RefMap.swift  # @ref → AXUIElement mapping
│   │   ├── Snapshot.swift # Accessibility tree traversal & formatting
│   │   ├── Actions.swift # Command dispatch
│   │   ├── Input.swift   # CGEvent mouse/keyboard simulation
│   │   ├── Capture.swift # ScreenCaptureKit screenshots
│   │   └── Cache.swift   # Element cache with TTL
│   │
│   └── Shared/           # Protocol definitions, types
│       ├── Protocol.swift # JSON command/response schemas
│       └── Types.swift   # Element roles, ref types
│
├── Package.swift         # Swift Package Manager
└── install.sh           # Build + install to PATH
```

### Why Swift (not Rust CLI + Node.js daemon like agent-browser)?

| Factor | Decision |
|--------|----------|
| **macOS API access** | Swift has first-class access to AXUIElement, CGEvent, ScreenCaptureKit — no FFI needed |
| **Single binary** | No Node.js runtime dependency, no Swift ↔ Node.js IPC overhead |
| **Performance** | Native speed for tree traversal (critical for snapshot performance) |
| **Maintenance** | One language, one build system (SPM) |
| **AXorcist precedent** | Existing Swift accessibility library we can reference/learn from |

### Key Dependencies
- **Swift Argument Parser** — CLI argument parsing (Apple's official library)
- **Foundation** — Unix socket IPC, JSON encoding/decoding
- **ApplicationServices** — AXUIElement, CGEvent
- **ScreenCaptureKit** — Screenshot capture
- **Cocoa/AppKit** — NSWorkspace for app launching

### Build & Distribution
- Swift Package Manager for builds
- Single static binary output
- `brew install agent-desktop` (eventual goal)
- Minimum macOS 13 (Ventura) for ScreenCaptureKit features

---

## 6. Implementation Phases

### Phase 1: MVP — Snapshot + Click + Type (Weeks 1-3)

**Goal:** An agent can observe a Mac app and interact with basic elements.

```
agent-desktop snapshot -i          ✅ See interactive elements with @refs
agent-desktop click @e3            ✅ Click a button/link
agent-desktop type @e3 "hello"     ✅ Type into a text field
agent-desktop press Enter          ✅ Press a key
agent-desktop screenshot           ✅ Take a screenshot
agent-desktop open "Safari"        ✅ Focus/launch an app
agent-desktop status               ✅ Check daemon health + permissions
```

**Implementation priorities:**
1. Daemon skeleton with Unix socket server
2. CLI skeleton with argument parsing and IPC
3. AXUIElement tree traversal with interactive-only filtering
4. Ref map management (assign, store, resolve)
5. Snapshot text formatting
6. CGEvent click (resolve ref → frame center → mouse events)
7. CGEvent keyboard (type text, press keys)
8. ScreenCaptureKit basic screenshot
9. App open/focus via NSWorkspace
10. AI-friendly error messages

**Deliverable:** Working CLI that an AI agent (via pi's tool system) can use to control simple Mac apps (Finder, TextEdit, System Settings).

### Phase 1 Detailed Breakdown

#### Dependency Graph

```
                    ┌─────────────────────┐
                    │  T0: Swift Package   │
                    │  Setup + Skeleton    │
                    └──────┬──────────────┘
                           │
              ┌────────────┼────────────────┐
              ▼            ▼                ▼
    ┌─────────────┐  ┌──────────┐  ┌──────────────────┐
    │ T1: Shared  │  │ T2: CLI  │  │ T3: Daemon       │
    │ Protocol &  │  │ Argument │  │ Socket Server     │
    │ Types       │  │ Parsing  │  │ (listen + accept) │
    └──────┬──────┘  └────┬─────┘  └────────┬──────────┘
           │              │                  │
           │         ┌────┴──────┐           │
           │         │ T4: CLI   │           │
           └────────▶│ Socket    │◀──────────┘
                     │ Client    │
                     └────┬──────┘
                          │
          ┌───────────────┼─ CLI↔Daemon IPC working ──────────┐
          │               │                                    │
          ▼               ▼                                    ▼
  ┌──────────────┐ ┌─────────────┐                  ┌─────────────────┐
  │ T5: AX Tree  │ │ T8: CGEvent │                  │ T10: Screenshot  │
  │ Traversal    │ │ Input Sim   │                  │ (ScreenCapture   │
  │ (raw dump)   │ │ (mouse +    │                  │  Kit)            │
  └──────┬───────┘ │  keyboard)  │                  └────────┬────────┘
         │         └──────┬──────┘                           │
         ▼                │                                  │
  ┌──────────────┐        │         ┌───────────────┐        │
  │ T6: Ref Map  │        │         │ T9: Open/Focus│        │
  │ (assign +    │        ▼         │ App via       │        │
  │  resolve)    │  ┌──────────┐    │ NSWorkspace   │        │
  └──────┬───────┘  │ T8b:Click│    └───────────────┘        │
         │          │ (ref →   │                              │
         ▼          │ coords → │                              │
  ┌──────────────┐  │ CGEvent) │                              │
  │ T7: Snapshot │  └──────────┘                              │
  │ Formatter    │                                            │
  │ (text output)│                                            │
  └──────────────┘                                            │
         │                                                    │
         └──────────────── all merge ─────────────────────────┘
                              │
                              ▼
                     ┌────────────────┐
                     │ T11: Error     │
                     │ Handling &     │
                     │ AI-Friendly    │
                     │ Messages       │
                     └────────┬───────┘
                              │
                              ▼
                     ┌────────────────┐
                     │ T12: Status    │
                     │ Command +      │
                     │ Permission     │
                     │ Checks         │
                     └────────┬───────┘
                              │
                              ▼
                     ┌────────────────┐
                     │ T13: E2E       │
                     │ Integration    │
                     │ Tests          │
                     └────────────────┘
```

#### Parallelization Guide

**Can be built concurrently (no dependencies between them):**
- T1 (Protocol/Types) + T2 (CLI Parsing) + T3 (Daemon Server) — all three can start once T0 is done
- T5 (AX Tree Traversal) + T8 (CGEvent Input) + T10 (Screenshot) — all three are independent native API integrations, can start once IPC works
- T9 (Open/Focus App) — independent of all other action commands

**Must be sequential:**
- T0 → T1/T2/T3 (need project skeleton first)
- T1 + T2 + T3 → T4 (CLI client needs protocol types, CLI args, and daemon server)
- T4 → T5/T8/T10 (need working IPC before building commands that use it)
- T5 → T6 → T7 (tree traversal → ref assignment → text formatting — each builds on prior)
- T6 + T8 → T8b (click needs both ref resolution AND input simulation)
- All commands → T11 (error handling wraps everything)
- T11 → T12 → T13 (status needs errors, E2E tests need everything)

**Optimal 2-developer split:**
```
Developer A (daemon/native):     Developer B (CLI/protocol):
  T0: Package setup (shared)       T0: Package setup (shared)
  T3: Daemon socket server         T1: Protocol & types
  T5: AX tree traversal            T2: CLI argument parsing
  T6: Ref map                      T4: CLI socket client
  T7: Snapshot formatter           T10: Screenshot capture
  T8: CGEvent input sim            T9: Open/focus app
  T8b: Click via ref               T11: Error handling
                                   T12: Status command
                     ── merge ──
                   T13: E2E tests
```

#### Task Details with Acceptance Criteria

---

**T0: Swift Package Setup + Skeleton**
- Create `Package.swift` with targets: `agent-desktop` (CLI executable), `agent-desktop-daemon` (daemon executable), `AgentComputerShared` (library)
- Add `swift-argument-parser` dependency
- Scaffold directory structure matching Section 5
- Both `swift build` and `swift run agent-desktop --help` work
- **AC:** `swift build` succeeds, produces two binaries, `--help` prints usage

---

**T1: Shared Protocol & Types**
- Define `Command` enum (Codable) with all Phase 1 variants:
  ```swift
  enum Command: Codable {
      case snapshot(SnapshotOptions)
      case click(ClickTarget)
      case type(TypeTarget)
      case press(PressTarget)
      case screenshot(ScreenshotOptions)
      case open(OpenTarget)
      case status
  }
  ```
- Define `Response` type:
  ```swift
  struct Response: Codable {
      let success: Bool
      let data: ResponseData?  // command-specific payload
      let error: ErrorInfo?    // AI-friendly error
  }
  ```
- Define `ElementRef` type:
  ```swift
  struct ElementRef: Codable {
      let id: String           // "e1", "e2", ...
      let role: String         // "AXButton", "AXTextField", ...
      let label: String?       // human-readable label
      let frame: CGRect        // screen coordinates
      let axPath: [PathSegment] // role + index chain for re-traversal
      let actions: [String]    // available AX actions
  }
  ```
- Define `RefMap` as `[String: ElementRef]`
- **AC:** All types compile, round-trip through `JSONEncoder`/`JSONDecoder` correctly, unit test for serialization

---

**T2: CLI Argument Parsing**
- Use Swift Argument Parser to define commands matching Section 4 grammar (Phase 1 subset)
- Parse `@ref` syntax — strip `@` prefix, validate format `e\d+`
- Parse key names for `press` — map human names (Enter, Tab, Escape, Space, cmd+c) to internal representation
- `--json` global flag
- `--timeout` global flag (default 5000ms)
- `--verbose` global flag
- **AC:** `agent-desktop snapshot -i` parses correctly, `agent-desktop click @e3` extracts ref "e3", `agent-desktop press cmd+shift+s` parses modifier combo, invalid commands print helpful usage

---

**T3: Daemon Socket Server**
- Create Unix domain socket at `~/.agent-desktop/daemon.sock`
- Listen for connections, read newline-delimited JSON commands
- Dispatch to handler (stub handlers that return mock responses initially)
- Handle concurrent connections (one at a time is fine for MVP — serial queue)
- Auto-cleanup stale socket files on startup
- Graceful shutdown on SIGTERM/SIGINT
- Daemon auto-starts via `launchd` plist OR CLI spawns it on first use (prefer the latter for simplicity, matching agent-browser)
- **AC:** Daemon starts, accepts connection, receives JSON command, returns JSON response, exits cleanly on signal

---

**T4: CLI Socket Client**
- Connect to `~/.agent-desktop/daemon.sock`
- If daemon not running, spawn it as background process and retry connection (with 3s timeout)
- Send command as JSON, read response
- Format response for human output (colored text) or `--json` (raw JSON)
- Handle connection errors with helpful messages ("Daemon not running. Starting..." or "Failed to connect to daemon.")
- **AC:** CLI sends `status` command to daemon, receives response, prints formatted output. Auto-spawns daemon if not running.

---

**T5: AXUIElement Tree Traversal**
- Given a PID (or frontmost app), traverse the accessibility tree recursively
- Extract per-element: role, title, description, value, frame (position + size), children, available actions
- Use `AXUIElementCopyMultipleAttributeValues` for batch fetching (fetch role + title + frame + children in one call)
- Respect depth limit (default 10, configurable)
- Hard timeout (3s) — return partial results if exceeded
- Filter function: `isInteractive(role:) -> Bool` matching the Interactive Roles list in Section 3.3
- Return structured tree: `[AXNode]` where `AXNode` has `role, label, frame, children, isInteractive`
- **AC:** Given TextEdit PID, returns tree with buttons, text areas, menus identified. Completes in < 2s for TextEdit. Respects depth limit. Returns partial results on timeout.

---

**T6: Ref Map Management**
- After tree traversal, walk the filtered tree and assign sequential refs (`e1`, `e2`, ...)
- Only interactive elements get refs
- Store ref map in daemon memory: `[String: ElementRef]`
- Provide `resolve(ref:) -> ElementRef?` to look up by ref ID
- Provide `resolveToCoordinates(ref:) -> CGPoint?` — returns center of element's frame
- Re-traversal support: given an `ElementRef.axPath`, walk the live tree to re-find the element (it may have moved)
- Invalidate entire ref map when new `snapshot` is requested
- **AC:** Snapshot of Finder produces ref map with e1...eN. `resolve("e3")` returns correct element. After re-snapshot, old refs are invalid. `resolveToCoordinates("e3")` returns center of the button's frame.

**Ref Map Data Structure:**
```swift
class RefMap {
    private var refs: [String: ElementRef] = [:]
    private var counter: Int = 0
    
    func build(from tree: [AXNode]) -> [String: ElementRef]
    func resolve(_ refId: String) -> ElementRef?
    func resolveToCoordinates(_ refId: String) -> CGPoint?
    func relocate(_ refId: String) -> ElementRef?  // re-traverse to find current position
    func invalidate()
}
```

---

**T7: Snapshot Text Formatter**
- Take the ref-annotated tree and produce compact text output
- Format: indented tree with `@ref role "label"` per line
- Include window title as header: `[AppName — WindowTitle]`
- Structural context: show parent containers (toolbar, sidebar, content area) as unlabeled indent levels
- Compact mode (`-c`): collapse single-child containers
- Output should be < 500 tokens for a typical app window
- **AC:** Snapshot of TextEdit produces readable text tree, < 500 tokens, all interactive elements have refs, non-interactive structure is minimal but provides spatial context

**Example output format:**
```
[TextEdit — Untitled.txt]
  toolbar:
    @e1 button "New"
    @e2 button "Open"
    @e3 button "Save"
  content:
    @e4 text_area "Document content area" (editable, 0 chars)
  menu_bar:
    @e5 menu "File"
    @e6 menu "Edit"
    @e7 menu "Format"
```

---

**T8: CGEvent Input Simulation**
- **Mouse:** `mouseClick(at: CGPoint, button: .left/.right, clickCount: 1/2)`
  - Create mouseDown event, post, brief delay (10ms), create mouseUp, post
  - Support left click, right click, double click
- **Keyboard — press:** `keyPress(key: KeySpec)` where KeySpec maps human names to virtual keycodes
  - Support: Enter (36), Tab (48), Escape (53), Space (49), Delete (51), arrow keys, etc.
  - Support modifier combos: `cmd+c` → hold Cmd flag, press 'c', release
  - Full modifier support: cmd, shift, option/alt, control
- **Keyboard — type string:** `typeString(_ text: String)`
  - Convert each character to keyDown+keyUp events
  - Handle shifted characters (uppercase, symbols) by adding shift flag
  - Consider using `CGEvent(keyboardEventSource:virtualKey:keyDown:)` with `kCGEventKeyboardSetUnicodeString` for non-ASCII
- **AC:** `mouseClick(at: CGPoint(x: 100, y: 200))` clicks at those coordinates. `keyPress(.enter)` sends Enter. `keyPress(.combo([.cmd], .c))` sends Cmd+C. `typeString("Hello World!")` types the string including the shifted `!`.

---

**T8b: Click Command (ref → coordinates → CGEvent)**
- Receive `click @e3` command in daemon
- Resolve `e3` via RefMap → get `ElementRef`
- Try primary: re-traverse AX path to verify element still exists, get current frame
- If stale: try coordinate fallback using stored frame
- Compute click point: center of frame
- Call CGEvent mouseClick at computed point
- Return success with element info, or error with "Element @e3 not found. Run `snapshot` to refresh."
- **AC:** After snapshot, `click @e3` clicks the correct button. If UI has changed, returns actionable error.

---

**T9: Open/Focus App via NSWorkspace**
- `open "Safari"` → find running app by name, activate it. If not running, launch it.
- Use `NSWorkspace.shared.runningApplications` to find by `localizedName`
- Use `NSWorkspace.shared.open(URL)` or `NSRunningApplication.activate()` to bring to front
- Handle app not found: "Application 'Safarri' not found. Did you mean 'Safari'?" (fuzzy match)
- `open "/path/to/file"` → open with default app via `NSWorkspace.shared.open(URL(fileURLWithPath:))`
- **AC:** `open "TextEdit"` launches or focuses TextEdit. `open "nonexistent"` returns helpful error with suggestions.

---

**T10: Screenshot Capture**
- Use ScreenCaptureKit (`SCScreenshotManager`) for capture
- Default: capture frontmost window
- `--full`: capture entire screen
- `--app "Finder"`: capture specific app's frontmost window
- Save to temp file, return file path in response
- Handle Retina: save at full resolution but report logical dimensions
- **AC:** `screenshot` saves PNG of frontmost window, returns path. `screenshot --full` captures full screen. File is valid PNG at correct resolution.

---

**T11: Error Handling & AI-Friendly Messages**
- Wrap all daemon command handlers with error catching
- Map common errors to actionable messages:
  | Error | Message |
  |-------|---------|
  | Ref not found | "Element @e3 not found. The UI may have changed. Run `snapshot` to refresh element references." |
  | No permission | "Accessibility permission required. Run `agent-desktop setup` to grant access." |
  | App not found | "Application 'X' not found. Running apps: Safari, Finder, TextEdit. Did you mean 'Y'?" |
  | Daemon not running | "Starting agent-desktop daemon..." (auto-start) |
  | Timeout | "Snapshot timed out after 3s. Returning partial results (15 of ~40 elements). Try `snapshot -d 5` to reduce depth." |
  | No refs available | "No element references available. Run `snapshot -i` first to discover interactive elements." |
- Non-zero exit codes for all failures
- `--json` mode: `{"success": false, "error": {"code": "REF_NOT_FOUND", "message": "...", "suggestion": "..."}}`
- **AC:** Every error path returns a message with a concrete next-step suggestion. JSON mode includes machine-parseable error codes.

---

**T12: Status Command + Permission Checks**
- `agent-desktop status` returns:
  ```
  agent-desktop daemon: running (pid 12345)
  Accessibility permission: ✅ granted
  Screen Recording permission: ✅ granted
  Frontmost app: Finder (pid 456)
  Active window: ~/Documents — 5 items
  Display: 2560×1440 @2x (Retina)
  Ref map: 9 elements (from last snapshot 3s ago)
  ```
- Check accessibility permission via `AXIsProcessTrusted()`
- Check screen recording permission by attempting a test capture
- **AC:** `status` shows all fields. Missing permissions show ❌ with instructions. Works when daemon is running or not (starts daemon if needed).

---

**T13: E2E Integration Tests**
- Test script that exercises the full flow:
  1. `agent-desktop open "TextEdit"` → verify TextEdit is frontmost
  2. `agent-desktop snapshot -i` → verify output contains text_area and menu refs
  3. `agent-desktop click @e<text_area_ref>` → verify text area is focused
  4. `agent-desktop type @e<ref> "Hello from agent-desktop!"` → verify text appears
  5. `agent-desktop screenshot` → verify PNG file exists
  6. `agent-desktop press cmd+a` → select all
  7. `agent-desktop press cmd+c` → copy
  8. `agent-desktop status` → verify all green
- Can be run as a Swift test target or shell script
- **AC:** Full script runs end-to-end without manual intervention. TextEdit ends up with typed text visible.

---

#### Technical Spikes (Investigate Before Estimating)

These are unknowns that should be resolved in the first 2-3 days before committing to estimates:

**Spike S1: AXUIElement Traversal Performance**
- **Question:** How fast is tree traversal on real apps? (TextEdit, Finder, Safari, VS Code, Slack)
- **Method:** Write a minimal Swift script that traverses the full tree of each app, measures time and element count
- **Output:** Table of `app | element_count | traversal_time_ms | interactive_count`
- **Impacts:** Determines default depth limit, timeout values, whether we need more aggressive caching
- **Time:** 2-4 hours

**Spike S2: AXorcist as Dependency vs. Roll Our Own**
- **Question:** Can we use AXorcist as a Swift Package dependency, or should we write our own traversal?
- **Method:** Try adding AXorcist to Package.swift, call its query/collectAll APIs, evaluate:
  - Does it build cleanly as a dependency?
  - Does its output format work for our ref system?
  - Is the API ergonomic for our snapshot pipeline?
  - What's the performance overhead vs. raw AXUIElement calls?
- **Output:** Decision: "use AXorcist", "fork AXorcist", or "roll our own with AXorcist as reference"
- **Time:** 3-4 hours

**Spike S3: CGEvent Reliability for Typing**
- **Question:** How reliable is CGEvent keyboard simulation for typing strings? Edge cases with Unicode, special characters, IME?
- **Method:** Write test script that types various strings into TextEdit via CGEvent and verifies the result
- **Test strings:** "Hello World", "café", "日本語", "price: $19.99", "path/to/file", "cmd+shift+s triggers", emoji 🎉
- **Output:** Table of `input | result | match? | notes`
- **Impacts:** Determines if we need `AXSetValue` as primary method for text fields instead of key events
- **Time:** 2-3 hours

**Spike S4: ScreenCaptureKit Permission Flow**
- **Question:** What's the actual UX for granting Screen Recording permission? Can we detect it programmatically? What happens on first run?
- **Method:** Build minimal ScreenCaptureKit capture, test on a fresh permission state (revoke via `tccutil reset ScreenCapture`)
- **Output:** Document the permission flow, whether we can detect denied state, and best UX for guiding users
- **Time:** 1-2 hours

**Spike S5: Daemon Auto-Start Reliability**
- **Question:** When CLI spawns daemon as a background process, how reliable is it across terminal emulators (Terminal.app, iTerm2, Warp, Alacritty)?
- **Method:** Test spawning daemon via `Process()` with stdout/stderr redirected to log file, verify it stays alive after CLI exits
- **Output:** Working daemon spawn code or decision to use launchd plist instead
- **Time:** 2-3 hours

---

### Phase 2: Robustness + Full Command Set (Weeks 4-6)

**Goal:** Handle real-world apps reliably. Full command coverage.

- `fill @ref <text>` — clear + type
- `select @ref <value>` — dropdown/popup interaction
- `scroll` — directional scrolling with optional element targeting
- `drag` — drag between elements or coordinates
- `click --double`, `click --right` — click variants
- `press cmd+c` — modifier key combos
- `focus --window <title>` — window management
- `get text @ref`, `get apps`, `get windows` — information queries
- `wait @ref` — poll for element appearance
- `diff snapshot` — before/after comparison
- `--json` output mode for structured responses

**Performance hardening:**
- Per-window accessibility tree caching (1.5s TTL)
- Batch attribute fetching
- Timeout handling with partial results
- Depth limiting defaults tuned per-app

**Error handling hardening:**
- Stale ref detection with helpful messages
- Permission checking with setup instructions
- App not responding detection
- Element obscured detection

### Phase 3: Advanced Features + Polish (Weeks 7-10)

**Goal:** Production-quality tool ready for daily AI agent use.

- **Coordinate fallback mode** — For apps with poor accessibility (games, canvas-based apps, Electron apps): `click 500 300`, detect elements via screenshot + OCR
- **OCR integration** — Apple's Vision framework for text recognition in screenshots. Useful when accessibility labels are missing.
- **Compact snapshot modes** — `-c` compact (remove empty containers), custom depth, app-specific profiles
- **Named sessions** — `--session <name>` for parallel agent isolation
- **Content boundaries** — Nonce markers to separate trusted/untrusted content
- **Output limits** — `--max-output` hard truncation
- **Snapshot scoping** — Focus on specific window regions, menu bars, dialogs
- **App-specific adapters** — Custom snapshot logic for common problematic apps (Electron, Java/Swing, etc.)

### Phase 4: Cross-Platform + Ecosystem (Future)

- **Linux support** — AT-SPI for accessibility tree, XDotool/Ydotool for input, grim/Flameshot for screenshots
- **Windows support** — UI Automation API for accessibility, SendInput for events
- **MCP server mode** — Expose as Model Context Protocol tool server
- **pi integration** — First-class `agent-desktop` tool in pi (like `agent-browser` tool today)
- **Playwright-style recording** — Record user actions → generate command scripts
- **Visual grounding fallback** — Integration with vision models (e.g., ShowUI) for elements not in accessibility tree

---

## 7. Key Challenges & Mitigations

### Challenge 1: Accessibility Tree Performance
**Problem:** Full tree traversal can take 1-30s on complex apps.
**Mitigations:**
- Aggressive depth limiting (default 10)
- 1.5s per-window cache with TTL
- Interactive-only filtering (skip branches without actionable elements)
- 3s hard timeout with partial results
- Batch attribute fetching via `AXUIElementCopyMultipleAttributeValues`
- Lazy expansion (don't recurse into off-screen/collapsed sections)

### Challenge 2: Element Re-identification
**Problem:** After snapshot, the AXUIElement might be stale when the agent acts.
**Mitigations:**
- Store element path (role + index chain from app root) for re-traversal
- Store element frame (coordinates) as fallback targeting
- Store element label for AX search-based re-location
- Detect stale refs and suggest `snapshot` refresh

### Challenge 3: Apps with Poor Accessibility
**Problem:** Some apps (games, Electron with custom rendering, Java/Swing) have incomplete accessibility trees.
**Mitigations:**
- Coordinate-based fallback commands (`click 500 300`)
- Screenshot + OCR for text extraction (Apple Vision framework)
- Hybrid mode: combine partial accessibility data with visual information
- App-specific adapters for known problematic apps
- Log warnings when accessibility tree seems unusually sparse

### Challenge 4: Permission UX
**Problem:** Requires two system permissions (Accessibility + Screen Recording) that users must grant manually.
**Mitigations:**
- `agent-desktop status` shows permission state clearly
- First-run wizard with step-by-step instructions
- `agent-desktop setup` command to open System Settings to the right pane
- Clear error messages: "Accessibility permission required. Run `agent-desktop setup` for instructions."

### Challenge 5: Dynamic/Modal UI State
**Problem:** Menus, dialogs, sheets, and popups appear/disappear dynamically.
**Mitigations:**
- Detect modal dialogs in snapshot and highlight them
- Auto-include menu bar items when a menu is open
- Ref invalidation on state change
- `wait` command for synchronization

### Challenge 6: Multi-Monitor and Scaled Displays
**Problem:** Retina displays, multiple monitors, coordinate mapping.
**Mitigations:**
- Use AXUIElement frame positions (logical coordinates)
- ScreenCaptureKit handles Retina natively
- CGEvent coordinates are in global screen space — need proper mapping
- `status` command reports display configuration

---

## 8. Success Criteria

### MVP (Phase 1) is successful when:
- [ ] An AI agent can open TextEdit, type a document, save it — entirely via `agent-desktop` commands
- [ ] An AI agent can navigate Finder, find a file, and open it
- [ ] An AI agent can change a System Settings preference
- [ ] Snapshot-to-action latency < 2 seconds for typical apps
- [ ] Token cost per observation < 500 tokens

### Full tool is successful when:
- [ ] Works reliably with 90%+ of common macOS apps
- [ ] Integrated as a pi tool (like `agent-browser`)
- [ ] AI agents prefer it over screenshot-based approaches for Mac automation
- [ ] Community adoption and contribution

---

## Appendix A: Prior Art Reference

| Tool | Relevance | What to learn |
|------|-----------|---------------|
| **agent-browser** | Direct inspiration | Architecture, ref system, command grammar, AI-friendliness |
| **Peekaboo** | macOS automation | AXorcist usage, snapshot persistence, permission handling |
| **AXorcist** | Swift accessibility lib | AXUIElement traversal, element search, batch operations |
| **mac-use-mcp** | MCP approach | Alternative interaction model, accessibility tree extraction |
| **Ghost OS** | Hybrid approach | Vision model fallback for visual grounding |

## Appendix B: macOS API Quick Reference

| API | Purpose | Framework | Permission |
|-----|---------|-----------|------------|
| AXUIElement | Accessibility tree | ApplicationServices | Accessibility |
| CGEvent | Mouse/keyboard simulation | CoreGraphics | Accessibility |
| ScreenCaptureKit | Screen capture | ScreenCaptureKit | Screen Recording |
| NSWorkspace | App launching/info | AppKit | None |
| VNRecognizeTextRequest | OCR | Vision | None |
