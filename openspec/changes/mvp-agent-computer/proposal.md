## Why

AI agents need a way to observe and interact with macOS desktop applications efficiently. Current approaches either use expensive screenshot-based vision (1,500-3,000 tokens per observation) or require complex MCP server setups. There is no simple CLI tool — like agent-browser for the web — that gives agents a compact, text-first view of the desktop with deterministic element references. We need an MVP that proves the accessibility-tree-based snapshot + @ref approach works for desktop automation.

## What Changes

- New Swift CLI tool `agent-computer` that AI agents invoke to control macOS
- Persistent background daemon that maintains state (ref maps, element cache) between CLI calls
- Accessibility tree snapshot system that produces compact text with `@e1, @e2` refs (~200-400 tokens)
- Input simulation via CGEvent for mouse clicks, keyboard typing, and key presses
- Screenshot capture via ScreenCaptureKit
- App launching and focusing via NSWorkspace
- Unix domain socket IPC protocol between CLI and daemon (JSON over newline-delimited messages)
- Permission detection and setup guidance for Accessibility and Screen Recording

## Capabilities

### New Capabilities
- `snapshot`: Accessibility tree traversal, interactive element filtering, @ref assignment, and compact text output
- `input-simulation`: Mouse clicks (left/right/double), keyboard typing (Unicode), key presses with modifiers, and cursor positioning
- `screenshot-capture`: Full screen and per-window screenshot capture via ScreenCaptureKit with Retina support
- `daemon-ipc`: Persistent daemon process with Unix socket server, CLI client, auto-start on first use, and JSON protocol
- `app-management`: App launching, focusing, window enumeration via NSWorkspace and Accessibility APIs
- `cli-interface`: Command parsing, @ref syntax, human-readable and JSON output formatting, AI-friendly error messages

### Modified Capabilities

_None — this is a greenfield project._

## Impact

- **New binary targets**: `agent-computer` (CLI) and `agent-computer-daemon` (background process)
- **Dependencies**: Swift Argument Parser (CLI parsing), Foundation (IPC/JSON), ApplicationServices (AXUIElement, CGEvent), ScreenCaptureKit, AppKit (NSWorkspace)
- **System permissions**: Requires Accessibility + Screen Recording permissions granted to the terminal app
- **Platform**: macOS 14+ (Sonoma) due to SCScreenshotManager requirement
- **Files**: New Swift Package with Sources/CLI, Sources/Daemon, Sources/Shared directories
