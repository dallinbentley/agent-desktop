## Why

The daemon's IPC round-trip is dominated by subprocess calls to `osascript` (75-525ms per command) and hardcoded `thread::sleep` delays in input simulation (30ms per keystroke). Cold start wastes 100-200ms polling for the daemon socket at 100ms intervals. These add up: `status` takes 100ms, `get apps` takes 530ms, `press` takes 42ms, and cold start takes 350ms — all 5-20× slower than they need to be.

## What Changes

- **Replace all `osascript` calls with native APIs**: Use `NSWorkspace.runningApplications` for app listing (~2ms vs 525ms), AX `kAXTitleAttribute` for window titles (~1ms vs 75ms), and `kill -0` / AX for process detection (~0.1ms vs 140ms)
- **Remove hardcoded sleeps in input.rs**: Reduce 20ms key-down→key-up gap and 10ms post-keystroke sleep to 2ms+1ms (validated by CGEvent spike at 1.4ms avg)
- **Replace socket polling with ready-pipe**: Daemon writes a byte to an inherited pipe fd after binding the socket; CLI blocks on pipe read instead of polling at 100ms intervals
- **Profile and optimize AX tree walk**: Snapshot takes 1.8s — investigate depth, redundant attribute queries, and caching opportunities

## Capabilities

### New Capabilities

- `daemon-ready-signal`: Pipe-based daemon startup signaling (replaces socket polling)

### Modified Capabilities

- `daemon-ipc`: Cold start uses ready-pipe instead of 100ms polling loop
- `input-simulation`: Reduced inter-event delays (20ms→2ms key gap, 10ms→1ms post-key)
- `app-management`: All osascript calls replaced with native NSWorkspace/AX APIs
- `snapshot`: Performance profiling and optimization of AX tree walk

## Impact

- **Files**: `input.rs` (sleep reduction), `app.rs` (osascript→native), `connection.rs` (ready-pipe), `main.rs` (pipe setup), `ax_engine.rs` (snapshot profiling)
- **Dependencies**: May need `objc2-app-kit` or raw `NSWorkspace` FFI for running apps API
- **Performance targets**: status <25ms, click <10ms, press <12ms, get apps <10ms, cold start <50ms
- **Risk**: Reducing input sleeps too aggressively could cause dropped keystrokes on slow apps — need validation
