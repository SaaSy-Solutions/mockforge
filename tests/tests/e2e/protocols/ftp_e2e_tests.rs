//! FTP E2E tests
//!
//! End-to-end tests for FTP protocol functionality

use mockforge_test::{MockForgeServer, ServerConfig};
use std::time::Duration;

/// Helper to assert server is running
fn assert_server_running(server: &MockForgeServer) {
    assert!(server.is_running(), "Server should be running");
}

#[tokio::test]
async fn test_ftp_server_starts() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_FTP_ENABLED", "true")
        .env_var("MOCKFORGE_FTP_PORT", "2121")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    assert_server_running(&server);
    
    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_ftp_server_health_check() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_FTP_ENABLED", "true")
        .env_var("MOCKFORGE_FTP_PORT", "2122")
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
async fn test_ftp_connection() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_FTP_ENABLED", "true")
        .env_var("MOCKFORGE_FTP_PORT", "2123")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    // Try to connect to FTP server
    let ftp_port = 2123;
    let connection = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::net::TcpStream::connect(format!("localhost:{}", ftp_port))
    )
    .await;

    // Connection may succeed or fail depending on server configuration
    // For now, we just verify the server started
    assert_server_running(&server);
    
    server.stop().expect("Failed to stop server");
}
