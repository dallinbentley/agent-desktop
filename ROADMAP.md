# agent-computer — Project Roadmap

> A CLI tool for AI agents to control macOS desktops efficiently, inspired by [agent-browser](https://github.com/anthropics/agent-browser).

---

## 1. Project Vision

**agent-computer** is to the desktop what agent-browser is to the web — a lightweight, text-first CLI tool that lets AI agents observe and interact with any macOS application through compact accessibility tree snapshots and simple atomic commands.

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

### 3.2 Concept Mapping: agent-browser → agent-computer

| agent-browser | agent-computer | Implementation |
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

### 3.4 Performance Strategy

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

### Grammar: `agent-computer <command> [args] [options]`

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
agent-computer/
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
- `brew install agent-computer` (eventual goal)
- Minimum macOS 13 (Ventura) for ScreenCaptureKit features

---

## 6. Implementation Phases

### Phase 1: MVP — Snapshot + Click + Type (Weeks 1-3)

**Goal:** An agent can observe a Mac app and interact with basic elements.

```
agent-computer snapshot -i          ✅ See interactive elements with @refs
agent-computer click @e3            ✅ Click a button/link
agent-computer type @e3 "hello"     ✅ Type into a text field
agent-computer press Enter          ✅ Press a key
agent-computer screenshot           ✅ Take a screenshot
agent-computer open "Safari"        ✅ Focus/launch an app
agent-computer status               ✅ Check daemon health + permissions
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
- **pi integration** — First-class `agent-computer` tool in pi (like `agent-browser` tool today)
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
- `agent-computer status` shows permission state clearly
- First-run wizard with step-by-step instructions
- `agent-computer setup` command to open System Settings to the right pane
- Clear error messages: "Accessibility permission required. Run `agent-computer setup` for instructions."

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
- [ ] An AI agent can open TextEdit, type a document, save it — entirely via `agent-computer` commands
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
