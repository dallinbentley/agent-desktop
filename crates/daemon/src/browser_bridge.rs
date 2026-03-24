// Browser Bridge — subprocess bridge to agent-browser CLI
// Tasks 2.1-2.5

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

// MARK: - 2.3 Parsed Element

/// A parsed element from agent-browser snapshot output
#[derive(Debug, Clone)]
pub struct ParsedElement {
    /// Original agent-browser ref ID (e.g., "e14")
    pub ref_id: String,
    /// Element role: "button", "combobox", etc.
    pub role: String,
    /// Element label: "Home", "What do you want to play?"
    pub label: Option<String>,
    /// Indentation depth (number of 2-space levels)
    pub depth: usize,
    /// Additional attributes parsed from brackets (e.g., "expanded=false")
    pub attributes: HashMap<String, String>,
    /// Value text after the colon (e.g., "Luke Combs")
    pub value: Option<String>,
}

// MARK: - 2.1 BrowserBridge struct

/// Bridge to agent-browser CLI for web/Electron interaction via CDP.
/// Detects the binary at construction and caches the path.
pub struct BrowserBridge {
    /// Cached path to the agent-browser binary, None if not found
    binary_path: Option<PathBuf>,
    /// Active sessions: session_name → cdp_port
    pub active_sessions: HashMap<String, u16>,
}

impl BrowserBridge {
    /// Create a new BrowserBridge, detecting the agent-browser binary.
    pub fn new() -> Self {
        let binary_path = Self::detect_binary();
        if let Some(ref path) = binary_path {
            eprintln!("[BrowserBridge] Found agent-browser at: {}", path.display());
        } else {
            eprintln!("[BrowserBridge] agent-browser not found. Web/Electron features disabled.");
        }
        Self {
            binary_path,
            active_sessions: HashMap::new(),
        }
    }

    /// Check if agent-browser is available
    pub fn is_available(&self) -> bool {
        self.binary_path.is_some()
    }

    /// Get the binary path (for diagnostics)
    #[allow(dead_code)]
    pub fn binary_path(&self) -> Option<&PathBuf> {
        self.binary_path.as_ref()
    }

    /// Detect agent-browser binary by checking PATH and common locations
    fn detect_binary() -> Option<PathBuf> {
        // 1. Check PATH via `which`
        if let Ok(output) = Command::new("which").arg("agent-browser").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    let p = PathBuf::from(&path);
                    if p.exists() {
                        return Some(p);
                    }
                }
            }
        }

        // 2. Check common npm/nvm global paths
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let common_paths = [
            // Specific known path (from project context)
            home.join(".nvm/versions/node/v24.14.0/bin/agent-browser"),
            // Global npm
            PathBuf::from("/usr/local/bin/agent-browser"),
            // Homebrew
            PathBuf::from("/opt/homebrew/bin/agent-browser"),
            // npm global (macOS)
            home.join(".npm-global/bin/agent-browser"),
        ];

        for path in &common_paths {
            if path.exists() {
                return Some(path.clone());
            }
        }

        // 3. Try nvm glob: ~/.nvm/versions/node/*/bin/agent-browser
        let nvm_base = home.join(".nvm/versions/node");
        if let Ok(entries) = std::fs::read_dir(&nvm_base) {
            for entry in entries.flatten() {
                let candidate = entry.path().join("bin/agent-browser");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }

        None
    }

    // MARK: - 2.2 Execute

    /// Execute an agent-browser command via subprocess.
    /// Passes `--session <session> --cdp <port>` then the provided args.
    /// Returns stdout on success, Err(stderr) on failure.
    pub fn execute(&self, session: &str, cdp_port: u16, args: &[&str]) -> Result<String, String> {
        let binary = self.binary_path.as_ref().ok_or_else(|| {
            "agent-browser not found. Install with: npm install -g agent-browser".to_string()
        })?;

        let mut cmd = Command::new(binary);
        cmd.arg("--session").arg(session);
        cmd.arg("--cdp").arg(cdp_port.to_string());
        for arg in args {
            cmd.arg(arg);
        }

        let output = cmd.output().map_err(|e| {
            format!("Failed to execute agent-browser: {}", e)
        })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Err(if stderr.is_empty() { stdout } else { stderr })
        }
    }

    /// Execute an agent-browser command with only session (no cdp port).
    /// Used for commands like `close` that don't need a CDP port.
    fn execute_session_only(&self, session: &str, args: &[&str]) -> Result<String, String> {
        let binary = self.binary_path.as_ref().ok_or_else(|| {
            "agent-browser not found. Install with: npm install -g agent-browser".to_string()
        })?;

        let mut cmd = Command::new(binary);
        cmd.arg("--session").arg(session);
        for arg in args {
            cmd.arg(arg);
        }

        let output = cmd.output().map_err(|e| {
            format!("Failed to execute agent-browser: {}", e)
        })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Err(if stderr.is_empty() { stdout } else { stderr })
        }
    }

    // MARK: - 2.3 Snapshot

    /// Take an agent-browser snapshot and return parsed elements.
    /// If `interactive` is true, passes `-i` for interactive-only elements.
    /// If `selector` is Some, passes `-s "<selector>"` to scope the snapshot.
    pub fn snapshot(
        &self,
        session: &str,
        cdp_port: u16,
        interactive: bool,
        selector: Option<&str>,
    ) -> Result<Vec<ParsedElement>, String> {
        let mut args = vec!["snapshot"];
        if interactive {
            args.push("-i");
        }

        let selector_owned: String;
        if let Some(sel) = selector {
            args.push("-s");
            selector_owned = sel.to_string();
            args.push(&selector_owned);
        }

        let raw_output = self.execute(session, cdp_port, &args)?;
        Ok(parse_snapshot_output(&raw_output))
    }

    /// Take an agent-browser snapshot and return both raw text and parsed elements.
    #[allow(dead_code)]
    pub fn snapshot_raw(
        &self,
        session: &str,
        cdp_port: u16,
        interactive: bool,
        selector: Option<&str>,
    ) -> Result<(String, Vec<ParsedElement>), String> {
        let mut args = vec!["snapshot"];
        if interactive {
            args.push("-i");
        }

        let selector_owned: String;
        if let Some(sel) = selector {
            args.push("-s");
            selector_owned = sel.to_string();
            args.push(&selector_owned);
        }

        let raw_output = self.execute(session, cdp_port, &args)?;
        let elements = parse_snapshot_output(&raw_output);
        Ok((raw_output, elements))
    }

    // MARK: - 2.4 Interaction Methods

    /// Click an element by its agent-browser ref
    pub fn click(&self, session: &str, cdp_port: u16, ab_ref: &str) -> Result<String, String> {
        let ref_arg = if ab_ref.starts_with('@') {
            ab_ref.to_string()
        } else {
            format!("@{}", ab_ref)
        };
        self.execute(session, cdp_port, &["click", &ref_arg])
    }

    /// Fill a field (clear + type) by its agent-browser ref
    pub fn fill(
        &self,
        session: &str,
        cdp_port: u16,
        ab_ref: &str,
        text: &str,
    ) -> Result<String, String> {
        let ref_arg = if ab_ref.starts_with('@') {
            ab_ref.to_string()
        } else {
            format!("@{}", ab_ref)
        };
        self.execute(session, cdp_port, &["fill", &ref_arg, text])
    }

    /// Type text into an element (append, no clear) by its agent-browser ref
    pub fn type_text(
        &self,
        session: &str,
        cdp_port: u16,
        ab_ref: &str,
        text: &str,
    ) -> Result<String, String> {
        let ref_arg = if ab_ref.starts_with('@') {
            ab_ref.to_string()
        } else {
            format!("@{}", ab_ref)
        };
        self.execute(session, cdp_port, &["type", &ref_arg, text])
    }

    /// Press a key (headless via CDP)
    pub fn press(
        &self,
        session: &str,
        cdp_port: u16,
        key: &str,
    ) -> Result<(), String> {
        self.execute(session, cdp_port, &["press", key])?;
        Ok(())
    }

    /// Scroll in a direction (headless via CDP)
    pub fn scroll(
        &self,
        session: &str,
        cdp_port: u16,
        direction: &str,
        amount: i32,
    ) -> Result<(), String> {
        let amount_str = amount.to_string();
        self.execute(session, cdp_port, &["scroll", direction, &amount_str])?;
        Ok(())
    }

    // MARK: - Wait

    /// Wait for an element, time, or page load state via agent-browser.
    /// Delegates to `agent-browser --session <s> --cdp <port> wait <args>`.
    pub fn wait(&self, session: &str, cdp_port: u16, args: &[&str]) -> Result<String, String> {
        let mut cmd_args = vec!["wait"];
        cmd_args.extend_from_slice(args);
        self.execute(session, cdp_port, &cmd_args)
    }

    // MARK: - Get Web Content

    /// Get text/title/url from web content via agent-browser.
    /// Delegates to `agent-browser --session <s> --cdp <port> get <what> [@ref]`.
    pub fn get_web(
        &self,
        session: &str,
        cdp_port: u16,
        what: &str,
        ab_ref: Option<&str>,
    ) -> Result<String, String> {
        let mut args = vec!["get", what];
        let ref_arg: String;
        if let Some(r) = ab_ref {
            ref_arg = if r.starts_with('@') {
                r.to_string()
            } else {
                format!("@{}", r)
            };
            args.push(&ref_arg);
        }
        self.execute(session, cdp_port, &args)
    }

    // MARK: - 2.5 Lifecycle

    /// Establish a persistent CDP connection for a session
    pub fn connect(&mut self, session: &str, cdp_port: u16) -> Result<(), String> {
        let port_str = cdp_port.to_string();
        self.execute_session_only(session, &["connect", &port_str])?;
        self.active_sessions.insert(session.to_string(), cdp_port);
        Ok(())
    }

    /// Close an agent-browser session
    pub fn close(&mut self, session: &str) -> Result<(), String> {
        let result = self.execute_session_only(session, &["close"]);
        self.active_sessions.remove(session);
        result.map(|_| ())
    }

    /// Close all active sessions (for daemon shutdown)
    pub fn close_all(&mut self) {
        let sessions: Vec<String> = self.active_sessions.keys().cloned().collect();
        for session in sessions {
            if let Err(e) = self.close(&session) {
                eprintln!(
                    "[BrowserBridge] Failed to close session '{}': {}",
                    session, e
                );
            }
        }
    }
}

// MARK: - Snapshot Output Parsing

/// Parse agent-browser snapshot text output into structured elements.
///
/// Each line looks like:
/// ```text
/// - button "Home" [ref=e14]
/// - combobox "What do you want to play?" [expanded=false, ref=e32]: Luke Combs
/// - navigation "Main" [ref=e8]
///   - button "Collapse Your Library" [ref=e41]
///     - heading "Your Library" [level=1, ref=e55]
/// ```
pub fn parse_snapshot_output(output: &str) -> Vec<ParsedElement> {
    let mut elements = Vec::new();
    for line in output.lines() {
        if let Some(elem) = parse_snapshot_line(line) {
            elements.push(elem);
        }
    }
    elements
}

/// Parse a single line of snapshot output
fn parse_snapshot_line(line: &str) -> Option<ParsedElement> {
    // Calculate depth from leading whitespace (2 spaces per level)
    let trimmed = line.trim_start();
    let leading_spaces = line.len() - trimmed.len();
    let depth = leading_spaces / 2;

    // Must start with "- " after indentation
    let content = trimmed.strip_prefix("- ")?;

    // Extract ref from [ref=eN] — required for a valid element
    let ref_id = extract_ref(content)?;

    // Extract role (first word before space or quote)
    let role = content.split(|c: char| c.is_whitespace() || c == '"')
        .next()?
        .to_string();

    // Extract label (quoted string after role)
    let label = extract_quoted_label(content);

    // Extract attributes from brackets
    let attributes = extract_attributes(content);

    // Extract value after colon (": value text")
    let value = extract_value(content);

    Some(ParsedElement {
        ref_id,
        role,
        label,
        depth,
        attributes,
        value,
    })
}

/// Extract the ref ID from a line containing [ref=eN]
fn extract_ref(content: &str) -> Option<String> {
    let ref_start = content.find("ref=")?;
    let after_ref = &content[ref_start + 4..];

    // Find the end of the ref value (next comma, bracket, or space)
    let end = after_ref
        .find(|c: char| c == ']' || c == ',' || c == ' ')
        .unwrap_or(after_ref.len());

    let ref_id = after_ref[..end].trim();
    if ref_id.is_empty() {
        return None;
    }

    Some(ref_id.to_string())
}

/// Extract the quoted label from content (e.g., `button "Home" [ref=e14]` → "Home")
fn extract_quoted_label(content: &str) -> Option<String> {
    let first_quote = content.find('"')?;
    let after_first = &content[first_quote + 1..];
    let second_quote = after_first.find('"')?;
    let label = &after_first[..second_quote];
    if label.is_empty() {
        None
    } else {
        Some(label.to_string())
    }
}

/// Extract attributes from bracket notation (e.g., `[expanded=false, ref=e32]`)
fn extract_attributes(content: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();

    let bracket_start = match content.find('[') {
        Some(i) => i,
        None => return attrs,
    };
    let bracket_end = match content[bracket_start..].find(']') {
        Some(i) => bracket_start + i,
        None => return attrs,
    };

    let bracket_content = &content[bracket_start + 1..bracket_end];
    for part in bracket_content.split(',') {
        let part = part.trim();
        if let Some(eq_pos) = part.find('=') {
            let key = part[..eq_pos].trim();
            let value = part[eq_pos + 1..].trim();
            if key != "ref" {
                // Skip ref, we handle it separately
                attrs.insert(key.to_string(), value.to_string());
            }
        }
    }

    attrs
}

/// Extract value text after the last `]:` pattern (e.g., `]: Luke Combs`)
fn extract_value(content: &str) -> Option<String> {
    let bracket_end = content.rfind(']')?;
    let after_bracket = &content[bracket_end + 1..];
    let colon_content = after_bracket.strip_prefix(':')?;
    let value = colon_content.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

// MARK: - Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_button() {
        let line = r#"- button "Home" [ref=e14]"#;
        let elem = parse_snapshot_line(line).unwrap();
        assert_eq!(elem.ref_id, "e14");
        assert_eq!(elem.role, "button");
        assert_eq!(elem.label.as_deref(), Some("Home"));
        assert_eq!(elem.depth, 0);
        assert!(elem.value.is_none());
    }

    #[test]
    fn test_parse_combobox_with_value() {
        let line = r#"- combobox "What do you want to play?" [expanded=false, ref=e32]: Luke Combs"#;
        let elem = parse_snapshot_line(line).unwrap();
        assert_eq!(elem.ref_id, "e32");
        assert_eq!(elem.role, "combobox");
        assert_eq!(elem.label.as_deref(), Some("What do you want to play?"));
        assert_eq!(elem.depth, 0);
        assert_eq!(elem.value.as_deref(), Some("Luke Combs"));
        assert_eq!(elem.attributes.get("expanded").map(|s| s.as_str()), Some("false"));
    }

    #[test]
    fn test_parse_nested_element() {
        let line = r#"    - button "Collapse Your Library" [ref=e41]"#;
        let elem = parse_snapshot_line(line).unwrap();
        assert_eq!(elem.ref_id, "e41");
        assert_eq!(elem.role, "button");
        assert_eq!(elem.label.as_deref(), Some("Collapse Your Library"));
        assert_eq!(elem.depth, 2);
    }

    #[test]
    fn test_parse_navigation_with_ref() {
        let line = r#"- navigation "Main" [ref=e8]"#;
        let elem = parse_snapshot_line(line).unwrap();
        assert_eq!(elem.ref_id, "e8");
        assert_eq!(elem.role, "navigation");
        assert_eq!(elem.label.as_deref(), Some("Main"));
        assert_eq!(elem.depth, 0);
    }

    #[test]
    fn test_parse_heading_nested() {
        let line = r#"      - heading "Your Library" [level=1, ref=e55]"#;
        let elem = parse_snapshot_line(line).unwrap();
        assert_eq!(elem.ref_id, "e55");
        assert_eq!(elem.role, "heading");
        assert_eq!(elem.label.as_deref(), Some("Your Library"));
        assert_eq!(elem.depth, 3);
        assert_eq!(elem.attributes.get("level").map(|s| s.as_str()), Some("1"));
    }

    #[test]
    fn test_parse_no_ref_returns_none() {
        let line = r#"- paragraph "Some text""#;
        assert!(parse_snapshot_line(line).is_none());
    }

    #[test]
    fn test_parse_no_label() {
        let line = r#"- separator [ref=e99]"#;
        let elem = parse_snapshot_line(line).unwrap();
        assert_eq!(elem.ref_id, "e99");
        assert_eq!(elem.role, "separator");
        assert!(elem.label.is_none());
    }

    #[test]
    fn test_parse_multiline_output() {
        let output = r#"- button "Home" [ref=e14]
- button "Search" [ref=e15]
  - textbox "Search" [ref=e16]
- navigation "Main" [ref=e8]"#;
        let elements = parse_snapshot_output(output);
        assert_eq!(elements.len(), 4);
        assert_eq!(elements[0].ref_id, "e14");
        assert_eq!(elements[1].ref_id, "e15");
        assert_eq!(elements[2].ref_id, "e16");
        assert_eq!(elements[2].depth, 1);
        assert_eq!(elements[3].ref_id, "e8");
    }

    #[test]
    fn test_extract_ref() {
        assert_eq!(extract_ref("[ref=e14]"), Some("e14".to_string()));
        assert_eq!(extract_ref("[expanded=false, ref=e32]"), Some("e32".to_string()));
        assert_eq!(extract_ref("[level=1, ref=e55]"), Some("e55".to_string()));
        assert_eq!(extract_ref("no ref here"), None);
    }

    #[test]
    fn test_extract_quoted_label() {
        assert_eq!(extract_quoted_label(r#"button "Home" [ref=e14]"#), Some("Home".to_string()));
        assert_eq!(extract_quoted_label(r#"combobox "Search query" [ref=e1]"#), Some("Search query".to_string()));
        assert_eq!(extract_quoted_label("separator [ref=e99]"), None);
    }

    #[test]
    fn test_extract_attributes() {
        let attrs = extract_attributes("[expanded=false, ref=e32]");
        assert_eq!(attrs.get("expanded").map(|s| s.as_str()), Some("false"));
        assert!(!attrs.contains_key("ref")); // ref is excluded

        let attrs2 = extract_attributes("[level=1, ref=e55]");
        assert_eq!(attrs2.get("level").map(|s| s.as_str()), Some("1"));
    }

    #[test]
    fn test_extract_value() {
        assert_eq!(extract_value("[ref=e32]: Luke Combs"), Some("Luke Combs".to_string()));
        assert_eq!(extract_value("[ref=e14]"), None);
        assert_eq!(extract_value("[ref=e1]:"), None);
    }

    #[test]
    fn test_bridge_not_available() {
        let bridge = BrowserBridge {
            binary_path: None,
            active_sessions: HashMap::new(),
        };
        assert!(!bridge.is_available());

        // Execute should fail gracefully
        let result = bridge.execute("test", 9222, &["snapshot", "-i"]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("agent-browser not found"));
    }

    #[test]
    fn test_bridge_detect() {
        // This test just verifies detection doesn't panic
        let bridge = BrowserBridge::new();
        let _ = bridge.is_available();
    }

    #[test]
    fn test_session_tracking() {
        let mut bridge = BrowserBridge {
            binary_path: None,
            active_sessions: HashMap::new(),
        };

        // Manually add sessions (since we can't actually connect without agent-browser)
        bridge.active_sessions.insert("spotify".to_string(), 9371);
        bridge.active_sessions.insert("chrome".to_string(), 9222);
        assert_eq!(bridge.active_sessions.len(), 2);

        bridge.active_sessions.remove("spotify");
        assert_eq!(bridge.active_sessions.len(), 1);
        assert!(!bridge.active_sessions.contains_key("spotify"));
        assert!(bridge.active_sessions.contains_key("chrome"));
    }
}
