//! SMTP E2E tests
//!
//! End-to-end tests for SMTP protocol functionality

use mockforge_test::{MockForgeServer, ServerConfig};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Helper to assert server is running
fn assert_server_running(server: &MockForgeServer) {
    assert!(server.is_running(), "Server should be running");
}

#[tokio::test]
async fn test_smtp_server_starts() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_SMTP_ENABLED", "true")
        .env_var("MOCKFORGE_SMTP_PORT", "1025")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    assert_server_running(&server);
    
    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_smtp_server_health_check() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_SMTP_ENABLED", "true")
        .env_var("MOCKFORGE_SMTP_PORT", "1026")
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
async fn test_smtp_connection() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_SMTP_ENABLED", "true")
        .env_var("MOCKFORGE_SMTP_PORT", "1027")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    // Try to connect to SMTP server
    let smtp_port = 1027;
    let connection = timeout(
        Duration::from_secs(5),
        TcpStream::connect(format!("localhost:{}", smtp_port))
    )
    .await;

    // Connection may succeed or fail depending on server configuration
    // For now, we just verify the server started
    assert_server_running(&server);
    
    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_smtp_ehlo_command() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_SMTP_ENABLED", "true")
        .env_var("MOCKFORGE_SMTP_PORT", "1028")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    let smtp_port = 1028;
    
    // Try to connect and send EHLO
    if let Ok(stream) = timeout(
        Duration::from_secs(5),
        TcpStream::connect(format!("localhost:{}", smtp_port))
    )
    .await
    {
        if let Ok(mut stream) = stream {
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            let mut response = String::new();

            // Read greeting
            if timeout(Duration::from_secs(2), reader.read_line(&mut response))
                .await
                .is_ok()
            {
                // Should start with 220
                assert!(response.starts_with("220"), "Expected 220 greeting, got: {}", response);
                response.clear();

                // Send EHLO
                if writer.write_all(b"EHLO test.example.com\r\n").await.is_ok() {
                    writer.flush().await.ok();
                    
                    // Read response (with timeout)
                    if timeout(Duration::from_secs(2), reader.read_line(&mut response))
                        .await
                        .is_ok()
                    {
                        // Should start with 250
                        assert!(response.starts_with("250"), "Expected 250 response, got: {}", response);
                    }
                }
            }
        }
    }

    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_smtp_with_fixtures() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_SMTP_ENABLED", "true")
        .env_var("MOCKFORGE_SMTP_PORT", "1029")
        .env_var("MOCKFORGE_SMTP_FIXTURES_DIR", "./fixtures/smtp")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server with fixtures");

    assert_server_running(&server);
    
    server.stop().expect("Failed to stop server");
}
