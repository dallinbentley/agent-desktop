import Foundation
import ApplicationServices
import AppKit

// =============================================================================
// AX Actions Spike: Test headless (background) AX interactions
// =============================================================================
// Tests whether AXUIElementPerformAction and AXUIElementSetAttributeValue
// work on apps that are NOT frontmost.
// =============================================================================

// MARK: - Helpers

func axGetString(_ element: AXUIElement, _ attr: String) -> String? {
    var value: AnyObject?
    let err = AXUIElementCopyAttributeValue(element, attr as CFString, &value)
    guard err == .success else { return nil }
    return value as? String
}

func axGetBool(_ element: AXUIElement, _ attr: String) -> Bool? {
    var value: AnyObject?
    let err = AXUIElementCopyAttributeValue(element, attr as CFString, &value)
    guard err == .success else { return nil }
    if let num = value as? NSNumber { return num.boolValue }
    return nil
}

func axGetChildren(_ element: AXUIElement) -> [AXUIElement] {
    var value: AnyObject?
    let err = AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &value)
    guard err == .success else { return [] }
    return (value as? [AXUIElement]) ?? []
}

func axGetActions(_ element: AXUIElement) -> [String] {
    var actions: CFArray?
    let err = AXUIElementCopyActionNames(element, &actions)
    guard err == .success, let acts = actions as? [String] else { return [] }
    return acts
}

func axGetFrame(_ element: AXUIElement) -> CGRect? {
    var posValue: AnyObject?
    var sizeValue: AnyObject?
    let posErr = AXUIElementCopyAttributeValue(element, kAXPositionAttribute as CFString, &posValue)
    let sizeErr = AXUIElementCopyAttributeValue(element, kAXSizeAttribute as CFString, &sizeValue)
    guard posErr == .success, sizeErr == .success,
          let pv = posValue, let sv = sizeValue else { return nil }
    guard CFGetTypeID(pv) == AXValueGetTypeID(),
          CFGetTypeID(sv) == AXValueGetTypeID() else { return nil }
    var point = CGPoint.zero
    var size = CGSize.zero
    guard AXValueGetValue(pv as! AXValue, .cgPoint, &point),
          AXValueGetValue(sv as! AXValue, .cgSize, &size) else { return nil }
    return CGRect(origin: point, size: size)
}

func axErrorName(_ err: AXError) -> String {
    switch err {
    case .success: return "success"
    case .failure: return "failure"
    case .illegalArgument: return "illegalArgument"
    case .invalidUIElement: return "invalidUIElement"
    case .invalidUIElementObserver: return "invalidUIElementObserver"
    case .cannotComplete: return "cannotComplete"
    case .attributeUnsupported: return "attributeUnsupported"
    case .actionUnsupported: return "actionUnsupported"
    case .notificationUnsupported: return "notificationUnsupported"
    case .notImplemented: return "notImplemented"
    case .notificationAlreadyRegistered: return "notificationAlreadyRegistered"
    case .notificationNotRegistered: return "notificationNotRegistered"
    case .apiDisabled: return "apiDisabled"
    case .noValue: return "noValue"
    case .parameterizedAttributeUnsupported: return "parameterizedAttributeUnsupported"
    case .notEnoughPrecision: return "notEnoughPrecision"
    @unknown default: return "unknown(\(err.rawValue))"
    }
}

// MARK: - Find Elements by Role (recursive)

struct FoundElement {
    let element: AXUIElement
    let role: String
    let title: String?
    let description: String?
    let value: String?
    let actions: [String]
    let depth: Int
}

func findElements(in element: AXUIElement, role targetRole: String? = nil, maxResults: Int = 10, maxDepth: Int = 15, depth: Int = 0) -> [FoundElement] {
    if depth > maxDepth { return [] }
    
    var results: [FoundElement] = []
    let role = axGetString(element, kAXRoleAttribute) ?? "?"
    let title = axGetString(element, kAXTitleAttribute)
    let desc = axGetString(element, kAXDescriptionAttribute)
    let value = axGetString(element, kAXValueAttribute)
    let actions = axGetActions(element)
    
    if targetRole == nil || role == targetRole {
        results.append(FoundElement(element: element, role: role, title: title, description: desc, value: value, actions: actions, depth: depth))
    }
    
    if results.count >= maxResults { return results }
    
    for child in axGetChildren(element) {
        results.append(contentsOf: findElements(in: child, role: targetRole, maxResults: maxResults - results.count, maxDepth: maxDepth, depth: depth + 1))
        if results.count >= maxResults { break }
    }
    
    return results
}

func findFirstElement(in element: AXUIElement, role: String, withAction action: String? = nil) -> FoundElement? {
    let candidates = findElements(in: element, role: role, maxResults: 50, maxDepth: 15)
    if let action = action {
        return candidates.first { $0.actions.contains(action) }
    }
    return candidates.first
}

// MARK: - Test Runner

var testResults: [(name: String, passed: Bool, detail: String)] = []

func test(_ name: String, _ block: () -> (Bool, String)) {
    let (passed, detail) = block()
    testResults.append((name, passed, detail))
    let icon = passed ? "✅" : "❌"
    print("  \(icon) \(name): \(detail)")
    fflush(stdout)
}

// MARK: - Find Target App

func findApp(named names: [String]) -> NSRunningApplication? {
    for name in names {
        if let app = NSWorkspace.shared.runningApplications.first(where: {
            $0.localizedName == name || ($0.localizedName?.contains(name) == true)
        }) {
            return app
        }
    }
    return nil
}

func isFrontmost(_ pid: pid_t) -> Bool {
    guard let front = NSWorkspace.shared.frontmostApplication else { return false }
    return front.processIdentifier == pid
}

// MARK: - Main

print("=" * 70)
print("AX ACTIONS HEADLESS SPIKE")
print("Testing whether AX actions work on background (non-frontmost) apps")
print("=" * 70)
print()
fflush(stdout)

// Check accessibility permission
guard AXIsProcessTrusted() else {
    print("ERROR: Accessibility permission not granted!")
    print("Go to System Settings > Privacy & Security > Accessibility")
    exit(1)
}

// Find test apps
let frontApp = NSWorkspace.shared.frontmostApplication!
let frontPid = frontApp.processIdentifier
let frontName = frontApp.localizedName ?? "Unknown"
print("Current frontmost app: \(frontName) (PID: \(frontPid))")
print()

// Try to find a background app to test with
// Priority: System Settings > Finder > TextEdit > Safari > any other
let backgroundApps = ["System Settings", "Finder", "TextEdit", "Safari", "Notes"]
var testApps: [(name: String, pid: pid_t, app: NSRunningApplication)] = []

for appName in backgroundApps {
    if let app = findApp(named: [appName]), app.processIdentifier != frontPid {
        testApps.append((app.localizedName ?? appName, app.processIdentifier, app))
    }
}

if testApps.isEmpty {
    print("WARNING: No background apps found to test. Will try to open System Settings.")
    // Launch System Settings in background
    let config = NSWorkspace.OpenConfiguration()
    config.activates = false // Don't bring to front!
    let semaphore = DispatchSemaphore(value: 0)
    var launchedApp: NSRunningApplication?
    
    NSWorkspace.shared.openApplication(
        at: URL(fileURLWithPath: "/System/Applications/System Settings.app"),
        configuration: config
    ) { app, error in
        launchedApp = app
        semaphore.signal()
    }
    semaphore.wait()
    
    if let app = launchedApp {
        sleep(2) // Wait for app to initialize
        testApps.append(("System Settings", app.processIdentifier, app))
        print("Launched System Settings (PID: \(app.processIdentifier))")
    } else {
        print("ERROR: Could not launch System Settings")
        exit(1)
    }
}

print("\nBackground apps to test: \(testApps.map { $0.name }.joined(separator: ", "))")
print()
fflush(stdout)

// ===========================================================================
// TEST SUITE
// ===========================================================================

for (appName, pid, runningApp) in testApps {
    print("-" * 70)
    print("TESTING: \(appName) (PID: \(pid), frontmost: \(isFrontmost(pid)))")
    print("-" * 70)
    fflush(stdout)
    
    let appElement = AXUIElementCreateApplication(pid)
    
    // Verify we can read from this app
    test("[\(appName)] Read app title") {
        let title = axGetString(appElement, kAXTitleAttribute)
        if let t = title {
            return (true, "Got title: '\(t)'")
        }
        return (false, "Could not read app title")
    }
    
    // Test 1: Read windows from background app
    test("[\(appName)] Read windows") {
        var windowsValue: AnyObject?
        let err = AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &windowsValue)
        if err == .success, let windows = windowsValue as? [AXUIElement] {
            let titles = windows.compactMap { axGetString($0, kAXTitleAttribute) }
            return (true, "\(windows.count) windows: \(titles.joined(separator: ", "))")
        }
        return (false, "Error: \(axErrorName(err))")
    }
    
    // Test 2: Find buttons in background app
    test("[\(appName)] Find buttons") {
        let buttons = findElements(in: appElement, role: "AXButton", maxResults: 5)
        if !buttons.isEmpty {
            let descs = buttons.map { "\($0.title ?? $0.description ?? "?") [actions: \($0.actions.joined(separator: ","))]" }
            return (true, "Found \(buttons.count) buttons: \(descs.joined(separator: "; "))")
        }
        return (false, "No buttons found")
    }
    
    // Test 3: AXPress on a button (THE KEY TEST)
    test("[\(appName)] AXPress on background button") {
        guard !isFrontmost(pid) else {
            return (false, "SKIP: App is frontmost, can't test headless")
        }
        
        guard let button = findFirstElement(in: appElement, role: "AXButton", withAction: kAXPressAction as String) else {
            return (false, "No pressable button found")
        }
        
        let buttonDesc = button.title ?? button.description ?? "unnamed"
        let err = AXUIElementPerformAction(button.element, kAXPressAction as CFString)
        
        if err == .success {
            // Check if the app stole focus
            usleep(100_000) // 100ms
            let stolenFocus = isFrontmost(pid)
            return (true, "AXPress succeeded on '\(buttonDesc)' (focus stolen: \(stolenFocus))")
        }
        return (false, "AXPress failed on '\(buttonDesc)': \(axErrorName(err))")
    }
    
    // Test 4: AXShowMenu on background app
    test("[\(appName)] AXShowMenu on background element") {
        guard !isFrontmost(pid) else {
            return (false, "SKIP: App is frontmost")
        }
        
        // Find an element with AXShowMenu action
        let allElements = findElements(in: appElement, maxResults: 100, maxDepth: 8)
        if let menuable = allElements.first(where: { $0.actions.contains(kAXShowMenuAction as String) }) {
            let desc = menuable.title ?? menuable.description ?? menuable.role
            let err = AXUIElementPerformAction(menuable.element, kAXShowMenuAction as CFString)
            let stolenFocus = isFrontmost(pid)
            if err == .success {
                return (true, "AXShowMenu succeeded on '\(desc)' (focus stolen: \(stolenFocus))")
            }
            return (false, "AXShowMenu failed on '\(desc)': \(axErrorName(err))")
        }
        return (false, "No element with AXShowMenu found")
    }
    
    // Test 5: AXRaise on a window
    test("[\(appName)] AXRaise on window") {
        var windowsValue: AnyObject?
        let err = AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &windowsValue)
        guard err == .success, let windows = windowsValue as? [AXUIElement], let win = windows.first else {
            return (false, "No windows to raise")
        }
        
        let winTitle = axGetString(win, kAXTitleAttribute) ?? "untitled"
        let raiseErr = AXUIElementPerformAction(win, kAXRaiseAction as CFString)
        usleep(100_000)
        let stolenFocus = isFrontmost(pid)
        if raiseErr == .success {
            return (true, "AXRaise succeeded on '\(winTitle)' (focus stolen: \(stolenFocus))")
        }
        return (false, "AXRaise failed: \(axErrorName(raiseErr))")
    }
    
    // Test 6: Find text fields
    test("[\(appName)] Find text fields/areas") {
        let textFields = findElements(in: appElement, role: "AXTextField", maxResults: 5)
        let textAreas = findElements(in: appElement, role: "AXTextArea", maxResults: 5)
        let searchFields = findElements(in: appElement, role: "AXSearchField", maxResults: 5)
        let all = textFields + textAreas + searchFields
        if !all.isEmpty {
            let descs = all.map { "\($0.role)(\($0.title ?? $0.description ?? "?")) value='\($0.value ?? "nil")' actions=[\($0.actions.joined(separator: ","))]" }
            return (true, "Found \(all.count): \(descs.joined(separator: "; "))")
        }
        return (false, "No text input elements found")
    }
    
    // Test 7: Set text value via kAXValueAttribute on background app
    test("[\(appName)] Set kAXValueAttribute on text field (background)") {
        guard !isFrontmost(pid) else {
            return (false, "SKIP: App is frontmost")
        }
        
        let textFields = findElements(in: appElement, role: "AXTextField", maxResults: 5) +
                         findElements(in: appElement, role: "AXSearchField", maxResults: 5)
        guard let tf = textFields.first else {
            return (false, "No text fields found to test")
        }
        
        let desc = tf.title ?? tf.description ?? tf.role
        let originalValue = tf.value
        let testText = "AX_TEST_\(Int.random(in: 1000...9999))"
        
        let err = AXUIElementSetAttributeValue(tf.element, kAXValueAttribute as CFString, testText as CFTypeRef)
        usleep(100_000) // 100ms for value to propagate
        
        // Read back
        let readBack = axGetString(tf.element, kAXValueAttribute)
        let stolenFocus = isFrontmost(pid)
        
        if err == .success && readBack == testText {
            // Restore original value
            if let orig = originalValue {
                AXUIElementSetAttributeValue(tf.element, kAXValueAttribute as CFString, orig as CFTypeRef)
            }
            return (true, "Set value on '\(desc)' successfully, read back matches (focus stolen: \(stolenFocus))")
        } else if err == .success {
            return (false, "Set returned success but read back '\(readBack ?? "nil")' != '\(testText)' (silent fail)")
        }
        return (false, "SetAttributeValue failed on '\(desc)': \(axErrorName(err))")
    }
    
    // Test 8: Set text via AXTextArea (for TextEdit-like apps)
    test("[\(appName)] Set kAXValueAttribute on text area (background)") {
        guard !isFrontmost(pid) else {
            return (false, "SKIP: App is frontmost")
        }
        
        let textAreas = findElements(in: appElement, role: "AXTextArea", maxResults: 5)
        guard let ta = textAreas.first else {
            return (false, "No text areas found to test")
        }
        
        let desc = ta.title ?? ta.description ?? ta.role
        let originalValue = ta.value
        let testText = "AX_AREA_TEST_\(Int.random(in: 1000...9999))"
        
        let err = AXUIElementSetAttributeValue(ta.element, kAXValueAttribute as CFString, testText as CFTypeRef)
        usleep(100_000)
        
        let readBack = axGetString(ta.element, kAXValueAttribute)
        let stolenFocus = isFrontmost(pid)
        
        if err == .success && readBack == testText {
            if let orig = originalValue {
                AXUIElementSetAttributeValue(ta.element, kAXValueAttribute as CFString, orig as CFTypeRef)
            }
            return (true, "Set text area value on '\(desc)' successfully (focus stolen: \(stolenFocus))")
        } else if err == .success {
            return (false, "Set returned success but read back '\(readBack ?? "nil")' != '\(testText)' (SILENT FAIL)")
        }
        return (false, "SetAttributeValue on text area '\(desc)': \(axErrorName(err))")
    }
    
    // Test 9: Selection-replace approach on text field
    test("[\(appName)] Selection-replace on text field (background)") {
        guard !isFrontmost(pid) else {
            return (false, "SKIP: App is frontmost")
        }
        
        let textFields = findElements(in: appElement, role: "AXTextField", maxResults: 5) +
                         findElements(in: appElement, role: "AXTextArea", maxResults: 5) +
                         findElements(in: appElement, role: "AXSearchField", maxResults: 5)
        guard let tf = textFields.first else {
            return (false, "No text input elements found")
        }
        
        let desc = "\(tf.role)(\(tf.title ?? tf.description ?? "?"))"
        
        // Read current text length
        let currentText = axGetString(tf.element, kAXValueAttribute) ?? ""
        let textLength = currentText.count
        
        // Select all via AXSelectedTextRange
        var range = CFRange(location: 0, length: max(textLength, 100000))
        guard let axRange = AXValueCreate(.cfRange, &range) else {
            return (false, "Could not create AXValue for range")
        }
        
        let selErr = AXUIElementSetAttributeValue(tf.element, kAXSelectedTextRangeAttribute as CFString, axRange)
        if selErr != .success {
            return (false, "Set selection range failed on \(desc): \(axErrorName(selErr))")
        }
        
        // Replace with test text
        let testText = "SEL_REPLACE_\(Int.random(in: 1000...9999))"
        let replaceErr = AXUIElementSetAttributeValue(tf.element, kAXSelectedTextAttribute as CFString, testText as CFTypeRef)
        usleep(100_000)
        
        let readBack = axGetString(tf.element, kAXValueAttribute)
        let stolenFocus = isFrontmost(pid)
        
        if replaceErr == .success && readBack?.contains("SEL_REPLACE") == true {
            // Restore
            AXUIElementSetAttributeValue(tf.element, kAXValueAttribute as CFString, currentText as CFTypeRef)
            return (true, "Selection-replace worked on \(desc) (focus stolen: \(stolenFocus))")
        } else if replaceErr == .success {
            return (false, "Replace returned success but text is '\(readBack ?? "nil")' (silent fail?)")
        }
        return (false, "Replace selected text failed on \(desc): \(axErrorName(replaceErr))")
    }
    
    // Test 10: Focus text field via AX without bringing app to front
    test("[\(appName)] Set kAXFocusedUIElement on background app") {
        guard !isFrontmost(pid) else {
            return (false, "SKIP: App is frontmost")
        }
        
        let textFields = findElements(in: appElement, role: "AXTextField", maxResults: 5) +
                         findElements(in: appElement, role: "AXTextArea", maxResults: 5) +
                         findElements(in: appElement, role: "AXSearchField", maxResults: 5)
        guard let tf = textFields.first else {
            return (false, "No text input elements found")
        }
        
        let desc = "\(tf.role)(\(tf.title ?? tf.description ?? "?"))"
        
        // Try to set focused element on the app
        let err = AXUIElementSetAttributeValue(appElement, kAXFocusedUIElementAttribute as CFString, tf.element)
        usleep(100_000)
        
        let stolenFocus = isFrontmost(pid)
        
        if err == .success {
            return (true, "Set focused element to \(desc) (focus stolen: \(stolenFocus))")
        }
        return (false, "Set focused element failed: \(axErrorName(err))")
    }
    
    // Test 11: Increment/Decrement on sliders or steppers
    test("[\(appName)] AXIncrement/AXDecrement on background") {
        guard !isFrontmost(pid) else {
            return (false, "SKIP: App is frontmost")
        }
        
        let sliders = findElements(in: appElement, role: "AXSlider", maxResults: 3)
        let incrementors = findElements(in: appElement, role: "AXIncrementor", maxResults: 3)
        let candidates = sliders + incrementors
        
        guard let el = candidates.first(where: { $0.actions.contains(kAXIncrementAction as String) }) else {
            return (false, "No incrementable elements found")
        }
        
        let desc = "\(el.role)(\(el.title ?? el.description ?? "?"))"
        let beforeValue = el.value
        let err = AXUIElementPerformAction(el.element, kAXIncrementAction as CFString)
        usleep(100_000)
        let afterValue = axGetString(el.element, kAXValueAttribute)
        let stolenFocus = isFrontmost(pid)
        
        if err == .success {
            return (true, "Increment on \(desc): '\(beforeValue ?? "nil")' -> '\(afterValue ?? "nil")' (focus stolen: \(stolenFocus))")
        }
        return (false, "Increment failed on \(desc): \(axErrorName(err))")
    }
    
    // Test 12: Check checkbox via AXPress
    test("[\(appName)] AXPress on checkbox (background)") {
        guard !isFrontmost(pid) else {
            return (false, "SKIP: App is frontmost")
        }
        
        let checkboxes = findElements(in: appElement, role: "AXCheckBox", maxResults: 5)
        guard let cb = checkboxes.first(where: { $0.actions.contains(kAXPressAction as String) }) else {
            return (false, "No checkboxes with AXPress found")
        }
        
        let desc = cb.title ?? cb.description ?? "unnamed"
        let beforeValue = cb.value
        let err = AXUIElementPerformAction(cb.element, kAXPressAction as CFString)
        usleep(100_000)
        let afterValue = axGetString(cb.element, kAXValueAttribute)
        let stolenFocus = isFrontmost(pid)
        
        if err == .success {
            let changed = beforeValue != afterValue
            // Toggle back
            if changed {
                AXUIElementPerformAction(cb.element, kAXPressAction as CFString)
            }
            return (true, "AXPress checkbox '\(desc)': '\(beforeValue ?? "nil")' -> '\(afterValue ?? "nil")' changed=\(changed) (focus stolen: \(stolenFocus))")
        }
        return (false, "AXPress failed on checkbox '\(desc)': \(axErrorName(err))")
    }
    
    // Test 13: Performance comparison - AX action vs noting (baseline)
    test("[\(appName)] AXPress latency (10 iterations)") {
        guard let button = findFirstElement(in: appElement, role: "AXButton", withAction: kAXPressAction as String) else {
            return (false, "No pressable button found")
        }
        
        var times: [Double] = []
        for _ in 0..<10 {
            let start = CFAbsoluteTimeGetCurrent()
            let _ = AXUIElementPerformAction(button.element, kAXPressAction as CFString)
            let elapsed = (CFAbsoluteTimeGetCurrent() - start) * 1000.0
            times.append(elapsed)
            usleep(20_000) // 20ms between presses
        }
        
        times.sort()
        let median = times[times.count / 2]
        let min = times.first!
        let max = times.last!
        return (true, "AXPress latency: median=\(String(format: "%.2f", median))ms, min=\(String(format: "%.2f", min))ms, max=\(String(format: "%.2f", max))ms")
    }
    
    // Test 14: List ALL available actions across all elements
    test("[\(appName)] Catalog all actions in app") {
        let allElements = findElements(in: appElement, maxResults: 200, maxDepth: 10)
        var actionCounts: [String: Int] = [:]
        var actionRoles: [String: Set<String>] = [:]
        
        for el in allElements {
            for action in el.actions {
                actionCounts[action, default: 0] += 1
                if actionRoles[action] == nil { actionRoles[action] = Set() }
                actionRoles[action]!.insert(el.role)
            }
        }
        
        let summary = actionCounts.sorted(by: { $0.value > $1.value }).map { action, count in
            let roles = actionRoles[action]?.sorted().joined(separator: ",") ?? ""
            return "\(action)(\(count)x on \(roles))"
        }
        
        return (true, "Actions found: \(summary.joined(separator: "; "))")
    }
    
    print()
}

// ===========================================================================
// SUMMARY
// ===========================================================================

print("=" * 70)
print("SUMMARY")
print("=" * 70)

let passed = testResults.filter { $0.passed }.count
let failed = testResults.filter { !$0.passed }.count
let skipped = testResults.filter { $0.detail.starts(with: "SKIP") }.count

print("Total: \(testResults.count), Passed: \(passed), Failed: \(failed - skipped), Skipped: \(skipped)")
print()

// Group by category
print("KEY FINDINGS:")
print()

// Check if AXPress worked on background
let pressTests = testResults.filter { $0.name.contains("AXPress on background") }
if let pt = pressTests.first {
    if pt.passed {
        print("  🟢 AXPress on buttons WORKS on background apps")
    } else {
        print("  🔴 AXPress on buttons DOES NOT WORK on background apps")
    }
}

let checkboxTests = testResults.filter { $0.name.contains("checkbox") }
if let ct = checkboxTests.first {
    if ct.passed {
        print("  🟢 AXPress on checkboxes WORKS on background apps")
    } else {
        print("  🔴 AXPress on checkboxes DOES NOT WORK on background apps")
    }
}

let textSetTests = testResults.filter { $0.name.contains("Set kAXValueAttribute on text field") }
if let tt = textSetTests.first {
    if tt.passed {
        print("  🟢 kAXValueAttribute text set WORKS on background apps")
    } else {
        print("  🔴 kAXValueAttribute text set DOES NOT WORK on background apps")
    }
}

let textAreaTests = testResults.filter { $0.name.contains("Set kAXValueAttribute on text area") }
if let ta = textAreaTests.first {
    if ta.passed {
        print("  🟢 kAXValueAttribute text area set WORKS on background apps")
    } else {
        print("  🔴 kAXValueAttribute text area set DOES NOT WORK on background apps")
    }
}

let selReplaceTests = testResults.filter { $0.name.contains("Selection-replace") }
if let sr = selReplaceTests.first {
    if sr.passed {
        print("  🟢 Selection-replace approach WORKS on background apps")
    } else {
        print("  🔴 Selection-replace approach DOES NOT WORK on background apps")
    }
}

let focusTests = testResults.filter { $0.name.contains("kAXFocusedUIElement") }
if let ft = focusTests.first {
    if ft.passed {
        let stolenFocus = ft.detail.contains("focus stolen: true")
        if stolenFocus {
            print("  🟡 kAXFocusedUIElement works but STEALS FOCUS")
        } else {
            print("  🟢 kAXFocusedUIElement works WITHOUT stealing focus")
        }
    } else {
        print("  🔴 kAXFocusedUIElement DOES NOT WORK on background apps")
    }
}

let showMenuTests = testResults.filter { $0.name.contains("AXShowMenu") }
if let sm = showMenuTests.first {
    if sm.passed {
        print("  🟢 AXShowMenu WORKS on background apps")
    } else {
        print("  🔴 AXShowMenu DOES NOT WORK on background apps")
    }
}

let raiseTests = testResults.filter { $0.name.contains("AXRaise") }
if let rt = raiseTests.first {
    if rt.passed {
        let stolenFocus = rt.detail.contains("focus stolen: true")
        print("  🟡 AXRaise: \(stolenFocus ? "steals focus (expected)" : "does not steal focus")")
    }
}

print()
print("FULL DETAILS:")
for r in testResults {
    let icon = r.passed ? "✅" : "❌"
    print("  \(icon) \(r.name): \(r.detail)")
}

print()
print("Done.")
fflush(stdout)

// MARK: - String repeat helper
extension String {
    static func * (left: String, right: Int) -> String {
        String(repeating: left, count: right)
    }
}
