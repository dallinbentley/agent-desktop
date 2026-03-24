use serde::{Deserialize, Serialize};

// MARK: - Request

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: String,
    pub command: String,
    pub args: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<RequestOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,
}

// MARK: - Command Args

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotArgs {
    #[serde(default = "default_true")]
    pub interactive: bool,
    #[serde(default)]
    pub compact: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(default)]
    pub double: bool,
    #[serde(default)]
    pub right: bool,
    /// Bring app to foreground (required for coordinate clicks in --app mode)
    #[serde(default)]
    pub foreground: bool,
    /// Target app name (for headless click routing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    /// Skip post-click wait (for SPA navigation)
    #[serde(default)]
    pub no_wait: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillArgs {
    pub r#ref: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressArgs {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifiers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollArgs {
    pub direction: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotArgs {
    #[serde(default)]
    pub full: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenArgs {
    pub target: String,
    #[serde(default)]
    pub with_cdp: bool,
    #[serde(default)]
    pub background: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetArgs {
    pub what: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitArgs {
    /// Element @ref (e.g. @e5) or milliseconds (e.g. "2000")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_or_ms: Option<String>,
    /// Wait for page load state: "networkidle", "domcontentloaded", "load"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load: Option<String>,
    /// Target app name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
}

// MARK: - Response

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub id: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ResponseData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::types::ErrorInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<Timing>,
}

impl Response {
    pub fn ok(id: String, data: ResponseData, elapsed_ms: f64) -> Self {
        Self { id, success: true, data: Some(data), error: None, timing: Some(Timing { elapsed_ms }) }
    }
    pub fn fail(id: String, error: crate::types::ErrorInfo, elapsed_ms: f64) -> Self {
        Self { id, success: false, data: None, error: Some(error), timing: Some(Timing { elapsed_ms }) }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timing {
    pub elapsed_ms: f64,
}

// MARK: - Response Data

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_type")]
pub enum ResponseData {
    #[serde(rename = "snapshot")]
    Snapshot(SnapshotData),
    #[serde(rename = "click")]
    Click(ClickData),
    #[serde(rename = "fill")]
    Fill(FillData),
    #[serde(rename = "type")]
    Type(TypeData),
    #[serde(rename = "press")]
    Press(PressData),
    #[serde(rename = "scroll")]
    Scroll(ScrollData),
    #[serde(rename = "screenshot")]
    Screenshot(ScreenshotData),
    #[serde(rename = "open")]
    Open(OpenData),
    #[serde(rename = "getApps")]
    GetApps(GetAppsData),
    #[serde(rename = "getText")]
    GetText(GetTextData),
    #[serde(rename = "status")]
    Status(StatusData),
    #[serde(rename = "wait")]
    Wait(WaitData),
}

// MARK: - Data Payloads

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotData {
    pub text: String,
    pub ref_count: i32,
    pub app: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<String>,
    /// Profiling report (only present when --verbose is used)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    pub coordinates: Point,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element: Option<ElementInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillData {
    pub r#ref: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressData {
    pub key: String,
    #[serde(default)]
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollData {
    pub direction: String,
    pub amount: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotData {
    pub path: String,
    pub width: u32,
    pub height: u32,
    #[serde(default = "default_one")]
    pub scale: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_origin_x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_origin_y: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
}

fn default_one() -> u32 { 1 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenData {
    pub app: String,
    pub pid: i32,
    pub was_running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cdp_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAppsData {
    pub apps: Vec<AppInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTextData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitData {
    pub waited_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusData {
    pub daemon_pid: i32,
    pub accessibility_permission: bool,
    pub screen_recording_permission: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontmost_app: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontmost_pid: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontmost_window: Option<String>,
    pub ref_map_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_map_age_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_cdp_connections: Option<i32>,
}

// MARK: - Supporting Types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInfo {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub pid: i32,
    pub is_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_json_roundtrip() {
        let req = Request {
            id: "test_1".to_string(),
            command: "snapshot".to_string(),
            args: serde_json::json!({"interactive": true, "depth": 10}),
            options: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, "test_1");
        assert_eq!(decoded.command, "snapshot");
    }

    #[test]
    fn test_response_ok_roundtrip() {
        let resp = Response::ok(
            "r1".to_string(),
            ResponseData::Status(StatusData {
                daemon_pid: 1234,
                accessibility_permission: true,
                screen_recording_permission: true,
                frontmost_app: Some("Finder".to_string()),
                frontmost_pid: Some(456),
                frontmost_window: None,
                ref_map_count: 0,
                ref_map_age_ms: None,
                active_cdp_connections: Some(0),
            }),
            15.5,
        );
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: Response = serde_json::from_str(&json).unwrap();
        assert!(decoded.success);
        assert_eq!(decoded.id, "r1");
    }

    #[test]
    fn test_response_fail_roundtrip() {
        let resp = Response::fail(
            "r2".to_string(),
            crate::types::ErrorInfo {
                code: "REF_NOT_FOUND".to_string(),
                message: "Element @e3 not found.".to_string(),
                suggestion: Some("Run `snapshot` to refresh.".to_string()),
            },
            2.1,
        );
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: Response = serde_json::from_str(&json).unwrap();
        assert!(!decoded.success);
        assert!(decoded.error.is_some());
    }
}
