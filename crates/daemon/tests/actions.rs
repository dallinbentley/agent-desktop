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

// Use a shared daemon for all action tests



// --- Group 6: Action Integration Tests ---

/// 6.1 Test click with invalid ref @e9999 — verify error "ref not found"
#[tokio::test]
async fn test_click_invalid_ref() {
    let daemon = TestDaemon::start().await;

    // First do a snapshot so the ref map exists but doesn't have @e9999
    let snap_req = make_request(
        "snap_for_click",
        "snapshot",
        serde_json::json!({
            "interactive": true,
            "app": "Finder"
        }),
    );
    let snap_resp = daemon.send_request(&snap_req).await;
    assert!(snap_resp.success, "Snapshot should succeed first");

    // Now try to click an invalid ref
    let req = make_request(
        "click_invalid",
        "click",
        serde_json::json!({
            "ref": "@e9999"
        }),
    );
    let resp = daemon.send_request(&req).await;

    assert!(!resp.success, "Click with invalid ref should fail");
    assert!(resp.error.is_some());
    let error = resp.error.unwrap();
    assert_eq!(error.code, "REF_NOT_FOUND");
}

/// 6.2 Test press escape --app Finder — verify success response
#[tokio::test]
async fn test_press_escape() {
    let daemon = TestDaemon::start().await;
    let req = make_request(
        "press_esc",
        "press",
        serde_json::json!({
            "key": "escape",
            "app": "Finder"
        }),
    );
    let resp = daemon.send_request(&req).await;

    assert!(resp.success, "Press escape should succeed: {:?}", resp.error);
    if let Some(ResponseData::Press(data)) = &resp.data {
        assert_eq!(data.key, "escape");
    } else {
        panic!("Expected Press response data, got {:?}", resp.data);
    }
}

/// 6.3 Test press key combo cmd+shift+n — verify success response
/// Note: This tests the modifier parsing. We use a combo that won't cause
/// destructive side effects. cmd+shift+n in Finder creates a new folder.
/// We use press without --app so it goes to current frontmost app.
#[tokio::test]
async fn test_press_key_combo() {
    let daemon = TestDaemon::start().await;
    let req = make_request(
        "press_combo",
        "press",
        serde_json::json!({
            "key": "n",
            "modifiers": ["cmd", "shift"]
        }),
    );
    let resp = daemon.send_request(&req).await;

    assert!(resp.success, "Press key combo should succeed: {:?}", resp.error);
    if let Some(ResponseData::Press(data)) = &resp.data {
        assert_eq!(data.key, "n");
        assert!(data.modifiers.contains(&"cmd".to_string()));
        assert!(data.modifiers.contains(&"shift".to_string()));
    } else {
        panic!("Expected Press response data, got {:?}", resp.data);
    }
}

/// 6.4 Test scroll down 100 — verify success response with direction/amount
#[tokio::test]
async fn test_scroll_down() {
    let daemon = TestDaemon::start().await;
    let req = make_request(
        "scroll_down",
        "scroll",
        serde_json::json!({
            "direction": "down",
            "amount": 100
        }),
    );
    let resp = daemon.send_request(&req).await;

    assert!(resp.success, "Scroll down should succeed: {:?}", resp.error);
    if let Some(ResponseData::Scroll(data)) = &resp.data {
        assert_eq!(data.direction, "down");
        assert_eq!(data.amount, 100);
    } else {
        panic!("Expected Scroll response data, got {:?}", resp.data);
    }
}

/// 6.5 Test fill with invalid ref — verify error
#[tokio::test]
async fn test_fill_invalid_ref() {
    let daemon = TestDaemon::start().await;

    // Take a snapshot first to populate the ref map
    let snap_req = make_request(
        "snap_for_fill",
        "snapshot",
        serde_json::json!({
            "interactive": true,
            "app": "Finder"
        }),
    );
    let snap_resp = daemon.send_request(&snap_req).await;
    assert!(snap_resp.success);

    let req = make_request(
        "fill_invalid",
        "fill",
        serde_json::json!({
            "ref": "@e9999",
            "text": "test text"
        }),
    );
    let resp = daemon.send_request(&req).await;

    assert!(!resp.success, "Fill with invalid ref should fail");
    assert!(resp.error.is_some());
    let error = resp.error.unwrap();
    assert_eq!(error.code, "REF_NOT_FOUND");
}

/// 6.6 Test type without ref (types into frontmost app) — verify success
#[tokio::test]
async fn test_type_without_ref() {
    let daemon = TestDaemon::start().await;
    let req = make_request(
        "type_no_ref",
        "type",
        serde_json::json!({
            "text": ""
        }),
    );
    let resp = daemon.send_request(&req).await;

    assert!(resp.success, "Type without ref should succeed: {:?}", resp.error);
    if let Some(ResponseData::Type(data)) = &resp.data {
        assert!(data.r#ref.is_none(), "ref should be None when typing without target");
    } else {
        panic!("Expected Type response data, got {:?}", resp.data);
    }
}

/// 6.7 Test click after snapshot (valid ref) — snapshot Finder, click first ref
#[tokio::test]
async fn test_click_valid_ref_after_snapshot() {
    let daemon = TestDaemon::start().await;

    // Snapshot Finder with interactive mode to get refs
    let snap_req = make_request(
        "snap_for_valid_click",
        "snapshot",
        serde_json::json!({
            "interactive": true,
            "app": "Finder"
        }),
    );
    let snap_resp = daemon.send_request(&snap_req).await;
    assert!(snap_resp.success, "Snapshot should succeed");

    if let Some(ResponseData::Snapshot(snap_data)) = &snap_resp.data {
        if snap_data.ref_count > 0 {
            // Click the first ref (@e1)
            let click_req = make_request(
                "click_valid",
                "click",
                serde_json::json!({
                    "ref": "@e1"
                }),
            );
            let click_resp = daemon.send_request(&click_req).await;

            assert!(
                click_resp.success,
                "Click on valid ref @e1 should succeed: {:?}",
                click_resp.error
            );
            if let Some(ResponseData::Click(click_data)) = &click_resp.data {
                // The ref might be "e1" or "@e1" depending on how the daemon stores it
                let ref_val = click_data.r#ref.as_deref().unwrap_or("");
                assert!(
                    ref_val == "e1" || ref_val == "@e1",
                    "Expected ref 'e1' or '@e1', got '{}'", ref_val
                );
            }
        }
        // If ref_count is 0, skip — Finder might have no interactive elements visible
    }
}
