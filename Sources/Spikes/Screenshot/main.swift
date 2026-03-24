import Foundation
import ScreenCaptureKit
import CoreGraphics
import AppKit

// MARK: - Helpers

func timestamp() -> String {
    let f = DateFormatter()
    f.dateFormat = "HH:mm:ss.SSS"
    return f.string(from: Date())
}

func log(_ msg: String) {
    print("[\(timestamp())] \(msg)")
    fflush(stdout)
}

func measure<T>(_ label: String, _ block: () throws -> T) rethrows -> T {
    let start = CFAbsoluteTimeGetCurrent()
    let result = try block()
    let elapsed = (CFAbsoluteTimeGetCurrent() - start) * 1000.0
    log("\(label): \(String(format: "%.1f", elapsed)) ms")
    return result
}

func savePNG(_ image: CGImage, to path: String) -> (Bool, Int) {
    let url = URL(fileURLWithPath: path)
    guard let dest = CGImageDestinationCreateWithURL(url as CFURL, "public.png" as CFString, 1, nil) else {
        log("ERROR: Could not create image destination at \(path)")
        return (false, 0)
    }
    CGImageDestinationAddImage(dest, image, nil)
    let ok = CGImageDestinationFinalize(dest)
    if ok {
        let size = (try? FileManager.default.attributesOfItem(atPath: path)[.size] as? Int) ?? 0
        return (true, size)
    }
    return (false, 0)
}

let outputDir = FileManager.default.currentDirectoryPath + "/spikes/screenshots"

// MARK: - Results tracking

struct CaptureResult {
    let method: String
    let target: String
    let latencyMs: Double
    let width: Int
    let height: Int
    let fileSizeBytes: Int
    let success: Bool
    let notes: String
}

var results: [CaptureResult] = []

// MARK: - 1. Permission Detection

log("=== SPIKE S4: SCREENCAPTUREKIT PERMISSION FLOW ===")
log("macOS version: \(ProcessInfo.processInfo.operatingSystemVersionString)")
log("Process: \(ProcessInfo.processInfo.processIdentifier)")

log("Testing CGPreflightScreenCaptureAccess()...")
let preflightGranted = CGPreflightScreenCaptureAccess()
log("CGPreflightScreenCaptureAccess() = \(preflightGranted)")

if !preflightGranted {
    log("Screen Recording permission NOT granted.")
    log("Calling CGRequestScreenCaptureAccess()...")
    let requested = CGRequestScreenCaptureAccess()
    log("CGRequestScreenCaptureAccess() = \(requested)")
    if !requested {
        log("⚠️  Permission denied. Will still try captures to document behavior.")
    }
}

// MARK: - 2. CGWindowListCreateImage (Legacy Approach)

log("")
log("=== LEGACY: CGWindowListCreateImage ===")
log("Note: Deprecated in macOS 14.0 in favor of ScreenCaptureKit")

// Full screen capture
do {
    log("Attempting full screen capture...")
    let start = CFAbsoluteTimeGetCurrent()
    let image = CGWindowListCreateImage(
        CGRect.null,  // entire desktop
        .optionOnScreenOnly,
        kCGNullWindowID,
        [.boundsIgnoreFraming]
    )
    let elapsed = (CFAbsoluteTimeGetCurrent() - start) * 1000.0

    if let image = image {
        let path = "\(outputDir)/legacy_fullscreen.png"
        let (saved, size) = savePNG(image, to: path)
        log("Full screen: \(image.width)x\(image.height), \(String(format: "%.1f", elapsed)) ms, \(size) bytes, saved=\(saved)")
        results.append(CaptureResult(
            method: "CGWindowListCreateImage", target: "Full Screen",
            latencyMs: elapsed, width: image.width, height: image.height,
            fileSizeBytes: size, success: saved, notes: "Deprecated API"
        ))
    } else {
        log("Full screen capture returned nil (permission denied or API removed)")
        results.append(CaptureResult(
            method: "CGWindowListCreateImage", target: "Full Screen",
            latencyMs: elapsed, width: 0, height: 0,
            fileSizeBytes: 0, success: false, notes: "Returned nil"
        ))
    }
}

// Frontmost window capture via CGWindowListCreateImage
do {
    log("Getting window list...")
    let windowList = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
    log("Found \(windowList.count) on-screen windows")

    let myPID = ProcessInfo.processInfo.processIdentifier
    var targetWindow: [String: Any]? = nil
    for w in windowList {
        let ownerPID = w[kCGWindowOwnerPID as String] as? Int32 ?? 0
        let layer = w[kCGWindowLayer as String] as? Int ?? 999
        let name = w[kCGWindowOwnerName as String] as? String ?? ""
        let alpha = w[kCGWindowAlpha as String] as? Double ?? 0
        if ownerPID != myPID && layer == 0 && alpha > 0 && !name.isEmpty {
            targetWindow = w
            break
        }
    }

    if let w = targetWindow {
        let windowID = w[kCGWindowNumber as String] as? CGWindowID ?? 0
        let ownerName = w[kCGWindowOwnerName as String] as? String ?? "unknown"
        log("Target window: \(ownerName) (ID: \(windowID))")

        let start = CFAbsoluteTimeGetCurrent()
        let image = CGWindowListCreateImage(
            CGRect.null,
            .optionIncludingWindow,
            windowID,
            [.boundsIgnoreFraming]
        )
        let elapsed = (CFAbsoluteTimeGetCurrent() - start) * 1000.0

        if let image = image {
            let path = "\(outputDir)/legacy_window_\(ownerName.replacingOccurrences(of: " ", with: "_")).png"
            let (saved, size) = savePNG(image, to: path)
            log("Window capture: \(image.width)x\(image.height), \(String(format: "%.1f", elapsed)) ms, \(size) bytes")
            results.append(CaptureResult(
                method: "CGWindowListCreateImage", target: "Window: \(ownerName)",
                latencyMs: elapsed, width: image.width, height: image.height,
                fileSizeBytes: size, success: saved, notes: "Deprecated API"
            ))
        } else {
            log("Window capture returned nil")
            results.append(CaptureResult(
                method: "CGWindowListCreateImage", target: "Window: \(ownerName)",
                latencyMs: elapsed, width: 0, height: 0,
                fileSizeBytes: 0, success: false, notes: "Returned nil"
            ))
        }
    } else {
        log("No suitable target window found for legacy single-window capture")
    }
}

// MARK: - 3. ScreenCaptureKit (Modern Approach)

log("")
log("=== SCREENCAPTUREKIT ===")

let semaphore = DispatchSemaphore(value: 0)

Task {
    do {
        log("Fetching SCShareableContent...")
        let startContent = CFAbsoluteTimeGetCurrent()
        let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
        let contentElapsed = (CFAbsoluteTimeGetCurrent() - startContent) * 1000.0
        log("SCShareableContent fetch: \(String(format: "%.1f", contentElapsed)) ms")
        log("  Displays: \(content.displays.count)")
        log("  Windows: \(content.windows.count)")
        log("  Applications: \(content.applications.count)")

        let myPID = ProcessInfo.processInfo.processIdentifier
        for (i, w) in content.windows.prefix(10).enumerated() {
            let appName = w.owningApplication?.applicationName ?? "?"
            let title = w.title ?? "(no title)"
            log("  Window[\(i)]: \(appName) - \"\(title)\" [\(Int(w.frame.width))x\(Int(w.frame.height))] id=\(w.windowID)")
        }

        guard let display = content.displays.first else {
            log("ERROR: No displays found")
            semaphore.signal()
            return
        }

        log("")
        log("Display: \(display.width)x\(display.height) (frame: \(display.frame))")

        // --- Full screen capture at 2x (Retina) ---
        let fullFilter = SCContentFilter(display: display, excludingWindows: [])
        let fullConfig = SCStreamConfiguration()
        fullConfig.width = display.width * 2
        fullConfig.height = display.height * 2
        fullConfig.showsCursor = false
        fullConfig.captureResolution = .best

        log("Capturing full screen at 2x...")
        let startFull = CFAbsoluteTimeGetCurrent()
        let fullImage = try await SCScreenshotManager.captureImage(contentFilter: fullFilter, configuration: fullConfig)
        let fullElapsed = (CFAbsoluteTimeGetCurrent() - startFull) * 1000.0

        let fullPath = "\(outputDir)/sck_fullscreen_2x.png"
        let (fullSaved, fullSize) = savePNG(fullImage, to: fullPath)
        log("SCK Full screen 2x: \(fullImage.width)x\(fullImage.height), \(String(format: "%.1f", fullElapsed)) ms, \(fullSize) bytes")
        results.append(CaptureResult(
            method: "SCScreenshotManager", target: "Full Screen (2x Retina)",
            latencyMs: fullElapsed, width: fullImage.width, height: fullImage.height,
            fileSizeBytes: fullSize, success: fullSaved, notes: "captureResolution=.best"
        ))

        // --- Full screen at 1x (logical resolution) ---
        let config1x = SCStreamConfiguration()
        config1x.width = display.width
        config1x.height = display.height
        config1x.showsCursor = false
        config1x.captureResolution = .best

        log("Capturing full screen at 1x...")
        let start1x = CFAbsoluteTimeGetCurrent()
        let image1x = try await SCScreenshotManager.captureImage(contentFilter: fullFilter, configuration: config1x)
        let elapsed1x = (CFAbsoluteTimeGetCurrent() - start1x) * 1000.0

        let path1x = "\(outputDir)/sck_fullscreen_1x.png"
        let (saved1x, size1x) = savePNG(image1x, to: path1x)
        log("SCK Full screen 1x: \(image1x.width)x\(image1x.height), \(String(format: "%.1f", elapsed1x)) ms, \(size1x) bytes")
        results.append(CaptureResult(
            method: "SCScreenshotManager", target: "Full Screen (1x Logical)",
            latencyMs: elapsed1x, width: image1x.width, height: image1x.height,
            fileSizeBytes: size1x, success: saved1x, notes: "1x logical resolution"
        ))

        // --- Single window capture ---
        var targetSCWindow: SCWindow? = nil
        for w in content.windows {
            let appName = w.owningApplication?.applicationName ?? ""
            let pid = w.owningApplication?.processID ?? 0
            if pid != myPID && w.frame.width > 100 && w.frame.height > 100 && !appName.isEmpty && w.isOnScreen {
                targetSCWindow = w
                break
            }
        }

        if let targetW = targetSCWindow {
            let appName = targetW.owningApplication?.applicationName ?? "unknown"
            let title = targetW.title ?? "(no title)"
            log("")
            log("SCK window target: \(appName) - \"\(title)\" frame=\(targetW.frame)")

            let windowFilter = SCContentFilter(desktopIndependentWindow: targetW)
            let windowConfig = SCStreamConfiguration()
            windowConfig.width = Int(targetW.frame.width) * 2
            windowConfig.height = Int(targetW.frame.height) * 2
            windowConfig.showsCursor = false
            windowConfig.captureResolution = .best

            let startWindow = CFAbsoluteTimeGetCurrent()
            let windowImage = try await SCScreenshotManager.captureImage(contentFilter: windowFilter, configuration: windowConfig)
            let windowElapsed = (CFAbsoluteTimeGetCurrent() - startWindow) * 1000.0

            let safeName = appName.replacingOccurrences(of: " ", with: "_")
            let windowPath = "\(outputDir)/sck_window_\(safeName).png"
            let (windowSaved, windowSize) = savePNG(windowImage, to: windowPath)
            log("SCK Window: \(windowImage.width)x\(windowImage.height), \(String(format: "%.1f", windowElapsed)) ms, \(windowSize) bytes")
            results.append(CaptureResult(
                method: "SCScreenshotManager", target: "Window: \(appName)",
                latencyMs: windowElapsed, width: windowImage.width, height: windowImage.height,
                fileSizeBytes: windowSize, success: windowSaved, notes: "2x Retina"
            ))
        } else {
            log("No suitable window found for SCK single-window capture")
        }

        // --- Benchmark: Multiple rapid captures ---
        log("")
        log("=== RAPID CAPTURE BENCHMARK (10 iterations) ===")

        var sckTimes: [Double] = []
        for _ in 0..<10 {
            let s = CFAbsoluteTimeGetCurrent()
            let _ = try await SCScreenshotManager.captureImage(contentFilter: fullFilter, configuration: config1x)
            let e = (CFAbsoluteTimeGetCurrent() - s) * 1000.0
            sckTimes.append(e)
        }
        let sckAvg = sckTimes.reduce(0, +) / Double(sckTimes.count)
        let sckMin = sckTimes.min() ?? 0
        let sckMax = sckTimes.max() ?? 0
        log("SCK 1x fullscreen avg: \(String(format: "%.1f", sckAvg)) ms (min: \(String(format: "%.1f", sckMin)), max: \(String(format: "%.1f", sckMax)))")

        var legacyTimes: [Double] = []
        for _ in 0..<10 {
            let s = CFAbsoluteTimeGetCurrent()
            let _ = CGWindowListCreateImage(CGRect.null, .optionOnScreenOnly, kCGNullWindowID, [.boundsIgnoreFraming])
            let e = (CFAbsoluteTimeGetCurrent() - s) * 1000.0
            legacyTimes.append(e)
        }
        let legAvg = legacyTimes.reduce(0, +) / Double(legacyTimes.count)
        let legMin = legacyTimes.min() ?? 0
        let legMax = legacyTimes.max() ?? 0
        log("Legacy fullscreen avg: \(String(format: "%.1f", legAvg)) ms (min: \(String(format: "%.1f", legMin)), max: \(String(format: "%.1f", legMax)))")

        results.append(CaptureResult(
            method: "SCScreenshotManager", target: "Benchmark 10x (1x)",
            latencyMs: sckAvg, width: 0, height: 0, fileSizeBytes: 0, success: true,
            notes: "min=\(String(format: "%.1f", sckMin))ms max=\(String(format: "%.1f", sckMax))ms"
        ))
        results.append(CaptureResult(
            method: "CGWindowListCreateImage", target: "Benchmark 10x",
            latencyMs: legAvg, width: 0, height: 0, fileSizeBytes: 0, success: true,
            notes: "min=\(String(format: "%.1f", legMin))ms max=\(String(format: "%.1f", legMax))ms"
        ))

    } catch {
        log("SCK ERROR: \(error)")
        log("Error type: \(type(of: error))")
        log("Error details: \(String(describing: error))")
    }

    semaphore.signal()
}

semaphore.wait()

// MARK: - Summary

log("")
log("=== RESULTS SUMMARY ===")
log("Method                    | Target                         | Latency    | Dimensions      | File Size    | Notes")
log(String(repeating: "-", count: 120))
for r in results {
    let dims = r.width > 0 ? "\(r.width)x\(r.height)" : "N/A"
    let sizeStr = r.fileSizeBytes > 0 ? "\(r.fileSizeBytes / 1024) KB" : "N/A"
    let latStr = String(format: "%.1f ms", r.latencyMs)
    log("\(r.method.padding(toLength: 25, withPad: " ", startingAt: 0)) | \(r.target.padding(toLength: 30, withPad: " ", startingAt: 0)) | \(latStr.padding(toLength: 10, withPad: " ", startingAt: 0)) | \(dims.padding(toLength: 15, withPad: " ", startingAt: 0)) | \(sizeStr.padding(toLength: 12, withPad: " ", startingAt: 0)) | \(r.notes)")
}

log("")
log("Screenshots saved to: \(outputDir)")
log("Done!")
