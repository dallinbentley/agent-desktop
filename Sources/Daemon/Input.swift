import Foundation
import ApplicationServices
import AgentComputerShared

// MARK: - Mouse Input

func mouseClick(at point: CGPoint, button: CGMouseButton = .left, clickCount: Int = 1) {
    // Use CGWarpMouseCursorPosition for reliable absolute positioning
    // (NOT .mouseMoved which drifts up to 288px!)
    CGWarpMouseCursorPosition(point)
    usleep(10_000) // 10ms settle
    
    let downType: CGEventType
    let upType: CGEventType
    
    switch button {
    case .left:
        downType = .leftMouseDown
        upType = .leftMouseUp
    case .right:
        downType = .rightMouseDown
        upType = .rightMouseUp
    default:
        downType = .leftMouseDown
        upType = .leftMouseUp
    }
    
    if clickCount == 2 {
        // Double click
        if let down1 = CGEvent(mouseEventSource: nil, mouseType: downType, mouseCursorPosition: point, mouseButton: button),
           let up1 = CGEvent(mouseEventSource: nil, mouseType: upType, mouseCursorPosition: point, mouseButton: button),
           let down2 = CGEvent(mouseEventSource: nil, mouseType: downType, mouseCursorPosition: point, mouseButton: button),
           let up2 = CGEvent(mouseEventSource: nil, mouseType: upType, mouseCursorPosition: point, mouseButton: button) {
            down1.setIntegerValueField(.mouseEventClickState, value: 1)
            up1.setIntegerValueField(.mouseEventClickState, value: 1)
            down2.setIntegerValueField(.mouseEventClickState, value: 2)
            up2.setIntegerValueField(.mouseEventClickState, value: 2)
            
            down1.post(tap: .cghidEventTap)
            usleep(10_000)
            up1.post(tap: .cghidEventTap)
            usleep(10_000)
            down2.post(tap: .cghidEventTap)
            usleep(10_000)
            up2.post(tap: .cghidEventTap)
        }
    } else {
        // Single click
        if let down = CGEvent(mouseEventSource: nil, mouseType: downType, mouseCursorPosition: point, mouseButton: button),
           let up = CGEvent(mouseEventSource: nil, mouseType: upType, mouseCursorPosition: point, mouseButton: button) {
            down.setIntegerValueField(.mouseEventClickState, value: Int64(clickCount))
            up.setIntegerValueField(.mouseEventClickState, value: Int64(clickCount))
            down.post(tap: .cghidEventTap)
            usleep(10_000)
            up.post(tap: .cghidEventTap)
        }
    }
    usleep(30_000) // 30ms settle after click
}

// MARK: - Keyboard Input

func keyPress(keycode: UInt16, modifiers: CGEventFlags = []) {
    if let keyDown = CGEvent(keyboardEventSource: nil, virtualKey: keycode, keyDown: true),
       let keyUp = CGEvent(keyboardEventSource: nil, virtualKey: keycode, keyDown: false) {
        keyDown.flags = modifiers
        keyUp.flags = modifiers
        keyDown.post(tap: .cghidEventTap)
        usleep(20_000)
        keyUp.post(tap: .cghidEventTap)
    }
    usleep(10_000)
}

func parseModifierFlags(_ modifiers: [String]?) -> CGEventFlags {
    var flags = CGEventFlags()
    guard let mods = modifiers else { return flags }
    
    for mod in mods {
        switch mod.lowercased() {
        case "cmd", "command", "meta":
            flags.insert(.maskCommand)
        case "shift":
            flags.insert(.maskShift)
        case "alt", "option", "opt":
            flags.insert(.maskAlternate)
        case "ctrl", "control":
            flags.insert(.maskControl)
        default:
            break
        }
    }
    return flags
}

// MARK: - String Typing

func typeString(_ text: String) {
    let utf16 = Array(text.utf16)
    let chunkSize = 20
    var offset = 0
    
    while offset < utf16.count {
        // Find the next chunk, splitting on newlines
        var end = min(offset + chunkSize, utf16.count)
        var hasNewline = false
        
        for i in offset..<end {
            let idx = text.index(text.utf16.startIndex, offsetBy: i)
            if text.utf16[idx] == 0x000A { // newline
                end = i
                hasNewline = true
                break
            }
        }
        
        if hasNewline && end == offset {
            // Current position is a newline — press Return
            keyPress(keycode: 36) // Return
            offset += 1
            continue
        }
        
        if end > offset {
            let chunk = Array(utf16[offset..<end])
            
            guard let keyDown = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: true),
                  let keyUp = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: false) else { break }
            
            chunk.withUnsafeBufferPointer { buffer in
                guard let base = buffer.baseAddress else { return }
                keyDown.keyboardSetUnicodeString(stringLength: chunk.count, unicodeString: base)
                keyUp.keyboardSetUnicodeString(stringLength: 0, unicodeString: base)
            }
            
            keyDown.post(tap: .cghidEventTap)
            usleep(20_000)
            keyUp.post(tap: .cghidEventTap)
            usleep(20_000)
        }
        
        offset = end + (hasNewline ? 1 : 0)
    }
}

// MARK: - Fill Element (AX Selection-Replace)

func fillElement(axElement: AXUIElement, text: String) -> Bool {
    // Strategy: Select all text via AX, then replace with new text
    // Do NOT use kAXValueAttribute — it silently fails!
    
    // First, get current text length to select all
    var currentValue: AnyObject?
    let readErr = AXUIElementCopyAttributeValue(axElement, kAXValueAttribute as CFString, &currentValue)
    
    let textLength: Int
    if readErr == .success, let currentText = currentValue as? String {
        textLength = currentText.count
    } else {
        textLength = 100000 // large number to select everything
    }
    
    // Set selection range to cover all text
    var range = CFRange(location: 0, length: textLength)
    guard let axRange = AXValueCreate(.cfRange, &range) else { return false }
    
    let selRangeErr = AXUIElementSetAttributeValue(axElement, kAXSelectedTextRangeAttribute as CFString, axRange)
    if selRangeErr != .success {
        // Fallback: try Cmd+A via CGEvent
        keyPress(keycode: 0, modifiers: .maskCommand) // Cmd+A
        usleep(50_000)
    }
    
    // Replace selected text with new text
    let replaceErr = AXUIElementSetAttributeValue(axElement, kAXSelectedTextAttribute as CFString, text as CFTypeRef)
    if replaceErr == .success {
        return true
    }
    
    // Fallback: type the text via keyboard
    typeString(text)
    return true
}

// MARK: - Scroll

func scroll(direction: String, amount: Int = 3) {
    var dx: Int32 = 0
    var dy: Int32 = 0
    
    switch direction.lowercased() {
    case "up":
        dy = Int32(amount)
    case "down":
        dy = -Int32(amount)
    case "left":
        dx = Int32(amount)
    case "right":
        dx = -Int32(amount)
    default:
        return
    }
    
    if let scrollEvent = CGEvent(scrollWheelEvent2Source: nil, units: .line, wheelCount: 2, wheel1: dy, wheel2: dx, wheel3: 0) {
        scrollEvent.post(tap: .cghidEventTap)
    }
    usleep(30_000)
}

// MARK: - Command Handlers

func handleClick(id: String, args: ClickArgs, startTime: CFAbsoluteTime) -> Response {
    func elapsed() -> Double { (CFAbsoluteTimeGetCurrent() - startTime) * 1000.0 }
    
    let point: CGPoint
    var elementInfo: ElementInfo? = nil
    var refStr: String? = nil
    
    if let ref = args.ref {
        // Resolve ref
        if globalRefMap.isEmpty() {
            return Response.fail(id: id, error: Errors.noRefMap(), elapsed: elapsed())
        }
        guard let elementRef = globalRefMap.resolve(ref: ref) else {
            return Response.fail(id: id, error: Errors.refNotFound(ref), elapsed: elapsed())
        }
        
        // Try path re-traversal first for accurate coordinates
        if let axElement = globalRefMap.resolveToAXElement(ref: ref),
           let frame = safeGetFrame(axElement) {
            point = CGPoint(x: frame.midX, y: frame.midY)
        } else {
            // Fallback to stored coordinates
            point = CGPoint(x: elementRef.center.x, y: elementRef.center.y)
        }
        
        refStr = ref
        elementInfo = ElementInfo(role: elementRef.role, label: elementRef.label)
    } else if let x = args.x, let y = args.y {
        point = CGPoint(x: x, y: y)
    } else {
        return Response.fail(id: id, error: Errors.invalidCommand("click requires ref or x/y coordinates"), elapsed: elapsed())
    }
    
    let button: CGMouseButton = args.right ? .right : .left
    let clickCount = args.double ? 2 : 1
    
    mouseClick(at: point, button: button, clickCount: clickCount)
    
    let data = ClickData(
        ref: refStr,
        coordinates: Point(x: Double(point.x), y: Double(point.y)),
        element: elementInfo
    )
    
    return Response.ok(id: id, data: .click(data), elapsed: elapsed())
}

func handleType(id: String, args: TypeArgs, startTime: CFAbsoluteTime) -> Response {
    func elapsed() -> Double { (CFAbsoluteTimeGetCurrent() - startTime) * 1000.0 }
    
    // If ref is provided, click on it first to focus
    if let ref = args.ref {
        if globalRefMap.isEmpty() {
            return Response.fail(id: id, error: Errors.noRefMap(), elapsed: elapsed())
        }
        guard let elementRef = globalRefMap.resolve(ref: ref) else {
            return Response.fail(id: id, error: Errors.refNotFound(ref), elapsed: elapsed())
        }
        
        let point: CGPoint
        if let axElement = globalRefMap.resolveToAXElement(ref: ref),
           let frame = safeGetFrame(axElement) {
            point = CGPoint(x: frame.midX, y: frame.midY)
        } else {
            point = CGPoint(x: elementRef.center.x, y: elementRef.center.y)
        }
        
        mouseClick(at: point)
        usleep(50_000)
    }
    
    typeString(args.text)
    
    let data = TypeData(ref: args.ref, text: args.text)
    return Response.ok(id: id, data: .type(data), elapsed: elapsed())
}

func handleFill(id: String, args: FillArgs, startTime: CFAbsoluteTime) -> Response {
    func elapsed() -> Double { (CFAbsoluteTimeGetCurrent() - startTime) * 1000.0 }
    
    if globalRefMap.isEmpty() {
        return Response.fail(id: id, error: Errors.noRefMap(), elapsed: elapsed())
    }
    guard let _ = globalRefMap.resolve(ref: args.ref) else {
        return Response.fail(id: id, error: Errors.refNotFound(args.ref), elapsed: elapsed())
    }
    
    guard let axElement = globalRefMap.resolveToAXElement(ref: args.ref) else {
        return Response.fail(id: id, error: Errors.refStale(args.ref), elapsed: elapsed())
    }
    
    let success = fillElement(axElement: axElement, text: args.text)
    if !success {
        return Response.fail(id: id, error: Errors.inputError(detail: "Failed to fill element \(args.ref)"), elapsed: elapsed())
    }
    
    let data = FillData(ref: args.ref, text: args.text)
    return Response.ok(id: id, data: .fill(data), elapsed: elapsed())
}

func handlePress(id: String, args: PressArgs, startTime: CFAbsoluteTime) -> Response {
    func elapsed() -> Double { (CFAbsoluteTimeGetCurrent() - startTime) * 1000.0 }
    
    let keyName = args.key.lowercased()
    guard let keycode = keyNameToCode[keyName] else {
        return Response.fail(id: id, error: Errors.invalidCommand("Unknown key: '\(args.key)'. Valid keys: \(keyNameToCode.keys.sorted().joined(separator: ", "))"), elapsed: elapsed())
    }
    
    let flags = parseModifierFlags(args.modifiers)
    keyPress(keycode: keycode, modifiers: flags)
    
    let data = PressData(key: args.key, modifiers: args.modifiers ?? [])
    return Response.ok(id: id, data: .press(data), elapsed: elapsed())
}

func handleScroll(id: String, args: ScrollArgs, startTime: CFAbsoluteTime) -> Response {
    func elapsed() -> Double { (CFAbsoluteTimeGetCurrent() - startTime) * 1000.0 }
    
    let amount = args.amount ?? 3
    
    // If ref provided, move mouse there first
    if let ref = args.ref {
        if let point = globalRefMap.resolveToCoordinates(ref: ref) {
            CGWarpMouseCursorPosition(point)
            usleep(10_000)
        }
    }
    
    scroll(direction: args.direction, amount: amount)
    
    let data = ScrollData(direction: args.direction, amount: amount)
    return Response.ok(id: id, data: .scroll(data), elapsed: elapsed())
}
