//! WebSocket E2E tests
//!
//! End-to-end tests for WebSocket protocol functionality

use futures_util::{SinkExt, StreamExt};
use mockforge_test::MockForgeServer;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

/// Open a WebSocket connection with retries.
///
/// `MockForgeServer::build()` only waits for HTTP `/health` to go ready.
/// The WS task binds a moment later, so the first `connect_async` after
/// build() can occasionally race the bind on slower CI runners (macOS in
/// particular). Retry a handful of times before giving up.
async fn connect_with_retries(
    ws_url: &str,
) -> WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>> {
    let overall_deadline = std::time::Instant::now() + Duration::from_secs(10);
    let mut last_err = String::new();
    while std::time::Instant::now() < overall_deadline {
        match connect_async(ws_url).await {
            Ok((stream, _)) => return stream,
            Err(e) => {
                last_err = format!("{e:?}");
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
    panic!("Failed to connect to WebSocket {ws_url} after retries: {last_err}");
}

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

    let _ws_port = server.ws_port().expect("WebSocket port not assigned");
    let ws_url = server.ws_url().expect("WebSocket URL not available");

    // Connect to WebSocket (retrying briefly so we don't race the bind)
    let mut ws_stream = connect_with_retries(&ws_url).await;

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
            assert!(
                text.contains("pong") || text.contains("ping"),
                "Expected pong response, got: {}",
                text
            );
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

    // Connect multiple clients (retrying the first one to guard against
    // the HTTP-ready-before-WS-bound race; once the first is up the rest
    // should connect cleanly).
    let mut clients = Vec::new();
    clients.push(connect_with_retries(&ws_url).await);
    for _ in 1..3 {
        let (ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect to WebSocket");
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

    // Connect to WebSocket (retrying briefly so we don't race the bind)
    let mut ws_stream = connect_with_retries(&ws_url).await;

    // Send binary message
    let binary_data = b"test binary data";
    ws_stream
        .send(Message::Binary(binary_data.to_vec()))
        .await
        .expect("Failed to send binary message");

    // If we can still send a second message without error, the connection is open.
    ws_stream
        .send(Message::Text("after-binary".to_string()))
        .await
        .expect("Connection should remain open after binary send");

    server.stop().expect("Failed to stop server");
}
