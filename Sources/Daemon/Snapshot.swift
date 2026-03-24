import Foundation
import ApplicationServices
import AgentComputerShared

// MARK: - AX Tree Node

struct AXNode {
    let role: String
    let title: String?
    let description: String?
    let value: String?
    let frame: CGRect?
    let actions: [String]
    let isInteractive: Bool
    let children: [AXNode]
    let depth: Int
    let pathSegment: PathSegment
}

// MARK: - Safe AX Attribute Access

func safeGetString(_ element: AXUIElement, _ attr: String) -> String? {
    var value: AnyObject?
    let err = AXUIElementCopyAttributeValue(element, attr as CFString, &value)
    guard err == .success else { return nil }
    return value as? String
}

func safeGetFrame(_ element: AXUIElement) -> CGRect? {
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

func safeGetActions(_ element: AXUIElement) -> [String] {
    var actions: CFArray?
    let err = AXUIElementCopyActionNames(element, &actions)
    guard err == .success, let acts = actions as? [String] else { return [] }
    return acts
}

func safeGetChildren(_ element: AXUIElement) -> [AXUIElement] {
    var value: AnyObject?
    let err = AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &value)
    guard err == .success else { return [] }
    return (value as? [AXUIElement]) ?? []
}

// MARK: - Batch Attribute Fetching

func batchGetAttributes(_ element: AXUIElement) -> (role: String?, title: String?, description: String?, value: String?) {
    let attrs = [
        kAXRoleAttribute,
        kAXTitleAttribute,
        kAXDescriptionAttribute,
        kAXValueAttribute
    ] as CFArray
    
    var values: CFArray?
    let err = AXUIElementCopyMultipleAttributeValues(element, attrs, .stopOnError, &values)
    
    if err == .success || err == .attributeUnsupported, let vals = values as? [Any] {
        func str(_ idx: Int) -> String? {
            guard idx < vals.count else { return nil }
            let v = vals[idx]
            if v is NSNull || CFGetTypeID(v as CFTypeRef) == CFGetTypeID(kCFNull) { return nil }
            // Check for AXError sentinel
            if let axErr = v as? AXError, axErr != .success { return nil }
            return v as? String
        }
        return (str(0), str(1), str(2), str(3))
    }
    
    // Fallback to individual fetches
    return (
        safeGetString(element, kAXRoleAttribute),
        safeGetString(element, kAXTitleAttribute),
        safeGetString(element, kAXDescriptionAttribute),
        safeGetString(element, kAXValueAttribute)
    )
}

// MARK: - AX Tree Traversal

func traverseAXTree(
    element: AXUIElement,
    depth: Int,
    maxDepth: Int,
    deadline: CFAbsoluteTime,
    parentRole: String = "",
    indexInParent: Int = 0,
    roleCounts: inout [String: Int]
) -> AXNode? {
    // Check timeout
    if CFAbsoluteTimeGetCurrent() > deadline { return nil }
    
    // Check depth
    if depth > maxDepth { return nil }
    
    let attrs = batchGetAttributes(element)
    let role = attrs.role ?? "AXUnknown"
    let frame = safeGetFrame(element)
    let actions = safeGetActions(element)
    let isInteractiveElement = interactiveRoles.contains(role)
    
    // Track role counts for path segments
    let segment = PathSegment(role: role, index: indexInParent)
    
    // Get children and recurse
    let childElements = safeGetChildren(element)
    var childNodes: [AXNode] = []
    var childRoleCounts = [String: Int]()
    
    for child in childElements {
        // Get child role for path indexing
        let childRole = safeGetString(child, kAXRoleAttribute) ?? "AXUnknown"
        let childIdx = childRoleCounts[childRole, default: 0]
        childRoleCounts[childRole] = childIdx + 1
        
        if let childNode = traverseAXTree(
            element: child,
            depth: depth + 1,
            maxDepth: maxDepth,
            deadline: deadline,
            parentRole: role,
            indexInParent: childIdx,
            roleCounts: &childRoleCounts
        ) {
            childNodes.append(childNode)
        }
    }
    
    return AXNode(
        role: role,
        title: attrs.title,
        description: attrs.description,
        value: isInteractiveElement ? attrs.value : nil,
        frame: frame,
        actions: actions,
        isInteractive: isInteractiveElement,
        children: childNodes,
        depth: depth,
        pathSegment: segment
    )
}

// MARK: - Snapshot for PID

func takeSnapshot(pid: pid_t, depth: Int = 10, timeoutSeconds: Double = 3.0) -> (tree: [AXNode], appName: String, windowTitle: String?) {
    let appElement = AXUIElementCreateApplication(pid)
    let deadline = CFAbsoluteTimeGetCurrent() + timeoutSeconds
    
    // Get app name
    let appName = safeGetString(appElement, kAXTitleAttribute) ?? "Unknown"
    
    // Get windows
    var windowsValue: AnyObject?
    let err = AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &windowsValue)
    
    var windowTitle: String? = nil
    var rootNodes: [AXNode] = []
    
    if err == .success, let windows = windowsValue as? [AXUIElement] {
        for (winIdx, window) in windows.enumerated() {
            let winTitle = safeGetString(window, kAXTitleAttribute)
            if winIdx == 0 { windowTitle = winTitle }
            
            var roleCounts = [String: Int]()
            if let node = traverseAXTree(
                element: window,
                depth: 0,
                maxDepth: depth,
                deadline: deadline,
                indexInParent: winIdx,
                roleCounts: &roleCounts
            ) {
                rootNodes.append(node)
            }
        }
    } else {
        // No windows — traverse the app element itself
        var roleCounts = [String: Int]()
        if let node = traverseAXTree(
            element: appElement,
            depth: 0,
            maxDepth: depth,
            deadline: deadline,
            roleCounts: &roleCounts
        ) {
            rootNodes.append(node)
        }
    }
    
    return (rootNodes, appName, windowTitle)
}

// MARK: - Get Frontmost App

func getFrontmostApp() -> (name: String, pid: pid_t)? {
    let systemWide = AXUIElementCreateSystemWide()
    var focusedApp: AnyObject?
    let err = AXUIElementCopyAttributeValue(systemWide, kAXFocusedApplicationAttribute as CFString, &focusedApp)
    guard err == .success, let app = focusedApp else { return nil }
    
    let appElement = app as! AXUIElement
    var pidValue: pid_t = 0
    AXUIElementGetPid(appElement, &pidValue)
    let name = safeGetString(appElement, kAXTitleAttribute) ?? "Unknown"
    return (name, pidValue)
}

// MARK: - Snapshot Text Formatter

func formatSnapshotText(tree: [AXNode], appName: String, windowTitle: String?, interactiveOnly: Bool) -> (text: String, refs: [ElementRef]) {
    var output = ""
    var refs: [ElementRef] = []
    var refCounter = 1
    
    // Header
    if let winTitle = windowTitle, !winTitle.isEmpty {
        output += "[\(appName) — \(winTitle)]\n"
    } else {
        output += "[\(appName)]\n"
    }
    
    func formatNode(_ node: AXNode, indent: Int, path: [PathSegment]) {
        let currentPath = path + [node.pathSegment]
        let indentStr = String(repeating: "  ", count: indent)
        
        if node.isInteractive {
            let refId = "e\(refCounter)"
            refCounter += 1
            
            // Build label
            let label = node.title ?? node.description ?? node.value
            var line = "\(indentStr)@\(refId) \(node.role)"
            if let l = label, !l.isEmpty {
                // Truncate long labels
                let truncated = l.count > 60 ? String(l.prefix(57)) + "..." : l
                line += " \"\(truncated)\""
            }
            output += line + "\n"
            
            // Store ref
            let frame: Rect
            if let f = node.frame {
                frame = Rect(x: f.origin.x, y: f.origin.y, width: f.size.width, height: f.size.height)
            } else {
                frame = Rect(x: 0, y: 0, width: 0, height: 0)
            }
            
            refs.append(ElementRef(
                id: refId,
                role: node.role,
                label: label,
                frame: frame,
                axPath: currentPath,
                actions: node.actions
            ))
        } else if !interactiveOnly {
            // Show structural parents for context
            let role = node.role
            let contextRoles: Set<String> = [
                "AXWindow", "AXToolbar", "AXGroup", "AXScrollArea",
                "AXSplitGroup", "AXTabGroup", "AXSheet", "AXMenuBar",
                "AXList", "AXOutline", "AXTable", "AXBrowser",
                "AXWebArea", "AXApplication"
            ]
            
            let hasInteractiveDescendants = nodeHasInteractiveDescendants(node)
            if contextRoles.contains(role) && hasInteractiveDescendants {
                let label = node.title ?? node.description
                var line = "\(indentStr)\(role)"
                if let l = label, !l.isEmpty {
                    let truncated = l.count > 40 ? String(l.prefix(37)) + "..." : l
                    line += " \"\(truncated)\""
                }
                output += line + "\n"
            }
        }
        
        // Recurse children
        for child in node.children {
            formatNode(child, indent: node.isInteractive || !interactiveOnly ? indent + 1 : indent, path: currentPath)
        }
    }
    
    for node in tree {
        formatNode(node, indent: 1, path: [])
    }
    
    return (output, refs)
}

func nodeHasInteractiveDescendants(_ node: AXNode) -> Bool {
    if node.isInteractive { return true }
    for child in node.children {
        if nodeHasInteractiveDescendants(child) { return true }
    }
    return false
}

// MARK: - Snapshot Command Handler

func handleSnapshot(id: String, args: SnapshotArgs, startTime: CFAbsoluteTime) -> Response {
    func elapsed() -> Double { (CFAbsoluteTimeGetCurrent() - startTime) * 1000.0 }
    
    // Determine target app
    let targetPid: pid_t
    let targetAppName: String
    
    if let appName = args.app {
        // Find app by name
        guard let app = findRunningApp(name: appName) else {
            return Response.fail(id: id, error: Errors.appNotFound(appName), elapsed: elapsed())
        }
        targetPid = app.pid
        targetAppName = app.name
    } else {
        // Use frontmost app
        guard let front = getFrontmostApp() else {
            return Response.fail(id: id, error: Errors.axError(detail: "Could not determine frontmost application"), elapsed: elapsed())
        }
        targetPid = front.pid
        targetAppName = front.name
    }
    
    let depth = args.depth ?? 10
    let (tree, appName, windowTitle) = takeSnapshot(pid: targetPid, depth: depth)
    
    let (text, refs) = formatSnapshotText(
        tree: tree,
        appName: appName.isEmpty ? targetAppName : appName,
        windowTitle: windowTitle,
        interactiveOnly: args.interactive
    )
    
    // Update global ref map
    globalRefMap.update(refs: refs, appName: appName.isEmpty ? targetAppName : appName, pid: targetPid)
    
    let data = SnapshotData(
        text: text,
        refCount: refs.count,
        app: appName.isEmpty ? targetAppName : appName,
        window: windowTitle
    )
    
    return Response.ok(id: id, data: .snapshot(data), elapsed: elapsed())
}
