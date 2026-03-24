import Foundation
import AgentComputerShared

// MARK: - Logging

func log(_ msg: String) {
    let f = DateFormatter()
    f.dateFormat = "HH:mm:ss.SSS"
    print("[\(f.string(from: Date()))] \(msg)")
    fflush(stdout)
}

// MARK: - Signal Handling

var keepRunning = true

func setupSignalHandlers() {
    signal(SIGTERM) { _ in keepRunning = false }
    signal(SIGINT) { _ in keepRunning = false }
}

// MARK: - Stale Socket Handling

func handleStaleSocket(at path: String) {
    guard FileManager.default.fileExists(atPath: path) else { return }
    
    // Try to connect — if it succeeds, another daemon is running
    let testFd = socket(AF_UNIX, SOCK_STREAM, 0)
    guard testFd >= 0 else {
        try? FileManager.default.removeItem(atPath: path)
        return
    }
    defer { close(testFd) }
    
    var addr = sockaddr_un()
    addr.sun_family = sa_family_t(AF_UNIX)
    withUnsafeMutablePointer(to: &addr.sun_path) { ptr in
        ptr.withMemoryRebound(to: Int8.self, capacity: 104) { dest in
            for (i, byte) in path.utf8CString.enumerated() {
                if i >= 104 { break }
                dest[i] = byte
            }
        }
    }
    
    let connected = withUnsafePointer(to: &addr) { ptr in
        ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
            connect(testFd, sockPtr, socklen_t(MemoryLayout<sockaddr_un>.size))
        }
    }
    
    if connected == 0 {
        log("Another daemon is already running on \(path). Exiting.")
        exit(1)
    } else {
        log("Stale socket found, removing...")
        try? FileManager.default.removeItem(atPath: path)
    }
}

// MARK: - Command Dispatch

func handleCommand(json: [String: Any], rawData: Data) -> Response {
    let start = CFAbsoluteTimeGetCurrent()
    let id = json["id"] as? String ?? "unknown"
    let command = json["command"] as? String ?? ""
    
    func elapsed() -> Double {
        (CFAbsoluteTimeGetCurrent() - start) * 1000.0
    }
    
    let decoder = JSONDecoder()
    
    switch command {
    case "snapshot":
        let args: SnapshotArgs
        if let argsJson = json["args"] {
            let argsData = try! JSONSerialization.data(withJSONObject: argsJson)
            args = (try? decoder.decode(SnapshotArgs.self, from: argsData)) ?? SnapshotArgs()
        } else {
            args = SnapshotArgs()
        }
        return handleSnapshot(id: id, args: args, startTime: start)
        
    case "click":
        guard let argsJson = json["args"],
              let argsData = try? JSONSerialization.data(withJSONObject: argsJson),
              let args = try? decoder.decode(ClickArgs.self, from: argsData) else {
            return Response.fail(id: id, error: Errors.invalidCommand("click requires args with ref or x/y"), elapsed: elapsed())
        }
        return handleClick(id: id, args: args, startTime: start)
        
    case "fill":
        guard let argsJson = json["args"],
              let argsData = try? JSONSerialization.data(withJSONObject: argsJson),
              let args = try? decoder.decode(FillArgs.self, from: argsData) else {
            return Response.fail(id: id, error: Errors.invalidCommand("fill requires args with ref and text"), elapsed: elapsed())
        }
        return handleFill(id: id, args: args, startTime: start)
        
    case "type":
        guard let argsJson = json["args"],
              let argsData = try? JSONSerialization.data(withJSONObject: argsJson),
              let args = try? decoder.decode(TypeArgs.self, from: argsData) else {
            return Response.fail(id: id, error: Errors.invalidCommand("type requires args with text"), elapsed: elapsed())
        }
        return handleType(id: id, args: args, startTime: start)
        
    case "press":
        guard let argsJson = json["args"],
              let argsData = try? JSONSerialization.data(withJSONObject: argsJson),
              let args = try? decoder.decode(PressArgs.self, from: argsData) else {
            return Response.fail(id: id, error: Errors.invalidCommand("press requires args with key"), elapsed: elapsed())
        }
        return handlePress(id: id, args: args, startTime: start)
        
    case "scroll":
        guard let argsJson = json["args"],
              let argsData = try? JSONSerialization.data(withJSONObject: argsJson),
              let args = try? decoder.decode(ScrollArgs.self, from: argsData) else {
            return Response.fail(id: id, error: Errors.invalidCommand("scroll requires args with direction"), elapsed: elapsed())
        }
        return handleScroll(id: id, args: args, startTime: start)
        
    case "screenshot":
        let args: ScreenshotArgs
        if let argsJson = json["args"],
           let argsData = try? JSONSerialization.data(withJSONObject: argsJson) {
            args = (try? decoder.decode(ScreenshotArgs.self, from: argsData)) ?? ScreenshotArgs()
        } else {
            args = ScreenshotArgs()
        }
        return handleScreenshot(id: id, args: args, startTime: start)
        
    case "open":
        guard let argsJson = json["args"],
              let argsData = try? JSONSerialization.data(withJSONObject: argsJson),
              let args = try? decoder.decode(OpenArgs.self, from: argsData) else {
            return Response.fail(id: id, error: Errors.invalidCommand("open requires args with target"), elapsed: elapsed())
        }
        return handleOpen(id: id, args: args, startTime: start)
        
    case "get":
        guard let argsJson = json["args"],
              let argsData = try? JSONSerialization.data(withJSONObject: argsJson),
              let args = try? decoder.decode(GetArgs.self, from: argsData) else {
            return Response.fail(id: id, error: Errors.invalidCommand("get requires args with what"), elapsed: elapsed())
        }
        return handleGet(id: id, args: args, startTime: start)
        
    case "status":
        return handleStatus(id: id, startTime: start)
        
    default:
        return Response.fail(id: id, error: Errors.invalidCommand("Unknown command: '\(command)'"), elapsed: elapsed())
    }
}

// MARK: - Client Handling

func handleClient(_ fd: Int32) {
    // Read data in a loop to handle multiple messages (newline-delimited)
    var buffer = [UInt8](repeating: 0, count: 65536)
    var accumulated = Data()
    
    // Set client socket to blocking with a read timeout
    var tv = timeval(tv_sec: 5, tv_usec: 0)
    setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, socklen_t(MemoryLayout<timeval>.size))
    
    while keepRunning {
        let bytesRead = read(fd, &buffer, buffer.count)
        if bytesRead <= 0 { break }
        accumulated.append(contentsOf: buffer[0..<bytesRead])
        
        // Process complete lines
        while let newlineIndex = accumulated.firstIndex(of: UInt8(ascii: "\n")) {
            let lineData = accumulated[accumulated.startIndex..<newlineIndex]
            accumulated = Data(accumulated[accumulated.index(after: newlineIndex)...])
            
            guard !lineData.isEmpty else { continue }
            
            let response: Response
            if let json = try? JSONSerialization.jsonObject(with: lineData) as? [String: Any],
               let _ = json["command"] as? String {
                response = handleCommand(json: json, rawData: Data(lineData))
            } else {
                response = Response.fail(
                    id: "unknown",
                    error: Errors.invalidCommand("Malformed JSON request"),
                    elapsed: 0
                )
            }
            
            // Send response
            let encoder = JSONEncoder()
            encoder.outputFormatting = [] // compact
            if var respData = try? encoder.encode(response) {
                respData.append(UInt8(ascii: "\n"))
                respData.withUnsafeBytes { ptr in
                    _ = write(fd, ptr.baseAddress!, respData.count)
                }
            }
        }
        
        // If we still have data with no newline and it looks complete, try processing it
        if !accumulated.isEmpty && !accumulated.contains(UInt8(ascii: "\n")) {
            // Check if this is a complete JSON object (no trailing newline)
            if let json = try? JSONSerialization.jsonObject(with: accumulated) as? [String: Any],
               let _ = json["command"] as? String {
                let response = handleCommand(json: json, rawData: accumulated)
                accumulated = Data()
                
                let encoder = JSONEncoder()
                if var respData = try? encoder.encode(response) {
                    respData.append(UInt8(ascii: "\n"))
                    respData.withUnsafeBytes { ptr in
                        _ = write(fd, ptr.baseAddress!, respData.count)
                    }
                }
            }
        }
    }
}

// MARK: - Main Entry Point

log("agent-computer-daemon starting...")
log("PID: \(ProcessInfo.processInfo.processIdentifier)")

let socketDirPath = daemonSocketDir.path
let socketFilePath = daemonSocketPath.path

// Create socket directory
try? FileManager.default.createDirectory(atPath: socketDirPath, withIntermediateDirectories: true)

// Handle stale socket
handleStaleSocket(at: socketFilePath)

// Create Unix domain socket
let serverFd = socket(AF_UNIX, SOCK_STREAM, 0)
guard serverFd >= 0 else {
    log("ERROR: Failed to create socket: \(String(cString: strerror(errno)))")
    exit(1)
}

var addr = sockaddr_un()
addr.sun_family = sa_family_t(AF_UNIX)
withUnsafeMutablePointer(to: &addr.sun_path) { ptr in
    ptr.withMemoryRebound(to: Int8.self, capacity: 104) { dest in
        for (i, byte) in socketFilePath.utf8CString.enumerated() {
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
    try? FileManager.default.removeItem(atPath: socketFilePath)
    exit(1)
}

log("Listening on \(socketFilePath)")

// Set socket to non-blocking for graceful shutdown
let flags = fcntl(serverFd, F_GETFL)
_ = fcntl(serverFd, F_SETFL, flags | O_NONBLOCK)

setupSignalHandlers()

// Accept loop
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
    
    usleep(10_000) // 10ms to avoid busy-waiting
}

// Cleanup
log("Shutting down...")
close(serverFd)
try? FileManager.default.removeItem(atPath: socketFilePath)
log("Daemon exited cleanly")
exit(0)
