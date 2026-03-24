import Foundation
import ApplicationServices
import AgentComputerShared

// MARK: - Global Ref Map

class RefMap {
    private var refs: [String: ElementRef] = [:]  // "e1" -> ElementRef
    private var orderedRefs: [ElementRef] = []
    private var appName: String = ""
    private var pid: pid_t = 0
    private var createdAt: CFAbsoluteTime = 0
    
    var count: Int { refs.count }
    var ageMs: Double? {
        guard createdAt > 0 else { return nil }
        return (CFAbsoluteTimeGetCurrent() - createdAt) * 1000.0
    }
    var currentApp: String { appName }
    var currentPid: pid_t { pid }
    
    func update(refs newRefs: [ElementRef], appName: String, pid: pid_t) {
        self.refs.removeAll()
        self.orderedRefs = newRefs
        self.appName = appName
        self.pid = pid
        self.createdAt = CFAbsoluteTimeGetCurrent()
        
        for ref in newRefs {
            self.refs[ref.id] = ref
        }
    }
    
    func resolve(ref: String) -> ElementRef? {
        // Strip @ prefix if present
        let cleanRef = ref.hasPrefix("@") ? String(ref.dropFirst()) : ref
        return refs[cleanRef]
    }
    
    func resolveToCoordinates(ref: String) -> CGPoint? {
        guard let elementRef = resolve(ref: ref) else { return nil }
        return CGPoint(x: elementRef.center.x, y: elementRef.center.y)
    }
    
    func resolveToAXElement(ref: String) -> AXUIElement? {
        guard let elementRef = resolve(ref: ref) else { return nil }
        return reTraverseToElement(path: elementRef.axPath, pid: pid)
    }
    
    func isEmpty() -> Bool {
        return refs.isEmpty
    }
}

// MARK: - Path Re-traversal

func reTraverseToElement(path: [PathSegment], pid: pid_t) -> AXUIElement? {
    guard !path.isEmpty else { return nil }
    
    let appElement = AXUIElementCreateApplication(pid)
    
    // First segment is the window
    var windowsValue: AnyObject?
    let err = AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &windowsValue)
    guard err == .success, let windows = windowsValue as? [AXUIElement] else { return nil }
    
    // The first segment's index tells us which window
    let firstSeg = path[0]
    guard firstSeg.index < windows.count else { return nil }
    var current = windows[firstSeg.index]
    
    // Walk remaining path segments
    for seg in path.dropFirst() {
        let children = safeGetChildren(current)
        
        // Find child matching role and index
        var roleCount = 0
        var found = false
        for child in children {
            let childRole = safeGetString(child, kAXRoleAttribute) ?? "AXUnknown"
            if childRole == seg.role {
                if roleCount == seg.index {
                    current = child
                    found = true
                    break
                }
                roleCount += 1
            }
        }
        
        if !found { return nil }
    }
    
    return current
}

// MARK: - Global Instance

var globalRefMap = RefMap()
