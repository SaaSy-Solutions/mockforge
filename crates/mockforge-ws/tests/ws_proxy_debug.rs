//! Debug tests for WebSocket proxy functionality.
//!
//! These tests verify WebSocket proxy behavior including connection handling,
//! message forwarding, and debugging capabilities.

use futures_util::{SinkExt, StreamExt};
use mockforge_core::ws_proxy::{WsProxyConfig, WsProxyHandler};
use tokio_tungstenite::tungstenite::protocol::Message;

#[tokio::test]
async fn debug_ws_proxy_basic() {
    // Create a simple proxy config
    let mut config = WsProxyConfig::new("ws://127.0.0.1:9999".to_string());
    config.enabled = true;
    config.passthrough_by_default = true; // Proxy all connections

    let proxy_handler = WsProxyHandler::new(config);

    // Start proxy server
    let proxy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();
    let proxy_server = tokio::spawn(async move {
        let app = mockforge_ws::router_with_proxy(proxy_handler);
        axum::serve(proxy_listener, app).await.unwrap()
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test connection to /ws (should be proxied but will fail to connect to upstream)
    let url = format!("ws://{}/ws", proxy_addr);
    println!("Connecting to: {}", url);

    match tokio_tungstenite::connect_async(url).await {
        Ok((mut ws_stream, _)) => {
            println!("Connected successfully!");

            // Send a message
            if let Err(e) = ws_stream.send(Message::Text("test".into())).await {
                println!("Failed to send message: {}", e);
            } else {
                println!("Message sent successfully");
            }

            // Try to receive a message with timeout
            match tokio::time::timeout(tokio::time::Duration::from_secs(5), ws_stream.next()).await
            {
                Ok(Some(Ok(msg))) => {
                    println!("Received message: {:?}", msg);
                }
                Ok(Some(Err(e))) => {
                    println!("Received error: {}", e);
                }
                Ok(None) => {
                    println!("Connection closed by server");
                }
                Err(_) => {
                    println!("Timeout waiting for message");
                }
            }
        }
        Err(e) => {
            println!("Failed to connect: {}", e);
        }
    }

    // Clean up
    drop(proxy_server);
}
