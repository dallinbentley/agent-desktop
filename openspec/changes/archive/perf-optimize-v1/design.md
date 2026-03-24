## Context

The daemon currently uses `osascript` (AppleScript interpreter subprocess) for app discovery, window title retrieval, and process state checks. Each `osascript` call takes 75-525ms due to process spawn + AppleScript runtime overhead. The input engine has hardcoded `thread::sleep` delays totaling 30ms per keystroke. Cold start polls for the daemon socket at 100ms intervals instead of receiving a signal.

Profiled baselines (warm daemon):
- `status`: 100ms (75ms is osascript window title)
- `get apps`: 530ms (525ms is osascript app enumeration)
- `press`: 42ms (30ms is hardcoded sleeps)
- `click`: 6ms (already fast — ref lookup + AXPress)
- `snapshot`: 1.8s (AX tree walk, separate investigation)
- Cold start: 350ms (100-200ms socket polling)

## Goals / Non-Goals

**Goals:**
- Reduce `status` to <25ms, `get apps` to <10ms, `press` to <12ms
- Reduce cold start to <50ms
- Profile snapshot performance and identify optimization path
- Zero behavioral changes — same output, same API, just faster

**Non-Goals:**
- Persistent CLI connections (rearchitect for later)
- Snapshot caching (needs staleness semantics — separate change)
- Changing the daemon socket IPC protocol

## Decisions

### D1: Replace osascript with NSWorkspace + AX APIs

**osascript calls and their replacements:**

| Current (osascript) | Replacement | Expected speedup |
|---|---|---|
| `get name of every process whose background only is false` | `NSWorkspace.shared.runningApplications` via objc FFI | 525ms → ~2ms |
| `get name of front window of process "X"` | AXUIElement → kAXFocusedWindowAttribute → kAXTitleAttribute | 75ms → ~1ms |
| `set appList to name of every process whose name is "X"` | `kill -0 <pid>` or `NSRunningApplication` lookup | 140ms → ~0.1ms |
| `tell application "X" to quit` | `NSRunningApplication.terminate()` or keep osascript (infrequent) | N/A (only at relaunch) |

**Implementation approach:** Use raw `objc` crate FFI to call `NSWorkspace.shared.runningApplications`. We already depend on `core-foundation` and `core-graphics`; adding `objc` is lightweight. Alternatively, use the `sysinfo` crate for process listing (cross-platform friendly).

**Decision:** Use `objc` FFI for NSWorkspace (most direct, no extra deps beyond what Rust's macOS FFI already provides). For window title, use existing AXUIElement APIs already in `ax_engine.rs`.

### D2: Reduce input sleeps with validation

Current `key_press()`:
```rust
key_down.post(HID);
sleep(20ms);    // → reduce to 2ms
key_up.post(HID);
sleep(10ms);    // → reduce to 1ms
```

The CGEvent spike (S3) measured 1.4ms avg per keystroke end-to-end. The 20ms gap was likely defensive but is excessive. Start with 2ms+1ms, validate with rapid typing test.

**Fallback:** If some apps drop keystrokes, add an `--input-delay` config option, but default to fast.

### D3: Ready-pipe for daemon cold start

```
CLI                              Daemon
 │                                 │
 ├─ create pipe(read_fd, write_fd) │
 ├─ spawn daemon(write_fd)────────▶│
 │                                 ├─ bind socket
 │                                 ├─ write(write_fd, 0x01)
 │◀────────────────────────────────┤
 ├─ read(read_fd) → instant wake   │
 ├─ connect socket                 ├─ accept loop
 └─ send command                   └─
```

Pass the write end of the pipe as an inherited fd. Daemon writes 1 byte after successful socket bind. CLI blocks on the read end — wakes up immediately instead of polling.

**Fallback:** Keep 10ms polling as fallback if pipe fd isn't available (e.g., daemon started manually).

### D4: Snapshot profiling (investigation only)

Add timing instrumentation to the AX tree walk:
- Time per-depth-level
- Count AX attribute queries
- Identify if certain apps have pathologically deep/wide trees
- Output with `--verbose` flag

This informs a future change, not implementation in this one.

## Risks / Trade-offs

- **Input sleep reduction**: Some slow apps (e.g., Electron apps with heavy JS) might need the delay. Mitigation: test with Spotify/Slack/VS Code, add configurable delay if needed.
- **NSWorkspace FFI**: Raw objc calls are unsafe. Mitigation: isolate in a single module with thorough error handling.
- **Ready-pipe portability**: Pipe fd inheritance works on macOS/Linux but not Windows. Acceptable since we're macOS-first.
- **Snapshot investigation**: May reveal that 1.8s is inherent to deep AX trees (macOS AX API limitation). Would need a different strategy (partial tree, caching).
