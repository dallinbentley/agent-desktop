import Foundation
#if canImport(ApplicationServices)
import ApplicationServices
#endif
#if canImport(CoreGraphics)
import CoreGraphics
#endif

// MARK: - Standard Error Constructors

public enum Errors {
    
    public static func refNotFound(_ ref: String) -> ErrorInfo {
        ErrorInfo(
            code: ErrorCode.refNotFound,
            message: "Element \(ref) not found in current ref map.",
            suggestion: "Run 'snapshot -i' to get current element references, then use a valid @ref."
        )
    }
    
    public static func refStale(_ ref: String) -> ErrorInfo {
        ErrorInfo(
            code: ErrorCode.refStale,
            message: "Element \(ref) exists in ref map but could not be located on screen. The UI may have changed.",
            suggestion: "Run 'snapshot -i' to refresh element references and retry with the new @ref."
        )
    }
    
    public static func noRefMap() -> ErrorInfo {
        ErrorInfo(
            code: ErrorCode.noRefMap,
            message: "No element ref map available. You must take a snapshot before referencing elements.",
            suggestion: "Run 'snapshot -i' first to discover interactive elements and their @refs."
        )
    }
    
    public static func appNotFound(_ name: String) -> ErrorInfo {
        ErrorInfo(
            code: ErrorCode.appNotFound,
            message: "Application '\(name)' not found.",
            suggestion: "Run 'get apps' to list running applications. Check spelling or use 'open \"\(name)\"' to launch it."
        )
    }
    
    public static func windowNotFound(_ app: String) -> ErrorInfo {
        ErrorInfo(
            code: ErrorCode.windowNotFound,
            message: "No window found for application '\(app)'.",
            suggestion: "The app may be running but has no open windows. Try 'open \"\(app)\"' to activate it, or check 'get windows'."
        )
    }
    
    public static func permissionDenied(permission: String, instructions: String) -> ErrorInfo {
        ErrorInfo(
            code: ErrorCode.permissionDenied,
            message: "\(permission) permission is not granted.",
            suggestion: instructions
        )
    }
    
    public static func accessibilityDenied() -> ErrorInfo {
        permissionDenied(
            permission: "Accessibility",
            instructions: "Open System Settings → Privacy & Security → Accessibility → Enable agent-computer-daemon. You may need to restart the daemon after granting permission."
        )
    }
    
    public static func screenRecordingDenied() -> ErrorInfo {
        permissionDenied(
            permission: "Screen Recording",
            instructions: "Open System Settings → Privacy & Security → Screen Recording → Enable agent-computer-daemon. You may need to restart the daemon after granting permission."
        )
    }
    
    public static func timeout(command: String, timeoutMs: Int) -> ErrorInfo {
        ErrorInfo(
            code: ErrorCode.timeout,
            message: "Command '\(command)' timed out after \(timeoutMs)ms.",
            suggestion: "The operation took too long. Try increasing --timeout or simplify the command (e.g., use --app to scope snapshot to one app)."
        )
    }
    
    public static func axError(detail: String) -> ErrorInfo {
        ErrorInfo(
            code: ErrorCode.axError,
            message: "Accessibility API error: \(detail)",
            suggestion: "This may be a transient issue. Try again, or check that the target app is responsive."
        )
    }
    
    public static func inputError(detail: String) -> ErrorInfo {
        ErrorInfo(
            code: ErrorCode.inputError,
            message: "Input simulation error: \(detail)",
            suggestion: "Ensure the target element is focused and the app is in the foreground."
        )
    }
    
    public static func invalidCommand(_ detail: String) -> ErrorInfo {
        ErrorInfo(
            code: ErrorCode.invalidCommand,
            message: "Invalid command: \(detail)",
            suggestion: "Run 'agent-computer --help' for usage information."
        )
    }
    
    public static func daemonError(_ detail: String) -> ErrorInfo {
        ErrorInfo(
            code: ErrorCode.daemonError,
            message: "Daemon error: \(detail)",
            suggestion: "Try restarting the daemon. If the issue persists, check ~/.agent-computer/ for logs."
        )
    }
}

// MARK: - Permission Checking Helpers (Task 8.3)

public enum PermissionCheck {
    
    /// Check if Accessibility permission is granted.
    /// Note: AXIsProcessTrusted() is only meaningful in the daemon process.
    /// The CLI should use the status command to check this remotely.
    public static func checkAccessibility() -> ErrorInfo? {
        // This will be called in the daemon context
        #if canImport(ApplicationServices)
        if !AXIsProcessTrusted() {
            return Errors.accessibilityDenied()
        }
        #endif
        return nil
    }
    
    /// Check if Screen Recording permission is granted.
    /// Uses CGPreflightScreenCaptureAccess() available on macOS 10.15+.
    public static func checkScreenRecording() -> ErrorInfo? {
        #if canImport(CoreGraphics)
        if !CGPreflightScreenCaptureAccess() {
            return Errors.screenRecordingDenied()
        }
        #endif
        return nil
    }
    
    /// Check all required permissions, returns array of errors for any missing.
    public static func checkAll() -> [ErrorInfo] {
        var errors: [ErrorInfo] = []
        if let err = checkAccessibility() { errors.append(err) }
        if let err = checkScreenRecording() { errors.append(err) }
        return errors
    }
}
