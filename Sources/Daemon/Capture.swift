import Foundation
import ScreenCaptureKit
import CoreGraphics
import AgentComputerShared

// MARK: - Capture Result

enum CaptureResult {
    case success(path: String, width: Int, height: Int)
    case failure(ErrorInfo)
}

// MARK: - Screenshot Capture

func captureScreenshot(full: Bool, app: String? = nil) -> CaptureResult {
    // Check permission first
    if !CGPreflightScreenCaptureAccess() {
        return .failure(Errors.screenRecordingDenied())
    }
    
    let semaphore = DispatchSemaphore(value: 0)
    var captureResult: CaptureResult = .failure(Errors.daemonError("Screenshot capture failed"))
    
    Task {
        do {
            let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
            
            guard let display = content.displays.first else {
                captureResult = .failure(Errors.daemonError("No display found"))
                semaphore.signal()
                return
            }
            
            let filter: SCContentFilter
            let config = SCStreamConfiguration()
            config.showsCursor = false
            config.captureResolution = .best
            
            if full {
                // Full screen capture at 1x
                filter = SCContentFilter(display: display, excludingWindows: [])
                config.width = display.width
                config.height = display.height
            } else {
                // Find target window
                let myPID = ProcessInfo.processInfo.processIdentifier
                var targetWindow: SCWindow? = nil
                
                if let appName = app {
                    // Find specific app's window
                    for w in content.windows {
                        let wAppName = w.owningApplication?.applicationName ?? ""
                        if wAppName.lowercased().contains(appName.lowercased()) && w.isOnScreen && w.frame.width > 10 {
                            targetWindow = w
                            break
                        }
                    }
                } else {
                    // Find frontmost window (first non-self on-screen window)
                    for w in content.windows {
                        let pid = w.owningApplication?.processID ?? 0
                        if pid != myPID && w.isOnScreen && w.frame.width > 50 && w.frame.height > 50 {
                            targetWindow = w
                            break
                        }
                    }
                }
                
                guard let window = targetWindow else {
                    captureResult = .failure(Errors.windowNotFound(app ?? "frontmost"))
                    semaphore.signal()
                    return
                }
                
                filter = SCContentFilter(desktopIndependentWindow: window)
                config.width = Int(window.frame.width)
                config.height = Int(window.frame.height)
            }
            
            let image = try await SCScreenshotManager.captureImage(contentFilter: filter, configuration: config)
            
            // Save to temp directory
            let tempDir = FileManager.default.temporaryDirectory.appendingPathComponent("agent-computer")
            try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
            
            let filename = "screenshot_\(Int(Date().timeIntervalSince1970)).png"
            let filePath = tempDir.appendingPathComponent(filename)
            
            guard let dest = CGImageDestinationCreateWithURL(filePath as CFURL, "public.png" as CFString, 1, nil) else {
                captureResult = .failure(Errors.daemonError("Failed to create image destination"))
                semaphore.signal()
                return
            }
            
            CGImageDestinationAddImage(dest, image, nil)
            guard CGImageDestinationFinalize(dest) else {
                captureResult = .failure(Errors.daemonError("Failed to write PNG file"))
                semaphore.signal()
                return
            }
            
            captureResult = .success(path: filePath.path, width: image.width, height: image.height)
            
        } catch {
            captureResult = .failure(Errors.daemonError("Screenshot capture error: \(error.localizedDescription)"))
        }
        
        semaphore.signal()
    }
    
    // Wait up to 10 seconds for capture
    let waitResult = semaphore.wait(timeout: .now() + 10)
    if waitResult == .timedOut {
        return .failure(Errors.timeout(command: "screenshot", timeoutMs: 10000))
    }
    
    return captureResult
}

// MARK: - Screenshot Command Handler

func handleScreenshot(id: String, args: ScreenshotArgs, startTime: CFAbsoluteTime) -> Response {
    func elapsed() -> Double { (CFAbsoluteTimeGetCurrent() - startTime) * 1000.0 }
    
    let result = captureScreenshot(full: args.full, app: args.app)
    
    switch result {
    case .success(let path, let width, let height):
        let data = ScreenshotData(
            path: path,
            width: width,
            height: height,
            scale: 1
        )
        return Response.ok(id: id, data: .screenshot(data), elapsed: elapsed())
        
    case .failure(let error):
        return Response.fail(id: id, error: error, elapsed: elapsed())
    }
}
