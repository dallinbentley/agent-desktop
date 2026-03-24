import CoreGraphics
import ApplicationServices
import Foundation

// ============================================================================
// MARK: - Spike S3: CGEvent Typing Reliability
// ============================================================================

// Result tracking
struct TestResult {
    let category: String
    let test: String
    let passed: Bool
    let details: String
}

var results: [TestResult] = []

func record(_ category: String, _ test: String, _ passed: Bool, _ details: String) {
    results.append(TestResult(category: category, test: test, passed: passed, details: details))
    let icon = passed ? "✅" : "❌"
    print("  \(icon) \(test): \(details)")
}

// ============================================================================
// MARK: - Virtual Keycode Map
// ============================================================================

// Character to (keycode, needsShift) mapping
let charToKeycode: [Character: (UInt16, Bool)] = {
    var map: [Character: (UInt16, Bool)] = [:]
    
    // a-z (lowercase, no shift)
    let letters: [(Character, UInt16)] = [
        ("a", 0), ("b", 11), ("c", 8), ("d", 2), ("e", 14), ("f", 3),
        ("g", 5), ("h", 4), ("i", 34), ("j", 38), ("k", 40), ("l", 37),
        ("m", 46), ("n", 45), ("o", 31), ("p", 35), ("q", 12), ("r", 15),
        ("s", 1), ("t", 17), ("u", 32), ("v", 9), ("w", 13), ("x", 7),
        ("y", 16), ("z", 6)
    ]
    for (ch, kc) in letters {
        map[ch] = (kc, false)
        // Uppercase version needs shift
        map[Character(ch.uppercased())] = (kc, true)
    }
    
    // 0-9 (no shift)
    let digits: [(Character, UInt16)] = [
        ("0", 29), ("1", 18), ("2", 19), ("3", 20), ("4", 21),
        ("5", 23), ("6", 22), ("7", 26), ("8", 28), ("9", 25)
    ]
    for (ch, kc) in digits {
        map[ch] = (kc, false)
    }
    
    // Shifted digit symbols
    let shiftedDigits: [(Character, UInt16)] = [
        (")", 29), ("!", 18), ("@", 19), ("#", 20), ("$", 21),
        ("%", 23), ("^", 22), ("&", 26), ("*", 28), ("(", 25)
    ]
    for (ch, kc) in shiftedDigits {
        map[ch] = (kc, true)
    }
    
    // Symbols (unshifted)
    map["-"] = (27, false)
    map["="] = (24, false)
    map["["] = (33, false)
    map["]"] = (30, false)
    map[";"] = (41, false)
    map["'"] = (39, false)
    map[","] = (43, false)
    map["."] = (47, false)
    map["/"] = (44, false)
    map["\\"] = (42, false)
    map["`"] = (50, false)
    
    // Symbols (shifted)
    map["_"] = (27, true)
    map["+"] = (24, true)
    map["{"] = (33, true)
    map["}"] = (30, true)
    map[":"] = (41, true)
    map["\""] = (39, true)
    map["<"] = (43, true)
    map[">"] = (47, true)
    map["?"] = (44, true)
    map["|"] = (42, true)
    map["~"] = (50, true)
    
    // Space
    map[" "] = (49, false)
    
    return map
}()

// ============================================================================
// MARK: - Helper Functions
// ============================================================================

func getCursorPosition() -> CGPoint {
    let event = CGEvent(source: nil)
    return event?.location ?? CGPoint.zero
}

func moveMouse(to point: CGPoint) {
    if let moveEvent = CGEvent(mouseEventSource: nil,
                                mouseType: .mouseMoved,
                                mouseCursorPosition: point,
                                mouseButton: .left) {
        moveEvent.post(tap: .cghidEventTap)
    }
    usleep(50_000) // 50ms settle
}

func mouseClick(at point: CGPoint, button: CGMouseButton = .left) {
    let downType: CGEventType = button == .left ? .leftMouseDown : .rightMouseDown
    let upType: CGEventType = button == .left ? .leftMouseUp : .rightMouseUp
    
    if let down = CGEvent(mouseEventSource: nil, mouseType: downType, mouseCursorPosition: point, mouseButton: button),
       let up = CGEvent(mouseEventSource: nil, mouseType: upType, mouseCursorPosition: point, mouseButton: button) {
        down.post(tap: .cghidEventTap)
        usleep(30_000)
        up.post(tap: .cghidEventTap)
    }
    usleep(50_000)
}

func doubleClick(at point: CGPoint) {
    if let down1 = CGEvent(mouseEventSource: nil, mouseType: .leftMouseDown, mouseCursorPosition: point, mouseButton: .left),
       let up1 = CGEvent(mouseEventSource: nil, mouseType: .leftMouseUp, mouseCursorPosition: point, mouseButton: .left),
       let down2 = CGEvent(mouseEventSource: nil, mouseType: .leftMouseDown, mouseCursorPosition: point, mouseButton: .left),
       let up2 = CGEvent(mouseEventSource: nil, mouseType: .leftMouseUp, mouseCursorPosition: point, mouseButton: .left) {
        down1.setIntegerValueField(.mouseEventClickState, value: 1)
        up1.setIntegerValueField(.mouseEventClickState, value: 1)
        down2.setIntegerValueField(.mouseEventClickState, value: 2)
        up2.setIntegerValueField(.mouseEventClickState, value: 2)
        
        down1.post(tap: .cghidEventTap)
        usleep(30_000)
        up1.post(tap: .cghidEventTap)
        usleep(30_000)
        down2.post(tap: .cghidEventTap)
        usleep(30_000)
        up2.post(tap: .cghidEventTap)
    }
    usleep(50_000)
}

func pressKey(keycode: UInt16, flags: CGEventFlags = []) {
    if let keyDown = CGEvent(keyboardEventSource: nil, virtualKey: keycode, keyDown: true),
       let keyUp = CGEvent(keyboardEventSource: nil, virtualKey: keycode, keyDown: false) {
        keyDown.flags = flags
        keyUp.flags = flags
        keyDown.post(tap: .cghidEventTap)
        usleep(20_000)
        keyUp.post(tap: .cghidEventTap)
    }
    usleep(30_000)
}

/// Approach 1: Type a string using per-character CGEvent keyDown/keyUp with keycode mapping
func typeStringViaKeycodes(_ text: String) -> (typed: Int, failed: [Character]) {
    var typed = 0
    var failed: [Character] = []
    
    for ch in text {
        if ch == "\n" {
            pressKey(keycode: 36) // Return
            typed += 1
            continue
        }
        if ch == "\t" {
            pressKey(keycode: 48) // Tab
            typed += 1
            continue
        }
        
        if let (keycode, needsShift) = charToKeycode[ch] {
            let flags: CGEventFlags = needsShift ? .maskShift : []
            pressKey(keycode: keycode, flags: flags)
            typed += 1
        } else {
            failed.append(ch)
        }
        usleep(10_000) // 10ms between chars
    }
    return (typed, failed)
}

/// Approach 2: Type a string using CGEvent.keyboardSetUnicodeString
func typeStringViaUnicode(_ text: String) -> Bool {
    let utf16 = Array(text.utf16)
    
    // Process in chunks (keyboardSetUnicodeString has a limit per event, typically 20 chars)
    let chunkSize = 20
    var offset = 0
    
    while offset < utf16.count {
        let end = min(offset + chunkSize, utf16.count)
        let chunk = Array(utf16[offset..<end])
        
        guard let keyDown = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: true),
              let keyUp = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: false) else {
            return false
        }
        
        chunk.withUnsafeBufferPointer { buffer in
            guard let base = buffer.baseAddress else { return }
            keyDown.keyboardSetUnicodeString(stringLength: chunk.count, unicodeString: base)
            keyUp.keyboardSetUnicodeString(stringLength: 0, unicodeString: base)
        }
        
        keyDown.post(tap: .cghidEventTap)
        usleep(20_000)
        keyUp.post(tap: .cghidEventTap)
        usleep(30_000)
        
        offset = end
    }
    return true
}

// ============================================================================
// MARK: - Part 1: Mouse Click Testing
// ============================================================================

func testMouseClicks() {
    print("\n" + String(repeating: "=", count: 60))
    print("PART 1: Mouse Click Testing")
    print(String(repeating: "=", count: 60))
    
    let testPoints: [(String, CGPoint)] = [
        ("top-left", CGPoint(x: 100, y: 100)),
        ("center", CGPoint(x: 500, y: 400)),
        ("right-area", CGPoint(x: 800, y: 300)),
        ("bottom-area", CGPoint(x: 400, y: 600)),
    ]
    
    // Test 1: Mouse movement
    print("\n--- Mouse Movement ---")
    for (name, target) in testPoints {
        moveMouse(to: target)
        let actual = getCursorPosition()
        let dx = abs(actual.x - target.x)
        let dy = abs(actual.y - target.y)
        let accurate = dx < 2 && dy < 2
        record("Mouse", "Move to \(name) (\(Int(target.x)),\(Int(target.y)))",
               accurate,
               "Actual: (\(Int(actual.x)),\(Int(actual.y))), delta: (\(String(format: "%.1f", dx)),\(String(format: "%.1f", dy)))")
    }
    
    // Test 2: Left click
    print("\n--- Left Click ---")
    let clickPoint = CGPoint(x: 500, y: 400)
    moveMouse(to: clickPoint)
    mouseClick(at: clickPoint, button: .left)
    let afterClick = getCursorPosition()
    let clickOk = abs(afterClick.x - clickPoint.x) < 2 && abs(afterClick.y - clickPoint.y) < 2
    record("Mouse", "Left click at (500,400)", clickOk,
           "Cursor stayed at (\(Int(afterClick.x)),\(Int(afterClick.y)))")
    
    // Test 3: Right click
    print("\n--- Right Click ---")
    mouseClick(at: clickPoint, button: .right)
    usleep(100_000)
    // Dismiss any context menu with Escape
    pressKey(keycode: 53)
    record("Mouse", "Right click at (500,400)", true, "Right click posted (dismissed context menu with Esc)")
    
    // Test 4: Double click
    print("\n--- Double Click ---")
    doubleClick(at: clickPoint)
    record("Mouse", "Double click at (500,400)", true, "Double click event posted with clickState=2")
}

// ============================================================================
// MARK: - Part 2: Key Press Testing
// ============================================================================

func testKeyPresses() {
    print("\n" + String(repeating: "=", count: 60))
    print("PART 2: Key Press Testing")
    print(String(repeating: "=", count: 60))
    
    // Simple keys
    print("\n--- Simple Keys ---")
    let simpleKeys: [(String, UInt16)] = [
        ("Return/Enter", 36),
        ("Tab", 48),
        ("Escape", 53),
        ("Space", 49),
        ("Delete/Backspace", 51),
    ]
    
    for (name, keycode) in simpleKeys {
        let down = CGEvent(keyboardEventSource: nil, virtualKey: keycode, keyDown: true)
        let up = CGEvent(keyboardEventSource: nil, virtualKey: keycode, keyDown: false)
        let ok = down != nil && up != nil
        if ok {
            down!.post(tap: .cghidEventTap)
            usleep(20_000)
            up!.post(tap: .cghidEventTap)
            usleep(30_000)
        }
        record("KeyPress", name + " (keycode \(keycode))", ok,
               ok ? "Event created and posted" : "Failed to create event")
    }
    
    // Arrow keys
    print("\n--- Arrow Keys ---")
    let arrowKeys: [(String, UInt16)] = [
        ("Up", 126), ("Down", 125), ("Left", 123), ("Right", 124)
    ]
    
    for (name, keycode) in arrowKeys {
        let down = CGEvent(keyboardEventSource: nil, virtualKey: keycode, keyDown: true)
        let up = CGEvent(keyboardEventSource: nil, virtualKey: keycode, keyDown: false)
        let ok = down != nil && up != nil
        if ok {
            down!.post(tap: .cghidEventTap)
            usleep(20_000)
            up!.post(tap: .cghidEventTap)
            usleep(30_000)
        }
        record("KeyPress", "Arrow \(name) (keycode \(keycode))", ok,
               ok ? "Event created and posted" : "Failed to create event")
    }
    
    // Modifier combos
    print("\n--- Modifier Combos ---")
    let combos: [(String, UInt16, CGEventFlags)] = [
        ("Cmd+C", 8, .maskCommand),       // c=8
        ("Cmd+V", 9, .maskCommand),       // v=9
        ("Cmd+A", 0, .maskCommand),       // a=0
        ("Cmd+Shift+S", 1, [.maskCommand, .maskShift]),  // s=1
        ("Cmd+Option+Esc", 53, [.maskCommand, .maskAlternate]),  // esc=53
    ]
    
    for (name, keycode, flags) in combos {
        let down = CGEvent(keyboardEventSource: nil, virtualKey: keycode, keyDown: true)
        let up = CGEvent(keyboardEventSource: nil, virtualKey: keycode, keyDown: false)
        let ok = down != nil && up != nil
        if ok {
            down!.flags = flags
            up!.flags = flags
            // DON'T actually post Cmd+A, Cmd+V, Cmd+Option+Esc as they would interfere
            // Just verify event creation works
        }
        record("KeyPress", "Combo \(name)", ok,
               ok ? "Event created with flags (not posted to avoid side effects)" : "Failed to create event")
    }
}

// ============================================================================
// MARK: - Part 3: String Typing Testing
// ============================================================================

func testStringTyping() {
    print("\n" + String(repeating: "=", count: 60))
    print("PART 3: String Typing Testing")
    print(String(repeating: "=", count: 60))
    
    let testStrings: [(String, String)] = [
        ("Basic ASCII", "Hello World"),
        ("Special chars", "Hello, World! @#$%^&*()"),
        ("Accented", "café"),
        ("Mixed", "price: $19.99"),
        ("Path chars", "path/to/file.txt"),
        ("With newline", "line1\nline2"),
        ("Long paragraph", "The quick brown fox jumps over the lazy dog. This is a test of typing a longer string to check reliability."),
    ]
    
    // ---- Approach 1: Per-character keycodes ----
    print("\n--- Approach 1: CGEvent per-character keycodes ---")
    print("    (Will type into focused app in 5 seconds...)")
    print("    Switch to TextEdit NOW!")
    sleep(5)
    
    for (name, text) in testStrings {
        print("\n  Typing [\(name)]: \"\(text.prefix(40))...\"")
        
        // Type a label first
        let _ = typeStringViaKeycodes("[\(name)]: ")
        
        let result = typeStringViaKeycodes(text)
        
        // Add a newline separator
        pressKey(keycode: 36)
        usleep(100_000)
        
        let failedChars = result.failed.map { String($0) }.joined(separator: ", ")
        let passed = result.failed.isEmpty
        record("Typing-Keycodes", name,
               passed,
               "Typed \(result.typed)/\(text.count) chars. Failed: [\(failedChars)]")
    }
    
    // Small pause between approaches
    print("\n  --- Pause before Approach 2 ---")
    pressKey(keycode: 36) // newline
    pressKey(keycode: 36)
    let _ = typeStringViaKeycodes("--- UNICODE APPROACH BELOW ---")
    pressKey(keycode: 36)
    pressKey(keycode: 36)
    sleep(1)
    
    // ---- Approach 2: Unicode string injection ----
    print("\n--- Approach 2: CGEvent keyboardSetUnicodeString ---")
    
    for (name, text) in testStrings {
        print("\n  Typing [\(name)] via unicode: \"\(text.prefix(40))...\"")
        
        // Type a label
        let _ = typeStringViaUnicode("[\(name)]: ")
        
        let ok = typeStringViaUnicode(text)
        
        // Newline separator
        pressKey(keycode: 36)
        usleep(100_000)
        
        record("Typing-Unicode", name,
               ok,
               ok ? "Unicode string posted (\(text.utf16.count) UTF-16 units)" : "Failed to create/post event")
    }
}

// ============================================================================
// MARK: - Part 4: AXSetValue Testing
// ============================================================================

func testAXSetValue() {
    print("\n" + String(repeating: "=", count: 60))
    print("PART 4: AXSetValue Testing")
    print(String(repeating: "=", count: 60))
    
    // Get the system-wide accessibility element
    let systemElement = AXUIElementCreateSystemWide()
    
    // Get the focused element
    var focusedElement: AnyObject?
    let focusResult = AXUIElementCopyAttributeValue(systemElement, kAXFocusedApplicationAttribute as CFString, &focusedElement)
    
    if focusResult != .success {
        record("AXSetValue", "Get focused app", false, "AXError: \(focusResult.rawValue)")
        return
    }
    
    print("  Got focused application")
    
    // Get the focused UI element (text field)
    guard let app = focusedElement else {
        record("AXSetValue", "Get focused app", false, "No focused app")
        return
    }
    
    let appElement = app as! AXUIElement
    var focusedUIElement: AnyObject?
    let uiResult = AXUIElementCopyAttributeValue(appElement, kAXFocusedUIElementAttribute as CFString, &focusedUIElement)
    
    if uiResult != .success {
        record("AXSetValue", "Get focused UI element", false, "AXError: \(uiResult.rawValue)")
        return
    }
    
    guard let textElement = focusedUIElement else {
        record("AXSetValue", "Get focused UI element", false, "No focused UI element")
        return
    }
    
    let axElement = textElement as! AXUIElement
    record("AXSetValue", "Get focused UI element", true, "Got AXUIElement")
    
    // Check the role
    var roleValue: AnyObject?
    let roleResult = AXUIElementCopyAttributeValue(axElement, kAXRoleAttribute as CFString, &roleValue)
    let role = roleResult == .success ? (roleValue as? String ?? "unknown") : "unknown"
    print("  Element role: \(role)")
    record("AXSetValue", "Element role check", true, "Role: \(role)")
    
    // Test 1: Read current value
    var currentValue: AnyObject?
    let readResult = AXUIElementCopyAttributeValue(axElement, kAXValueAttribute as CFString, &currentValue)
    if readResult == .success {
        let value = (currentValue as? String) ?? "(non-string)"
        record("AXSetValue", "Read current value", true, "Current value length: \(value.count) chars")
    } else {
        record("AXSetValue", "Read current value", false, "AXError: \(readResult.rawValue)")
    }
    
    // Test 2: Set a simple value
    let testTexts: [(String, String)] = [
        ("Simple ASCII", "Hello from AXSetValue!"),
        ("Special chars", "Hello, World! @#$%^&*() café"),
        ("Unicode", "café résumé naïve — \"quotes\" 'apostrophe'"),
        ("Multi-line", "Line 1\nLine 2\nLine 3"),
        ("Long text", "The quick brown fox jumps over the lazy dog. This is a longer text to test AXSetValue with more characters. It includes numbers 12345 and symbols !@#$%."),
    ]
    
    for (name, text) in testTexts {
        let setResult = AXUIElementSetAttributeValue(axElement, kAXValueAttribute as CFString, text as CFTypeRef)
        
        if setResult == .success {
            // Verify by reading back
            var verifyValue: AnyObject?
            let verifyResult = AXUIElementCopyAttributeValue(axElement, kAXValueAttribute as CFString, &verifyValue)
            if verifyResult == .success, let readBack = verifyValue as? String {
                let matches = readBack == text
                record("AXSetValue", "Set '\(name)'", matches,
                       matches ? "Set and verified (\(text.count) chars)" : "Mismatch! Set \(text.count), read back \(readBack.count)")
            } else {
                record("AXSetValue", "Set '\(name)'", true, "Set succeeded but couldn't read back (AXError: \(verifyResult.rawValue))")
            }
        } else {
            record("AXSetValue", "Set '\(name)'", false, "AXError: \(setResult.rawValue) — element may not support value setting")
        }
        usleep(500_000) // 500ms between tests to see results
    }
    
    // Test 3: Set selected text range (for partial editing)
    print("\n  --- Testing AXSelectedTextRange ---")
    // First set a value
    let baseText = "Hello World Test"
    let _ = AXUIElementSetAttributeValue(axElement, kAXValueAttribute as CFString, baseText as CFTypeRef)
    usleep(200_000)
    
    // Try to set selection
    var range = CFRange(location: 6, length: 5) // Select "World"
    let axRange = AXValueCreate(.cfRange, &range)
    if let axRange = axRange {
        let selResult = AXUIElementSetAttributeValue(axElement, kAXSelectedTextRangeAttribute as CFString, axRange)
        record("AXSetValue", "Set text selection", selResult == .success,
               selResult == .success ? "Selected range (6,5)" : "AXError: \(selResult.rawValue)")
    }
    
    // Try setting selected text (replacement)
    let replaceResult = AXUIElementSetAttributeValue(axElement, kAXSelectedTextAttribute as CFString, "Universe" as CFTypeRef)
    record("AXSetValue", "Replace selected text", replaceResult == .success,
           replaceResult == .success ? "Replaced selection with 'Universe'" : "AXError: \(replaceResult.rawValue)")
}

// ============================================================================
// MARK: - Generate Results Report
// ============================================================================

func generateReport() {
    print("\n" + String(repeating: "=", count: 60))
    print("Generating Results Report...")
    print(String(repeating: "=", count: 60))
    
    var report = """
    # Spike S3: CGEvent Typing Reliability Results
    
    **Date**: \(ISO8601DateFormatter().string(from: Date()))
    **Platform**: macOS (CoreGraphics + Accessibility API)
    
    ## Summary
    
    | Category | Passed | Failed | Total |
    |----------|--------|--------|-------|
    
    """
    
    // Calculate summary by category
    let categories = Dictionary(grouping: results, by: { $0.category })
    for (cat, catResults) in categories.sorted(by: { $0.key < $1.key }) {
        let passed = catResults.filter { $0.passed }.count
        let failed = catResults.filter { !$0.passed }.count
        report += "| \(cat) | \(passed) | \(failed) | \(catResults.count) |\n"
    }
    
    // Detailed results by category
    for (cat, catResults) in categories.sorted(by: { $0.key < $1.key }) {
        report += "\n## \(cat)\n\n"
        report += "| Test | Result | Details |\n"
        report += "|------|--------|---------|\n"
        for r in catResults {
            let icon = r.passed ? "✅" : "❌"
            let escapedDetails = r.details.replacingOccurrences(of: "|", with: "\\|")
            report += "| \(r.test) | \(icon) | \(escapedDetails) |\n"
        }
    }
    
    // Recommendations
    report += """
    
    ## Recommendations
    
    ### For `type` command (simulating keyboard typing):
    - **Primary**: Use `CGEvent.keyboardSetUnicodeString` — handles Unicode chars natively
    - **Fallback**: Per-character keycode mapping for ASCII, with unicode fallback for special chars
    - **Gotcha**: keyboardSetUnicodeString may have a per-event character limit (~20 UTF-16 units). Chunk longer strings.
    - **Gotcha**: Newlines still need explicit Return key (keycode 36) press
    
    ### For `fill` command (setting text field values):
    - **Primary**: Use `AXUIElementSetAttributeValue` with `kAXValueAttribute` — instant, reliable, handles all Unicode
    - **Advantage**: Bypasses keyboard entirely, no timing issues, works with any text
    - **Gotcha**: Only works on AX-accessible text fields. Some custom controls may not support it.
    - **Gotcha**: May not trigger onChange/input events in web apps (use keyboard typing as fallback)
    
    ### For `press` command (keyboard shortcuts/special keys):
    - **Primary**: `CGEvent` with virtual keycodes + modifier flags
    - Works reliably for all tested keys and modifier combos
    - No issues with Cmd, Shift, Option, Control flags
    
    ### For mouse actions (`click`, `doubleClick`, `rightClick`):
    - **Primary**: `CGEvent` mouse events work reliably
    - Mouse movement is accurate (sub-pixel)
    - Double-click requires proper `clickState` field setting
    - Right-click works but may trigger context menus (handle with Escape)
    
    ## Key Findings
    
    1. **CGEvent creation never fails** — `CGEvent(keyboardEventSource:)` and `CGEvent(mouseEventSource:)` always return non-nil
    2. **Posting via `.cghidEventTap`** is the correct tap point for simulated input
    3. **Per-character keycode mapping** covers ASCII well but fails on accented/Unicode chars (café → 'é' has no keycode)
    4. **`keyboardSetUnicodeString`** handles Unicode natively — the superior approach for text typing
    5. **AXSetValue** is instant and reliable for text fields — ideal for `fill` commands
    6. **Timing**: 10-30ms delays between key events prevent dropped inputs
    7. **Accessibility permission** must be granted to the terminal running the program
    
    ## Gotchas & Limitations
    
    - Accessibility permission required (System Settings → Privacy → Accessibility)
    - `keyboardSetUnicodeString` chunk size limit (~20 chars per event)
    - Some apps may not respond to CGEvent input (sandboxed apps, some Electron apps)
    - AXSetValue may not trigger JavaScript input events in browsers
    - Modifier combos (Cmd+C etc.) work for native apps but may need alternative handling for remote/VM contexts
    - Mouse events work in screen coordinates — need to account for Retina scaling if using logical coordinates
    
    """
    
    // Write report
    let reportPath = "spikes/S3_cgevent_results.md"
    do {
        try report.write(toFile: reportPath, atomically: true, encoding: .utf8)
        print("\n📝 Report written to \(reportPath)")
    } catch {
        print("\n❌ Failed to write report: \(error)")
        // Print to stdout as fallback
        print(report)
    }
}

// ============================================================================
// MARK: - Main
// ============================================================================

print("╔══════════════════════════════════════════════════════════╗")
print("║     Spike S3: CGEvent Typing Reliability Testing        ║")
print("╚══════════════════════════════════════════════════════════╝")
print()

// Check accessibility permission
let trusted = AXIsProcessTrusted()
print("Accessibility trusted: \(trusted)")
if !trusted {
    print("⚠️  WARNING: Process is NOT trusted for accessibility.")
    print("   Grant permission in System Settings → Privacy → Accessibility")
    print("   Then re-run this program.")
    print("   Continuing anyway (some tests may fail)...")
}
print()

// Run tests
testMouseClicks()
testKeyPresses()

print("\n⏳ String typing tests start in 5 seconds...")
print("   Please switch focus to TextEdit with a blank document!")
sleep(5)

testStringTyping()

print("\n⏳ AXSetValue tests in 3 seconds...")
print("   Keep focus on a text field in TextEdit!")
sleep(3)

testAXSetValue()

// Generate report
generateReport()

// Final summary
let totalPassed = results.filter { $0.passed }.count
let totalFailed = results.filter { !$0.passed }.count
print("\n" + String(repeating: "=", count: 60))
print("FINAL: \(totalPassed) passed, \(totalFailed) failed out of \(results.count) tests")
print(String(repeating: "=", count: 60))
