//! End-to-end tests for WebSocket message templating.
//!
//! These tests verify that template tokens in WebSocket messages are correctly
//! expanded with dynamic values during replay and response generation.

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::protocol::Message;

#[tokio::test]
async fn ws_replay_expands_tokens() {
    std::env::set_var("MOCKFORGE_WS_REPLAY_FILE", "examples/ws-demo.jsonl");
    std::env::set_var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND", "true");

    // start WS server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server =
        tokio::spawn(async move { axum::serve(listener, mockforge_ws::router()).await.unwrap() });

    // Connect
    let url = format!("ws://{}/ws", addr);
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(url).await.unwrap();
    ws_stream.send(Message::Text("CLIENT_READY".into())).await.unwrap();
    if let Some(Ok(Message::Text(t))) = ws_stream.next().await {
        assert!(t.contains("HELLO"));
        assert!(!t.contains("{{uuid}}"));
    }
    drop(server);
}
