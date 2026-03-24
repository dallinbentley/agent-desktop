# Command Reference

Complete reference for all agent-computer commands. For quick start and common patterns, see [SKILL.md](../SKILL.md).

## App Management

```bash
agent-computer open <app-name>        # Launch or focus application by name
agent-computer open <file-path>       # Open file with default application
agent-computer open <app> --with-cdp  # Launch Electron app with Chrome DevTools Protocol
agent-computer open <app> --background  # Launch without stealing focus
agent-computer close                  # Close frontmost window
```

### App name matching
- Case-insensitive: `"safari"`, `"Safari"`, `"SAFARI"` all work
- Fuzzy matching on error: if `"Safarri"` fails, suggests `"Safari"`
- Bundle IDs work too: `"com.apple.Safari"`

## Snapshot (UI Analysis)

```bash
agent-computer snapshot               # Full accessibility tree
agent-computer snapshot -i            # Interactive elements only (recommended)
agent-computer snapshot -c            # Compact output (collapse single-child containers)
agent-computer snapshot -d <depth>    # Limit tree depth (default: 10)
agent-computer snapshot --app <name>  # Snapshot specific app (default: frontmost)
agent-computer snapshot -s <selector> # CSS selector scope (CDP mode only)
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
agent-computer click @e1              # Left click
agent-computer click @e1 --double     # Double-click
agent-computer click @e1 --right      # Right-click (context menu)
agent-computer click @e1 --foreground # Bring app to front first
agent-computer click @e1 --no-wait    # Skip post-click wait
agent-computer click @e1 --app <name> # Click in specific app
agent-computer click <x> <y>          # Click at absolute screen coordinates
```

**Ref resolution order:**
1. Re-traverse AX path to verify element still exists, get current frame
2. Fall back to stored frame coordinates if path stale
3. Click center of resolved frame via CGEvent

### Fill (Clear + Type)

```bash
agent-computer fill @e2 "text"        # Clear field, then type text
agent-computer fill @e2 "text" --app <name>  # Fill in specific app
```

Uses `AXSetValue` when available, falls back to select-all + type.

### Type (Append)

```bash
agent-computer type @e2 "text"        # Type into specific element (no clear)
agent-computer type "text"            # Type into currently focused element
agent-computer type @e2 "text" --app <name>  # Type in specific app
```

Uses CGEvent key events to simulate real typing.

### Press (Keyboard)

```bash
agent-computer press <key>            # Press single key
agent-computer press <modifier+key>   # Press key combo
agent-computer press <key> --app <name>  # Send to specific app
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
agent-computer press enter
agent-computer press cmd+c
agent-computer press cmd+shift+s
agent-computer press ctrl+alt+delete
agent-computer press cmd+shift+4     # macOS screenshot
```

### Scroll

```bash
agent-computer scroll <direction>     # Scroll (default: 300px)
agent-computer scroll <direction> <amount>  # Custom pixel amount
agent-computer scroll <dir> --app <name>    # Scroll in specific app
```

Directions: `up`, `down`, `left`, `right`

### Select

```bash
agent-computer select @e5 "value"     # Select dropdown/popup option
```

## Get Information

```bash
agent-computer get text @e1           # Get element's text content
agent-computer get text               # Get focused element's text
agent-computer get title              # Get frontmost window title
agent-computer get apps               # List running GUI applications
agent-computer get windows            # List all windows
agent-computer get windows --app <name>  # List windows for specific app
```

### `get apps` output format
```
Safari (pid 1234) ●     # ● = active/frontmost
Finder (pid 456)
TextEdit (pid 789)
```

## Wait

```bash
agent-computer wait @e1               # Poll until element appears in accessibility tree
agent-computer wait <milliseconds>    # Wait fixed duration
agent-computer wait --load <state>    # Wait for page load state (CDP mode)
agent-computer wait @e1 --app <name>  # Wait for element in specific app
```

## Screenshot

```bash
agent-computer screenshot             # Capture frontmost window
agent-computer screenshot --full      # Capture entire screen
agent-computer screenshot --app <name>  # Capture specific app's window
```

Screenshots saved to temp directory. Response includes file path, dimensions, and scale factor.

## Status

```bash
agent-computer status                 # Show daemon health and environment info
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
agent-computer snapshot -i --json
# Returns full JSON response with data, timing, etc.

agent-computer click @e1 --json
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
