//! gRPC E2E tests
//!
//! End-to-end tests for gRPC protocol functionality
//!
//! Note: These tests require gRPC server to be running and may need
//! actual proto definitions for full testing.

use mockforge_test::{MockForgeServer, ServerConfig};
use std::time::Duration;

#[tokio::test]
async fn test_grpc_server_starts() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .grpc_port(0)
        .admin_port(0)
        .enable_admin(true)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    // Verify gRPC port is assigned
    let grpc_port = server.grpc_port();
    assert!(grpc_port.is_some(), "gRPC port should be assigned");

    // Server should be running
    assert!(server.is_running());

    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_grpc_health_check() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .grpc_port(0)
        .admin_port(0)
        .enable_admin(true)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    // Perform health check via HTTP endpoint
    let health = server.health_check().await;
    assert!(health.is_ok(), "Health check should succeed");
    let health = health.unwrap();
    assert!(health.is_healthy(), "Server should be healthy");

    server.stop().expect("Failed to stop server");
}

// TODO: Add actual gRPC client tests once proto definitions are available
// These would test:
// - Unary RPC calls
// - Server streaming
// - Client streaming
// - Bidirectional streaming
// - Error handling
// - Reflection API
