# Spike S5: Daemon Auto-Start Reliability — Results

**Date:** 2026-03-24  
**macOS:** 26.3.1 (Build 25D771280a)  
**Hardware:** Apple M3 Max  

## Test Results: 11/11 Passed ✅

## 1. Working Daemon Spawn Approach

### Unix Domain Socket Daemon (Swift)

The daemon uses a standard POSIX Unix domain socket:

```swift
// Create socket
let serverFd = socket(AF_UNIX, SOCK_STREAM, 0)

// Bind to path
var addr = sockaddr_un()
addr.sun_family = sa_family_t(AF_UNIX)
// ... set addr.sun_path to ~/.agent-desktop/test-daemon.sock

bind(serverFd, &addr, ...)
listen(serverFd, 5)

// Non-blocking accept loop with SIGTERM handler
fcntl(serverFd, F_SETFL, flags | O_NONBLOCK)
while keepRunning {
    let clientFd = accept(serverFd, ...)
    if clientFd >= 0 { handleClient(clientFd) }
    usleep(10_000)  // 10ms poll interval
}
```

### Spawning from client (Swift)

```swift
let daemon = Process()
daemon.executableURL = URL(fileURLWithPath: "/path/to/spike-daemon")
daemon.arguments = ["--daemon"]
daemon.standardOutput = FileHandle.nullDevice  // Detach from terminal
daemon.standardError = FileHandle.nullDevice
try daemon.run()
// daemon.processIdentifier gives the PID
```

### Spawning from shell

```bash
spike-daemon --daemon &
# Or for full detach:
nohup spike-daemon --daemon > /dev/null 2>&1 &
```

## 2. Socket Lifecycle

| Phase | Timing | Notes |
|-------|--------|-------|
| Socket creation | ~100 ms from spawn | Includes process startup + socket bind |
| Socket ready (first connection) | ~100 ms | Same as creation — immediate listen |
| Client connect + round-trip | < 5 ms | Fast IPC via Unix socket |
| Clean shutdown (SIGTERM) | < 500 ms | Socket file removed by signal handler |
| Socket detection (poll) | FileManager.default.fileExists() | Reliable, ~0.1 ms per check |

### Socket Path
- **Location:** `~/.agent-desktop/test-daemon.sock`
- **Directory created** automatically if missing
- **Socket file** is a regular file from `stat()` perspective

## 3. Stale Socket Handling ✅

**Scenario:** A previous daemon crashed without cleaning up, leaving a stale socket file.

**Solution:** The daemon checks for an existing socket file on startup and removes it:

```swift
if FileManager.default.fileExists(atPath: socketPath) {
    log("Stale socket found, removing...")
    try? FileManager.default.removeItem(atPath: socketPath)
}
```

**Verified:** New daemon starts successfully after removing stale socket.

**Alternative detection:** Could also try connecting to the existing socket — if connection fails, it's stale.

## 4. Signal Handling

```swift
signal(SIGTERM) { _ in keepRunning = false }
signal(SIGINT) { _ in keepRunning = false }
```

- **SIGTERM:** Clean shutdown — socket removed, exit code 0 ✅
- **SIGINT (Ctrl-C):** Same clean shutdown ✅
- **SIGKILL:** No handler possible — socket file left behind (stale socket handling covers this)
- **Crash:** Same as SIGKILL — stale socket handling needed

## 5. Process Visibility

- Daemon is visible in `ps -p <PID>` ✅
- Process name shows full path: `/path/to/spike-daemon`
- Can be monitored with standard Unix tools

## 6. Terminal Emulator Notes

Testing was done from Ghostty terminal. The daemon:
- **Survives client disconnect** ✅ (verified with explicit test)
- **Survives terminal close** when spawned with `nohup` or as a `Process()` from another process
- **NOT tested with:** iTerm2, Terminal.app, VS Code terminal — but standard POSIX behavior means no issues expected

## 7. Recommendation: Spawn-on-First-Use vs launchd

### Recommended: Spawn-on-First-Use ✅

**Approach:**
1. CLI command checks if socket exists at `~/.agent-desktop/daemon.sock`
2. Try to connect — if successful, daemon is running
3. If connection fails (or socket doesn't exist), spawn daemon as background process
4. Wait for socket to appear (poll with ~100ms intervals, 5s timeout)
5. Connect and proceed

**Why spawn-on-first-use over launchd:**

| Factor | Spawn-on-first-use | launchd plist |
|--------|-------------------|---------------|
| Setup complexity | None | Requires plist installation |
| Permissions | Same as user | Same as user |
| Auto-start on login | No (not needed) | Yes (unnecessary) |
| Resource usage | Only when needed | Always running or socket-activated |
| Cross-platform | Works anywhere | macOS only |
| Version management | Always runs matching CLI version | May run stale version after update |
| Uninstall | Kill process, done | Must remove plist |
| Debugging | Simple `ps`, `kill` | `launchctl` commands |

**Key advantages:**
- Zero setup/installation required
- Daemon binary is always the same version as the CLI
- No root/admin permissions needed
- Natural lifecycle: starts when needed, killed when done
- Clean state management via socket file

### When launchd makes sense (for later):
- If the daemon needs to be a persistent system service
- If socket-activation is needed (macOS launches daemon on first connection)
- If crash recovery with automatic restart is needed
- If running as a different user is needed

### Implementation plan:
```
agent-desktop connect:
  1. socket = ~/.agent-desktop/daemon.sock
  2. if canConnect(socket) → return connection
  3. removeStaleSocket(socket)
  4. spawn("agent-desktop", "--daemon")
  5. poll(socket, timeout: 5s)
  6. connect(socket)
  7. return connection
```

## 8. Performance Summary

| Operation | Time |
|-----------|------|
| Daemon startup to socket ready | ~100 ms |
| Client connect + JSON round-trip | < 5 ms |
| Clean shutdown | < 500 ms |
| Socket detection (file exists) | < 0.1 ms |
| Stale socket recovery | ~100 ms (remove + restart) |
