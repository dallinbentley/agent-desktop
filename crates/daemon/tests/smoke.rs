mod common;

use agent_computer_shared::protocol::Request;
use common::TestDaemon;

#[tokio::test]
async fn test_daemon_starts_and_responds_to_status() {
    let daemon = TestDaemon::start().await;

    let request = Request {
        id: "test-1".to_string(),
        command: "status".to_string(),
        args: serde_json::json!({}),
        options: None,
    };

    let response = daemon.send_request(&request).await;
    assert!(response.success, "Status request should succeed");
    assert_eq!(response.id, "test-1");
}

#[tokio::test]
async fn test_daemon_handles_unknown_command() {
    let daemon = TestDaemon::start().await;

    let request = Request {
        id: "test-2".to_string(),
        command: "nonexistent".to_string(),
        args: serde_json::json!({}),
        options: None,
    };

    let response = daemon.send_request(&request).await;
    assert!(!response.success, "Unknown command should fail");
    assert!(
        response.error.is_some(),
        "Should have error info for unknown command"
    );
}

#[tokio::test]
async fn test_daemon_handles_malformed_json() {
    let daemon = TestDaemon::start().await;

    let raw_response = daemon.send_raw("this is not json at all").await;
    let parsed: serde_json::Value =
        serde_json::from_str(&raw_response).expect("Response should still be valid JSON");

    assert_eq!(
        parsed["success"], false,
        "Malformed JSON should return failure"
    );
}

#[tokio::test]
async fn test_daemon_handles_multiple_sequential_requests() {
    let daemon = TestDaemon::start().await;

    for i in 0..3 {
        let request = Request {
            id: format!("seq-{}", i),
            command: "status".to_string(),
            args: serde_json::json!({}),
            options: None,
        };

        let response = daemon.send_request(&request).await;
        assert!(response.success);
        assert_eq!(response.id, format!("seq-{}", i));
    }
}
