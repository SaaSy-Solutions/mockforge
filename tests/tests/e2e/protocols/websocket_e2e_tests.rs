//! WebSocket E2E tests
//!
//! End-to-end tests for WebSocket protocol functionality

use mockforge_test::{MockForgeServer, ServerConfig};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;

#[tokio::test]
async fn test_websocket_connection() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .ws_port(0)
        .admin_port(0)
        .enable_admin(true)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    let ws_port = server.ws_port().expect("WebSocket port not assigned");
    let ws_url = server.ws_url().expect("WebSocket URL not available");

    // Connect to WebSocket
    let (mut ws_stream, _) = connect_async(&ws_url)
        .await
        .expect("Failed to connect to WebSocket");

    // Send a ping message
    ws_stream
        .send(Message::Text(r#"{"type": "ping"}"#.to_string()))
        .await
        .expect("Failed to send message");

    // Receive response (with timeout)
    let response = tokio::time::timeout(Duration::from_secs(5), ws_stream.next())
        .await
        .expect("Timeout waiting for response")
        .expect("Stream closed")
        .expect("Failed to receive message");

    match response {
        Message::Text(text) => {
            assert!(text.contains("pong") || text.contains("ping"), "Expected pong response, got: {}", text);
        }
        _ => panic!("Expected text message, got: {:?}", response),
    }

    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_websocket_multiple_connections() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .ws_port(0)
        .admin_port(0)
        .enable_admin(true)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    let ws_url = server.ws_url().expect("WebSocket URL not available");

    // Connect multiple clients
    let mut clients = Vec::new();
    for _ in 0..3 {
        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .expect("Failed to connect to WebSocket");
        clients.push(ws_stream);
    }

    // Verify all clients are connected
    assert_eq!(clients.len(), 3);

    // Send messages from each client
    for (i, mut client) in clients.into_iter().enumerate() {
        client
            .send(Message::Text(format!(r#"{{"type": "test", "id": {}}}"#, i)))
            .await
            .expect("Failed to send message");
    }

    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_websocket_binary_message() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .ws_port(0)
        .admin_port(0)
        .enable_admin(true)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    let ws_url = server.ws_url().expect("WebSocket URL not available");

    // Connect to WebSocket
    let (mut ws_stream, _) = connect_async(&ws_url)
        .await
        .expect("Failed to connect to WebSocket");

    // Send binary message
    let binary_data = b"test binary data";
    ws_stream
        .send(Message::Binary(binary_data.to_vec()))
        .await
        .expect("Failed to send binary message");

    // Connection should remain open
    assert!(!ws_stream.is_closed());

    server.stop().expect("Failed to stop server");
}
