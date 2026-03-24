mod common;

use common::TestDaemon;
use agent_desktop_shared::protocol::*;
use std::sync::Once;

/// Helper to build a Request.
fn make_request(id: &str, command: &str, args: serde_json::Value) -> Request {
    Request {
        id: id.to_string(),
        command: command.to_string(),
        args,
        options: None,
    }
}

// Screenshot tests use ScreenCaptureKit which can't handle concurrent capture
// from multiple daemon processes. Run all screenshot tests through a single daemon
// sequentially in one test function.

/// 7.1-7.3 Screenshot integration tests (serialized to avoid ScreenCaptureKit conflicts)
#[tokio::test]
async fn test_screenshots() {
    let daemon = TestDaemon::start().await;

    // 7.1 Screenshot --app Finder
    {
        let req = make_request(
            "screenshot_finder",
            "screenshot",
            serde_json::json!({ "app": "Finder" }),
        );
        let resp = daemon.send_request(&req).await;

        // Finder might not have a visible window
        if resp.success {
            if let Some(ResponseData::Screenshot(data)) = &resp.data {
                assert!(!data.path.is_empty(), "Screenshot path should not be empty");
                let path = std::path::Path::new(&data.path);
                assert!(path.exists(), "Screenshot file should exist at {}", data.path);
                let metadata = std::fs::metadata(path).expect("Should read file metadata");
                assert!(metadata.len() > 0, "Screenshot file should be non-empty");
                assert!(data.width > 0, "Width should be positive");
                assert!(data.height > 0, "Height should be positive");
                let _ = std::fs::remove_file(path);
            }
        } else {
            let err = resp.error.as_ref().unwrap();
            assert!(
                err.message.contains("Window not found") || err.message.contains("not found"),
                "Expected 'Window not found' error, got: {}", err.message
            );
        }
    }

    // 7.2 Screenshot --full
    {
        let req = make_request(
            "screenshot_full",
            "screenshot",
            serde_json::json!({ "full": true }),
        );
        let resp = daemon.send_request(&req).await;

        assert!(resp.success, "Full screenshot should succeed: {:?}", resp.error);
        if let Some(ResponseData::Screenshot(data)) = &resp.data {
            assert!(!data.path.is_empty(), "Screenshot path should not be empty");
            let path = std::path::Path::new(&data.path);
            assert!(path.exists(), "Screenshot file should exist at {}", data.path);
            let metadata = std::fs::metadata(path).expect("Should read file metadata");
            assert!(metadata.len() > 0, "Screenshot file should be non-empty");
            assert!(data.width > 0, "Width should be positive");
            assert!(data.height > 0, "Height should be positive");
            let _ = std::fs::remove_file(path);
        }
    }

    // 7.3 Screenshot of non-existent app
    {
        let req = make_request(
            "screenshot_noapp",
            "screenshot",
            serde_json::json!({ "app": "ThisAppDoesNotExist12345" }),
        );
        let resp = daemon.send_request(&req).await;

        assert!(!resp.success, "Screenshot of non-existent app should fail");
        assert!(resp.error.is_some(), "Should have error info");
        let error = resp.error.unwrap();
        assert!(!error.message.is_empty(), "Error message should not be empty");
    }
}
