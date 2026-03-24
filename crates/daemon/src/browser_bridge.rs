// Browser Bridge — subprocess bridge to agent-browser CLI
// Groups 1 + 4: JSON output mode + async subprocess

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use serde::Deserialize;
use tokio::process::Command as AsyncCommand;

// MARK: - 1.1 JSON Response Structs

/// Top-level JSON response from agent-browser --json
#[derive(Debug, Clone, Deserialize)]
pub struct AgentBrowserResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Snapshot-specific data from the JSON response
#[derive(Debug, Clone, Deserialize)]
pub struct SnapshotJsonData {
    pub origin: Option<String>,
    pub refs: Option<HashMap<String, RefInfo>>,
    pub snapshot: Option<String>,
}

/// Info about a single element ref from snapshot JSON
#[derive(Debug, Clone, Deserialize)]
pub struct RefInfo {
    pub name: Option<String>,
    pub role: Option<String>,
}

/// Result of a snapshot call: structured refs + formatted text
#[derive(Debug, Clone)]
pub struct SnapshotResult {
    /// Parsed element refs from the JSON `refs` map
    pub elements: Vec<ParsedElement>,
    /// Pre-formatted snapshot text from `data.snapshot`
    pub snapshot_text: Option<String>,
    /// Origin URL if available
    pub origin: Option<String>,
}

/// A parsed element from agent-browser snapshot output
#[derive(Debug, Clone)]
pub struct ParsedElement {
    /// Original agent-browser ref ID (e.g., "e14")
    pub ref_id: String,
    /// Element role: "button", "combobox", etc.
    pub role: String,
    /// Element label: "Home", "What do you want to play?"
    pub label: Option<String>,
}

// MARK: - BrowserBridge struct

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

    const AGENT_BROWSER_VERSION: &'static str = "0.22.1";

    /// Detect agent-browser binary by checking bundled path, PATH, and common locations.
    /// Auto-downloads if not found anywhere.
    fn detect_binary() -> Option<PathBuf> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));

        // 1. Check bundled path first (~/.agent-computer/bin/agent-browser)
        let bundled = home.join(".agent-computer/bin/agent-browser");
        if bundled.exists() {
            return Some(bundled);
        }

        // 2. Check PATH via `which`
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

        // 3. Check common npm/nvm global paths
        let common_paths = [
            home.join(".nvm/versions/node/v24.14.0/bin/agent-browser"),
            PathBuf::from("/usr/local/bin/agent-browser"),
            PathBuf::from("/opt/homebrew/bin/agent-browser"),
            home.join(".npm-global/bin/agent-browser"),
        ];

        for path in &common_paths {
            if path.exists() {
                return Some(path.clone());
            }
        }

        // 4. Try nvm glob: ~/.nvm/versions/node/*/bin/agent-browser
        let nvm_base = home.join(".nvm/versions/node");
        if let Ok(entries) = std::fs::read_dir(&nvm_base) {
            for entry in entries.flatten() {
                let candidate = entry.path().join("bin/agent-browser");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }

        // 5. Auto-download as last resort
        eprintln!("[BrowserBridge] agent-browser not found. Attempting auto-download...");
        match Self::download_binary(&home) {
            Ok(path) => {
                eprintln!("[BrowserBridge] ✓ Downloaded agent-browser to {}", path.display());
                // Run agent-browser install for Chrome for Testing
                eprintln!("[BrowserBridge] Running 'agent-browser install' for Chrome for Testing...");
                match Command::new(&path).arg("install").output() {
                    Ok(o) if o.status.success() => {
                        eprintln!("[BrowserBridge] ✓ Chrome for Testing installed.");
                    }
                    Ok(o) => {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        eprintln!("[BrowserBridge] Warning: 'agent-browser install' exited with {}: {}", o.status, stderr.trim());
                    }
                    Err(e) => {
                        eprintln!("[BrowserBridge] Warning: Failed to run 'agent-browser install': {}", e);
                    }
                }
                Some(path)
            }
            Err(e) => {
                eprintln!("[BrowserBridge] Auto-download failed: {}", e);
                eprintln!("[BrowserBridge] Install manually: npm install -g agent-browser");
                None
            }
        }
    }

    /// Download agent-browser binary from npm registry for the current platform.
    fn download_binary(home: &PathBuf) -> Result<PathBuf, String> {
        let os_name = if cfg!(target_os = "macos") { "darwin" }
            else if cfg!(target_os = "linux") { "linux" }
            else { return Err("Unsupported OS".to_string()) };

        let arch = if cfg!(target_arch = "aarch64") { "arm64" }
            else if cfg!(target_arch = "x86_64") { "x64" }
            else { return Err("Unsupported architecture".to_string()) };

        let binary_name = format!("agent-browser-{}-{}", os_name, arch);
        let bin_dir = home.join(".agent-computer/bin");
        let target_path = bin_dir.join("agent-browser");

        std::fs::create_dir_all(&bin_dir)
            .map_err(|e| format!("Failed to create {}: {}", bin_dir.display(), e))?;

        let url = format!(
            "https://registry.npmjs.org/agent-browser/-/agent-browser-{}.tgz",
            Self::AGENT_BROWSER_VERSION
        );
        eprintln!("[BrowserBridge] Downloading v{} ({})...", Self::AGENT_BROWSER_VERSION, binary_name);

        // Download tgz
        let tmp_dir = bin_dir.join(".download-tmp");
        if tmp_dir.exists() { let _ = std::fs::remove_dir_all(&tmp_dir); }
        std::fs::create_dir_all(&tmp_dir)
            .map_err(|e| format!("Failed to create tmp dir: {}", e))?;

        let tgz_path = tmp_dir.join("agent-browser.tgz");

        // Use curl since it's available on macOS/Linux
        let dl = Command::new("curl")
            .args(["-sSfL", "-o"])
            .arg(tgz_path.to_str().unwrap())
            .arg(&url)
            .output()
            .map_err(|e| format!("curl failed: {}", e))?;

        if !dl.status.success() {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            let stderr = String::from_utf8_lossy(&dl.stderr);
            return Err(format!("Download failed: {}", stderr.trim()));
        }

        // Extract
        let tar = Command::new("tar")
            .args(["xzf"])
            .arg(tgz_path.to_str().unwrap())
            .arg("-C")
            .arg(tmp_dir.to_str().unwrap())
            .output()
            .map_err(|e| format!("tar failed: {}", e))?;

        if !tar.status.success() {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return Err("tar extraction failed".to_string());
        }

        let extracted = tmp_dir.join("package/bin").join(&binary_name);
        if !extracted.exists() {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return Err(format!("Binary '{}' not found in npm package", binary_name));
        }

        std::fs::copy(&extracted, &target_path)
            .map_err(|e| format!("Failed to copy binary: {}", e))?;

        // chmod +x
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&target_path) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&target_path, perms);
            }
        }

        let _ = std::fs::remove_dir_all(&tmp_dir);
        Ok(target_path)
    }

    // MARK: - 1.2 / 4.2 Execute (async + JSON)

    /// Execute an agent-browser command via async subprocess with --json flag.
    /// Returns parsed AgentBrowserResponse on success.
    /// Includes a 10-second timeout (task 4.5).
    pub async fn execute(
        &self,
        session: &str,
        cdp_port: u16,
        args: &[&str],
    ) -> Result<AgentBrowserResponse, String> {
        let binary = self.binary_path.as_ref().ok_or_else(|| {
            "agent-browser not found. Install with: npm install -g agent-browser".to_string()
        })?;

        let mut cmd = AsyncCommand::new(binary);
        cmd.arg("--session").arg(session);
        cmd.arg("--cdp").arg(cdp_port.to_string());
        cmd.arg("--json");
        for arg in args {
            cmd.arg(arg);
        }

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            cmd.output(),
        )
        .await
        .map_err(|_| "agent-browser command timed out after 10 seconds".to_string())?
        .map_err(|e| format!("Failed to execute agent-browser: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        if !output.status.success() {
            // Try to parse JSON error from stdout first
            if let Ok(resp) = serde_json::from_str::<AgentBrowserResponse>(&stdout) {
                if let Some(err) = resp.error {
                    return Err(err);
                }
            }
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(if stderr.is_empty() { stdout } else { stderr });
        }

        serde_json::from_str::<AgentBrowserResponse>(&stdout)
            .map_err(|e| format!("Failed to parse agent-browser JSON response: {} — raw output: {}", e, stdout))
    }

    /// Execute an agent-browser command with only session (no cdp port).
    /// Used for commands like `close` that don't need a CDP port.
    async fn execute_session_only(
        &self,
        session: &str,
        args: &[&str],
    ) -> Result<AgentBrowserResponse, String> {
        let binary = self.binary_path.as_ref().ok_or_else(|| {
            "agent-browser not found. Install with: npm install -g agent-browser".to_string()
        })?;

        let mut cmd = AsyncCommand::new(binary);
        cmd.arg("--session").arg(session);
        cmd.arg("--json");
        for arg in args {
            cmd.arg(arg);
        }

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            cmd.output(),
        )
        .await
        .map_err(|_| "agent-browser command timed out after 10 seconds".to_string())?
        .map_err(|e| format!("Failed to execute agent-browser: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        if !output.status.success() {
            if let Ok(resp) = serde_json::from_str::<AgentBrowserResponse>(&stdout) {
                if let Some(err) = resp.error {
                    return Err(err);
                }
            }
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(if stderr.is_empty() { stdout } else { stderr });
        }

        serde_json::from_str::<AgentBrowserResponse>(&stdout)
            .map_err(|e| format!("Failed to parse agent-browser JSON response: {} — raw output: {}", e, stdout))
    }

    // MARK: - 1.3 Snapshot (JSON-based)

    /// Take an agent-browser snapshot and return structured result.
    /// Uses JSON output to extract refs directly (no regex parsing).
    pub async fn snapshot(
        &self,
        session: &str,
        cdp_port: u16,
        interactive: bool,
        selector: Option<&str>,
    ) -> Result<SnapshotResult, String> {
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

        let response = self.execute(session, cdp_port, &args).await?;

        if !response.success {
            return Err(response.error.unwrap_or_else(|| "Snapshot failed".to_string()));
        }

        let data = response.data.ok_or("No data in snapshot response")?;

        // Parse the snapshot-specific data
        let snapshot_data: SnapshotJsonData = serde_json::from_value(data)
            .map_err(|e| format!("Failed to parse snapshot data: {}", e))?;

        // Build ParsedElements from the refs map
        let mut elements = Vec::new();
        if let Some(refs) = snapshot_data.refs {
            // Sort by ref ID numerically for stable ordering
            let mut ref_entries: Vec<_> = refs.into_iter().collect();
            ref_entries.sort_by(|a, b| {
                let a_num: usize = a.0.trim_start_matches('e').parse().unwrap_or(0);
                let b_num: usize = b.0.trim_start_matches('e').parse().unwrap_or(0);
                a_num.cmp(&b_num)
            });

            for (ref_id, info) in ref_entries {
                elements.push(ParsedElement {
                    ref_id,
                    role: info.role.unwrap_or_else(|| "unknown".to_string()),
                    label: info.name,
                });
            }
        }

        Ok(SnapshotResult {
            elements,
            snapshot_text: snapshot_data.snapshot,
            origin: snapshot_data.origin,
        })
    }

    // MARK: - 1.4 Interaction Methods (async + success checking)

    /// Click an element by its agent-browser ref
    pub async fn click(
        &self,
        session: &str,
        cdp_port: u16,
        ab_ref: &str,
    ) -> Result<String, String> {
        let ref_arg = if ab_ref.starts_with('@') {
            ab_ref.to_string()
        } else {
            format!("@{}", ab_ref)
        };
        let response = self.execute(session, cdp_port, &["click", &ref_arg]).await?;
        if response.success {
            Ok(response.data.map(|d| d.to_string()).unwrap_or_default())
        } else {
            Err(response.error.unwrap_or_else(|| "Click failed".to_string()))
        }
    }

    /// Fill a field (clear + type) by its agent-browser ref
    pub async fn fill(
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
        let response = self.execute(session, cdp_port, &["fill", &ref_arg, text]).await?;
        if response.success {
            Ok(response.data.map(|d| d.to_string()).unwrap_or_default())
        } else {
            Err(response.error.unwrap_or_else(|| "Fill failed".to_string()))
        }
    }

    /// Type text into an element (append, no clear) by its agent-browser ref
    pub async fn type_text(
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
        let response = self.execute(session, cdp_port, &["type", &ref_arg, text]).await?;
        if response.success {
            Ok(response.data.map(|d| d.to_string()).unwrap_or_default())
        } else {
            Err(response.error.unwrap_or_else(|| "Type failed".to_string()))
        }
    }

    /// Press a key (headless via CDP)
    pub async fn press(
        &self,
        session: &str,
        cdp_port: u16,
        key: &str,
    ) -> Result<(), String> {
        let response = self.execute(session, cdp_port, &["press", key]).await?;
        if response.success {
            Ok(())
        } else {
            Err(response.error.unwrap_or_else(|| "Press failed".to_string()))
        }
    }

    /// Scroll in a direction (headless via CDP)
    pub async fn scroll(
        &self,
        session: &str,
        cdp_port: u16,
        direction: &str,
        amount: i32,
    ) -> Result<(), String> {
        let amount_str = amount.to_string();
        let response = self.execute(session, cdp_port, &["scroll", direction, &amount_str]).await?;
        if response.success {
            Ok(())
        } else {
            Err(response.error.unwrap_or_else(|| "Scroll failed".to_string()))
        }
    }

    // MARK: - Wait

    /// Wait for an element, time, or page load state via agent-browser.
    pub async fn wait(
        &self,
        session: &str,
        cdp_port: u16,
        args: &[&str],
    ) -> Result<String, String> {
        let mut cmd_args = vec!["wait"];
        cmd_args.extend_from_slice(args);
        let response = self.execute(session, cdp_port, &cmd_args).await?;
        if response.success {
            Ok(response.data.map(|d| d.to_string()).unwrap_or_default())
        } else {
            Err(response.error.unwrap_or_else(|| "Wait failed".to_string()))
        }
    }

    // MARK: - Get Web Content

    /// Get text/title/url from web content via agent-browser.
    pub async fn get_web(
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
        let response = self.execute(session, cdp_port, &args).await?;
        if response.success {
            // For get commands, extract the text content from data
            match response.data {
                Some(serde_json::Value::String(s)) => Ok(s),
                Some(d) => Ok(d.to_string()),
                None => Ok(String::new()),
            }
        } else {
            Err(response.error.unwrap_or_else(|| "Get failed".to_string()))
        }
    }

    // MARK: - Lifecycle

    /// Establish a persistent CDP connection for a session
    pub async fn connect(&mut self, session: &str, cdp_port: u16) -> Result<(), String> {
        let port_str = cdp_port.to_string();
        let response = self.execute_session_only(session, &["connect", &port_str]).await?;
        if response.success {
            self.active_sessions.insert(session.to_string(), cdp_port);
            Ok(())
        } else {
            Err(response.error.unwrap_or_else(|| "Connect failed".to_string()))
        }
    }

    /// Close an agent-browser session
    pub async fn close(&mut self, session: &str) -> Result<(), String> {
        let response = self.execute_session_only(session, &["close"]).await;
        self.active_sessions.remove(session);
        match response {
            Ok(r) if r.success => Ok(()),
            Ok(r) => Err(r.error.unwrap_or_else(|| "Close failed".to_string())),
            Err(e) => Err(e),
        }
    }

    /// Close all active sessions (for daemon shutdown)
    pub async fn close_all(&mut self) {
        let sessions: Vec<String> = self.active_sessions.keys().cloned().collect();
        for session in sessions {
            if let Err(e) = self.close(&session).await {
                eprintln!(
                    "[BrowserBridge] Failed to close session '{}': {}",
                    session, e
                );
            }
        }
    }
}

// MARK: - Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_snapshot_json_response() {
        let json_str = r#"{
            "success": true,
            "data": {
                "origin": "https://example.com",
                "refs": {
                    "e1": {"name": "Home", "role": "button"},
                    "e2": {"name": "Search", "role": "textbox"},
                    "e3": {"name": null, "role": "separator"}
                },
                "snapshot": "- button \"Home\" [ref=e1]\n- textbox \"Search\" [ref=e2]\n- separator [ref=e3]"
            },
            "error": null
        }"#;

        let resp: AgentBrowserResponse = serde_json::from_str(json_str).unwrap();
        assert!(resp.success);
        assert!(resp.error.is_none());

        let data: SnapshotJsonData = serde_json::from_value(resp.data.unwrap()).unwrap();
        assert_eq!(data.origin.as_deref(), Some("https://example.com"));
        assert!(data.snapshot.is_some());

        let refs = data.refs.unwrap();
        assert_eq!(refs.len(), 3);

        let home = refs.get("e1").unwrap();
        assert_eq!(home.name.as_deref(), Some("Home"));
        assert_eq!(home.role.as_deref(), Some("button"));

        let sep = refs.get("e3").unwrap();
        assert!(sep.name.is_none());
        assert_eq!(sep.role.as_deref(), Some("separator"));
    }

    #[test]
    fn test_parse_action_json_response() {
        let json_str = r#"{"success": true, "data": {"clicked": "@e5"}, "error": null}"#;
        let resp: AgentBrowserResponse = serde_json::from_str(json_str).unwrap();
        assert!(resp.success);
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_parse_error_json_response() {
        let json_str = r#"{"success": false, "data": null, "error": "Element not found: @e99"}"#;
        let resp: AgentBrowserResponse = serde_json::from_str(json_str).unwrap();
        assert!(!resp.success);
        assert_eq!(resp.error.as_deref(), Some("Element not found: @e99"));
    }

    #[test]
    fn test_snapshot_result_from_json() {
        let json_str = r#"{
            "success": true,
            "data": {
                "origin": "https://spotify.com",
                "refs": {
                    "e14": {"name": "Home", "role": "button"},
                    "e32": {"name": "What do you want to play?", "role": "combobox"},
                    "e8": {"name": "Main", "role": "navigation"}
                },
                "snapshot": "- button \"Home\" [ref=e14]\n- combobox \"What do you want to play?\" [ref=e32]\n- navigation \"Main\" [ref=e8]"
            },
            "error": null
        }"#;

        let resp: AgentBrowserResponse = serde_json::from_str(json_str).unwrap();
        let data: SnapshotJsonData = serde_json::from_value(resp.data.unwrap()).unwrap();
        let refs = data.refs.unwrap();

        // Build elements sorted by ref number
        let mut ref_entries: Vec<_> = refs.into_iter().collect();
        ref_entries.sort_by(|a, b| {
            let a_num: usize = a.0.trim_start_matches('e').parse().unwrap_or(0);
            let b_num: usize = b.0.trim_start_matches('e').parse().unwrap_or(0);
            a_num.cmp(&b_num)
        });

        let elements: Vec<ParsedElement> = ref_entries
            .into_iter()
            .map(|(ref_id, info)| ParsedElement {
                ref_id,
                role: info.role.unwrap_or_else(|| "unknown".to_string()),
                label: info.name,
            })
            .collect();

        assert_eq!(elements.len(), 3);
        assert_eq!(elements[0].ref_id, "e8");
        assert_eq!(elements[0].role, "navigation");
        assert_eq!(elements[0].label.as_deref(), Some("Main"));

        assert_eq!(elements[1].ref_id, "e14");
        assert_eq!(elements[1].role, "button");
        assert_eq!(elements[1].label.as_deref(), Some("Home"));

        assert_eq!(elements[2].ref_id, "e32");
        assert_eq!(elements[2].role, "combobox");
    }

    #[test]
    fn test_bridge_not_available() {
        let bridge = BrowserBridge {
            binary_path: None,
            active_sessions: HashMap::new(),
        };
        assert!(!bridge.is_available());
    }

    #[tokio::test]
    async fn test_execute_not_available() {
        let bridge = BrowserBridge {
            binary_path: None,
            active_sessions: HashMap::new(),
        };
        let result = bridge.execute("test", 9222, &["snapshot", "-i"]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("agent-browser not found"));
    }

    #[test]
    fn test_bridge_detect() {
        let bridge = BrowserBridge::new();
        let _ = bridge.is_available();
    }

    #[test]
    fn test_session_tracking() {
        let mut bridge = BrowserBridge {
            binary_path: None,
            active_sessions: HashMap::new(),
        };

        bridge.active_sessions.insert("spotify".to_string(), 9371);
        bridge.active_sessions.insert("chrome".to_string(), 9222);
        assert_eq!(bridge.active_sessions.len(), 2);

        bridge.active_sessions.remove("spotify");
        assert_eq!(bridge.active_sessions.len(), 1);
        assert!(!bridge.active_sessions.contains_key("spotify"));
        assert!(bridge.active_sessions.contains_key("chrome"));
    }
}
