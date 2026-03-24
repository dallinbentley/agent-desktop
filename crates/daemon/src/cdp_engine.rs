// CDP Engine — Chrome DevTools Protocol client over WebSocket
// Tasks 6.1-6.6

use std::collections::HashMap;
use std::io::Read as _;
use std::net::TcpStream;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use agent_computer_shared::types::{ElementRef, RefSource};
use serde::Deserialize;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

// MARK: - CDP Types

/// CDP browser version info from /json/version
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CdpVersionInfo {
    #[serde(rename = "Browser")]
    pub browser: Option<String>,
    #[serde(rename = "Protocol-Version")]
    pub protocol_version: Option<String>,
    #[serde(rename = "User-Agent")]
    pub user_agent: Option<String>,
    #[serde(rename = "V8-Version")]
    pub v8_version: Option<String>,
    #[serde(rename = "WebKit-Version")]
    pub webkit_version: Option<String>,
    #[serde(rename = "webSocketDebuggerUrl")]
    pub web_socket_debugger_url: Option<String>,
}

/// CDP target/tab from /json/list
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct CdpTarget {
    pub description: Option<String>,
    pub devtools_frontend_url: Option<String>,
    pub id: String,
    pub title: Option<String>,
    pub r#type: Option<String>,
    pub url: Option<String>,
    pub web_socket_debugger_url: Option<String>,
}

/// A node from CDP Accessibility.getFullAXTree
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct CdpAXNode {
    pub node_id: String,
    #[serde(default)]
    pub ignored: bool,
    pub role: Option<CdpAXValue>,
    pub name: Option<CdpAXValue>,
    #[serde(default)]
    pub properties: Vec<CdpAXProperty>,
    #[serde(default)]
    pub child_ids: Vec<String>,
    #[serde(alias = "backendDOMNodeId")]
    pub backend_dom_node_id: Option<i64>,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CdpAXValue {
    pub value: Option<serde_json::Value>,
}

impl CdpAXValue {
    pub fn as_str(&self) -> Option<&str> {
        self.value.as_ref().and_then(|v| v.as_str())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CdpAXProperty {
    pub name: Option<String>,
    pub value: Option<CdpAXValue>,
}

/// Result of a CDP command
#[derive(Debug, Clone, Deserialize)]
pub struct CdpResponse {
    pub id: Option<i64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<CdpError>,
    pub method: Option<String>,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CdpError {
    pub code: Option<i64>,
    pub message: Option<String>,
}

// MARK: - CDP Interactive Roles

/// Roles from CDP AX tree that correspond to interactive elements
fn is_interactive_cdp_role(role: &str) -> bool {
    matches!(
        role,
        "button"
            | "link"
            | "textbox"
            | "searchbox"
            | "checkbox"
            | "radio"
            | "combobox"
            | "listbox"
            | "menuitem"
            | "menuitemcheckbox"
            | "menuitemradio"
            | "option"
            | "slider"
            | "spinbutton"
            | "switch"
            | "tab"
            | "treeitem"
            | "textField"
            | "TextField"
    )
}

/// Map CDP AX roles to our display format
fn normalize_cdp_role(role: &str) -> &str {
    match role {
        "textbox" | "searchbox" | "textField" | "TextField" => "textbox",
        "menuitem" | "menuitemcheckbox" | "menuitemradio" => "menuitem",
        "radio" => "radiobutton",
        "combobox" | "listbox" => "combobox",
        "spinbutton" => "incrementor",
        "treeitem" => "outline",
        other => other,
    }
}

// MARK: - CDP Connection

/// A live CDP WebSocket connection to a single tab/page
pub struct CdpConnection {
    ws: WebSocket<MaybeTlsStream<TcpStream>>,
    next_id: AtomicI64,
    pub port: u16,
    pub target_id: String,
}

impl CdpConnection {
    /// Connect to a CDP target's WebSocket debugger URL
    fn from_ws_url(ws_url: &str, port: u16, target_id: String) -> Result<Self, String> {
        let (ws, _response) = tungstenite::connect(ws_url)
            .map_err(|e| format!("WebSocket connect failed: {e}"))?;
        Ok(Self {
            ws,
            next_id: AtomicI64::new(1),
            port,
            target_id,
        })
    }

    /// Send a CDP JSON-RPC command and wait for the matching response
    pub fn send_command(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let msg = serde_json::json!({
            "id": id,
            "method": method,
            "params": params,
        });
        self.ws
            .send(Message::Text(msg.to_string()))
            .map_err(|e| format!("WebSocket send error: {e}"))?;

        // Read messages until we get a response with matching id
        loop {
            let raw = self
                .ws
                .read()
                .map_err(|e| format!("WebSocket read error: {e}"))?;
            match raw {
                Message::Text(text) => {
                    let resp: CdpResponse = serde_json::from_str(&text)
                        .map_err(|e| format!("CDP response parse error: {e}"))?;
                    // Check if this is our response (has matching id)
                    if resp.id == Some(id) {
                        if let Some(err) = resp.error {
                            return Err(format!(
                                "CDP error {}: {}",
                                err.code.unwrap_or(-1),
                                err.message.unwrap_or_default()
                            ));
                        }
                        return Ok(resp.result.unwrap_or(serde_json::Value::Null));
                    }
                    // Otherwise it's an event — ignore and continue
                }
                Message::Close(_) => return Err("WebSocket closed".to_string()),
                _ => {} // Ping/Pong/Binary — ignore
            }
        }
    }

    /// Close the WebSocket connection gracefully
    pub fn close(mut self) {
        let _ = self.ws.close(None);
    }
}

// MARK: - 6.5 CDP Port Probing

/// Probe result from a CDP port
#[derive(Debug, Clone)]
pub struct CdpProbeResult {
    pub available: bool,
    pub port: u16,
    pub version_info: Option<CdpVersionInfo>,
}

/// Probe a single CDP port with 500ms timeout (task 6.5)
pub fn probe_cdp_port(port: u16) -> CdpProbeResult {
    let url = format!("http://localhost:{port}/json/version");
    match ureq::get(&url).timeout(Duration::from_millis(500)).call() {
        Ok(resp) => {
            let mut body = String::new();
            if resp.into_reader().read_to_string(&mut body).is_ok() {
                if let Ok(info) = serde_json::from_str::<CdpVersionInfo>(&body) {
                    return CdpProbeResult {
                        available: true,
                        port,
                        version_info: Some(info),
                    };
                }
            }
            CdpProbeResult {
                available: false,
                port,
                version_info: None,
            }
        }
        Err(_) => CdpProbeResult {
            available: false,
            port,
            version_info: None,
        },
    }
}

/// Scan standard CDP ports 9222-9229 and return the first available (task 6.5)
pub fn scan_cdp_ports() -> Option<CdpProbeResult> {
    for port in 9222..=9229 {
        let result = probe_cdp_port(port);
        if result.available {
            return Some(result);
        }
    }
    None
}

/// Probe a specific port, or scan standard range
pub fn find_cdp_port(specific_port: Option<u16>) -> Option<CdpProbeResult> {
    if let Some(port) = specific_port {
        let result = probe_cdp_port(port);
        if result.available {
            return Some(result);
        }
        None
    } else {
        scan_cdp_ports()
    }
}

// MARK: - 6.1 & 6.2 Connect to CDP

/// List targets/pages from a CDP port (task 6.2)
pub fn list_targets(port: u16) -> Result<Vec<CdpTarget>, String> {
    let url = format!("http://localhost:{port}/json/list");
    let resp = ureq::get(&url)
        .timeout(Duration::from_millis(2000))
        .call()
        .map_err(|e| format!("Failed to list CDP targets: {e}"))?;
    let mut body = String::new();
    resp.into_reader()
        .read_to_string(&mut body)
        .map_err(|e| format!("Failed to read target list: {e}"))?;
    serde_json::from_str(&body).map_err(|e| format!("Failed to parse target list: {e}"))
}

/// Find the active/visible page target (task 6.2)
pub fn find_active_page(targets: &[CdpTarget]) -> Option<&CdpTarget> {
    // Prefer targets of type "page"
    let pages: Vec<&CdpTarget> = targets
        .iter()
        .filter(|t| t.r#type.as_deref() == Some("page"))
        .collect();

    if pages.is_empty() {
        return None;
    }

    // If there's only one page, use it
    if pages.len() == 1 {
        return Some(pages[0]);
    }

    // Prefer pages that have a real URL (not about:blank, chrome://, etc.)
    let real_pages: Vec<&&CdpTarget> = pages
        .iter()
        .filter(|t| {
            if let Some(url) = &t.url {
                !url.starts_with("about:")
                    && !url.starts_with("chrome://")
                    && !url.starts_with("chrome-extension://")
                    && !url.starts_with("devtools://")
            } else {
                false
            }
        })
        .collect();

    if let Some(page) = real_pages.first() {
        return Some(page);
    }

    // Fallback: first page
    Some(pages[0])
}

/// Connect to the active page on a CDP port (tasks 6.1 + 6.2)
pub fn connect_to_active_page(port: u16) -> Result<CdpConnection, String> {
    let targets = list_targets(port)?;
    let target = find_active_page(&targets)
        .ok_or_else(|| "No active page found via CDP".to_string())?;

    let ws_url = target
        .web_socket_debugger_url
        .as_ref()
        .ok_or_else(|| "Target has no webSocketDebuggerUrl".to_string())?;

    let target_id = target.id.clone();
    CdpConnection::from_ws_url(ws_url, port, target_id)
}

// MARK: - 6.3 CDP Accessibility Tree

/// Result of a CDP snapshot
pub struct CdpSnapshotResult {
    pub text: String,
    pub refs: Vec<ElementRef>,
}

/// Get the CDP accessibility tree, filter to interactive elements,
/// and produce snapshot text + ElementRef entries (task 6.3)
pub fn get_cdp_snapshot(
    conn: &mut CdpConnection,
    ref_start: usize,
) -> Result<CdpSnapshotResult, String> {
    // Call Accessibility.enable first (required for some browsers)
    let _ = conn.send_command("Accessibility.enable", serde_json::json!({}));

    // Get the full accessibility tree
    let result = conn.send_command(
        "Accessibility.getFullAXTree",
        serde_json::json!({}),
    )?;

    let nodes: Vec<CdpAXNode> = serde_json::from_value(
        result
            .get("nodes")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![])),
    )
    .map_err(|e| format!("Failed to parse AX nodes: {e}"))?;

    // Also get the document info for bounding boxes
    // Enable DOM domain for resolving nodes later
    let _ = conn.send_command("DOM.enable", serde_json::json!({}));

    // Filter to interactive nodes and build refs
    let mut refs = Vec::new();
    let mut lines = Vec::new();
    let mut ref_counter = ref_start;

    for node in &nodes {
        if node.ignored {
            continue;
        }

        let role = match &node.role {
            Some(r) => match r.as_str() {
                Some(s) if !s.is_empty() && s != "none" && s != "generic" => s,
                _ => continue,
            },
            None => continue,
        };

        if !is_interactive_cdp_role(role) {
            continue;
        }

        let label = node
            .name
            .as_ref()
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());

        let display_role = normalize_cdp_role(role);
        let ref_id = format!("e{ref_counter}");
        ref_counter += 1;

        // Format: @eN role "label"
        let line = if let Some(ref lbl) = label {
            format!("  @{ref_id} {display_role} \"{lbl}\"")
        } else {
            format!("  @{ref_id} {display_role}")
        };
        lines.push(line);

        // Parse the node_id as an i64 for CDP node reference
        let cdp_node_id = node.node_id.parse::<i64>().ok();

        refs.push(ElementRef {
            id: ref_id,
            source: RefSource::CDP,
            role: display_role.to_string(),
            label: label.clone(),
            frame: None, // CDP doesn't give easy bounding boxes from AX tree
            ax_path: None,
            ax_actions: None,
            ax_pid: None,
            cdp_node_id: cdp_node_id,
            cdp_backend_node_id: node.backend_dom_node_id,
            cdp_port: Some(conn.port),
        });
    }

    let text = lines.join("\n");

    Ok(CdpSnapshotResult { text, refs })
}

// MARK: - 6.4 CDP Interactions

/// Click an element via CDP (task 6.4)
/// Uses DOM.resolveNode + Runtime.callFunctionOn to invoke .click()
pub fn cdp_click(conn: &mut CdpConnection, backend_node_id: i64) -> Result<(), String> {
    // Resolve the backend node to a remote object
    let resolve_result = conn.send_command(
        "DOM.resolveNode",
        serde_json::json!({
            "backendNodeId": backend_node_id,
        }),
    )?;

    let object_id = resolve_result
        .get("object")
        .and_then(|o| o.get("objectId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Failed to resolve DOM node to object".to_string())?
        .to_string();

    // Focus the element first
    let _ = conn.send_command(
        "DOM.focus",
        serde_json::json!({
            "backendNodeId": backend_node_id,
        }),
    );

    // Call .click() on the element
    conn.send_command(
        "Runtime.callFunctionOn",
        serde_json::json!({
            "objectId": object_id,
            "functionDeclaration": "function() { this.click(); }",
            "returnByValue": true,
        }),
    )?;

    Ok(())
}

/// Click at specific coordinates via CDP Input.dispatchMouseEvent (task 6.4)
pub fn cdp_click_at(conn: &mut CdpConnection, x: f64, y: f64) -> Result<(), String> {
    // mousePressed
    conn.send_command(
        "Input.dispatchMouseEvent",
        serde_json::json!({
            "type": "mousePressed",
            "x": x,
            "y": y,
            "button": "left",
            "clickCount": 1,
        }),
    )?;
    // mouseReleased
    conn.send_command(
        "Input.dispatchMouseEvent",
        serde_json::json!({
            "type": "mouseReleased",
            "x": x,
            "y": y,
            "button": "left",
            "clickCount": 1,
        }),
    )?;
    Ok(())
}

/// Type text into the focused element via CDP (task 6.4)
pub fn cdp_type_text(conn: &mut CdpConnection, text: &str) -> Result<(), String> {
    // Use Input.insertText for simple text
    conn.send_command(
        "Input.insertText",
        serde_json::json!({
            "text": text,
        }),
    )?;
    Ok(())
}

/// Send a key event via CDP (task 6.4)
pub fn cdp_press_key(conn: &mut CdpConnection, key: &str) -> Result<(), String> {
    // Map common key names to CDP key identifiers
    let (key_code, text, code) = match key.to_lowercase().as_str() {
        "enter" | "return" => (13, Some("\r"), "Enter"),
        "tab" => (9, Some("\t"), "Tab"),
        "escape" | "esc" => (27, None, "Escape"),
        "backspace" | "delete" => (8, None, "Backspace"),
        "arrowup" | "up" => (38, None, "ArrowUp"),
        "arrowdown" | "down" => (40, None, "ArrowDown"),
        "arrowleft" | "left" => (37, None, "ArrowLeft"),
        "arrowright" | "right" => (39, None, "ArrowRight"),
        "space" => (32, Some(" "), "Space"),
        _ => {
            // For single characters, send as insertText
            if key.len() == 1 {
                return cdp_type_text(conn, key);
            }
            return Err(format!("Unknown CDP key: {key}"));
        }
    };

    let mut params = serde_json::json!({
        "type": "keyDown",
        "windowsVirtualKeyCode": key_code,
        "nativeVirtualKeyCode": key_code,
        "code": code,
        "key": code,
    });
    if let Some(t) = text {
        params["text"] = serde_json::json!(t);
    }
    conn.send_command("Input.dispatchKeyEvent", params.clone())?;

    params["type"] = serde_json::json!("keyUp");
    conn.send_command("Input.dispatchKeyEvent", params)?;

    Ok(())
}

/// Fill a field: focus → select all → insert text (task 6.4)
pub fn cdp_fill(conn: &mut CdpConnection, backend_node_id: i64, text: &str) -> Result<(), String> {
    // Focus the element
    conn.send_command(
        "DOM.focus",
        serde_json::json!({
            "backendNodeId": backend_node_id,
        }),
    )?;

    // Select all existing content (Ctrl+A / Cmd+A)
    conn.send_command(
        "Input.dispatchKeyEvent",
        serde_json::json!({
            "type": "keyDown",
            "key": "a",
            "code": "KeyA",
            "windowsVirtualKeyCode": 65,
            "nativeVirtualKeyCode": 0,
            "modifiers": 4, // Meta (Cmd on macOS)
        }),
    )?;
    conn.send_command(
        "Input.dispatchKeyEvent",
        serde_json::json!({
            "type": "keyUp",
            "key": "a",
            "code": "KeyA",
            "windowsVirtualKeyCode": 65,
            "nativeVirtualKeyCode": 0,
            "modifiers": 4,
        }),
    )?;

    // Insert the new text (replaces selection)
    cdp_type_text(conn, text)?;

    Ok(())
}

// MARK: - 6.6 CDP Connection Manager

/// Key for identifying a CDP connection
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CdpConnectionKey {
    pub pid: i32,
    pub port: u16,
}

/// Manages active CDP connections, reusing across commands (task 6.6)
pub struct CdpManager {
    connections: HashMap<CdpConnectionKey, CdpConnection>,
}

impl CdpManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    /// Get or create a connection for a given PID/port
    pub fn get_or_connect(
        &mut self,
        pid: i32,
        port: u16,
    ) -> Result<&mut CdpConnection, String> {
        let key = CdpConnectionKey { pid, port };
        if !self.connections.contains_key(&key) {
            let conn = connect_to_active_page(port)?;
            self.connections.insert(key.clone(), conn);
        }
        Ok(self.connections.get_mut(&key).unwrap())
    }

    /// Remove a specific connection
    pub fn disconnect(&mut self, pid: i32, port: u16) {
        let key = CdpConnectionKey { pid, port };
        if let Some(conn) = self.connections.remove(&key) {
            conn.close();
        }
    }

    /// Close all connections (daemon shutdown)
    pub fn shutdown(&mut self) {
        let keys: Vec<CdpConnectionKey> = self.connections.keys().cloned().collect();
        for key in keys {
            if let Some(conn) = self.connections.remove(&key) {
                conn.close();
            }
        }
    }

    /// Number of active connections
    pub fn active_count(&self) -> usize {
        self.connections.len()
    }
}

impl Drop for CdpManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// MARK: - Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_nonexistent_port() {
        // Port 19999 should not have CDP running
        let result = probe_cdp_port(19999);
        assert!(!result.available);
        assert!(result.version_info.is_none());
    }

    #[test]
    fn test_interactive_roles() {
        assert!(is_interactive_cdp_role("button"));
        assert!(is_interactive_cdp_role("link"));
        assert!(is_interactive_cdp_role("textbox"));
        assert!(is_interactive_cdp_role("checkbox"));
        assert!(!is_interactive_cdp_role("generic"));
        assert!(!is_interactive_cdp_role("document"));
        assert!(!is_interactive_cdp_role("heading"));
    }

    #[test]
    fn test_role_normalization() {
        assert_eq!(normalize_cdp_role("textbox"), "textbox");
        assert_eq!(normalize_cdp_role("searchbox"), "textbox");
        assert_eq!(normalize_cdp_role("button"), "button");
        assert_eq!(normalize_cdp_role("menuitem"), "menuitem");
        assert_eq!(normalize_cdp_role("menuitemcheckbox"), "menuitem");
        assert_eq!(normalize_cdp_role("radio"), "radiobutton");
    }

    #[test]
    fn test_parse_cdp_ax_node() {
        let json = r#"{
            "nodeId": "42",
            "ignored": false,
            "role": {"value": "button"},
            "name": {"value": "Submit"},
            "properties": [],
            "childIds": ["43"],
            "backendDOMNodeId": 100
        }"#;
        let node: CdpAXNode = serde_json::from_str(json).unwrap();
        assert_eq!(node.node_id, "42");
        assert!(!node.ignored);
        assert_eq!(node.role.as_ref().unwrap().as_str(), Some("button"));
        assert_eq!(node.name.as_ref().unwrap().as_str(), Some("Submit"));
        assert_eq!(node.backend_dom_node_id, Some(100));
    }

    #[test]
    fn test_parse_version_info() {
        let json = r#"{
            "Browser": "Chrome/120.0.6099.71",
            "Protocol-Version": "1.3",
            "User-Agent": "Mozilla/5.0",
            "V8-Version": "12.0.267.8",
            "WebKit-Version": "537.36",
            "webSocketDebuggerUrl": "ws://localhost:9222/devtools/browser/abc123"
        }"#;
        let info: CdpVersionInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.browser, Some("Chrome/120.0.6099.71".to_string()));
        assert!(info.web_socket_debugger_url.is_some());
    }

    #[test]
    fn test_parse_targets() {
        let json = r#"[{
            "description": "",
            "devtoolsFrontendUrl": "/devtools/inspector.html?ws=localhost:9222/devtools/page/ABC",
            "id": "ABC123",
            "title": "Google",
            "type": "page",
            "url": "https://google.com",
            "webSocketDebuggerUrl": "ws://localhost:9222/devtools/page/ABC"
        }, {
            "id": "DEF456",
            "type": "background_page",
            "url": "chrome-extension://xyz/background.html"
        }]"#;
        let targets: Vec<CdpTarget> = serde_json::from_str(json).unwrap();
        assert_eq!(targets.len(), 2);
        let active = find_active_page(&targets);
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, "ABC123");
    }

    #[test]
    fn test_cdp_manager_new() {
        let mgr = CdpManager::new();
        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn test_cdp_connection_key_hash() {
        let k1 = CdpConnectionKey { pid: 123, port: 9222 };
        let k2 = CdpConnectionKey { pid: 123, port: 9222 };
        let k3 = CdpConnectionKey { pid: 456, port: 9222 };
        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }
}
