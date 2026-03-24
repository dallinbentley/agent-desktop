import Foundation

// MARK: - Spike S5: Daemon Auto-Start Reliability
// Minimal daemon that listens on a Unix domain socket, accepts JSON commands, and echoes responses.
// Supports:
//   --daemon     : Run as daemon (listen on socket)
//   --test       : Run the full test suite (spawn daemon, test, kill)
//   --connect    : Connect to running daemon and send a test command

let socketDir = NSHomeDirectory() + "/.agent-computer"
let socketPath = socketDir + "/test-daemon.sock"

// MARK: - Signal Handling

var keepRunning = true

func setupSignalHandlers() {
    signal(SIGTERM) { _ in
        keepRunning = false
    }
    signal(SIGINT) { _ in
        keepRunning = false
    }
}

func cleanupSocket() {
    try? FileManager.default.removeItem(atPath: socketPath)
    log("Socket cleaned up: \(socketPath)")
}

func log(_ msg: String) {
    let f = DateFormatter()
    f.dateFormat = "HH:mm:ss.SSS"
    print("[\(f.string(from: Date()))] \(msg)")
    fflush(stdout)
}

// MARK: - Daemon Mode

func runDaemon() {
    log("Starting daemon...")
    log("PID: \(ProcessInfo.processInfo.processIdentifier)")

    // Create socket directory
    try? FileManager.default.createDirectory(atPath: socketDir, withIntermediateDirectories: true)

    // Handle stale socket
    if FileManager.default.fileExists(atPath: socketPath) {
        log("Stale socket found, removing...")
        try? FileManager.default.removeItem(atPath: socketPath)
    }

    // Create Unix domain socket
    let serverFd = socket(AF_UNIX, SOCK_STREAM, 0)
    guard serverFd >= 0 else {
        log("ERROR: Failed to create socket: \(String(cString: strerror(errno)))")
        exit(1)
    }

    var addr = sockaddr_un()
    addr.sun_family = sa_family_t(AF_UNIX)
    let pathBytes = socketPath.utf8CString
    withUnsafeMutablePointer(to: &addr.sun_path) { ptr in
        let bound = ptr.withMemoryRebound(to: Int8.self, capacity: Int(104)) { dest in
            for (i, byte) in pathBytes.enumerated() {
                if i >= 104 { break }
                dest[i] = byte
            }
        }
    }

    let bindResult = withUnsafePointer(to: &addr) { ptr in
        ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
            bind(serverFd, sockPtr, socklen_t(MemoryLayout<sockaddr_un>.size))
        }
    }
    guard bindResult == 0 else {
        log("ERROR: Failed to bind: \(String(cString: strerror(errno)))")
        close(serverFd)
        exit(1)
    }

    guard listen(serverFd, 5) == 0 else {
        log("ERROR: Failed to listen: \(String(cString: strerror(errno)))")
        close(serverFd)
        cleanupSocket()
        exit(1)
    }

    log("Listening on \(socketPath)")

    // Set socket to non-blocking for graceful shutdown
    let flags = fcntl(serverFd, F_GETFL)
    fcntl(serverFd, F_SETFL, flags | O_NONBLOCK)

    setupSignalHandlers()

    while keepRunning {
        var clientAddr = sockaddr_un()
        var clientLen = socklen_t(MemoryLayout<sockaddr_un>.size)

        let clientFd = withUnsafeMutablePointer(to: &clientAddr) { ptr in
            ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
                accept(serverFd, sockPtr, &clientLen)
            }
        }

        if clientFd >= 0 {
            log("Client connected (fd=\(clientFd))")
            handleClient(clientFd)
            close(clientFd)
            log("Client disconnected")
        } else if errno != EAGAIN && errno != EWOULDBLOCK {
            if keepRunning {
                log("Accept error: \(String(cString: strerror(errno)))")
            }
        }

        // Small sleep to avoid busy-waiting
        usleep(10_000) // 10ms
    }

    log("Shutting down...")
    close(serverFd)
    cleanupSocket()
    log("Daemon exited cleanly")
    exit(0)
}

func handleClient(_ fd: Int32) {
    var buffer = [UInt8](repeating: 0, count: 4096)
    let bytesRead = read(fd, &buffer, buffer.count - 1)

    guard bytesRead > 0 else {
        log("Client sent no data")
        return
    }

    let input = String(bytes: buffer[0..<bytesRead], encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
    log("Received: \(input)")

    // Parse JSON
    guard let data = input.data(using: .utf8),
          let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
          let command = json["command"] as? String else {
        let errorResp = "{\"error\": \"invalid JSON\", \"received\": \"\(input)\"}\n"
        write(fd, errorResp, errorResp.utf8.count)
        return
    }

    // Build response
    let response: [String: Any] = [
        "status": "ok",
        "command": command,
        "pid": ProcessInfo.processInfo.processIdentifier,
        "timestamp": ISO8601DateFormatter().string(from: Date()),
        "echo": json
    ]

    if let respData = try? JSONSerialization.data(withJSONObject: response),
       var respStr = String(data: respData, encoding: .utf8) {
        respStr += "\n"
        write(fd, respStr, respStr.utf8.count)
        log("Sent response for command: \(command)")
    }
}

// MARK: - Client Mode

func connectAndSend(_ message: String) -> String? {
    let clientFd = socket(AF_UNIX, SOCK_STREAM, 0)
    guard clientFd >= 0 else {
        log("ERROR: Failed to create client socket")
        return nil
    }
    defer { close(clientFd) }

    var addr = sockaddr_un()
    addr.sun_family = sa_family_t(AF_UNIX)
    let pathBytes = socketPath.utf8CString
    withUnsafeMutablePointer(to: &addr.sun_path) { ptr in
        ptr.withMemoryRebound(to: Int8.self, capacity: 104) { dest in
            for (i, byte) in pathBytes.enumerated() {
                if i >= 104 { break }
                dest[i] = byte
            }
        }
    }

    let connectResult = withUnsafePointer(to: &addr) { ptr in
        ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
            connect(clientFd, sockPtr, socklen_t(MemoryLayout<sockaddr_un>.size))
        }
    }
    guard connectResult == 0 else {
        log("ERROR: Failed to connect: \(String(cString: strerror(errno)))")
        return nil
    }

    // Send
    let msgWithNewline = message + "\n"
    write(clientFd, msgWithNewline, msgWithNewline.utf8.count)

    // Read response
    var buffer = [UInt8](repeating: 0, count: 4096)
    let bytesRead = read(clientFd, &buffer, buffer.count - 1)
    guard bytesRead > 0 else { return nil }

    return String(bytes: buffer[0..<bytesRead], encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines)
}

// MARK: - Test Mode

func runTests() {
    log("=== SPIKE S5: DAEMON AUTO-START TEST SUITE ===")
    log("")

    var passed = 0
    var failed = 0

    func assert(_ condition: Bool, _ msg: String) {
        if condition {
            log("✅ PASS: \(msg)")
            passed += 1
        } else {
            log("❌ FAIL: \(msg)")
            failed += 1
        }
    }

    // Clean up any existing socket/daemon
    if FileManager.default.fileExists(atPath: socketPath) {
        log("Cleaning up existing socket...")
        try? FileManager.default.removeItem(atPath: socketPath)
    }

    // Get our own executable path
    let execPath = CommandLine.arguments[0]
    log("Executable: \(execPath)")

    // Test 1: Spawn daemon as background process
    log("")
    log("--- Test 1: Spawn daemon ---")
    let daemonProcess = Process()
    daemonProcess.executableURL = URL(fileURLWithPath: execPath)
    daemonProcess.arguments = ["--daemon"]
    daemonProcess.standardOutput = FileHandle.nullDevice
    daemonProcess.standardError = FileHandle.nullDevice

    let spawnStart = CFAbsoluteTimeGetCurrent()
    do {
        try daemonProcess.run()
    } catch {
        log("ERROR: Failed to spawn daemon: \(error)")
        exit(1)
    }
    let daemonPID = daemonProcess.processIdentifier
    log("Daemon spawned with PID: \(daemonPID)")

    // Test 2: Wait for socket to appear (poll with timeout)
    log("")
    log("--- Test 2: Socket appearance ---")
    var socketAppeared = false
    for i in 0..<50 { // 5 seconds max
        if FileManager.default.fileExists(atPath: socketPath) {
            let elapsed = (CFAbsoluteTimeGetCurrent() - spawnStart) * 1000.0
            log("Socket appeared after \(String(format: "%.0f", elapsed)) ms")
            socketAppeared = true
            break
        }
        usleep(100_000) // 100ms
    }
    assert(socketAppeared, "Socket file appeared at \(socketPath)")

    // Test 3: Verify daemon visible in ps
    log("")
    log("--- Test 3: Process visibility ---")
    let psProcess = Process()
    psProcess.executableURL = URL(fileURLWithPath: "/bin/ps")
    psProcess.arguments = ["-p", "\(daemonPID)", "-o", "pid,comm"]
    let psPipe = Pipe()
    psProcess.standardOutput = psPipe
    try? psProcess.run()
    psProcess.waitUntilExit()
    let psOutput = String(data: psPipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
    let isVisible = psOutput.contains("\(daemonPID)")
    assert(isVisible, "Daemon visible in ps (PID \(daemonPID))")
    log("ps output: \(psOutput.trimmingCharacters(in: .whitespacesAndNewlines))")

    // Test 4: Connect and send command
    log("")
    log("--- Test 4: First connection ---")
    let cmd1 = "{\"command\": \"test1\", \"data\": \"hello\"}"
    let resp1 = connectAndSend(cmd1)
    assert(resp1 != nil, "Got response from daemon")
    if let r = resp1 {
        log("Response: \(r)")
        let hasOk = r.contains("\"status\":\"ok\"") || r.contains("\"status\" : \"ok\"")
        assert(hasOk, "Response contains status=ok")
    }

    // Test 5: Daemon stays alive after client disconnects
    log("")
    log("--- Test 5: Daemon survives client disconnect ---")
    usleep(500_000) // Wait 500ms
    assert(daemonProcess.isRunning, "Daemon still running after first client disconnected")

    // Test 6: Second connection on new socket
    log("")
    log("--- Test 6: Second connection ---")
    let cmd2 = "{\"command\": \"test2\", \"data\": \"world\"}"
    let resp2 = connectAndSend(cmd2)
    assert(resp2 != nil, "Got response on second connection")
    if let r = resp2 {
        log("Response: \(r)")
    }

    // Test 7: Kill daemon via SIGTERM
    log("")
    log("--- Test 7: Clean shutdown via SIGTERM ---")
    kill(daemonPID, SIGTERM)
    usleep(500_000) // Wait for shutdown
    daemonProcess.waitUntilExit()
    let exitStatus = daemonProcess.terminationStatus
    log("Daemon exit status: \(exitStatus)")
    assert(!daemonProcess.isRunning, "Daemon stopped after SIGTERM")

    // Test 8: Socket cleaned up
    log("")
    log("--- Test 8: Socket cleanup ---")
    usleep(200_000)
    let socketCleaned = !FileManager.default.fileExists(atPath: socketPath)
    assert(socketCleaned, "Socket file removed after clean shutdown")

    // Test 9: Stale socket handling
    log("")
    log("--- Test 9: Stale socket handling ---")
    // Create a stale socket file
    try? FileManager.default.createDirectory(atPath: socketDir, withIntermediateDirectories: true)
    FileManager.default.createFile(atPath: socketPath, contents: "stale".data(using: .utf8))
    assert(FileManager.default.fileExists(atPath: socketPath), "Created fake stale socket")

    // Start daemon - it should handle the stale socket
    let daemon2 = Process()
    daemon2.executableURL = URL(fileURLWithPath: execPath)
    daemon2.arguments = ["--daemon"]
    daemon2.standardOutput = FileHandle.nullDevice
    daemon2.standardError = FileHandle.nullDevice
    try? daemon2.run()

    // Wait for socket
    var socket2Appeared = false
    for _ in 0..<50 {
        // Try to connect to verify it's a real socket
        if let _ = connectAndSend("{\"command\": \"stale_test\"}") {
            socket2Appeared = true
            break
        }
        usleep(100_000)
    }
    assert(socket2Appeared, "Daemon started successfully despite stale socket")

    // Cleanup
    kill(daemon2.processIdentifier, SIGTERM)
    daemon2.waitUntilExit()
    usleep(200_000)

    // Test 10: Connection to non-existent daemon
    log("")
    log("--- Test 10: Connection to non-existent daemon ---")
    try? FileManager.default.removeItem(atPath: socketPath)
    let resp3 = connectAndSend("{\"command\": \"should_fail\"}")
    assert(resp3 == nil, "Connection correctly fails when no daemon running")

    // Summary
    log("")
    log("=== TEST RESULTS ===")
    log("Passed: \(passed)")
    log("Failed: \(failed)")
    log("Total:  \(passed + failed)")

    exit(failed > 0 ? 1 : 0)
}

// MARK: - Main

let args = CommandLine.arguments
if args.contains("--daemon") {
    runDaemon()
} else if args.contains("--test") {
    runTests()
} else if args.contains("--connect") {
    let msg = args.last != "--connect" ? args.last! : "{\"command\": \"ping\"}"
    if let resp = connectAndSend(msg) {
        print(resp)
    } else {
        log("Failed to connect to daemon")
        exit(1)
    }
} else {
    print("""
    Usage:
      spike-daemon --daemon    Run as daemon (listen on Unix socket)
      spike-daemon --test      Run full test suite
      spike-daemon --connect   Connect and send a test command
    """)
}
