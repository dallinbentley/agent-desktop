# Electron Apps & CDP Mode

agent-computer can use Chrome DevTools Protocol (CDP) for enhanced inspection and interaction with Electron-based desktop apps. This provides web-level detail for apps built on Chromium.

**Related**: [commands.md](commands.md) for full command reference, [snapshot-refs.md](snapshot-refs.md) for ref system details.

## Prerequisites

CDP mode requires [agent-browser](https://github.com/vercel-labs/agent-browser) installed as a subprocess dependency:

```bash
npm install -g agent-browser
```

You **never call agent-browser directly**. The daemon uses it internally to communicate with Electron apps via CDP. All commands go through `agent-computer` — the daemon handles routing transparently. If agent-browser is not installed, the daemon falls back to native macOS accessibility (which works but provides less detail for web content inside Electron apps).

## What is CDP Mode?

Many popular desktop apps are built with Electron (VS Code, Slack, Discord, Spotify, Notion, etc.). These apps run Chromium internally. CDP mode connects to the app's Chromium instance, enabling:

- **Richer snapshots**: Web-level DOM inspection instead of macOS accessibility tree
- **CSS selectors**: Scope snapshots with `-s "#selector"`
- **Better text extraction**: Access to full DOM text content
- **More reliable interaction**: CDP-based clicking instead of coordinate-based CGEvent

## Usage

### Launch with CDP

```bash
# Launch an Electron app with CDP debugging enabled
agent-computer open "Spotify" --with-cdp
agent-computer wait 2000
agent-computer snapshot -i
```

The `--with-cdp` flag:
1. Launches the app with remote debugging enabled on an available port
2. The daemon connects to the CDP endpoint via agent-browser (subprocess)
3. Snapshots merge native AX (window chrome, menus) with web content (CDP)
4. All refs are unified — you interact with them identically

### Snapshot with CSS Selector Scope

```bash
# Scope snapshot to a specific part of the app
agent-computer snapshot -i -s "#main-content"
agent-computer snapshot -i -s ".sidebar"
```

## Known Electron Apps

These common desktop apps are Electron-based and support CDP mode:

| App | Bundle ID | Notes |
|---|---|---|
| VS Code | `com.microsoft.VSCode` | Rich CDP support |
| Slack | `com.tinyspeck.slackmacgap` | Good CDP support |
| Discord | `com.hnc.Discord` | Good CDP support |
| Spotify | `com.spotify.client` | CEF-based (similar to Electron) |
| Notion | `notion.id` | Good CDP support |
| Figma | `com.figma.Desktop` | Good CDP support |
| 1Password | `com.1password.1password` | Electron-based |

## How It Works

```
agent-computer open "Slack" --with-cdp
         │
         ▼
┌─────────────────────┐
│  Daemon detects       │
│  Electron/CEF app     │
│  Launches with        │
│  --remote-debugging   │
│  -port=<port>         │
└─────────┬─────────────┘
          │
          ▼
┌──────────────────────────────┐
│  Daemon's BrowserBridge       │
│  shells out to agent-browser  │
│  (subprocess) with            │
│  --session <name> --cdp <port>│
│                               │
│  Snapshot → parsed elements   │
│  Click/Fill → delegated       │
│  Press/Scroll → delegated     │
└──────────────────────────────┘
```

When CDP is active:
- The daemon calls `agent-browser --session <name> --cdp <port> snapshot -i` and parses the output
- **Snapshots** merge native AX (title bar, menus) with CDP web content — unified into one ref map
- **Clicks** on CDP-sourced refs are delegated to agent-browser (more reliable than coordinate CGEvent)
- **Refs** carry metadata tracking their source (`AX`, `CDP`, or `Coordinate`) — the daemon routes automatically
- **Press/scroll** on CDP-sourced content is delegated to agent-browser for accuracy
- You never see or interact with agent-browser directly — the daemon handles all routing

## Ref Sources

In CDP mode, refs may come from different sources:

| Source | When | How to tell |
|---|---|---|
| `AX` | Native macOS element | Standard accessibility |
| `CDP` | Web element inside Electron | Enhanced web detail |
| `Coordinate` | Fallback | Position-based |

The daemon handles routing transparently — you interact with all refs the same way (`click @e3`, `fill @e5 "text"`).

## Limitations

- **Not all Electron apps support CDP**: Some apps disable remote debugging or use custom Chromium builds
- **CDP mode launches a fresh instance**: Existing app state (logged-in sessions) may not be preserved — the app relaunches with debugging flags
- **Port conflicts**: If the assigned CDP port is in use, the daemon tries alternative ports
- **Background launch**: `--with-cdp` implies `--background` — the app launches hidden to avoid focus stealing. Use `agent-computer open <app>` afterward to bring it to the foreground

## Troubleshooting

### "CDP not available for this app"

The app either:
- Isn't Electron/CEF-based (use native AX mode instead)
- Blocks remote debugging
- Hasn't finished launching (try `agent-computer wait 3000`)

### CDP snapshot is empty

The app's web content may not have loaded yet:
```bash
agent-computer wait 3000
agent-computer snapshot -i
```

### Mixed AX + CDP refs

This is normal. The daemon unifies refs from both sources. Native chrome (title bar, native menus) comes from AX; web content comes from CDP.
