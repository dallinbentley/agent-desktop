import Foundation
import AgentComputerShared

/// Output formatting for CLI responses.
enum Output {
    
    // MARK: - ANSI Colors
    
    private static let reset = "\u{001B}[0m"
    private static let bold = "\u{001B}[1m"
    private static let dim = "\u{001B}[2m"
    private static let red = "\u{001B}[31m"
    private static let green = "\u{001B}[32m"
    private static let yellow = "\u{001B}[33m"
    private static let blue = "\u{001B}[34m"
    private static let cyan = "\u{001B}[36m"
    
    /// Format and print response. Returns true if successful.
    @discardableResult
    static func printResponse(_ response: Response, jsonMode: Bool) -> Bool {
        if jsonMode {
            return printJSON(response)
        }
        
        if response.success, let data = response.data {
            printData(data)
            return true
        } else if let error = response.error {
            printError(error)
            return false
        } else if !response.success {
            printError(ErrorInfo(code: "UNKNOWN", message: "Command failed with no error details.", suggestion: nil))
            return false
        }
        
        return response.success
    }
    
    // MARK: - JSON Mode
    
    private static func printJSON(_ response: Response) -> Bool {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        if let data = try? encoder.encode(response),
           let json = String(data: data, encoding: .utf8) {
            print(json)
        }
        return response.success
    }
    
    // MARK: - Human-Readable Output
    
    private static func printData(_ data: ResponseData) {
        switch data {
        case .snapshot(let d):
            printSnapshot(d)
        case .click(let d):
            printClick(d)
        case .fill(let d):
            printFill(d)
        case .type(let d):
            printType(d)
        case .press(let d):
            printPress(d)
        case .scroll(let d):
            printScroll(d)
        case .screenshot(let d):
            printScreenshot(d)
        case .open(let d):
            printOpen(d)
        case .getApps(let d):
            printGetApps(d)
        case .getText(let d):
            printGetText(d)
        case .status(let d):
            printStatus(d)
        case .raw(let s):
            print(s)
        }
    }
    
    private static func printSnapshot(_ data: SnapshotData) {
        // The text tree is already formatted by the daemon — print as-is
        print(data.text)
    }
    
    private static func printClick(_ data: ClickData) {
        var parts = ["Clicked"]
        if let ref = data.ref {
            parts.append("\(cyan)@\(ref)\(reset)")
        }
        if let elem = data.element {
            let roleShort = elem.role.replacingOccurrences(of: "AX", with: "").lowercased()
            parts.append(roleShort)
            if let label = elem.label {
                parts.append("\(bold)\"\(label)\"\(reset)")
            }
        }
        let coords = "(\(Int(data.coordinates.x)), \(Int(data.coordinates.y)))"
        parts.append("at \(dim)\(coords)\(reset)")
        print(parts.joined(separator: " "))
    }
    
    private static func printFill(_ data: FillData) {
        print("Filled \(cyan)@\(data.ref)\(reset) with \(bold)\"\(data.text)\"\(reset)")
    }
    
    private static func printType(_ data: TypeData) {
        if let ref = data.ref {
            print("Typed \(bold)\"\(data.text)\"\(reset) into \(cyan)@\(ref)\(reset)")
        } else {
            print("Typed \(bold)\"\(data.text)\"\(reset)")
        }
    }
    
    private static func printPress(_ data: PressData) {
        var keyCombo = data.modifiers.joined(separator: "+")
        if !keyCombo.isEmpty { keyCombo += "+" }
        keyCombo += data.key
        print("Pressed \(bold)\(keyCombo)\(reset)")
    }
    
    private static func printScroll(_ data: ScrollData) {
        print("Scrolled \(bold)\(data.direction)\(reset) by \(data.amount) pixels")
    }
    
    private static func printScreenshot(_ data: ScreenshotData) {
        print("Screenshot saved to \(cyan)\(data.path)\(reset) (\(data.width)×\(data.height))")
    }
    
    private static func printOpen(_ data: OpenData) {
        if data.wasRunning {
            print("Activated \(bold)\(data.app)\(reset) (pid \(data.pid))")
        } else {
            print("Launched \(bold)\(data.app)\(reset) (pid \(data.pid))")
        }
    }
    
    private static func printGetApps(_ data: GetAppsData) {
        for app in data.apps {
            let activeMarker = app.isActive ? " \(green)●\(reset)" : ""
            print("\(bold)\(app.name)\(reset) (pid \(app.pid))\(activeMarker)")
        }
        if data.apps.isEmpty {
            print("\(dim)No running GUI applications found.\(reset)")
        }
    }
    
    private static func printGetText(_ data: GetTextData) {
        if let ref = data.ref {
            print("\(cyan)@\(ref)\(reset): \(data.text)")
        } else {
            print(data.text)
        }
    }
    
    private static func printStatus(_ data: StatusData) {
        print("\(bold)agent-computer daemon\(reset)")
        print("  PID: \(data.daemonPid)")
        print("  Accessibility: \(data.accessibilityPermission ? "\(green)✅ granted\(reset)" : "\(red)❌ denied\(reset)")")
        print("  Screen Recording: \(data.screenRecordingPermission ? "\(green)✅ granted\(reset)" : "\(red)❌ denied\(reset)")")
        if let app = data.frontmostApp {
            var frontLine = "  Frontmost App: \(bold)\(app)\(reset)"
            if let pid = data.frontmostPid { frontLine += " (pid \(pid))" }
            print(frontLine)
            if let window = data.frontmostWindow {
                print("  Frontmost Window: \(window)")
            }
        }
        print("  Ref Map: \(data.refMapCount) elements", terminator: "")
        if let age = data.refMapAgeMs {
            let ageSec = age / 1000.0
            print(" (age: \(String(format: "%.1f", ageSec))s)")
        } else {
            print(" (no snapshot taken)")
        }
    }
    
    // MARK: - Error Output
    
    static func printError(_ error: ErrorInfo) {
        FileHandle.standardError.write(Data("\(red)\(bold)Error\(reset)\(red) [\(error.code)]: \(error.message)\(reset)\n".utf8))
        if let suggestion = error.suggestion {
            FileHandle.standardError.write(Data("\(yellow)Suggestion: \(suggestion)\(reset)\n".utf8))
        }
    }
    
    /// Print a connection/local error (not from daemon).
    static func printLocalError(_ message: String, suggestion: String? = nil) {
        FileHandle.standardError.write(Data("\(red)\(bold)Error\(reset)\(red): \(message)\(reset)\n".utf8))
        if let suggestion = suggestion {
            FileHandle.standardError.write(Data("\(yellow)Suggestion: \(suggestion)\(reset)\n".utf8))
        }
    }
}
