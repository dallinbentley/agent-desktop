import Foundation

// MARK: - Element Ref (stored in daemon's RefMap)

public struct ElementRef: Codable {
    public let id: String           // "e1", "e2", ...
    public let role: String         // "AXButton", "AXTextField", ...
    public let label: String?       // human-readable label
    public let frame: Rect          // screen coordinates
    public let axPath: [PathSegment] // role + index chain for re-traversal
    public let actions: [String]    // available AX actions
    
    public init(id: String, role: String, label: String?, frame: Rect, axPath: [PathSegment], actions: [String]) {
        self.id = id
        self.role = role
        self.label = label
        self.frame = frame
        self.axPath = axPath
        self.actions = actions
    }
    
    /// Center point of the element's frame
    public var center: Point {
        Point(x: frame.x + frame.width / 2, y: frame.y + frame.height / 2)
    }
}

public struct PathSegment: Codable {
    public let role: String
    public let index: Int
    
    public init(role: String, index: Int) {
        self.role = role
        self.index = index
    }
}

public struct Rect: Codable {
    public var x: Double
    public var y: Double
    public var width: Double
    public var height: Double
    
    public init(x: Double, y: Double, width: Double, height: Double) {
        self.x = x
        self.y = y
        self.width = width
        self.height = height
    }
}

// MARK: - Error Info

public struct ErrorInfo: Codable {
    public let code: String
    public let message: String
    public let suggestion: String?
    
    public init(code: String, message: String, suggestion: String? = nil) {
        self.code = code
        self.message = message
        self.suggestion = suggestion
    }
}

// MARK: - Error Codes

public enum ErrorCode {
    public static let refNotFound = "REF_NOT_FOUND"
    public static let refStale = "REF_STALE"
    public static let noRefMap = "NO_REF_MAP"
    public static let appNotFound = "APP_NOT_FOUND"
    public static let windowNotFound = "WINDOW_NOT_FOUND"
    public static let permissionDenied = "PERMISSION_DENIED"
    public static let timeout = "TIMEOUT"
    public static let axError = "AX_ERROR"
    public static let inputError = "INPUT_ERROR"
    public static let invalidCommand = "INVALID_COMMAND"
    public static let daemonError = "DAEMON_ERROR"
}

// MARK: - Interactive Roles

public let interactiveRoles: Set<String> = [
    "AXButton",
    "AXTextField",
    "AXTextArea",
    "AXCheckBox",
    "AXRadioButton",
    "AXPopUpButton",
    "AXComboBox",
    "AXSlider",
    "AXLink",
    "AXMenuItem",
    "AXMenuButton",
    "AXTab",
    "AXTabGroup",
    "AXScrollArea",
    "AXTable",
    "AXOutline",
    "AXSwitch",
    "AXSearchField",
    "AXIncrementor",
]

// MARK: - Key Mapping

public let keyNameToCode: [String: UInt16] = [
    "enter": 36, "return": 36,
    "tab": 48,
    "escape": 53, "esc": 53,
    "space": 49,
    "delete": 51, "backspace": 51,
    "forwarddelete": 117,
    "up": 126,
    "down": 125,
    "left": 123,
    "right": 124,
    "home": 115,
    "end": 119,
    "pageup": 116,
    "pagedown": 121,
    "f1": 122, "f2": 120, "f3": 99, "f4": 118,
    "f5": 96, "f6": 97, "f7": 98, "f8": 100,
    "f9": 101, "f10": 109, "f11": 103, "f12": 111,
    // Letter keys
    "a": 0, "b": 11, "c": 8, "d": 2, "e": 14, "f": 3,
    "g": 5, "h": 4, "i": 34, "j": 38, "k": 40, "l": 37,
    "m": 46, "n": 45, "o": 31, "p": 35, "q": 12, "r": 15,
    "s": 1, "t": 17, "u": 32, "v": 9, "w": 13, "x": 7,
    "y": 16, "z": 6,
    // Number keys
    "0": 29, "1": 18, "2": 19, "3": 20, "4": 21,
    "5": 23, "6": 22, "7": 26, "8": 28, "9": 25,
    // Symbols
    "-": 27, "=": 24, "[": 33, "]": 30,
    ";": 41, "'": 39, ",": 43, ".": 47,
    "/": 44, "\\": 42, "`": 50,
]

// MARK: - Socket Path

public let daemonSocketDir = FileManager.default.homeDirectoryForCurrentUser
    .appendingPathComponent(".agent-computer")

public let daemonSocketPath = daemonSocketDir
    .appendingPathComponent("daemon.sock")
