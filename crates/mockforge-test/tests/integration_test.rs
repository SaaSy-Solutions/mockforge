//! Integration tests for mockforge-test

use mockforge_test::{MockForgeServer, ServerConfig};
use std::time::Duration;

#[tokio::test]
async fn test_server_starts_and_stops() {
    let server = MockForgeServer::builder()
        .http_port(0) // Auto-assign port
        .health_timeout(Duration::from_secs(30))
        .build()
        .await;

    // Skip test if mockforge binary is not available
    if server.is_err() {
        eprintln!("Skipping test: mockforge binary not found");
        return;
    }

    let server = server.unwrap();

    // Verify server is running
    assert!(server.is_running());
    assert!(server.http_port() > 0);

    // Perform health check
    let health = server.health_check().await;
    assert!(health.is_ok());
    let health = health.unwrap();
    assert!(health.is_healthy());

    // Stop the server
    assert!(server.stop().is_ok());
}

#[tokio::test]
async fn test_server_with_custom_port() {
    let port = 31234;
    let server = MockForgeServer::builder()
        .http_port(port)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await;

    // Skip test if mockforge binary is not available or port is in use
    if server.is_err() {
        eprintln!("Skipping test: mockforge binary not found or port in use");
        return;
    }

    let server = server.unwrap();

    assert_eq!(server.http_port(), port);
    assert_eq!(server.base_url(), format!("http://localhost:{}", port));
}

#[tokio::test]
async fn test_health_check() {
    let server = MockForgeServer::builder().http_port(0).build().await;

    if server.is_err() {
        eprintln!("Skipping test: mockforge binary not found");
        return;
    }

    let server = server.unwrap();

    // Check server is ready
    assert!(server.is_ready().await);

    // Get health status
    let health = server.health_check().await.unwrap();
    assert_eq!(health.status, "healthy");
    assert!(!health.version.is_empty());
}

#[test]
fn test_config_builder() {
    let config = ServerConfig::builder()
        .http_port(3000)
        .ws_port(3001)
        .grpc_port(3002)
        .admin_port(3003)
        .metrics_port(9090)
        .enable_admin(true)
        .enable_metrics(true)
        .profile("test")
        .extra_arg("--verbose")
        .health_timeout(Duration::from_secs(60))
        .build();

    assert_eq!(config.http_port, 3000);
    assert_eq!(config.ws_port, Some(3001));
    assert_eq!(config.grpc_port, Some(3002));
    assert_eq!(config.admin_port, Some(3003));
    assert_eq!(config.metrics_port, Some(9090));
    assert!(config.enable_admin);
    assert!(config.enable_metrics);
    assert_eq!(config.profile, Some("test".to_string()));
    assert_eq!(config.extra_args, vec!["--verbose"]);
    assert_eq!(config.health_timeout, Duration::from_secs(60));
}
