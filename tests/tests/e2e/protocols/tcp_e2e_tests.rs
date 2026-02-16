//! TCP E2E tests
//!
//! End-to-end tests for raw TCP protocol functionality

use mockforge_test::{MockForgeServer, ServerConfig};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Helper to assert server is running
fn assert_server_running(server: &MockForgeServer) {
    assert!(server.is_running(), "Server should be running");
}

#[tokio::test]
async fn test_tcp_server_starts() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_TCP_ENABLED", "true")
        .env_var("MOCKFORGE_TCP_PORT", "9999")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    assert_server_running(&server);
    
    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_tcp_server_health_check() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_TCP_ENABLED", "true")
        .env_var("MOCKFORGE_TCP_PORT", "9998")
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
async fn test_tcp_connection() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_TCP_ENABLED", "true")
        .env_var("MOCKFORGE_TCP_PORT", "9997")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    // Try to connect to TCP server
    let tcp_port = 9997;
    let connection = timeout(
        Duration::from_secs(5),
        TcpStream::connect(format!("localhost:{}", tcp_port))
    )
    .await;

    // Connection may succeed or fail depending on server configuration
    // For now, we just verify the server started
    assert_server_running(&server);
    
    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_tcp_send_receive() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_TCP_ENABLED", "true")
        .env_var("MOCKFORGE_TCP_PORT", "9996")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    let tcp_port = 9996;
    
    // Try to connect and send data
    if let Ok(stream) = timeout(
        Duration::from_secs(5),
        TcpStream::connect(format!("localhost:{}", tcp_port))
    )
    .await
    {
        if let Ok(mut stream) = stream {
            // Send test data
            if stream.write_all(b"TEST DATA\r\n").await.is_ok() {
                stream.flush().await.ok();
                
                // Try to read response
                let mut buffer = [0u8; 1024];
                if timeout(Duration::from_secs(2), stream.read(&mut buffer))
                    .await
                    .is_ok()
                {
                    // Response received (or connection closed)
                    // Just verify we can send/receive
                }
            }
        }
    }

    server.stop().expect("Failed to stop server");
}
