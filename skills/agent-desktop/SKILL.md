---
name: agent-desktop
description: macOS desktop automation CLI for AI agents. Use when the user needs to interact with native desktop applications, including clicking buttons, filling text fields, navigating menus, changing system settings, managing files in Finder, automating any macOS app, or any task requiring programmatic desktop control. Triggers include requests to "open an app", "click a button in Finder", "change a system setting", "type in TextEdit", "automate a desktop workflow", "take a screenshot of an app", or any task requiring macOS GUI interaction.
allowed-tools: Bash(agent-desktop:*)
---

# Desktop Automation with agent-desktop

A CLI tool that lets AI agents control any macOS application through accessibility tree snapshots and simple atomic commands. The desktop equivalent of agent-browser — same ref system, same command grammar, different target.

Install from source: `cargo install --path crates/cli && cargo install --path crates/daemon`. macOS 13+ (Ventura) required. On first run, grant **Accessibility** and **Screen Recording** permissions in System Settings → Privacy & Security. Run `agent-desktop status` to verify.

**Electron/CDP support** is built in — the daemon automatically downloads and bundles [agent-browser](https://github.com/vercel-labs/agent-browser) on first use (cached at `~/.agent-desktop/bin/`). No manual install needed. You never call agent-browser directly.

## Core Workflow

Every desktop automation follows this pattern:

1. **Open/focus app**: `agent-desktop open "TextEdit"`
2. **Snapshot**: `agent-desktop snapshot -i` (get element refs like `@e1`, `@e2`)
3. **Interact**: Use refs to click, fill, press keys
4. **Re-snapshot**: After UI changes (clicking buttons, opening menus, switching views), get fresh refs

```bash
agent-desktop open "TextEdit"
agent-desktop snapshot -i
# Output: @e1 button "New", @e2 button "Open", @e3 text_area (editable)

agent-desktop click @e3
agent-desktop type @e3 "Hello from agent-desktop!"
agent-desktop press cmd+s
agent-desktop wait 1000
agent-desktop snapshot -i  # Check result
```

## Command Chaining

Commands can be chained with `&&` in a single shell invocation. The daemon persists between commands, so chaining is safe and more efficient than separate calls.

```bash
# Chain open + wait + snapshot in one call
agent-desktop open "Finder" && agent-desktop wait 1000 && agent-desktop snapshot -i

# Chain multiple interactions
agent-desktop click @e3 && agent-desktop type @e3 "Hello" && agent-desktop press cmd+s

# Open app and capture
agent-desktop open "System Settings" && agent-desktop wait 2000 && agent-desktop screenshot
```

**When to chain:** Use `&&` when you don't need to read intermediate output before proceeding (e.g., open + wait + screenshot). Run commands separately when you need to parse the output first (e.g., snapshot to discover refs, then interact using those refs).

## Essential Commands

```bash
# App management
agent-desktop open "Safari"          # Launch or focus app
agent-desktop open "/path/to/file"   # Open file with default app
agent-desktop open "Spotify" --with-cdp  # Open Electron app with CDP support
agent-desktop close                  # Close frontmost window

# Snapshot
agent-desktop snapshot -i            # Interactive elements with refs (recommended)
agent-desktop snapshot -i -d 15      # Deeper tree traversal (default depth: 10)
agent-desktop snapshot -i -c         # Compact output (collapse containers)
agent-desktop snapshot -i --app "Finder"  # Snapshot specific app (default: frontmost)

# Interaction (use @refs from snapshot)
agent-desktop click @e1              # Click element
agent-desktop click @e1 --double     # Double-click
agent-desktop click @e1 --right      # Right-click (context menu)
agent-desktop click 500 300          # Click absolute coordinates (fallback)
agent-desktop fill @e2 "text"        # Clear and type text
agent-desktop type @e2 "text"        # Type without clearing
agent-desktop type "text"            # Type into focused element (no ref)
agent-desktop press enter            # Press key
agent-desktop press cmd+c            # Key combo (cmd, shift, alt/option, ctrl)
agent-desktop press cmd+shift+s      # Multiple modifiers
agent-desktop scroll down            # Scroll (default: 300px)
agent-desktop scroll up 500          # Scroll with custom amount

# Get information
agent-desktop get text @e1           # Get element text
agent-desktop get title              # Get frontmost window title
agent-desktop get apps               # List running GUI applications
agent-desktop get windows            # List all windows

# Wait
agent-desktop wait @e1               # Wait for element to appear
agent-desktop wait 2000              # Wait milliseconds

# Capture
agent-desktop screenshot             # Capture frontmost window
agent-desktop screenshot --full      # Capture entire screen
agent-desktop screenshot --app "Finder"  # Capture specific app's window

# Status
agent-desktop status                 # Daemon health, permissions, frontmost app
```

## Common Patterns

### Open an App and Interact

```bash
agent-desktop open "TextEdit"
agent-desktop wait 1000
agent-desktop snapshot -i
# @e1 button "New", @e2 button "Open", @e3 text_area (editable)
agent-desktop click @e3
agent-desktop type @e3 "Meeting notes for today..."
agent-desktop press cmd+s
agent-desktop wait 1000
agent-desktop screenshot
```

### Navigate Finder

```bash
agent-desktop open "Finder"
agent-desktop snapshot -i
# @e1 toolbar_button "Back", @e2 search_field "Search", @e3 icon "Documents" (folder)
agent-desktop click @e3 --double
agent-desktop wait 1000
agent-desktop snapshot -i
# New refs after navigation
agent-desktop click @e5 --double  # Open a file
```

### Change System Settings

```bash
agent-desktop open "System Settings"
agent-desktop wait 2000
agent-desktop snapshot -i
# @e1 search_field "Search", @e2 button "General", @e3 button "Appearance"
agent-desktop click @e3
agent-desktop wait 1000
agent-desktop snapshot -i
# @e4 radio "Light", @e5 radio "Dark", @e6 radio "Auto"
agent-desktop click @e5
agent-desktop screenshot
```

### Multi-App Workflow (Copy Between Apps)

```bash
# Copy from Safari
agent-desktop open "Safari"
agent-desktop snapshot -i --app "Safari"
agent-desktop click @e3
agent-desktop press cmd+a
agent-desktop press cmd+c

# Paste into TextEdit
agent-desktop open "TextEdit"
agent-desktop snapshot -i --app "TextEdit"
agent-desktop click @e4
agent-desktop press cmd+v
agent-desktop press cmd+s
```

### Menu Navigation

```bash
agent-desktop open "TextEdit"
agent-desktop snapshot -i
# @e5 menu "File", @e6 menu "Edit", @e7 menu "Format"
agent-desktop click @e5
agent-desktop wait 500
agent-desktop snapshot -i
# Menu is now open — new refs for menu items
# @e10 menu_item "New", @e11 menu_item "Open...", @e12 menu_item "Save"
agent-desktop click @e11
agent-desktop wait 1000
agent-desktop snapshot -i  # File picker dialog
```

### Coordinate Fallback (Poor Accessibility)

For apps with incomplete accessibility trees (games, canvas-based apps, some Electron apps):

```bash
agent-desktop screenshot
# Visually identify the target area from screenshot
agent-desktop click 500 300
agent-desktop wait 1000
agent-desktop screenshot  # Verify result
```

### Electron Apps with CDP

The daemon auto-downloads agent-browser on first CDP use — no manual install needed.

```bash
# Launch with Chrome DevTools Protocol for richer inspection
agent-desktop open "Spotify" --with-cdp
agent-desktop wait 2000
agent-desktop snapshot -i
# CDP-enhanced snapshot with web-level detail — refs from both native chrome
# and web content are unified into a single ref map
agent-desktop click @e5
agent-desktop screenshot
```

### App Targeting (Background Interaction)

Use `--app` to interact with a specific app without switching focus:

```bash
# Snapshot a specific app
agent-desktop snapshot -i --app "Finder"

# Send keys to a specific app
agent-desktop press cmd+n --app "TextEdit"

# Screenshot a specific app's window
agent-desktop screenshot --app "Safari"
```

## Architecture

```
agent-desktop (CLI)  ──Unix Socket──▶  agent-desktop-daemon  ──▶  macOS APIs
   (stateless)            IPC             (persistent)              AXUIElement
                       JSON over          • Ref map management      CGEvent
                    ~/.agent-desktop/    • Element cache            ScreenCaptureKit
                       daemon.sock       • Tree traversal           NSWorkspace
                                         • Browser bridge ─────▶  agent-browser
                                           (optional, for             (subprocess)
                                            Electron/CDP apps)
```

- The **CLI** is stateless — it serializes commands as JSON and sends them over a Unix socket
- The **daemon** runs in the background, maintaining element references, caching accessibility trees, and routing commands
- The daemon **auto-starts** when you run any CLI command — no manual setup needed
- Both binaries (`agent-desktop` and `agent-desktop-daemon`) must be on your PATH
- For **Electron/CDP apps**, the daemon internally delegates to [agent-browser](https://github.com/vercel-labs/agent-browser) via subprocess. It auto-downloads the correct platform binary on first use (pinned to a known-good version) and caches it at `~/.agent-desktop/bin/`. You never call agent-browser directly — all interaction goes through `agent-desktop` commands, and the daemon handles routing transparently. If auto-download fails (no network), it falls back to any agent-browser found in PATH

## Snapshot Output Format

The snapshot produces a compact text tree. Only interactive elements get `@ref` handles. Non-interactive containers provide spatial context.

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

### Interactive Roles (Elements That Get @refs)

`AXButton`, `AXTextField`, `AXTextArea`, `AXCheckBox`, `AXRadioButton`, `AXPopUpButton`, `AXComboBox`, `AXSlider`, `AXLink`, `AXMenuItem`, `AXMenuButton`, `AXTab`, `AXTabGroup`, `AXScrollArea`, `AXTable`, `AXOutline`, `AXSwitch`, `AXSearchField`, `AXIncrementor`

## Key Combos Quick Reference

| Combo | Action |
|---|---|
| `cmd+c` | Copy |
| `cmd+v` | Paste |
| `cmd+x` | Cut |
| `cmd+z` | Undo |
| `cmd+shift+z` | Redo |
| `cmd+a` | Select all |
| `cmd+s` | Save |
| `cmd+shift+s` | Save as |
| `cmd+n` | New |
| `cmd+w` | Close window |
| `cmd+q` | Quit app |
| `cmd+tab` | Switch app |
| `cmd+space` | Spotlight |
| `cmd+,` | Preferences/settings |

Modifiers: `cmd` (⌘), `shift` (⇧), `alt`/`option` (⌥), `ctrl` (⌃), `fn`

## Ref Lifecycle (Important)

Refs (`@e1`, `@e2`, etc.) are invalidated when the UI changes. Always re-snapshot after:

- Clicking buttons or links that change the view
- Opening/closing menus, dialogs, or sheets
- Switching tabs or windows
- Any navigation that updates the UI

```bash
agent-desktop click @e5             # Opens a menu
agent-desktop snapshot -i           # MUST re-snapshot — old refs are stale
agent-desktop click @e10            # Use new refs from fresh snapshot
```

## Permissions and Setup

```bash
# Check status
agent-desktop status
# Output:
# agent-desktop daemon
#   PID: 12345
#   Accessibility: ✅ granted
#   Screen Recording: ✅ granted
#   Frontmost App: Finder (pid 456)
#   Frontmost Window: ~/Documents — 5 items
#   Ref Map: 9 elements (age: 3.2s)
#   CDP Connections: 0
```

If permissions show ❌:
1. Open **System Settings → Privacy & Security → Accessibility**
2. Add and enable your terminal app (Terminal.app, iTerm2, Warp, etc.)
3. Repeat for **Screen Recording** if screenshots fail
4. Run `agent-desktop status` to verify

## Timeouts and Slow Apps

The default snapshot timeout is 3 seconds with partial results returned on timeout. For slow or complex apps:

```bash
# Wait for app to load before snapshotting
agent-desktop wait 2000
agent-desktop snapshot -i

# Reduce depth for faster snapshots on complex apps
agent-desktop snapshot -i -d 5

# Wait for a specific element to appear
agent-desktop wait @e1

# Use --timeout for longer operations
agent-desktop snapshot -i --timeout 10000
```

## Error Handling

agent-desktop provides AI-friendly error messages with recovery suggestions:

| Error | Message |
|---|---|
| Ref not found | `Element @e3 not found. The UI may have changed. Run 'snapshot' to refresh element references.` |
| No snapshot | `No element references available. Run 'snapshot -i' first to discover interactive elements.` |
| Stale ref | `Element @e3 existed but can't be re-located. Run 'snapshot' to refresh.` |
| App not found | `Application 'Safarri' not found. Running apps: Safari, Finder, TextEdit. Did you mean 'Safari'?` |
| Missing permission | `Accessibility permission required. Grant in System Settings → Privacy & Security.` |
| Timeout | `Snapshot timed out after 3s. Returning partial results. Try 'snapshot -d 5' to reduce depth.` |

In `--json` mode, errors include machine-parseable codes:
```json
{"success": false, "error": {"code": "REF_NOT_FOUND", "message": "...", "suggestion": "..."}}
```

## Comparison with agent-browser

| | agent-browser | agent-desktop |
|---|---|---|
| **Target** | Web pages (Chromium) | Any macOS app |
| **Entry point** | `open <url>` | `open <app-name>` |
| **Observation** | DOM accessibility tree | macOS accessibility tree (AXUIElement) |
| **Refs** | `@e1`, `@e2`, ... | `@e1`, `@e2`, ... (same system) |
| **Input simulation** | Playwright CDP events | CGEvent (mouse/keyboard) |
| **Screenshots** | Browser viewport | Window or full screen (ScreenCaptureKit) |
| **Daemon** | Node.js process | Rust daemon process |
| **Command grammar** | `click`, `fill`, `type`, `press`, `scroll` | Same grammar + app management |

The command grammar is intentionally identical — if you know agent-browser, you know agent-desktop.

## Deep-Dive Documentation

| Reference | When to Use |
|---|---|
| [references/commands.md](references/commands.md) | Full command reference with all flags and options |
| [references/snapshot-refs.md](references/snapshot-refs.md) | Ref lifecycle, interactive roles, troubleshooting stale refs |
| [references/permissions.md](references/permissions.md) | macOS permission setup, troubleshooting, terminal-specific guidance |
| [references/electron-cdp.md](references/electron-cdp.md) | Using CDP mode for Electron and browser-based desktop apps |

## Ready-to-Use Templates

| Template | Description |
|---|---|
| [templates/app-automation.sh](templates/app-automation.sh) | Open app, interact, verify with screenshot |
| [templates/system-settings.sh](templates/system-settings.sh) | Navigate and change a system setting |
| [templates/file-management.sh](templates/file-management.sh) | Finder file operations (navigate, open, move) |
