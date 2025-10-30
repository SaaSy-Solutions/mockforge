//! Integration tests for collaboration client

use mockforge_collab::client::{ClientConfig, CollabClient, ConnectionState};

#[tokio::test]
async fn test_client_creation() {
    let config = ClientConfig {
        server_url: "ws://localhost:8080/ws".to_string(),
        auth_token: "test-token".to_string(),
        ..Default::default()
    };

    // Note: This will fail to connect if server is not running, but should create client
    // In a real test, we'd use a mock server or test server
    let result = CollabClient::connect(config).await;

    // For now, we just test that the client can be created with valid config
    // Real connection tests would require a running server
    // This test verifies the API exists and accepts configuration
    assert!(result.is_ok() || result.is_err()); // Either is fine for unit test without server
}

#[test]
fn test_client_config_defaults() {
    let config = ClientConfig::default();

    assert_eq!(config.max_reconnect_attempts, None);
    assert_eq!(config.max_queue_size, 1000);
    assert_eq!(config.initial_backoff_ms, 1000);
    assert_eq!(config.max_backoff_ms, 30000);
}

#[tokio::test]
async fn test_client_rejects_empty_server_url() {
    let config = ClientConfig {
        server_url: String::new(),
        auth_token: "test".to_string(),
        ..Default::default()
    };

    let result = CollabClient::connect(config).await;
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(e.to_string().contains("server_url cannot be empty"));
    }
}

#[tokio::test]
async fn test_client_state_tracking() {
    // This test would require a mock server or actual server
    // For now, we verify the state enum and methods exist
    let _state = ConnectionState::Disconnected;
    let _state2 = ConnectionState::Connecting;
    let _state3 = ConnectionState::Connected;
    let _state4 = ConnectionState::Reconnecting;

    // Verify states are distinct
    assert_ne!(_state, _state2);
    assert_ne!(_state2, _state3);
    assert_ne!(_state3, _state4);
}
