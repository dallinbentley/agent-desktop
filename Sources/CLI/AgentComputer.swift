import Foundation
import ArgumentParser
import AgentComputerShared

// MARK: - Root Command

@main
struct AgentComputer: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "agent-computer",
        abstract: "Control macOS GUI applications via accessibility and input simulation.",
        version: "0.1.0",
        subcommands: [
            Snapshot.self,
            Click.self,
            Fill.self,
            TypeText.self,
            Press.self,
            Scroll.self,
            Screenshot.self,
            Open.self,
            Get.self,
            Status.self,
        ]
    )
}

// MARK: - Global Options

struct GlobalOptions: ParsableArguments {
    @Flag(name: .long, help: "Output raw JSON response.")
    var json = false
    
    @Option(name: .long, help: "Timeout in milliseconds.")
    var timeout: Int?
    
    @Flag(name: .long, help: "Enable verbose output.")
    var verbose = false
}

// MARK: - Helpers

/// Build a Request and send it, handling output and exit codes.
func runCommand(command: String, args: CommandArgs, options: GlobalOptions) throws {
    let requestOptions = RequestOptions(
        timeout: options.timeout,
        json: options.json,
        verbose: options.verbose
    )
    let request = Request(
        id: UUID().uuidString,
        command: command,
        args: args,
        options: requestOptions
    )
    
    do {
        let response = try Connection.send(request, verbose: options.verbose)
        let success = Output.printResponse(response, jsonMode: options.json)
        if !success {
            throw ExitCode(1)
        }
    } catch let error as ConnectionError {
        Output.printLocalError(
            error.description,
            suggestion: "Is the daemon running? Try 'agent-computer status'."
        )
        throw ExitCode(1)
    }
}

/// Parse an @ref string: strip @, validate e\d+ format.
func parseRef(_ input: String) -> String? {
    let stripped = input.hasPrefix("@") ? String(input.dropFirst()) : input
    // Validate e\d+ format: starts with 'e' followed by one or more digits
    guard stripped.hasPrefix("e"),
          stripped.count > 1,
          stripped.dropFirst().allSatisfy({ $0.isNumber }) else {
        return nil
    }
    return stripped
}

/// Parse key combo like "cmd+shift+s" into (key, modifiers).
func parseKeyCombo(_ input: String) -> (key: String, modifiers: [String]) {
    let parts = input.lowercased().split(separator: "+").map(String.init)
    guard !parts.isEmpty else { return (key: input, modifiers: []) }
    
    let modifierNames: Set<String> = ["cmd", "command", "shift", "alt", "option", "ctrl", "control", "fn"]
    
    var modifiers: [String] = []
    var keyParts: [String] = []
    
    for part in parts {
        if modifierNames.contains(part) {
            // Normalize modifier names
            switch part {
            case "command": modifiers.append("cmd")
            case "option": modifiers.append("alt")
            case "control": modifiers.append("ctrl")
            default: modifiers.append(part)
            }
        } else {
            keyParts.append(part)
        }
    }
    
    let key = keyParts.joined(separator: "+")
    return (key: key.isEmpty ? parts.last! : key, modifiers: modifiers)
}

// MARK: - Subcommands

struct Snapshot: ParsableCommand {
    static let configuration = CommandConfiguration(
        abstract: "Take an accessibility tree snapshot."
    )
    
    @Flag(name: .shortAndLong, help: "Show only interactive elements with @refs.")
    var interactive = false
    
    @Flag(name: .shortAndLong, help: "Compact output format.")
    var compact = false
    
    @Option(name: [.customShort("d"), .long], help: "Maximum tree depth.")
    var depth: Int?
    
    @Option(name: .long, help: "Target app name.")
    var app: String?
    
    @OptionGroup var global: GlobalOptions
    
    func run() throws {
        let args = SnapshotArgs(
            interactive: interactive,
            compact: compact,
            depth: depth,
            app: app
        )
        try runCommand(command: "snapshot", args: .snapshot(args), options: global)
    }
}

struct Click: ParsableCommand {
    static let configuration = CommandConfiguration(
        abstract: "Click an element by @ref or coordinates."
    )
    
    @Argument(help: "Element @ref (e.g. @e3) or X coordinate.")
    var refOrX: String
    
    @Argument(help: "Y coordinate (when using coordinate pair).")
    var y: Double?
    
    @Flag(name: .long, help: "Double-click.")
    var double = false
    
    @Flag(name: .long, help: "Right-click.")
    var right = false
    
    @OptionGroup var global: GlobalOptions
    
    func run() throws {
        let args: ClickArgs
        
        if let y = y, let x = Double(refOrX) {
            // Coordinate pair mode
            args = ClickArgs(x: x, y: y, double: self.double, right: self.right)
        } else if let ref = parseRef(refOrX) {
            // Ref mode
            args = ClickArgs(ref: ref, double: self.double, right: self.right)
        } else {
            // Try as number (might be missing Y)
            if Double(refOrX) != nil {
                throw ValidationError("Click by coordinates requires both X and Y values.")
            }
            throw ValidationError("Invalid ref '\(refOrX)'. Use @e<number> format (e.g. @e3) or provide X Y coordinates.")
        }
        
        try runCommand(command: "click", args: .click(args), options: global)
    }
}

struct Fill: ParsableCommand {
    static let configuration = CommandConfiguration(
        abstract: "Clear and fill a text field."
    )
    
    @Argument(help: "Element @ref (e.g. @e4).")
    var ref: String
    
    @Argument(help: "Text to fill.")
    var text: String
    
    @OptionGroup var global: GlobalOptions
    
    func run() throws {
        guard let parsedRef = parseRef(ref) else {
            throw ValidationError("Invalid ref '\(ref)'. Use @e<number> format (e.g. @e4).")
        }
        let args = FillArgs(ref: parsedRef, text: text)
        try runCommand(command: "fill", args: .fill(args), options: global)
    }
}

struct TypeText: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "type",
        abstract: "Type text, optionally into a specific element."
    )
    
    @Argument(help: "Element @ref (optional) or text to type.")
    var refOrText: String
    
    @Argument(help: "Text to type (when ref is provided).")
    var text: String?
    
    @OptionGroup var global: GlobalOptions
    
    func run() throws {
        let args: TypeArgs
        
        if let text = text {
            // Two arguments: first is ref, second is text
            guard let parsedRef = parseRef(refOrText) else {
                throw ValidationError("Invalid ref '\(refOrText)'. Use @e<number> format (e.g. @e4).")
            }
            args = TypeArgs(ref: parsedRef, text: text)
        } else {
            // One argument: could be ref-less text or check if it looks like a ref
            args = TypeArgs(text: refOrText)
        }
        
        try runCommand(command: "type", args: .type(args), options: global)
    }
}

struct Press: ParsableCommand {
    static let configuration = CommandConfiguration(
        abstract: "Press a key or key combination (e.g. cmd+c, enter)."
    )
    
    @Argument(help: "Key combo (e.g. cmd+shift+s, enter, tab).")
    var key: String
    
    @OptionGroup var global: GlobalOptions
    
    func run() throws {
        let (parsedKey, modifiers) = parseKeyCombo(key)
        let args = PressArgs(key: parsedKey, modifiers: modifiers.isEmpty ? nil : modifiers)
        try runCommand(command: "press", args: .press(args), options: global)
    }
}

struct Scroll: ParsableCommand {
    static let configuration = CommandConfiguration(
        abstract: "Scroll in a direction."
    )
    
    @Argument(help: "Direction: up, down, left, right.")
    var direction: String
    
    @Argument(help: "Amount in pixels (default: 300).")
    var amount: Int?
    
    @OptionGroup var global: GlobalOptions
    
    func run() throws {
        let validDirections = ["up", "down", "left", "right"]
        let dir = direction.lowercased()
        guard validDirections.contains(dir) else {
            throw ValidationError("Invalid direction '\(direction)'. Use: up, down, left, right.")
        }
        let args = ScrollArgs(direction: dir, amount: amount)
        try runCommand(command: "scroll", args: .scroll(args), options: global)
    }
}

struct Screenshot: ParsableCommand {
    static let configuration = CommandConfiguration(
        abstract: "Capture a screenshot."
    )
    
    @Flag(name: .long, help: "Capture full screen instead of frontmost window.")
    var full = false
    
    @Option(name: .long, help: "Target app name.")
    var app: String?
    
    @OptionGroup var global: GlobalOptions
    
    func run() throws {
        let args = ScreenshotArgs(full: full, app: app)
        try runCommand(command: "screenshot", args: .screenshot(args), options: global)
    }
}

struct Open: ParsableCommand {
    static let configuration = CommandConfiguration(
        abstract: "Open or focus an application."
    )
    
    @Argument(help: "Application name or bundle ID.")
    var target: String
    
    @OptionGroup var global: GlobalOptions
    
    func run() throws {
        let args = OpenArgs(target: target)
        try runCommand(command: "open", args: .open(args), options: global)
    }
}

struct Get: ParsableCommand {
    static let configuration = CommandConfiguration(
        abstract: "Get information (text, title, apps, windows)."
    )
    
    @Argument(help: "What to get: text, title, apps, windows.")
    var what: String
    
    @Argument(help: "Element @ref (for text/title).")
    var ref: String?
    
    @Option(name: .long, help: "Target app name.")
    var app: String?
    
    @OptionGroup var global: GlobalOptions
    
    func run() throws {
        let validWhats = ["text", "title", "apps", "windows"]
        guard validWhats.contains(what.lowercased()) else {
            throw ValidationError("Invalid target '\(what)'. Use: text, title, apps, windows.")
        }
        
        var parsedRef: String? = nil
        if let ref = ref {
            guard let r = parseRef(ref) else {
                throw ValidationError("Invalid ref '\(ref)'. Use @e<number> format (e.g. @e3).")
            }
            parsedRef = r
        }
        
        let args = GetArgs(what: what.lowercased(), ref: parsedRef, app: app)
        try runCommand(command: "get", args: .get(args), options: global)
    }
}

struct Status: ParsableCommand {
    static let configuration = CommandConfiguration(
        abstract: "Show daemon status and permissions."
    )
    
    @OptionGroup var global: GlobalOptions
    
    func run() throws {
        try runCommand(command: "status", args: .status, options: global)
    }
}
