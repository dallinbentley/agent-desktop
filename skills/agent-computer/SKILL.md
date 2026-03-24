---
name: agent-computer
description: macOS desktop automation CLI for AI agents. Use when the user needs to interact with native desktop applications, including clicking buttons, filling text fields, navigating menus, changing system settings, managing files in Finder, automating any macOS app, or any task requiring programmatic desktop control. Triggers include requests to "open an app", "click a button in Finder", "change a system setting", "type in TextEdit", "automate a desktop workflow", "take a screenshot of an app", or any task requiring macOS GUI interaction.
allowed-tools: Bash(agent-computer:*)
---

# Desktop Automation with agent-computer

A CLI tool that lets AI agents control any macOS application through accessibility tree snapshots and simple atomic commands. The desktop equivalent of agent-browser — same ref system, same command grammar, different target.

Install from source: `cargo install --path crates/cli && cargo install --path crates/daemon`. macOS 13+ (Ventura) required. On first run, grant **Accessibility** and **Screen Recording** permissions in System Settings → Privacy & Security. Run `agent-computer status` to verify.

## Core Workflow

Every desktop automation follows this pattern:

1. **Open/focus app**: `agent-computer open "TextEdit"`
2. **Snapshot**: `agent-computer snapshot -i` (get element refs like `@e1`, `@e2`)
3. **Interact**: Use refs to click, fill, press keys
4. **Re-snapshot**: After UI changes (clicking buttons, opening menus, switching views), get fresh refs

```bash
agent-computer open "TextEdit"
agent-computer snapshot -i
# Output: @e1 button "New", @e2 button "Open", @e3 text_area (editable)

agent-computer click @e3
agent-computer type @e3 "Hello from agent-computer!"
agent-computer press cmd+s
agent-computer wait 1000
agent-computer snapshot -i  # Check result
```

## Command Chaining

Commands can be chained with `&&` in a single shell invocation. The daemon persists between commands, so chaining is safe and more efficient than separate calls.

```bash
# Chain open + wait + snapshot in one call
agent-computer open "Finder" && agent-computer wait 1000 && agent-computer snapshot -i

# Chain multiple interactions
agent-computer click @e3 && agent-computer type @e3 "Hello" && agent-computer press cmd+s

# Open app and capture
agent-computer open "System Settings" && agent-computer wait 2000 && agent-computer screenshot
```

**When to chain:** Use `&&` when you don't need to read intermediate output before proceeding (e.g., open + wait + screenshot). Run commands separately when you need to parse the output first (e.g., snapshot to discover refs, then interact using those refs).

## Essential Commands

```bash
# App management
agent-computer open "Safari"          # Launch or focus app
agent-computer open "/path/to/file"   # Open file with default app
agent-computer open "Spotify" --with-cdp  # Open Electron app with CDP support
agent-computer close                  # Close frontmost window

# Snapshot
agent-computer snapshot -i            # Interactive elements with refs (recommended)
agent-computer snapshot -i -d 15      # Deeper tree traversal (default depth: 10)
agent-computer snapshot -i -c         # Compact output (collapse containers)
agent-computer snapshot -i --app "Finder"  # Snapshot specific app (default: frontmost)

# Interaction (use @refs from snapshot)
agent-computer click @e1              # Click element
agent-computer click @e1 --double     # Double-click
agent-computer click @e1 --right      # Right-click (context menu)
agent-computer click 500 300          # Click absolute coordinates (fallback)
agent-computer fill @e2 "text"        # Clear and type text
agent-computer type @e2 "text"        # Type without clearing
agent-computer type "text"            # Type into focused element (no ref)
agent-computer press enter            # Press key
agent-computer press cmd+c            # Key combo (cmd, shift, alt/option, ctrl)
agent-computer press cmd+shift+s      # Multiple modifiers
agent-computer scroll down            # Scroll (default: 300px)
agent-computer scroll up 500          # Scroll with custom amount

# Get information
agent-computer get text @e1           # Get element text
agent-computer get title              # Get frontmost window title
agent-computer get apps               # List running GUI applications
agent-computer get windows            # List all windows

# Wait
agent-computer wait @e1               # Wait for element to appear
agent-computer wait 2000              # Wait milliseconds

# Capture
agent-computer screenshot             # Capture frontmost window
agent-computer screenshot --full      # Capture entire screen
agent-computer screenshot --app "Finder"  # Capture specific app's window

# Status
agent-computer status                 # Daemon health, permissions, frontmost app
```

## Common Patterns

### Open an App and Interact

```bash
agent-computer open "TextEdit"
agent-computer wait 1000
agent-computer snapshot -i
# @e1 button "New", @e2 button "Open", @e3 text_area (editable)
agent-computer click @e3
agent-computer type @e3 "Meeting notes for today..."
agent-computer press cmd+s
agent-computer wait 1000
agent-computer screenshot
```

### Navigate Finder

```bash
agent-computer open "Finder"
agent-computer snapshot -i
# @e1 toolbar_button "Back", @e2 search_field "Search", @e3 icon "Documents" (folder)
agent-computer click @e3 --double
agent-computer wait 1000
agent-computer snapshot -i
# New refs after navigation
agent-computer click @e5 --double  # Open a file
```

### Change System Settings

```bash
agent-computer open "System Settings"
agent-computer wait 2000
agent-computer snapshot -i
# @e1 search_field "Search", @e2 button "General", @e3 button "Appearance"
agent-computer click @e3
agent-computer wait 1000
agent-computer snapshot -i
# @e4 radio "Light", @e5 radio "Dark", @e6 radio "Auto"
agent-computer click @e5
agent-computer screenshot
```

### Multi-App Workflow (Copy Between Apps)

```bash
# Copy from Safari
agent-computer open "Safari"
agent-computer snapshot -i --app "Safari"
agent-computer click @e3
agent-computer press cmd+a
agent-computer press cmd+c

# Paste into TextEdit
agent-computer open "TextEdit"
agent-computer snapshot -i --app "TextEdit"
agent-computer click @e4
agent-computer press cmd+v
agent-computer press cmd+s
```

### Menu Navigation

```bash
agent-computer open "TextEdit"
agent-computer snapshot -i
# @e5 menu "File", @e6 menu "Edit", @e7 menu "Format"
agent-computer click @e5
agent-computer wait 500
agent-computer snapshot -i
# Menu is now open — new refs for menu items
# @e10 menu_item "New", @e11 menu_item "Open...", @e12 menu_item "Save"
agent-computer click @e11
agent-computer wait 1000
agent-computer snapshot -i  # File picker dialog
```

### Coordinate Fallback (Poor Accessibility)

For apps with incomplete accessibility trees (games, canvas-based apps, some Electron apps):

```bash
agent-computer screenshot
# Visually identify the target area from screenshot
agent-computer click 500 300
agent-computer wait 1000
agent-computer screenshot  # Verify result
```

### Electron Apps with CDP

```bash
# Launch with Chrome DevTools Protocol for richer inspection
agent-computer open "Spotify" --with-cdp
agent-computer wait 2000
agent-computer snapshot -i
# CDP-enhanced snapshot with web-level detail
agent-computer click @e5
agent-computer screenshot
```

### App Targeting (Background Interaction)

Use `--app` to interact with a specific app without switching focus:

```bash
# Snapshot a specific app
agent-computer snapshot -i --app "Finder"

# Send keys to a specific app
agent-computer press cmd+n --app "TextEdit"

# Screenshot a specific app's window
agent-computer screenshot --app "Safari"
```

## Architecture

```
agent-computer (CLI)  ──Unix Socket──▶  agent-computer-daemon  ──▶  macOS APIs
   (stateless)            IPC             (persistent)              AXUIElement
                       JSON over                                    CGEvent
                    ~/.agent-computer/    • Ref map management      ScreenCaptureKit
                       daemon.sock       • Element cache            NSWorkspace
                                         • Tree traversal
```

- The **CLI** is stateless — it serializes commands as JSON and sends them over a Unix socket
- The **daemon** runs in the background, maintaining element references, caching accessibility trees, and routing commands
- The daemon **auto-starts** when you run any CLI command — no manual setup needed
- Both binaries (`agent-computer` and `agent-computer-daemon`) must be on your PATH

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
agent-computer click @e5             # Opens a menu
agent-computer snapshot -i           # MUST re-snapshot — old refs are stale
agent-computer click @e10            # Use new refs from fresh snapshot
```

## Permissions and Setup

```bash
# Check status
agent-computer status
# Output:
# agent-computer daemon
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
4. Run `agent-computer status` to verify

## Timeouts and Slow Apps

The default snapshot timeout is 3 seconds with partial results returned on timeout. For slow or complex apps:

```bash
# Wait for app to load before snapshotting
agent-computer wait 2000
agent-computer snapshot -i

# Reduce depth for faster snapshots on complex apps
agent-computer snapshot -i -d 5

# Wait for a specific element to appear
agent-computer wait @e1

# Use --timeout for longer operations
agent-computer snapshot -i --timeout 10000
```

## Error Handling

agent-computer provides AI-friendly error messages with recovery suggestions:

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

| | agent-browser | agent-computer |
|---|---|---|
| **Target** | Web pages (Chromium) | Any macOS app |
| **Entry point** | `open <url>` | `open <app-name>` |
| **Observation** | DOM accessibility tree | macOS accessibility tree (AXUIElement) |
| **Refs** | `@e1`, `@e2`, ... | `@e1`, `@e2`, ... (same system) |
| **Input simulation** | Playwright CDP events | CGEvent (mouse/keyboard) |
| **Screenshots** | Browser viewport | Window or full screen (ScreenCaptureKit) |
| **Daemon** | Node.js process | Rust daemon process |
| **Command grammar** | `click`, `fill`, `type`, `press`, `scroll` | Same grammar + app management |

The command grammar is intentionally identical — if you know agent-browser, you know agent-computer.

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
