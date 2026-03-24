import Foundation

// MARK: - Request

public struct Request: Codable {
    public let id: String
    public let command: String
    public let args: CommandArgs
    public let options: RequestOptions?
    
    public init(id: String, command: String, args: CommandArgs, options: RequestOptions? = nil) {
        self.id = id
        self.command = command
        self.args = args
        self.options = options
    }
}

public struct RequestOptions: Codable {
    public var timeout: Int?
    public var json: Bool?
    public var verbose: Bool?
    
    public init(timeout: Int? = nil, json: Bool? = nil, verbose: Bool? = nil) {
        self.timeout = timeout
        self.json = json
        self.verbose = verbose
    }
}

// MARK: - Command Args

public enum CommandArgs: Codable {
    case snapshot(SnapshotArgs)
    case click(ClickArgs)
    case fill(FillArgs)
    case type(TypeArgs)
    case press(PressArgs)
    case scroll(ScrollArgs)
    case screenshot(ScreenshotArgs)
    case open(OpenArgs)
    case get(GetArgs)
    case status
    
    private enum CodingKeys: String, CodingKey {
        case type
    }
    
    // Custom encoding to flatten for JSON protocol
    public func encode(to encoder: Encoder) throws {
        switch self {
        case .snapshot(let args): try args.encode(to: encoder)
        case .click(let args): try args.encode(to: encoder)
        case .fill(let args): try args.encode(to: encoder)
        case .type(let args): try args.encode(to: encoder)
        case .press(let args): try args.encode(to: encoder)
        case .scroll(let args): try args.encode(to: encoder)
        case .screenshot(let args): try args.encode(to: encoder)
        case .open(let args): try args.encode(to: encoder)
        case .get(let args): try args.encode(to: encoder)
        case .status: try [String: String]().encode(to: encoder)
        }
    }
    
    public init(from decoder: Decoder) throws {
        // Decoding is done by the daemon based on command name, not auto-decoded
        fatalError("CommandArgs should be decoded manually based on command name")
    }
}

// MARK: - Specific Arg Types

public struct SnapshotArgs: Codable {
    public var interactive: Bool
    public var compact: Bool
    public var depth: Int?
    public var app: String?
    
    public init(interactive: Bool = true, compact: Bool = false, depth: Int? = nil, app: String? = nil) {
        self.interactive = interactive
        self.compact = compact
        self.depth = depth
        self.app = app
    }
}

public struct ClickArgs: Codable {
    public var ref: String?
    public var x: Double?
    public var y: Double?
    public var double: Bool
    public var right: Bool
    
    public init(ref: String? = nil, x: Double? = nil, y: Double? = nil, double: Bool = false, right: Bool = false) {
        self.ref = ref
        self.x = x
        self.y = y
        self.double = double
        self.right = right
    }
}

public struct FillArgs: Codable {
    public var ref: String
    public var text: String
    
    public init(ref: String, text: String) {
        self.ref = ref
        self.text = text
    }
}

public struct TypeArgs: Codable {
    public var ref: String?
    public var text: String
    
    public init(ref: String? = nil, text: String) {
        self.ref = ref
        self.text = text
    }
}

public struct PressArgs: Codable {
    public var key: String
    public var modifiers: [String]?
    
    public init(key: String, modifiers: [String]? = nil) {
        self.key = key
        self.modifiers = modifiers
    }
}

public struct ScrollArgs: Codable {
    public var direction: String
    public var amount: Int?
    public var ref: String?
    
    public init(direction: String, amount: Int? = nil, ref: String? = nil) {
        self.direction = direction
        self.amount = amount
        self.ref = ref
    }
}

public struct ScreenshotArgs: Codable {
    public var full: Bool
    public var app: String?
    
    public init(full: Bool = false, app: String? = nil) {
        self.full = full
        self.app = app
    }
}

public struct OpenArgs: Codable {
    public var target: String
    
    public init(target: String) {
        self.target = target
    }
}

public struct GetArgs: Codable {
    public var what: String  // "text", "title", "apps", "windows"
    public var ref: String?
    public var app: String?
    
    public init(what: String, ref: String? = nil, app: String? = nil) {
        self.what = what
        self.ref = ref
        self.app = app
    }
}

// MARK: - Response

public struct Response: Codable {
    public let id: String
    public let success: Bool
    public let data: ResponseData?
    public let error: ErrorInfo?
    public let timing: Timing?
    
    public init(id: String, success: Bool, data: ResponseData? = nil, error: ErrorInfo? = nil, timing: Timing? = nil) {
        self.id = id
        self.success = success
        self.data = data
        self.error = error
        self.timing = timing
    }
    
    public static func ok(id: String, data: ResponseData, elapsed: Double) -> Response {
        Response(id: id, success: true, data: data, timing: Timing(elapsed_ms: elapsed))
    }
    
    public static func fail(id: String, error: ErrorInfo, elapsed: Double = 0) -> Response {
        Response(id: id, success: false, error: error, timing: Timing(elapsed_ms: elapsed))
    }
}

public struct Timing: Codable {
    public let elapsed_ms: Double
    
    public init(elapsed_ms: Double) {
        self.elapsed_ms = elapsed_ms
    }
}

// MARK: - Response Data

public enum ResponseData: Codable {
    case snapshot(SnapshotData)
    case click(ClickData)
    case fill(FillData)
    case type(TypeData)
    case press(PressData)
    case scroll(ScrollData)
    case screenshot(ScreenshotData)
    case open(OpenData)
    case getApps(GetAppsData)
    case getText(GetTextData)
    case status(StatusData)
    case raw(String) // fallback
    
    private enum DiscriminatorKey: String, CodingKey { case _type }
    
    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: DiscriminatorKey.self)
        switch self {
        case .snapshot(let d): try container.encode("snapshot", forKey: ._type); try d.encode(to: encoder)
        case .click(let d): try container.encode("click", forKey: ._type); try d.encode(to: encoder)
        case .fill(let d): try container.encode("fill", forKey: ._type); try d.encode(to: encoder)
        case .type(let d): try container.encode("type", forKey: ._type); try d.encode(to: encoder)
        case .press(let d): try container.encode("press", forKey: ._type); try d.encode(to: encoder)
        case .scroll(let d): try container.encode("scroll", forKey: ._type); try d.encode(to: encoder)
        case .screenshot(let d): try container.encode("screenshot", forKey: ._type); try d.encode(to: encoder)
        case .open(let d): try container.encode("open", forKey: ._type); try d.encode(to: encoder)
        case .getApps(let d): try container.encode("getApps", forKey: ._type); try d.encode(to: encoder)
        case .getText(let d): try container.encode("getText", forKey: ._type); try d.encode(to: encoder)
        case .status(let d): try container.encode("status", forKey: ._type); try d.encode(to: encoder)
        case .raw(let s): try container.encode("raw", forKey: ._type); try s.encode(to: encoder)
        }
    }
    
    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: DiscriminatorKey.self)
        let type = try container.decode(String.self, forKey: ._type)
        switch type {
        case "snapshot": self = .snapshot(try SnapshotData(from: decoder))
        case "click": self = .click(try ClickData(from: decoder))
        case "fill": self = .fill(try FillData(from: decoder))
        case "type": self = .type(try TypeData(from: decoder))
        case "press": self = .press(try PressData(from: decoder))
        case "scroll": self = .scroll(try ScrollData(from: decoder))
        case "screenshot": self = .screenshot(try ScreenshotData(from: decoder))
        case "open": self = .open(try OpenData(from: decoder))
        case "getApps": self = .getApps(try GetAppsData(from: decoder))
        case "getText": self = .getText(try GetTextData(from: decoder))
        case "status": self = .status(try StatusData(from: decoder))
        default: self = .raw(type)
        }
    }
}

// MARK: - Data Payloads

public struct SnapshotData: Codable {
    public var text: String
    public var refCount: Int
    public var app: String
    public var window: String?
    
    public init(text: String, refCount: Int, app: String, window: String? = nil) {
        self.text = text
        self.refCount = refCount
        self.app = app
        self.window = window
    }
}

public struct ClickData: Codable {
    public var ref: String?
    public var coordinates: Point
    public var element: ElementInfo?
    
    public init(ref: String? = nil, coordinates: Point, element: ElementInfo? = nil) {
        self.ref = ref
        self.coordinates = coordinates
        self.element = element
    }
}

public struct FillData: Codable {
    public var ref: String
    public var text: String
    
    public init(ref: String, text: String) {
        self.ref = ref
        self.text = text
    }
}

public struct TypeData: Codable {
    public var ref: String?
    public var text: String
    
    public init(ref: String? = nil, text: String) {
        self.ref = ref
        self.text = text
    }
}

public struct PressData: Codable {
    public var key: String
    public var modifiers: [String]
    
    public init(key: String, modifiers: [String] = []) {
        self.key = key
        self.modifiers = modifiers
    }
}

public struct ScrollData: Codable {
    public var direction: String
    public var amount: Int
    
    public init(direction: String, amount: Int) {
        self.direction = direction
        self.amount = amount
    }
}

public struct ScreenshotData: Codable {
    public var path: String
    public var width: Int
    public var height: Int
    public var scale: Int
    
    public init(path: String, width: Int, height: Int, scale: Int = 1) {
        self.path = path
        self.width = width
        self.height = height
        self.scale = scale
    }
}

public struct OpenData: Codable {
    public var app: String
    public var pid: Int
    public var wasRunning: Bool
    
    public init(app: String, pid: Int, wasRunning: Bool) {
        self.app = app
        self.pid = pid
        self.wasRunning = wasRunning
    }
}

public struct GetAppsData: Codable {
    public var apps: [AppInfo]
    
    public init(apps: [AppInfo]) {
        self.apps = apps
    }
}

public struct GetTextData: Codable {
    public var ref: String?
    public var text: String
    
    public init(ref: String? = nil, text: String) {
        self.ref = ref
        self.text = text
    }
}

public struct StatusData: Codable {
    public var daemonPid: Int
    public var accessibilityPermission: Bool
    public var screenRecordingPermission: Bool
    public var frontmostApp: String?
    public var frontmostPid: Int?
    public var frontmostWindow: String?
    public var refMapCount: Int
    public var refMapAgeMs: Double?
    
    public init(daemonPid: Int, accessibilityPermission: Bool, screenRecordingPermission: Bool,
                frontmostApp: String? = nil, frontmostPid: Int? = nil, frontmostWindow: String? = nil,
                refMapCount: Int = 0, refMapAgeMs: Double? = nil) {
        self.daemonPid = daemonPid
        self.accessibilityPermission = accessibilityPermission
        self.screenRecordingPermission = screenRecordingPermission
        self.frontmostApp = frontmostApp
        self.frontmostPid = frontmostPid
        self.frontmostWindow = frontmostWindow
        self.refMapCount = refMapCount
        self.refMapAgeMs = refMapAgeMs
    }
}

// MARK: - Supporting Types

public struct Point: Codable {
    public var x: Double
    public var y: Double
    
    public init(x: Double, y: Double) {
        self.x = x
        self.y = y
    }
}

public struct ElementInfo: Codable {
    public var role: String
    public var label: String?
    
    public init(role: String, label: String? = nil) {
        self.role = role
        self.label = label
    }
}

public struct AppInfo: Codable {
    public var name: String
    public var pid: Int
    public var isActive: Bool
    
    public init(name: String, pid: Int, isActive: Bool) {
        self.name = name
        self.pid = pid
        self.isActive = isActive
    }
}
