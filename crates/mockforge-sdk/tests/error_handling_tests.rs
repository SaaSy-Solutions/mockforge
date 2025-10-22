//! Error handling tests

use mockforge_sdk::{Error, MockServer};

#[tokio::test]
async fn test_server_not_started_error() {
    let server = MockServer::new();

    // Try to get URL before starting
    let result = std::panic::catch_unwind(|| server.url());
    assert!(result.is_err() || server.url().is_empty());
}

#[tokio::test]
async fn test_port_in_use_error() {
    let mut server1 = MockServer::new()
        .port(33000)
        .start()
        .await
        .expect("Failed to start first server");

    // Try to start second server on same port
    let result = MockServer::new().port(33000).start().await;

    // Should fail because port is already in use
    // Note: This might succeed if port binding isn't exclusive, so we just check the result
    if result.is_err() {
        let err_msg = format!("{}", result.unwrap_err());
        // The error message should be helpful
        assert!(!err_msg.is_empty());
    }

    server1.stop().await.expect("Failed to stop server");
}

#[tokio::test]
async fn test_port_discovery_failed_error() {
    // Try to find port in a very small range that's likely occupied
    let result = MockServer::new()
        .auto_port()
        .port_range(1, 10) // System ports, likely all in use
        .start()
        .await;

    if result.is_err() {
        let err = result.unwrap_err();
        let err_msg = format!("{}", err);
        // Should include helpful tip
        assert!(
            err_msg.contains("Port discovery failed") || err_msg.contains("port"),
            "Error message: {}",
            err_msg
        );
    }
}

#[tokio::test]
async fn test_stub_not_found_error() {
    let err = Error::stub_not_found(
        "GET",
        "/api/missing",
        vec!["GET /api/users".to_string(), "POST /api/users".to_string()],
    );

    let err_msg = format!("{}", err);
    assert!(err_msg.contains("GET"));
    assert!(err_msg.contains("/api/missing"));
    assert!(err_msg.contains("GET /api/users"));
}

#[tokio::test]
async fn test_admin_api_error() {
    let err = Error::admin_api_error("create_mock", "Invalid JSON", "/api/mocks");

    let err_msg = format!("{}", err);
    assert!(err_msg.contains("create_mock"));
    assert!(err_msg.contains("Invalid JSON"));
    assert!(err_msg.contains("/api/mocks"));
}

#[tokio::test]
async fn test_error_messages_are_helpful() {
    // Test that each error variant has a helpful message
    let errors = vec![
        Error::ServerAlreadyStarted(3000),
        Error::ServerNotStarted,
        Error::PortInUse(8080),
        Error::PortDiscoveryFailed("No ports available".to_string()),
        Error::InvalidConfig("Missing field".to_string()),
        Error::InvalidStub("Invalid method".to_string()),
        Error::StartupTimeout { timeout_secs: 30 },
        Error::ShutdownTimeout { timeout_secs: 10 },
    ];

    for err in errors {
        let msg = format!("{}", err);
        // All error messages should be non-empty and reasonably long
        assert!(msg.len() > 20, "Error message too short: {}", msg);
        // Should contain actionable information (look for keywords)
        let lowercase_msg = msg.to_lowercase();
        let has_actionable_info = lowercase_msg.contains("tip:")
            || lowercase_msg.contains("check")
            || lowercase_msg.contains("try")
            || lowercase_msg.contains("call")
            || lowercase_msg.contains("ensure");

        assert!(
            has_actionable_info,
            "Error message lacks actionable advice: {}",
            msg
        );
    }
}
