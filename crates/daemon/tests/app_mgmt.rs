mod common;

use common::TestDaemon;
use agent_desktop_shared::protocol::*;

/// Helper to build a Request.
fn make_request(id: &str, command: &str, args: serde_json::Value) -> Request {
    Request {
        id: id.to_string(),
        command: command.to_string(),
        args,
        options: None,
    }
}

// Use a shared daemon for all app management tests



// --- Group 8: App Management Integration Tests ---

/// 8.1 Test get apps — verify response contains list, includes "Finder"
#[tokio::test]
async fn test_get_apps() {
    let daemon = TestDaemon::start().await;
    let req = make_request(
        "get_apps",
        "get",
        serde_json::json!({
            "what": "apps"
        }),
    );
    let resp = daemon.send_request(&req).await;

    assert!(resp.success, "Get apps should succeed: {:?}", resp.error);
    if let Some(ResponseData::GetApps(data)) = &resp.data {
        assert!(!data.apps.is_empty(), "Should have at least one running app");
        let has_finder = data.apps.iter().any(|app| app.name == "Finder");
        assert!(has_finder, "Running apps should include Finder");

        // Verify app structure
        for app in &data.apps {
            assert!(!app.name.is_empty(), "App name should not be empty");
            assert!(app.pid > 0, "App PID should be positive");
        }
    } else {
        panic!("Expected GetApps response data, got {:?}", resp.data);
    }
}

/// 8.2 Test open Finder — verify success response
#[tokio::test]
async fn test_open_finder() {
    let daemon = TestDaemon::start().await;
    let req = make_request(
        "open_finder",
        "open",
        serde_json::json!({
            "target": "Finder",
            "with_cdp": false,
            "background": true
        }),
    );
    let resp = daemon.send_request(&req).await;

    assert!(resp.success, "Open Finder should succeed: {:?}", resp.error);
    if let Some(ResponseData::Open(data)) = &resp.data {
        assert_eq!(data.app, "Finder");
        assert!(data.pid > 0, "PID should be positive");
        // Finder is always running, so was_running should be true
        assert!(data.was_running, "Finder should already be running");
    } else {
        panic!("Expected Open response data, got {:?}", resp.data);
    }
}

/// 8.3 Test open non-existent app — verify error response
#[tokio::test]
async fn test_open_nonexistent_app() {
    let daemon = TestDaemon::start().await;
    let req = make_request(
        "open_noapp",
        "open",
        serde_json::json!({
            "target": "ThisAppDoesNotExist12345",
            "with_cdp": false,
            "background": false
        }),
    );
    let resp = daemon.send_request(&req).await;

    assert!(!resp.success, "Open non-existent app should fail");
    assert!(resp.error.is_some(), "Should have error info");
    let error = resp.error.unwrap();
    assert_eq!(error.code, "APP_NOT_FOUND");
}

/// 8.4 Test get windows --app Finder — returns data
/// Note: The "get" command handles "apps" but not "windows" as a separate type.
/// Based on the code, we test what the get command supports — which is apps, text, title, url.
/// We test "get apps" filtered behavior here.
#[tokio::test]
async fn test_get_apps_includes_finder_details() {
    let daemon = TestDaemon::start().await;
    let req = make_request(
        "get_apps_detail",
        "get",
        serde_json::json!({
            "what": "apps"
        }),
    );
    let resp = daemon.send_request(&req).await;

    assert!(resp.success, "Get apps should succeed: {:?}", resp.error);
    if let Some(ResponseData::GetApps(data)) = &resp.data {
        let finder = data.apps.iter().find(|app| app.name == "Finder");
        assert!(finder.is_some(), "Finder should be in the app list");
        let finder = finder.unwrap();
        assert!(finder.pid > 0, "Finder PID should be positive");
    } else {
        panic!("Expected GetApps response data");
    }
}
