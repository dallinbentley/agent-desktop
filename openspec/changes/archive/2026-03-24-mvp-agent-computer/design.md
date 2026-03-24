## Context

We're building `agent-desktop`, a CLI tool for AI agents to control macOS desktops. It's modeled directly on agent-browser (Vercel Labs) which proved that accessibility-tree snapshots with compact @refs are 10-100x more token-efficient than screenshot-based approaches for AI agent interaction.

Technical spikes (S1-S5) have validated all core assumptions:
- **AX tree traversal**: 50-422ms across 6 real apps (Finder, TextEdit, Safari, System Settings, IDE, Terminal)
- **CGEvent input**: 36/45 tests passed. `keyboardSetUnicodeString` handles all Unicode. `CGWarpMouseCursorPosition` for mouse.
- **ScreenCaptureKit**: 22ms captures, `CGPreflightScreenCaptureAccess()` for permission detection
- **Daemon IPC**: 100ms startup, <5ms JSON round-trip over Unix socket
- **AXorcist evaluation**: Too heavy — rolling our own traversal (~200 lines vs their 16K)

The spike code in `Sources/Spikes/` provides working reference implementations for each subsystem.

## Goals / Non-Goals

**Goals:**
- Working CLI that an AI agent can use to: snapshot a Mac app's UI, click buttons, type text, press keys, take screenshots, open apps
- Compact text snapshots with @refs (target <500 tokens for typical app window)
- Sub-2-second snapshot-to-action latency for typical apps
- AI-friendly error messages with recovery suggestions
- Demoable end-to-end: open TextEdit → snapshot → click → type → screenshot

**Non-Goals:**
- Cross-platform support (Linux/Windows) — future phases
- Performance optimization beyond "good enough for demo" — optimize later
- MCP server mode — future
- OCR/vision fallback for apps with poor accessibility — future
- Named sessions or parallel agent isolation — future
- Diff snapshots — future
- Compact/depth modes beyond basic `-d` flag — future

## Decisions

### D1: Pure Swift, Single Binary (not Rust CLI + Node.js daemon)

**Choice**: Both CLI and daemon in Swift, built with Swift Package Manager.

**Why**: First-class macOS API access (AXUIElement, CGEvent, ScreenCaptureKit) without FFI. Single language reduces complexity. agent-browser uses Rust+Node.js because Playwright is Node.js — we have no such constraint.

**Alternative considered**: Node.js wrapper calling Swift binary (like agent-browser's Rust→Node.js pattern). Rejected because it adds IPC overhead, two build systems, and runtime dependency.

### D2: Daemon auto-spawned on first CLI use (not launchd)

**Choice**: CLI checks for socket → if missing, spawns daemon as background process → polls for socket → connects.

**Why**: Zero setup, no root permissions, version-matched (CLI and daemon always same binary). Validated in spike S5.

**Alternative considered**: launchd plist for auto-start. Rejected — requires installation step, harder to version-match, overkill for MVP.

### D3: `keyboardSetUnicodeString` for typing, keycodes for press

**Choice**: `type` command uses `CGEvent.keyboardSetUnicodeString` (chunked at 20 UTF-16 units). `press` command uses virtual keycodes + modifier flags.

**Why**: Unicode API handles all characters (7/7 in spike). Keycodes are needed for `press` where specific key behavior matters (Enter, Cmd+C). Per-character keycode typing drops non-ASCII (spike finding).

### D4: AX selection-replace for `fill` (not `kAXValueAttribute`)

**Choice**: `fill` selects all text via AX `kAXSelectedTextRangeAttribute`, then replaces via `kAXSelectedTextAttribute`.

**Why**: Direct `kAXValueAttribute` setting returns success but silently fails on TextEdit (critical spike finding S3). Selection-replace is proven to work.

### D5: `CGWarpMouseCursorPosition` for mouse positioning (not `.mouseMoved`)

**Choice**: Use `CGWarpMouseCursorPosition()` then `CGEvent` click at target position.

**Why**: `.mouseMoved` CGEvent drifts up to 288px due to mouse acceleration (spike finding S3). Warp is exact.

### D6: Two-pass snapshot (minimal scan → selective attribute extraction)

**Choice**: First pass: traverse tree collecting only role + children (fast). Second pass: for interactive elements only, batch-fetch title, description, frame, actions.

**Why**: Per-element cost is 0.15-0.35ms with full attributes (spike S1). Two-pass with interactive-only detail reduces work. Target: <100ms for typical apps.

### D7: Element re-identification via stored AX path + coordinate fallback

**Choice**: RefMap stores `axPath` (role+index chain from app root) and `frame` (coordinates). On action: try re-traversing path first, fall back to coordinates.

**Why**: AXUIElement handles become stale. Path re-traversal is most reliable. Coordinates are fast fallback when UI hasn't moved but tree has changed.

### D8: SCScreenshotManager primary, CGWindowListCreateImage fallback

**Choice**: Use ScreenCaptureKit for screenshots. Keep legacy API as fallback.

**Why**: SCK is GPU-accelerated, future-proof, supports resolution control (1x for agents). Legacy is deprecated but still works and is faster (8.7ms vs 22ms). Both validated in spike S4.

## Risks / Trade-offs

**[Slow AX traversal on complex apps]** → Depth limit default 10, 3s timeout with partial results, interactive-only filtering. IDE spike was 422ms full / 95ms at depth 10 — acceptable.

**[Stale element refs between snapshot and action]** → Path re-traversal + coordinate fallback. If both fail, return actionable error suggesting `snapshot` refresh. Agents naturally re-snapshot frequently.

**[Apps with poor accessibility trees]** → Out of scope for MVP. Log warning when tree seems sparse. Coordinate-based `click x y` available as escape hatch.

**[Permission UX friction]** → `status` command shows permission state. Clear error messages with instructions. `setup` command opens System Settings. Two permissions (Accessibility + Screen Recording) is unavoidable.

**[Swift `String(format:)` segfault on macOS 26]** → Discovered in spike S1. Use string interpolation instead. Minor compiler bug, easy workaround.
