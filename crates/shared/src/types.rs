use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;

// MARK: - Element Ref (stored in daemon's RefMap)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementRef {
    pub id: String,
    pub source: RefSource,
    pub role: String,
    pub label: Option<String>,
    pub frame: Option<Rect>,
    // AX-specific
    pub ax_path: Option<Vec<PathSegment>>,
    pub ax_actions: Option<Vec<String>>,
    pub ax_pid: Option<i32>,
    // CDP-specific (legacy, used by refmap routing)
    pub cdp_node_id: Option<i64>,
    pub cdp_backend_node_id: Option<i64>,
    pub cdp_port: Option<u16>,
    // agent-browser bridge fields
    pub ab_ref: Option<String>,       // Original agent-browser ref ID (e.g. "e32")
    pub ab_session: Option<String>,   // agent-browser session name (e.g. "spotify")
}

impl ElementRef {
    pub fn center(&self) -> Option<(f64, f64)> {
        self.frame.as_ref().map(|f| (f.x + f.width / 2.0, f.y + f.height / 2.0))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RefSource {
    AX,
    CDP,
    Coordinate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathSegment {
    pub role: String,
    pub index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

// MARK: - Error Info

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

// MARK: - Error Codes

pub mod error_code {
    pub const REF_NOT_FOUND: &str = "REF_NOT_FOUND";
    pub const REF_STALE: &str = "REF_STALE";
    pub const NO_REF_MAP: &str = "NO_REF_MAP";
    pub const APP_NOT_FOUND: &str = "APP_NOT_FOUND";
    pub const WINDOW_NOT_FOUND: &str = "WINDOW_NOT_FOUND";
    pub const PERMISSION_DENIED: &str = "PERMISSION_DENIED";
    pub const TIMEOUT: &str = "TIMEOUT";
    pub const AX_ERROR: &str = "AX_ERROR";
    pub const INPUT_ERROR: &str = "INPUT_ERROR";
    pub const INVALID_COMMAND: &str = "INVALID_COMMAND";
    pub const DAEMON_ERROR: &str = "DAEMON_ERROR";
    pub const CDP_NOT_AVAILABLE: &str = "CDP_NOT_AVAILABLE";
    pub const CDP_ERROR: &str = "CDP_ERROR";
}

// MARK: - App Kind (detection result)

#[derive(Debug, Clone)]
pub enum AppKind {
    Native,
    Browser { cdp_port: Option<u16> },
    Electron { cdp_port: Option<u16> },
    CEF { cdp_port: Option<u16> },
    Unknown,
}

// MARK: - Interactive Roles

pub static INTERACTIVE_ROLES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "AXButton", "AXTextField", "AXTextArea", "AXCheckBox",
        "AXRadioButton", "AXPopUpButton", "AXComboBox", "AXSlider",
        "AXLink", "AXMenuItem", "AXMenuButton", "AXTab",
        "AXTabGroup", "AXScrollArea", "AXTable", "AXOutline",
        "AXSwitch", "AXSearchField", "AXIncrementor",
    ].into_iter().collect()
});

// MARK: - Key Mapping

pub static KEY_NAME_TO_CODE: LazyLock<HashMap<&'static str, u16>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    // Control keys
    m.insert("enter", 36); m.insert("return", 36);
    m.insert("tab", 48);
    m.insert("escape", 53); m.insert("esc", 53);
    m.insert("space", 49);
    m.insert("delete", 51); m.insert("backspace", 51);
    m.insert("forwarddelete", 117);
    // Arrow keys
    m.insert("up", 126); m.insert("down", 125);
    m.insert("left", 123); m.insert("right", 124);
    // Navigation
    m.insert("home", 115); m.insert("end", 119);
    m.insert("pageup", 116); m.insert("pagedown", 121);
    // Function keys
    m.insert("f1", 122); m.insert("f2", 120); m.insert("f3", 99); m.insert("f4", 118);
    m.insert("f5", 96); m.insert("f6", 97); m.insert("f7", 98); m.insert("f8", 100);
    m.insert("f9", 101); m.insert("f10", 109); m.insert("f11", 103); m.insert("f12", 111);
    // Letters
    m.insert("a", 0); m.insert("b", 11); m.insert("c", 8); m.insert("d", 2);
    m.insert("e", 14); m.insert("f", 3); m.insert("g", 5); m.insert("h", 4);
    m.insert("i", 34); m.insert("j", 38); m.insert("k", 40); m.insert("l", 37);
    m.insert("m", 46); m.insert("n", 45); m.insert("o", 31); m.insert("p", 35);
    m.insert("q", 12); m.insert("r", 15); m.insert("s", 1); m.insert("t", 17);
    m.insert("u", 32); m.insert("v", 9); m.insert("w", 13); m.insert("x", 7);
    m.insert("y", 16); m.insert("z", 6);
    // Numbers
    m.insert("0", 29); m.insert("1", 18); m.insert("2", 19); m.insert("3", 20);
    m.insert("4", 21); m.insert("5", 23); m.insert("6", 22); m.insert("7", 26);
    m.insert("8", 28); m.insert("9", 25);
    // Symbols
    m.insert("-", 27); m.insert("=", 24); m.insert("[", 33); m.insert("]", 30);
    m.insert(";", 41); m.insert("'", 39); m.insert(",", 43); m.insert(".", 47);
    m.insert("/", 44); m.insert("\\", 42); m.insert("`", 50);
    m
});

// MARK: - Known Browsers

pub static KNOWN_BROWSER_BUNDLE_IDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "com.apple.Safari",
        "com.google.Chrome",
        "org.mozilla.firefox",
        "com.microsoft.edgemac",
        "com.brave.Browser",
        "company.thebrowser.Browser",  // Arc
        "com.vivaldi.Vivaldi",
        "com.operasoftware.Opera",
    ].into_iter().collect()
});

// MARK: - Socket Path

pub fn daemon_socket_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".agent-desktop")
}

pub fn daemon_socket_path() -> PathBuf {
    if let Ok(custom) = std::env::var("AGENT_COMPUTER_SOCKET") {
        return PathBuf::from(custom);
    }
    daemon_socket_dir().join("daemon.sock")
}

#[cfg(test)]
mod tests {
    use super::*;

    // 12.4: Socket path resolution tests

    #[test]
    fn test_daemon_socket_dir_is_under_home() {
        let dir = daemon_socket_dir();
        let dir_str = dir.to_string_lossy();
        assert!(dir_str.ends_with(".agent-desktop"), "Socket dir should end with .agent-desktop, got: {}", dir_str);
    }

    #[test]
    fn test_daemon_socket_path_default() {
        // Remove env var to test default behavior
        std::env::remove_var("AGENT_COMPUTER_SOCKET");
        let path = daemon_socket_path();
        let path_str = path.to_string_lossy();
        assert!(path_str.ends_with("daemon.sock"), "Default socket should end with daemon.sock, got: {}", path_str);
        assert!(path_str.contains(".agent-desktop"), "Default socket should be under .agent-desktop, got: {}", path_str);
    }

    #[test]
    fn test_daemon_socket_path_custom_env() {
        let custom = "/tmp/test-agent-desktop.sock";
        std::env::set_var("AGENT_COMPUTER_SOCKET", custom);
        let path = daemon_socket_path();
        assert_eq!(path, PathBuf::from(custom));
        // Clean up
        std::env::remove_var("AGENT_COMPUTER_SOCKET");
    }
}
