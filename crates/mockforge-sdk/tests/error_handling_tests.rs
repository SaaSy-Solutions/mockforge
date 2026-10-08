//! Error handling tests

use mockforge_sdk::{Error, MockServer};

#[tokio::test]
async fn test_server_builder_creation() {
    let server = MockServer::new();

    // Verify builder can be created and configured without starting
    let builder = server.port(44000).auto_port();
    // Builder should be usable (not panic) — the actual start is tested elsewhere
    drop(builder);
}

#[tokio::test]
async fn test_port_in_use_error() {
    let server1 = Box::pin(MockServer::new().host("127.0.0.1").port(0).start())
        .await
        .expect("Failed to start first server");

    let port = server1.port();
    let address = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    // Capture the platform's native bind error while server1 owns the port.
    // Calling err() drops any unexpectedly successful control listener.
    let native_error = tokio::net::TcpListener::bind(address).await.err();

    let result = Box::pin(MockServer::new().host("127.0.0.1").port(port).start()).await;

    // Clean up both servers even if the second bind unexpectedly succeeds.
    let (second_error, second_stop_result) = match result {
        Ok(server2) => (None, Box::pin(server2.stop()).await),
        Err(error) => (Some(error), Ok(())),
    };
    let first_stop_result = Box::pin(server1.stop()).await;

    second_stop_result.expect("Failed to stop second server");
    first_stop_result.expect("Failed to stop first server");

    assert_ne!(port, 0, "First server should report its assigned port");

    let native_error = native_error.expect("Control bind should fail on the occupied port");
    assert_eq!(native_error.kind(), std::io::ErrorKind::AddrInUse);

    let error = second_error.expect("Second server should not bind to the occupied port");
    match error {
        Error::General(message) => {
            assert_eq!(message, format!("Failed to bind to {address}: {native_error}"));
        }
        other => panic!("Expected a bind failure, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_port_discovery_failed_error() {
    // Try to find port in a very small range that's likely occupied
    let result = Box::pin(MockServer::new()
        .auto_port()
        .port_range(1, 10) // System ports, likely all in use
        .start())
    .await;

    if let Err(err) = result {
        let err_msg = format!("{err:?}");
        // Should include helpful tip
        assert!(
            err_msg.contains("Port discovery failed") || err_msg.contains("port"),
            "Error message: {err_msg}"
        );
    }
}

#[tokio::test]
async fn test_stub_not_found_error() {
    let err = Error::stub_not_found(
        "GET",
        "/api/missing",
        &["GET /api/users".to_string(), "POST /api/users".to_string()],
    );

    let err_msg = format!("{err}");
    assert!(err_msg.contains("GET"));
    assert!(err_msg.contains("/api/missing"));
    assert!(err_msg.contains("GET /api/users"));
}

#[tokio::test]
async fn test_admin_api_error() {
    let err = Error::admin_api_error("create_mock", "Invalid JSON", "/api/mocks");

    let err_msg = format!("{err}");
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
        let msg = format!("{err}");
        // All error messages should be non-empty and reasonably long
        assert!(msg.len() > 20, "Error message too short: {msg}");
        // Should contain actionable information (look for keywords)
        let lowercase_msg = msg.to_lowercase();
        let has_actionable_info = lowercase_msg.contains("tip:")
            || lowercase_msg.contains("check")
            || lowercase_msg.contains("try")
            || lowercase_msg.contains("call")
            || lowercase_msg.contains("ensure");

        assert!(has_actionable_info, "Error message lacks actionable advice: {msg}");
    }
}
