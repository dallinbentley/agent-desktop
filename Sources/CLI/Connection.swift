import Foundation
import AgentComputerShared

/// Unix domain socket client for communicating with the daemon.
enum Connection {
    
    /// Send a request to the daemon and receive a response.
    /// Auto-starts the daemon if it's not running.
    static func send(_ request: Request, verbose: Bool = false) throws -> Response {
        let socketPath = daemonSocketPath.path
        
        // Try to connect, auto-start daemon if needed
        let fd = try connectOrStartDaemon(socketPath: socketPath, verbose: verbose)
        defer { close(fd) }
        
        // Encode request as JSON line
        let encoder = JSONEncoder()
        let data = try encoder.encode(request)
        guard var jsonLine = String(data: data, encoding: .utf8) else {
            throw ConnectionError.encodingFailed
        }
        jsonLine += "\n"
        
        if verbose {
            FileHandle.standardError.write(Data("[verbose] Sending: \(jsonLine.trimmingCharacters(in: .newlines))\n".utf8))
        }
        
        // Send
        let sendData = Array(jsonLine.utf8)
        let written = Darwin.write(fd, sendData, sendData.count)
        guard written == sendData.count else {
            throw ConnectionError.writeFailed
        }
        
        // Read response line
        let responseLine = try readLine(fd: fd)
        
        if verbose {
            FileHandle.standardError.write(Data("[verbose] Received: \(responseLine)\n".utf8))
        }
        
        // Decode response
        guard let responseData = responseLine.data(using: .utf8) else {
            throw ConnectionError.decodingFailed
        }
        let decoder = JSONDecoder()
        let response = try decoder.decode(Response.self, from: responseData)
        return response
    }
    
    /// Connect to existing socket, or start daemon and wait for it.
    private static func connectOrStartDaemon(socketPath: String, verbose: Bool) throws -> Int32 {
        // Try connecting first
        if let fd = tryConnect(socketPath: socketPath) {
            return fd
        }
        
        // Socket not available — start daemon
        FileHandle.standardError.write(Data("Starting agent-computer daemon...\n".utf8))
        try spawnDaemon(verbose: verbose)
        
        // Poll for socket availability (100ms intervals, 5s timeout)
        let deadline = Date().addingTimeInterval(5.0)
        while Date() < deadline {
            if let fd = tryConnect(socketPath: socketPath) {
                return fd
            }
            Thread.sleep(forTimeInterval: 0.1)
        }
        
        throw ConnectionError.daemonStartTimeout
    }
    
    /// Try to connect to Unix socket. Returns fd on success, nil on failure.
    private static func tryConnect(socketPath: String) -> Int32? {
        guard FileManager.default.fileExists(atPath: socketPath) else { return nil }
        
        let fd = socket(AF_UNIX, SOCK_STREAM, 0)
        guard fd >= 0 else { return nil }
        
        var addr = sockaddr_un()
        addr.sun_family = sa_family_t(AF_UNIX)
        
        let pathBytes = socketPath.utf8CString
        guard pathBytes.count <= MemoryLayout.size(ofValue: addr.sun_path) else {
            close(fd)
            return nil
        }
        
        withUnsafeMutablePointer(to: &addr.sun_path) { ptr in
            ptr.withMemoryRebound(to: CChar.self, capacity: pathBytes.count) { dest in
                pathBytes.withUnsafeBufferPointer { src in
                    _ = memcpy(dest, src.baseAddress!, pathBytes.count)
                }
            }
        }
        
        let addrLen = socklen_t(MemoryLayout<sockaddr_un>.size)
        let result = withUnsafePointer(to: &addr) { ptr in
            ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockPtr in
                connect(fd, sockPtr, addrLen)
            }
        }
        
        if result < 0 {
            close(fd)
            return nil
        }
        
        return fd
    }
    
    /// Spawn the daemon as a background process.
    private static func spawnDaemon(verbose: Bool) throws {
        // Find the daemon binary relative to the CLI binary
        let cliBinaryPath = CommandInfo.executableURL
        let binDir = cliBinaryPath.deletingLastPathComponent()
        let daemonPath = binDir.appendingPathComponent("agent-computer-daemon")
        
        // Fallback: try PATH
        let finalPath: URL
        if FileManager.default.isExecutableFile(atPath: daemonPath.path) {
            finalPath = daemonPath
        } else {
            // Try finding in PATH
            let whichProcess = Process()
            let whichPipe = Pipe()
            whichProcess.executableURL = URL(fileURLWithPath: "/usr/bin/which")
            whichProcess.arguments = ["agent-computer-daemon"]
            whichProcess.standardOutput = whichPipe
            whichProcess.standardError = FileHandle.nullDevice
            try? whichProcess.run()
            whichProcess.waitUntilExit()
            
            let whichData = whichPipe.fileHandleForReading.readDataToEndOfFile()
            let whichOutput = String(data: whichData, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            
            if !whichOutput.isEmpty && FileManager.default.isExecutableFile(atPath: whichOutput) {
                finalPath = URL(fileURLWithPath: whichOutput)
            } else {
                throw ConnectionError.daemonNotFound
            }
        }
        
        if verbose {
            FileHandle.standardError.write(Data("[verbose] Spawning daemon: \(finalPath.path)\n".utf8))
        }
        
        let process = Process()
        process.executableURL = finalPath
        process.standardOutput = FileHandle.nullDevice
        process.standardError = FileHandle.nullDevice
        
        // Ensure socket directory exists
        try FileManager.default.createDirectory(at: daemonSocketDir, withIntermediateDirectories: true)
        
        try process.run()
        // Don't wait — daemon runs in background
    }
    
    /// Read a newline-terminated line from a file descriptor.
    private static func readLine(fd: Int32) throws -> String {
        var buffer = [UInt8]()
        var byte: UInt8 = 0
        let maxSize = 10 * 1024 * 1024 // 10MB max response
        
        while buffer.count < maxSize {
            let n = Darwin.read(fd, &byte, 1)
            if n <= 0 {
                if buffer.isEmpty {
                    throw ConnectionError.connectionClosed
                }
                break
            }
            if byte == UInt8(ascii: "\n") {
                break
            }
            buffer.append(byte)
        }
        
        guard let line = String(bytes: buffer, encoding: .utf8) else {
            throw ConnectionError.decodingFailed
        }
        return line
    }
}

// MARK: - Helpers

private enum CommandInfo {
    static var executableURL: URL {
        URL(fileURLWithPath: ProcessInfo.processInfo.arguments[0])
    }
}

// MARK: - Errors

enum ConnectionError: Error, CustomStringConvertible {
    case daemonNotFound
    case daemonStartTimeout
    case connectionFailed
    case connectionClosed
    case writeFailed
    case encodingFailed
    case decodingFailed
    
    var description: String {
        switch self {
        case .daemonNotFound:
            return "Could not find agent-computer-daemon binary. Make sure it's in the same directory as agent-computer or on your PATH."
        case .daemonStartTimeout:
            return "Daemon did not start within 5 seconds. Check if another instance is running or if there are permission issues."
        case .connectionFailed:
            return "Failed to connect to daemon socket."
        case .connectionClosed:
            return "Daemon closed the connection unexpectedly."
        case .writeFailed:
            return "Failed to send data to daemon."
        case .encodingFailed:
            return "Failed to encode request as JSON."
        case .decodingFailed:
            return "Failed to decode response from daemon."
        }
    }
}
