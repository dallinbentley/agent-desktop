## Context

Spotify demo revealed critical reliability gaps. The agent-browser bridge works but routing, waiting, and command scoping need hardening.

## Goals / Non-Goals

**Goals:**
- Every interaction command works headlessly with `--app`
- CDP port always routes to the correct app
- SPA navigation clicks are reliable with auto-wait
- Snapshots can be scoped to reduce tokens
- Agent can wait for page transitions

**Non-Goals:**
- New interaction types (drag, hover, etc.)
- Multi-window support within a single app
- Performance optimization

## Decisions

### D1: Track CDP port ownership by PID

Current bug: port scan finds a CDP port but doesn't verify which app owns it. Fix: when `open --with-cdp` assigns a port, store `(pid, port, app_name)` in DaemonState. When routing, look up by PID first, only fall back to port scan if no stored mapping.

### D2: Add --app flag to all interaction commands

Currently only `click` has `--app`. Add it to: `fill`, `type`, `press`, `scroll`, `screenshot`, `get`, `wait`. For agent-browser-routed commands, `--app` determines which session/CDP port to use. For AX commands, `--app` determines which PID to target.

### D3: Wait command delegates to agent-browser

```
agent-computer wait @e5                    # Wait for element to appear
agent-computer wait 2000                   # Wait 2 seconds
agent-computer wait --load networkidle     # Wait for network idle (CDP only)
agent-computer wait --load domcontentloaded  # Wait for DOM ready
```

For CDP-sourced contexts, delegates to `agent-browser --session <s> --cdp <port> wait <args>`.
For AX contexts, polls the accessibility tree for element appearance.

### D4: Fix click reliability for SPAs

After every CDP click on a link/navigation element, auto-insert a brief wait (500ms) to let the SPA router update. Optionally, the agent can chain `click @e5 && wait --load networkidle`.

Also: ensure agent-browser's click is called correctly — it handles JS click handlers, event bubbling, and navigation automatically.

### D5: Fix open --with-cdp to force relaunch

Current bug: `open --with-cdp` may activate existing instance without relaunching. Fix:
1. Check if app is running → get PID
2. If running: `osascript -e 'quit app "Name"'` → wait for exit → verify PID gone
3. Launch with `--remote-debugging-port=<port>`
4. Wait for new PID
5. Probe CDP port until ready (up to 10s)
6. Store (new_pid, port, app_name) in state

### D6: Screenshot retry for fresh apps

After `open --with-cdp` relaunches an app, the window may not be rendered for ~1-2s. Fix: if screenshot returns blank/empty (< 1KB PNG), retry up to 3 times with 500ms delays.

### D7: Snapshot scoping via --selector

Pass through to agent-browser's `--selector` flag:
```
agent-computer snapshot -i --app Spotify --selector "#main-content"
```
Becomes: `agent-browser --session spotify --cdp 9371 snapshot -i -s "#main-content"`

Reduces 350+ refs down to relevant section.

## Risks / Trade-offs

**[Auto-wait after clicks may slow things down]** → 500ms is acceptable for agent workflows. Make it configurable via `--no-wait` flag if needed.

**[--app on every command is verbose]** → Consider a `target` or `focus` command that sets a default app for the session. Future improvement.
