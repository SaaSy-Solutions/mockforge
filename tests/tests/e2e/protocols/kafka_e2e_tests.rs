//! Kafka E2E tests
//!
//! End-to-end tests for Kafka protocol functionality

use mockforge_test::{MockForgeServer, ServerConfig};
use std::time::Duration;

/// Helper to assert server is running
fn assert_server_running(server: &MockForgeServer) {
    assert!(server.is_running(), "Server should be running");
}

#[tokio::test]
async fn test_kafka_broker_starts() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_KAFKA_ENABLED", "true")
        .env_var("MOCKFORGE_KAFKA_PORT", "9092")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    assert_server_running(&server);
    
    // Verify Kafka port is configured (if exposed via admin API)
    // Note: Kafka port may not be directly accessible via MockForgeServer
    // but we can verify the server started successfully
    
    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_kafka_broker_health_check() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_KAFKA_ENABLED", "true")
        .env_var("MOCKFORGE_KAFKA_PORT", "9093")
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

#[tokio::test]
async fn test_kafka_broker_with_fixtures() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_KAFKA_ENABLED", "true")
        .env_var("MOCKFORGE_KAFKA_PORT", "9094")
        .env_var("MOCKFORGE_KAFKA_FIXTURES_DIR", "./fixtures/kafka")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server with fixtures");

    assert_server_running(&server);
    
    // Note: Actual Kafka client testing would require rdkafka or similar
    // For now, we verify the server starts with Kafka enabled
    
    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_kafka_auto_production() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_KAFKA_ENABLED", "true")
        .env_var("MOCKFORGE_KAFKA_PORT", "9095")
        .env_var("MOCKFORGE_KAFKA_AUTO_PRODUCE_ENABLED", "true")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server with auto-production");

    assert_server_running(&server);
    
    // Auto-production would generate messages automatically
    // Actual verification would require Kafka client
    
    server.stop().expect("Failed to stop server");
}
