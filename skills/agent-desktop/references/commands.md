# Command Reference

Complete reference for all agent-desktop commands. For quick start and common patterns, see [SKILL.md](../SKILL.md).

## App Management

```bash
agent-desktop open <app-name>        # Launch or focus application by name
agent-desktop open <file-path>       # Open file with default application
agent-desktop open <app> --with-cdp  # Launch Electron app with Chrome DevTools Protocol
agent-desktop open <app> --background  # Launch without stealing focus
agent-desktop close                  # Close frontmost window
```

### App name matching
- Case-insensitive: `"safari"`, `"Safari"`, `"SAFARI"` all work
- Fuzzy matching on error: if `"Safarri"` fails, suggests `"Safari"`
- Bundle IDs work too: `"com.apple.Safari"`

## Snapshot (UI Analysis)

```bash
agent-desktop snapshot               # Full accessibility tree
agent-desktop snapshot -i            # Interactive elements only (recommended)
agent-desktop snapshot -c            # Compact output (collapse single-child containers)
agent-desktop snapshot -d <depth>    # Limit tree depth (default: 10)
agent-desktop snapshot --app <name>  # Snapshot specific app (default: frontmost)
agent-desktop snapshot -s <selector> # CSS selector scope (CDP mode only)
```

### Flags

| Flag | Long | Description |
|---|---|---|
| `-i` | `--interactive` | Show only interactive elements with `@ref` handles |
| `-c` | `--compact` | Compact output — collapse single-child containers |
| `-d N` | `--depth N` | Max tree depth (default: 10). Lower = faster, less detail |
| | `--app <name>` | Target a specific app instead of frontmost |
| `-s` | `--selector <sel>` | CSS selector to scope snapshot (CDP mode only) |

### Performance notes
- Default depth of 10 works for most apps
- Complex apps (VS Code, Xcode) may benefit from `-d 5` for faster results
- 3-second timeout with partial results on slow apps
- Cached per-window with 1.5s TTL

## Interactions (Use @refs from Snapshot)

### Click

```bash
agent-desktop click @e1              # Left click
agent-desktop click @e1 --double     # Double-click
agent-desktop click @e1 --right      # Right-click (context menu)
agent-desktop click @e1 --foreground # Bring app to front first
agent-desktop click @e1 --no-wait    # Skip post-click wait
agent-desktop click @e1 --app <name> # Click in specific app
agent-desktop click <x> <y>          # Click at absolute screen coordinates
```

**Ref resolution order:**
1. Re-traverse AX path to verify element still exists, get current frame
2. Fall back to stored frame coordinates if path stale
3. Click center of resolved frame via CGEvent

### Fill (Clear + Type)

```bash
agent-desktop fill @e2 "text"        # Clear field, then type text
agent-desktop fill @e2 "text" --app <name>  # Fill in specific app
```

Uses `AXSetValue` when available, falls back to select-all + type.

### Type (Append)

```bash
agent-desktop type @e2 "text"        # Type into specific element (no clear)
agent-desktop type "text"            # Type into currently focused element
agent-desktop type @e2 "text" --app <name>  # Type in specific app
```

Uses CGEvent key events to simulate real typing.

### Press (Keyboard)

```bash
agent-desktop press <key>            # Press single key
agent-desktop press <modifier+key>   # Press key combo
agent-desktop press <key> --app <name>  # Send to specific app
```

**Key names** (case-insensitive):
- Control: `enter`/`return`, `tab`, `escape`/`esc`, `space`, `delete`/`backspace`, `forwarddelete`
- Arrow: `up`, `down`, `left`, `right`
- Navigation: `home`, `end`, `pageup`, `pagedown`
- Function: `f1`–`f12`
- Letters: `a`–`z`
- Numbers: `0`–`9`
- Symbols: `-`, `=`, `[`, `]`, `;`, `'`, `,`, `.`, `/`, `\`, `` ` ``

**Modifier names** (use `+` to combine):
- `cmd` / `command` (⌘)
- `shift` (⇧)
- `alt` / `option` (⌥)
- `ctrl` / `control` (⌃)
- `fn`

**Examples:**
```bash
agent-desktop press enter
agent-desktop press cmd+c
agent-desktop press cmd+shift+s
agent-desktop press ctrl+alt+delete
agent-desktop press cmd+shift+4     # macOS screenshot
```

### Scroll

```bash
agent-desktop scroll <direction>     # Scroll (default: 300px)
agent-desktop scroll <direction> <amount>  # Custom pixel amount
agent-desktop scroll <dir> --app <name>    # Scroll in specific app
```

Directions: `up`, `down`, `left`, `right`

### Select

```bash
agent-desktop select @e5 "value"     # Select dropdown/popup option
```

## Get Information

```bash
agent-desktop get text @e1           # Get element's text content
agent-desktop get text               # Get focused element's text
agent-desktop get title              # Get frontmost window title
agent-desktop get apps               # List running GUI applications
agent-desktop get windows            # List all windows
agent-desktop get windows --app <name>  # List windows for specific app
```

### `get apps` output format
```
Safari (pid 1234) ●     # ● = active/frontmost
Finder (pid 456)
TextEdit (pid 789)
```

## Wait

```bash
agent-desktop wait @e1               # Poll until element appears in accessibility tree
agent-desktop wait <milliseconds>    # Wait fixed duration
agent-desktop wait --load <state>    # Wait for page load state (CDP mode)
agent-desktop wait @e1 --app <name>  # Wait for element in specific app
```

## Screenshot

```bash
agent-desktop screenshot             # Capture frontmost window
agent-desktop screenshot --full      # Capture entire screen
agent-desktop screenshot --app <name>  # Capture specific app's window
```

Screenshots saved to temp directory. Response includes file path, dimensions, and scale factor.

## Status

```bash
agent-desktop status                 # Show daemon health and environment info
```

Returns:
- Daemon PID and uptime
- Accessibility permission status (✅/❌)
- Screen Recording permission status (✅/❌)
- Frontmost app name, PID, and window title
- Ref map element count and age
- Active CDP connections

## Global Options

| Option | Description |
|---|---|
| `--json` | Output raw JSON response (machine-readable) |
| `--timeout <ms>` | Override default command timeout |
| `--verbose` | Include debug info and profiling data |

## JSON Output Mode

Use `--json` for structured output suitable for programmatic parsing:

```bash
agent-desktop snapshot -i --json
# Returns full JSON response with data, timing, etc.

agent-desktop click @e1 --json
# {"id":"...","success":true,"data":{"_type":"click","ref":"e1","coordinates":{"x":500,"y":300},...}}
```

### Error response format
```json
{
  "id": "req_001",
  "success": false,
  "error": {
    "code": "REF_NOT_FOUND",
    "message": "Element @e3 not found. The UI may have changed.",
    "suggestion": "Run `snapshot` to refresh element references."
  },
  "timing": {"elapsed_ms": 2.1}
}
```

### Error codes
| Code | Description |
|---|---|
| `REF_NOT_FOUND` | Element ref doesn't exist in current ref map |
| `REF_STALE` | Element existed but can't be re-located in live tree |
| `NO_REF_MAP` | No snapshot taken yet |
| `APP_NOT_FOUND` | Target app not running or not installed |
| `WINDOW_NOT_FOUND` | Target window doesn't exist |
| `PERMISSION_DENIED` | Missing Accessibility or Screen Recording permission |
| `TIMEOUT` | Command exceeded timeout (partial results may be available) |
| `AX_ERROR` | Accessibility API returned an error |
| `INPUT_ERROR` | CGEvent failed to post |
| `INVALID_COMMAND` | Malformed command or arguments |
| `DAEMON_ERROR` | Internal daemon error |
| `CDP_NOT_AVAILABLE` | CDP mode requested but not available for this app |
| `CDP_ERROR` | Chrome DevTools Protocol error |
