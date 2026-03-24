## Context

We built a native Rust CDP engine but it's inferior to agent-browser in every way — agent-browser handles CDP connection management, tab discovery, snapshot/ref systems, cursor-interactive detection, and multi-browser support (Chrome, Edge, Brave, Arc + experimental Safari). It's installed at `/Users/dallin/.nvm/versions/node/v24.14.0/bin/agent-browser` v0.22.1.

Testing confirmed agent-browser produces excellent output for Electron apps:
```
agent-browser --cdp 9371 snapshot -i   →   200+ labeled refs for Spotify
agent-browser --cdp 9229 snapshot -i   →   Full Slack UI with channels, messages
```

Key agent-browser features we leverage:
- `--cdp <port>` — connect to any CDP endpoint
- `--auto-connect` — discover running Chrome via DevToolsActivePort
- `snapshot -i` — interactive-only accessibility tree with @refs
- `snapshot --json` — machine-readable output
- `click @e5`, `fill @e3 "text"`, `type @e3 "text"`, `press Enter` — all headless via CDP
- Session persistence via daemon (Unix socket, stays alive between commands)

## Goals / Non-Goals

**Goals:**
- Replace native CDP engine with agent-browser bridge — zero CDP code in our codebase
- Fully headless `--app` mode — user is never interrupted
- Unified @ref experience — agent sees @e1 whether AX or agent-browser sourced
- Type-safe bridge with proper error handling and output parsing
- Manage agent-browser lifecycle alongside our daemon

**Non-Goals:**
- Bundling agent-browser binary into our release (future — for now require it installed)
- Supporting agent-browser features we don't need (PDF, iOS simulator, file upload)
- Writing our own CDP client
- Safari WebDriver support

## Decisions

### D1: Bridge via CLI subprocess, not library import

**Choice**: Shell out to `agent-browser` CLI via `std::process::Command`, parse stdout.

**Why**: agent-browser is a Rust+Node.js hybrid tool. There's no stable Rust library API. The CLI is the stable interface — it's what AI agents use, it's documented, it has `--json` output. Subprocess overhead is negligible (<10ms per call) compared to CDP operations (~50-200ms).

**Alternative considered**: Import agent-browser's daemon protocol (Unix socket JSON). Rejected — internal protocol, not stable, tight coupling.

### D2: agent-browser manages its own daemon/sessions

**Choice**: Let agent-browser manage its own daemon process and sessions. We don't start/stop it — we just call CLI commands and let its daemon auto-start.

**Why**: agent-browser already handles daemon lifecycle (auto-start, session persistence, clean shutdown). Trying to manage it ourselves adds complexity for no benefit.

**Session management**: Use `--session <app-name>` flag to isolate connections per app:
```
agent-browser --session spotify --cdp 9371 snapshot -i
agent-browser --session slack --cdp 9229 click @e5
```

### D3: Parse agent-browser text output, not JSON

**Choice**: Parse the human-readable snapshot text output (indented tree with `[ref=eN]`), not `--json`.

**Why**: The text format is simpler to parse — each line is `- role "name" [ref=eN] [attrs]` with indentation for hierarchy. The JSON format is more complex and may change. Text format is the primary interface AI agents use, so it's the most stable.

**Parsing regex**: `\[ref=(e\d+)\]` to extract refs, role is first word, label is in quotes.

### D4: Unified RefMap with agent-browser ref passthrough

**Choice**: When agent takes a snapshot of a browser/Electron app, we:
1. Take AX snapshot of browser chrome (address bar, tabs — stop at AXWebArea)
2. Call `agent-browser --cdp <port> snapshot -i` for web content
3. Merge into unified @e1... numbering
4. Store original agent-browser ref (e.g., "e32") in the ElementRef

When agent clicks @e7 (web-sourced):
- Look up stored agent-browser ref "e32"
- Shell out: `agent-browser --session <app> --cdp <port> click @e32`

**Why**: Transparent to the AI agent. It sees @e1-@eN, clicks any of them, we route correctly.

### D5: Headless mode is default for --app

**Choice**: When `--app <name>` is specified:
- Native apps: AX-first headless actions (AXPress, AXSetValue) — already implemented
- Browser/Electron apps: agent-browser CDP — inherently headless
- Coordinate fallback: only with explicit `--foreground` flag (brings app to front)

**Why**: The user asked for this explicitly. agent-browser never steals focus. AX actions never steal focus. Only CGEvent coordinate clicks require foreground.

### D6: Detect agent-browser at startup, graceful fallback

**Choice**: On daemon startup, check for agent-browser in PATH. Cache the binary path. If not found:
- Web/Electron snapshots return warning: "agent-browser not found. Install with: npm install -g agent-browser"
- Fall back to AX-only (which is sparse for Electron) + screenshot fallback
- Never crash or block

**Why**: Graceful degradation. The tool works for native apps regardless. Web support is additive.

### D7: CDP port management stays in our daemon

**Choice**: We still handle `open --with-cdp` (detecting Electron, relaunching with `--remote-debugging-port`). The assigned port is passed to agent-browser via `--cdp <port>`.

**Why**: agent-browser doesn't know about macOS app bundles or Electron detection. That's our domain. We handle app lifecycle, agent-browser handles web interaction.

## Risks / Trade-offs

**[agent-browser output format changes]** → Pin to known version range. Text format has been stable. Use lenient parsing that handles unknown attributes gracefully.

**[agent-browser not installed]** → Graceful fallback to AX-only + screenshots. Clear error message with install instructions.

**[Subprocess latency]** → ~10ms overhead per call. Negligible compared to CDP operations (50-200ms) and AX traversal (50-400ms).

**[Two daemon processes]** → Our daemon + agent-browser daemon. Memory overhead is small. agent-browser daemon auto-exits after inactivity.

**[Session isolation]** → Using `--session <app-name>` per app prevents cross-contamination between Spotify, Slack, Chrome, etc.
