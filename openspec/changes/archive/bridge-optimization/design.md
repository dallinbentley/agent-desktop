## Context

The browser bridge shells out to `agent-browser` CLI for every Electron/browser interaction. Each subprocess call costs ~160ms (3ms binary startup + 157ms socket connect/disconnect to agent-browser daemon). The bridge parses text output with regex to extract element refs. Users must install agent-browser separately via npm.

## Goals / Non-Goals

**Goals:**
- Zero-install experience for Electron app support
- Eliminate text regex parsing in favor of structured JSON
- Hide daemon startup latency behind `open --with-cdp`
- Non-blocking bridge calls

**Non-Goals:**
- Reducing the ~160ms per-call subprocess overhead (requires direct socket IPC, which is undocumented/unstable)
- Replacing agent-browser entirely with native CDP
- Supporting agent-browser's streaming/WebSocket API

## Decisions

### D1: Binary bundling strategy

**Approach**: Check for agent-browser in this order:
1. Bundled binary at `~/.agent-computer/bin/agent-browser` (platform-specific)
2. System PATH (existing `which agent-browser`)
3. Common npm/nvm paths (existing fallback)

**Auto-download**: If not found anywhere, download the correct platform binary from agent-browser's npm package (it ships pre-compiled Rust binaries for all platforms). Cache at `~/.agent-computer/bin/`.

The npm package structure is:
```
agent-browser/bin/
  agent-browser-darwin-arm64
  agent-browser-darwin-x64
  agent-browser-linux-x64
  ...
```

We download just the binary, not the full npm package.

### D2: --json output mode

**Current flow**:
```
agent-browser snapshot -i → text output → regex parse → ParsedElement → ElementRef
```

**New flow**:
```
agent-browser --json snapshot -i → JSON → serde deserialize → ElementRef
```

JSON response format:
```json
{
  "success": true,
  "data": {
    "origin": "https://...",
    "refs": { "e1": {"name": "Home", "role": "button"}, ... },
    "snapshot": "- button \"Home\" [ref=e1]\n..."
  }
}
```

We get both structured refs AND the formatted snapshot text in one call. The `snapshot` field is already formatted exactly like our output format.

### D3: Pre-warm sessions

When `open --with-cdp` launches an Electron app, immediately run:
```
agent-browser --session <app> connect <cdp_port>
```

This starts the agent-browser daemon for that session and establishes the CDP connection. The first `snapshot` command then doesn't pay the ~500ms cold start.

### D4: Async subprocess

Switch `std::process::Command` to `tokio::process::Command`. The daemon is already async (tokio runtime). This lets us:
- Not block the main event loop during bridge calls
- Handle native app commands while an Electron command is in flight
- Add timeouts to bridge calls

## Risks / Trade-offs

- **Binary download**: Needs network on first use. Mitigation: clear error message, manual install instructions as fallback.
- **agent-browser version pinning**: We should pin to a known-good version. Mitigation: store version in a constant, check on download.
- **Async complexity**: Converting sync bridge to async touches many call sites. Mitigation: the daemon's handle_* functions are already in async context.
