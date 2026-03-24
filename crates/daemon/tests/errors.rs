mod common;

use common::TestDaemon;
use agent_computer_shared::protocol::*;

/// Helper to build a Request.
fn make_request(id: &str, command: &str, args: serde_json::Value) -> Request {
    Request {
        id: id.to_string(),
        command: command.to_string(),
        args,
        options: None,
    }
}

// Use a shared daemon for all error handling tests



// --- Group 9: Error Handling Integration Tests ---

/// 9.1 Test malformed JSON request — verify daemon doesn't crash, returns error
#[tokio::test]
async fn test_malformed_json() {
    let daemon = TestDaemon::start().await;

    // Send malformed JSON
    let raw_response = daemon.send_raw("this is not json{{{").await;

    // Parse the response — daemon should return a valid JSON error response
    let resp: Response =
        serde_json::from_str(&raw_response).expect("Daemon should return valid JSON even for bad input");

    assert!(!resp.success, "Malformed JSON should result in failure");
    assert!(resp.error.is_some(), "Should have error info");
    let error = resp.error.unwrap();
    assert_eq!(error.code, "INVALID_COMMAND");

    // Verify daemon is still alive by sending a valid request
    let status_req = make_request("after_malformed", "status", serde_json::json!({}));
    let status_resp = daemon.send_request(&status_req).await;
    assert!(
        status_resp.success,
        "Daemon should still work after malformed input"
    );
}

/// 9.2 Test request with unknown command — verify error response
#[tokio::test]
async fn test_unknown_command() {
    let daemon = TestDaemon::start().await;
    let req = make_request(
        "unknown_cmd",
        "nonexistent_command_xyz",
        serde_json::json!({}),
    );
    let resp = daemon.send_request(&req).await;

    assert!(!resp.success, "Unknown command should fail");
    assert!(resp.error.is_some());
    let error = resp.error.unwrap();
    assert_eq!(error.code, "INVALID_COMMAND");
    assert!(
        error.message.contains("nonexistent_command_xyz"),
        "Error should mention the unknown command"
    );
}

/// 9.3 Test fill @e1 with empty refmap (no prior snapshot) — verify "no ref map" error
#[tokio::test]
async fn test_fill_no_refmap() {
    // Use a fresh daemon with no prior snapshot
    let daemon = TestDaemon::start().await;
    let req = make_request(
        "fill_no_refs",
        "fill",
        serde_json::json!({
            "ref": "@e1",
            "text": "test"
        }),
    );
    let resp = daemon.send_request(&req).await;

    assert!(!resp.success, "Fill without prior snapshot should fail");
    assert!(resp.error.is_some());
    let error = resp.error.unwrap();
    assert_eq!(error.code, "NO_REF_MAP");
}

/// 9.4 Test click @xyz (invalid ref format) — verify error
/// When the ref map is populated but the ref doesn't exist, we get REF_NOT_FOUND.
/// When the ref map is empty, we get NO_REF_MAP.
#[tokio::test]
async fn test_invalid_ref_format() {
    let daemon = TestDaemon::start().await;

    // First populate the ref map
    let snap_req = make_request(
        "snap_for_bad_ref",
        "snapshot",
        serde_json::json!({
            "interactive": true,
            "app": "Finder"
        }),
    );
    let snap_resp = daemon.send_request(&snap_req).await;
    assert!(snap_resp.success);

    // Try to click with a malformed ref
    let req = make_request(
        "click_bad_ref",
        "click",
        serde_json::json!({
            "ref": "@xyz"
        }),
    );
    let resp = daemon.send_request(&req).await;

    assert!(!resp.success, "Click with invalid ref format should fail");
    assert!(resp.error.is_some());
    let error = resp.error.unwrap();
    // The error should be REF_NOT_FOUND since the ref map exists but @xyz doesn't
    assert_eq!(error.code, "REF_NOT_FOUND");
}

/// Additional: Test that daemon handles empty JSON object gracefully
#[tokio::test]
async fn test_empty_command() {
    let daemon = TestDaemon::start().await;
    let req = make_request("empty_cmd", "", serde_json::json!({}));
    let resp = daemon.send_request(&req).await;

    assert!(
        !resp.success,
        "Empty command should fail"
    );
    assert!(resp.error.is_some());
    let error = resp.error.unwrap();
    assert_eq!(error.code, "INVALID_COMMAND");
}
