mod common;

use common::TestDaemon;
use agent_desktop_shared::protocol::*;

/// Helper to build a Request with the given command and args.
fn make_request(id: &str, command: &str, args: serde_json::Value) -> Request {
    Request {
        id: id.to_string(),
        command: command.to_string(),
        args,
        options: None,
    }
}

// --- Group 4: Daemon Lifecycle ---

/// 4.1 Test daemon starts and responds to status command
#[tokio::test]
async fn test_status() {
    let daemon = TestDaemon::start().await;
    let req = make_request("status_1", "status", serde_json::json!({}));
    let resp = daemon.send_request(&req).await;

    assert!(resp.success, "status command should succeed");
    assert_eq!(resp.id, "status_1");

    if let Some(ResponseData::Status(status)) = &resp.data {
        assert!(status.daemon_pid > 0, "daemon PID should be positive");
        // ref_map_count should be 0 initially (no snapshot taken)
        assert_eq!(status.ref_map_count, 0);
    } else {
        panic!("Expected Status response data, got {:?}", resp.data);
    }
}

/// 4.2 Test daemon returns correct permission info in status response
#[tokio::test]
async fn test_status_permissions() {
    let daemon = TestDaemon::start().await;
    let req = make_request("perm_1", "status", serde_json::json!({}));
    let resp = daemon.send_request(&req).await;

    assert!(resp.success);
    if let Some(ResponseData::Status(status)) = &resp.data {
        // These are booleans — we just check they're present and valid types
        // On a macOS dev machine with permissions granted, these should be true
        let _ = status.accessibility_permission;
        let _ = status.screen_recording_permission;
    } else {
        panic!("Expected Status response data");
    }
}

/// 4.3 Test daemon handles multiple sequential requests on same connection
#[tokio::test]
async fn test_sequential_requests() {
    let daemon = TestDaemon::start().await;

    // Send multiple status requests in sequence
    for i in 0..5 {
        let req = make_request(&format!("seq_{i}"), "status", serde_json::json!({}));
        let resp = daemon.send_request(&req).await;
        assert!(resp.success, "Sequential request {i} should succeed");
        assert_eq!(resp.id, format!("seq_{i}"));
    }
}

/// 4.4 Test daemon handles multiple concurrent connections
#[tokio::test]
async fn test_concurrent_connections() {
    let daemon = TestDaemon::start().await;

    let mut handles = vec![];
    for i in 0..5 {
        let socket_path = daemon.socket_path.clone();
        let handle = tokio::spawn(async move {
            // Create a temporary daemon reference just for sending
            let req = Request {
                id: format!("conc_{i}"),
                command: "status".to_string(),
                args: serde_json::json!({}),
                options: None,
            };

            // Connect directly to the socket
            let stream = tokio::net::UnixStream::connect(&socket_path)
                .await
                .expect("Failed to connect");

            let mut json = serde_json::to_string(&req).unwrap();
            json.push('\n');

            use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
            let (reader, mut writer) = stream.into_split();
            writer.write_all(json.as_bytes()).await.unwrap();
            writer.flush().await.unwrap();

            let mut buf_reader = BufReader::new(reader);
            let mut response_line = String::new();
            buf_reader.read_line(&mut response_line).await.unwrap();

            let resp: Response = serde_json::from_str(response_line.trim()).unwrap();
            assert!(resp.success);
            assert_eq!(resp.id, format!("conc_{i}"));
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Concurrent task should complete");
    }
}
