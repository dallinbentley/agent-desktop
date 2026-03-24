import Foundation
import Cocoa
import ApplicationServices
import AgentComputerShared

// MARK: - App Finding

struct RunningApp {
    let name: String
    let pid: pid_t
    let isActive: Bool
    let bundleIdentifier: String?
}

func findRunningApp(name: String) -> RunningApp? {
    let apps = NSWorkspace.shared.runningApplications
    let searchName = name.lowercased()
    
    // Exact match first
    if let app = apps.first(where: { $0.localizedName?.lowercased() == searchName }) {
        return RunningApp(
            name: app.localizedName ?? name,
            pid: app.processIdentifier,
            isActive: app.isActive,
            bundleIdentifier: app.bundleIdentifier
        )
    }
    
    // Contains match
    if let app = apps.first(where: { $0.localizedName?.lowercased().contains(searchName) == true }) {
        return RunningApp(
            name: app.localizedName ?? name,
            pid: app.processIdentifier,
            isActive: app.isActive,
            bundleIdentifier: app.bundleIdentifier
        )
    }
    
    return nil
}

func fuzzyMatchAppName(_ name: String) -> String? {
    let apps = NSWorkspace.shared.runningApplications
    let searchName = name.lowercased()
    
    // Find closest match by checking if any app name starts with the search term
    for app in apps {
        guard let appName = app.localizedName?.lowercased() else { continue }
        if appName.hasPrefix(searchName) || searchName.hasPrefix(appName) {
            return app.localizedName
        }
    }
    
    // Check bundle ID contains
    for app in apps {
        if let bundleId = app.bundleIdentifier?.lowercased(), bundleId.contains(searchName) {
            return app.localizedName
        }
    }
    
    return nil
}

func getRunningGUIApps() -> [AppInfo] {
    return NSWorkspace.shared.runningApplications
        .filter { $0.activationPolicy == .regular }
        .compactMap { app in
            guard let name = app.localizedName else { return nil }
            return AppInfo(
                name: name,
                pid: Int(app.processIdentifier),
                isActive: app.isActive
            )
        }
        .sorted { $0.name.lowercased() < $1.name.lowercased() }
}

// MARK: - Open/Launch Result

enum OpenResult {
    case success(appName: String, pid: pid_t, wasRunning: Bool)
    case failure(ErrorInfo)
}

enum GetTextResult {
    case success(String)
    case failure(ErrorInfo)
}

// MARK: - Open/Launch App

func openApp(name: String) -> OpenResult {
    // Check if already running
    if let running = findRunningApp(name: name) {
        // Activate it
        let apps = NSWorkspace.shared.runningApplications
        if let nsApp = apps.first(where: { $0.processIdentifier == running.pid }) {
            nsApp.activate()
            usleep(100_000) // 100ms to let activation happen
        }
        return .success(appName: running.name, pid: running.pid, wasRunning: true)
    }
    
    // Try to launch by name
    // First try common app locations
    let appPaths = [
        "/Applications/\(name).app",
        "/Applications/Utilities/\(name).app",
        "/System/Applications/\(name).app",
        "/System/Applications/Utilities/\(name).app",
    ]
    
    for path in appPaths {
        let url = URL(fileURLWithPath: path)
        if FileManager.default.fileExists(atPath: path) {
            do {
                let config = NSWorkspace.OpenConfiguration()
                config.activates = true
                
                let semaphore = DispatchSemaphore(value: 0)
                var launchedApp: NSRunningApplication? = nil
                var launchError: Error? = nil
                
                NSWorkspace.shared.openApplication(at: url, configuration: config) { app, error in
                    launchedApp = app
                    launchError = error
                    semaphore.signal()
                }
                
                let waitResult = semaphore.wait(timeout: .now() + 10)
                if waitResult == .timedOut {
                    return .failure(Errors.timeout(command: "open", timeoutMs: 10000))
                }
                
                if let error = launchError {
                    return .failure(Errors.daemonError("Failed to launch \(name): \(error.localizedDescription)"))
                }
                
                if let app = launchedApp {
                    return .success(appName: app.localizedName ?? name, pid: app.processIdentifier, wasRunning: false)
                }
            }
        }
    }
    
    // App not found — provide fuzzy suggestion
    if let suggestion = fuzzyMatchAppName(name) {
        return .failure(ErrorInfo(
            code: ErrorCode.appNotFound,
            message: "Application '\(name)' not found.",
            suggestion: "Did you mean '\(suggestion)'? Run 'get apps' to list running applications."
        ))
    }
    
    return .failure(Errors.appNotFound(name))
}

// MARK: - Get Text from Element

func getTextFromRef(ref: String) -> GetTextResult {
    if globalRefMap.isEmpty() {
        return .failure(Errors.noRefMap())
    }
    guard let _ = globalRefMap.resolve(ref: ref) else {
        return .failure(Errors.refNotFound(ref))
    }
    guard let axElement = globalRefMap.resolveToAXElement(ref: ref) else {
        return .failure(Errors.refStale(ref))
    }
    
    // Try value first, then title, then description
    if let value = safeGetString(axElement, kAXValueAttribute) {
        return .success(value)
    }
    if let title = safeGetString(axElement, kAXTitleAttribute) {
        return .success(title)
    }
    if let desc = safeGetString(axElement, kAXDescriptionAttribute) {
        return .success(desc)
    }
    
    return .success("") // Element has no text content
}

// MARK: - Command Handlers

func handleOpen(id: String, args: OpenArgs, startTime: CFAbsoluteTime) -> Response {
    func elapsed() -> Double { (CFAbsoluteTimeGetCurrent() - startTime) * 1000.0 }
    
    let result = openApp(name: args.target)
    
    switch result {
    case .success(let appName, let pid, let wasRunning):
        let data = OpenData(app: appName, pid: Int(pid), wasRunning: wasRunning)
        return Response.ok(id: id, data: .open(data), elapsed: elapsed())
    case .failure(let error):
        return Response.fail(id: id, error: error, elapsed: elapsed())
    }
}

func handleGet(id: String, args: GetArgs, startTime: CFAbsoluteTime) -> Response {
    func elapsed() -> Double { (CFAbsoluteTimeGetCurrent() - startTime) * 1000.0 }
    
    switch args.what.lowercased() {
    case "apps":
        let apps = getRunningGUIApps()
        let data = GetAppsData(apps: apps)
        return Response.ok(id: id, data: .getApps(data), elapsed: elapsed())
        
    case "text":
        guard let ref = args.ref else {
            return Response.fail(id: id, error: Errors.invalidCommand("'get text' requires a @ref"), elapsed: elapsed())
        }
        let result = getTextFromRef(ref: ref)
        switch result {
        case .success(let text):
            let data = GetTextData(ref: ref, text: text)
            return Response.ok(id: id, data: .getText(data), elapsed: elapsed())
        case .failure(let error):
            return Response.fail(id: id, error: error, elapsed: elapsed())
        }
        
    default:
        return Response.fail(id: id, error: Errors.invalidCommand("Unknown get target: '\(args.what)'. Valid: apps, text"), elapsed: elapsed())
    }
}

func handleStatus(id: String, startTime: CFAbsoluteTime) -> Response {
    func elapsed() -> Double { (CFAbsoluteTimeGetCurrent() - startTime) * 1000.0 }
    
    let pid = Int(ProcessInfo.processInfo.processIdentifier)
    let axTrusted = AXIsProcessTrusted()
    let screenPermission = CGPreflightScreenCaptureAccess()
    
    var frontApp: String? = nil
    var frontPid: Int? = nil
    var frontWindow: String? = nil
    
    if let front = getFrontmostApp() {
        frontApp = front.name
        frontPid = Int(front.pid)
        
        // Get frontmost window title
        let appElement = AXUIElementCreateApplication(front.pid)
        var windowsValue: AnyObject?
        if AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &windowsValue) == .success,
           let windows = windowsValue as? [AXUIElement],
           let firstWindow = windows.first {
            frontWindow = safeGetString(firstWindow, kAXTitleAttribute)
        }
    }
    
    let data = StatusData(
        daemonPid: pid,
        accessibilityPermission: axTrusted,
        screenRecordingPermission: screenPermission,
        frontmostApp: frontApp,
        frontmostPid: frontPid,
        frontmostWindow: frontWindow,
        refMapCount: globalRefMap.count,
        refMapAgeMs: globalRefMap.ageMs
    )
    
    return Response.ok(id: id, data: .status(data), elapsed: elapsed())
}
