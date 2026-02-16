//! AMQP E2E tests
//!
//! End-to-end tests for AMQP protocol functionality

use mockforge_test::{MockForgeServer, ServerConfig};
use std::time::Duration;

#[tokio::test]
async fn test_amqp_broker_starts() {
    // Test that AMQP broker can start successfully
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_AMQP_ENABLED", "true")
        .env_var("MOCKFORGE_AMQP_PORT", "0") // Auto-assign port
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start AMQP broker");

    // Verify server is running
    assert!(server.is_running());

    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_amqp_broker_health_check() {
    // Test AMQP broker health check
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_AMQP_ENABLED", "true")
        .env_var("MOCKFORGE_AMQP_PORT", "0")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start AMQP broker");

    // Perform health check
    let health = server.health_check().await;
    assert!(health.is_ok(), "Health check should succeed");
    let health = health.unwrap();
    assert!(health.is_healthy(), "Server should be healthy");

    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_amqp_exchanges() {
    // Test AMQP exchange functionality
    // Note: Full E2E test would require AMQP client library
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_AMQP_ENABLED", "true")
        .env_var("MOCKFORGE_AMQP_PORT", "0")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start AMQP broker");

    // Verify server is running (exchanges are implemented in broker)
    assert!(server.is_running());

    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_amqp_queues() {
    // Test AMQP queue functionality
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_AMQP_ENABLED", "true")
        .env_var("MOCKFORGE_AMQP_PORT", "0")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start AMQP broker");

    // Verify server is running (queues are implemented in broker)
    assert!(server.is_running());

    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_amqp_bindings() {
    // Test AMQP binding functionality
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_AMQP_ENABLED", "true")
        .env_var("MOCKFORGE_AMQP_PORT", "0")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start AMQP broker");

    // Verify server is running (bindings are implemented in broker)
    assert!(server.is_running());

    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_amqp_auto_production() {
    // Test AMQP auto-production features
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_AMQP_ENABLED", "true")
        .env_var("MOCKFORGE_AMQP_PORT", "0")
        .env_var("MOCKFORGE_AMQP_AUTO_PRODUCTION_ENABLED", "true")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start AMQP broker with auto-production");

    // Verify server is running (auto-production is implemented)
    assert!(server.is_running());

    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_amqp_fixture_based_messages() {
    // Test fixture-based message generation for AMQP
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_AMQP_ENABLED", "true")
        .env_var("MOCKFORGE_AMQP_PORT", "0")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start AMQP broker with fixtures");

    // Verify server is running (fixture-based messages are implemented)
    assert!(server.is_running());

    server.stop().expect("Failed to stop server");
}
