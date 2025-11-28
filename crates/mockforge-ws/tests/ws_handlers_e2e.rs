//! End-to-end tests for WebSocket handlers

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use mockforge_ws::{
    router_with_handlers, HandlerRegistry, HandlerResult, WsContext, WsHandler, WsMessage,
};
use std::sync::Arc;
use tokio_tungstenite::tungstenite::protocol::Message;

/// Simple echo handler for testing
struct TestEchoHandler;

#[async_trait]
impl WsHandler for TestEchoHandler {
    async fn on_connect(&self, ctx: &mut WsContext) -> HandlerResult<()> {
        ctx.send_text("Connected to echo handler").await?;
        Ok(())
    }

    async fn on_message(&self, ctx: &mut WsContext, msg: WsMessage) -> HandlerResult<()> {
        if let WsMessage::Text(text) = msg {
            ctx.send_text(&format!("ECHO: {}", text)).await?;
        }
        Ok(())
    }

    fn handles_path(&self, path: &str) -> bool {
        // Only handle exact /ws path, not /ws/chat
        path == "/ws"
    }
}

/// Chat handler for testing room functionality
struct TestChatHandler;

#[async_trait]
impl WsHandler for TestChatHandler {
    async fn on_connect(&self, ctx: &mut WsContext) -> HandlerResult<()> {
        ctx.join_room("general").await?;
        ctx.send_text("Welcome to chat!").await?;
        Ok(())
    }

    async fn on_message(&self, ctx: &mut WsContext, msg: WsMessage) -> HandlerResult<()> {
        if let WsMessage::Text(text) = msg {
            if text.starts_with("/join ") {
                let room = text.trim_start_matches("/join ").trim();
                ctx.join_room(room).await?;
                ctx.send_text(&format!("Joined: {}", room)).await?;
            } else if text.starts_with("/leave ") {
                let room = text.trim_start_matches("/leave ").trim();
                ctx.leave_room(room).await?;
                ctx.send_text(&format!("Left: {}", room)).await?;
            } else if text == "/rooms" {
                let rooms = ctx.get_rooms().await;
                ctx.send_text(&format!("Rooms: {:?}", rooms)).await?;
            } else {
                ctx.send_text(&format!("Message: {}", text)).await?;
            }
        }
        Ok(())
    }

    async fn on_disconnect(&self, ctx: &mut WsContext) -> HandlerResult<()> {
        println!("Client {} disconnected", ctx.connection_id);
        Ok(())
    }

    fn handles_path(&self, path: &str) -> bool {
        path.starts_with("/ws/chat")
    }
}

#[tokio::test]
async fn test_echo_handler_e2e() {
    // Create registry with echo handler
    let mut registry = HandlerRegistry::new();
    registry.register(TestEchoHandler);
    let registry = Arc::new(registry);

    // Start server
    let app = router_with_handlers(registry);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Connect to server
    let url = format!("ws://{}/ws", addr);
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(url).await.unwrap();

    // Receive welcome message
    if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
        assert!(text.contains("Connected to echo handler"));
    } else {
        panic!("Expected welcome message");
    }

    // Send a message
    ws_stream.send(Message::Text("Hello".into())).await.unwrap();

    // Receive echo
    if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
        assert_eq!(text, "ECHO: Hello");
    } else {
        panic!("Expected echo response");
    }

    // Send another message
    ws_stream.send(Message::Text("World".into())).await.unwrap();

    // Receive second echo
    if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
        assert_eq!(text, "ECHO: World");
    } else {
        panic!("Expected second echo response");
    }

    // Clean up
    drop(ws_stream);
    server.abort();
}

#[tokio::test]
async fn test_chat_handler_rooms() {
    // Create registry with chat handler
    let mut registry = HandlerRegistry::new();
    registry.register(TestChatHandler);
    let registry = Arc::new(registry);

    // Start server
    let app = router_with_handlers(registry);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Connect to chat endpoint
    let url = format!("ws://{}/ws/chat", addr);
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(url).await.unwrap();

    // Receive welcome message
    if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
        assert!(text.contains("Welcome to chat"));
    } else {
        panic!("Expected welcome message");
    }

    // Test /join command
    ws_stream.send(Message::Text("/join lobby".into())).await.unwrap();
    if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
        assert!(text.contains("Joined: lobby"));
    }

    // Test /rooms command
    ws_stream.send(Message::Text("/rooms".into())).await.unwrap();
    if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
        assert!(text.contains("general"));
        assert!(text.contains("lobby"));
    }

    // Test /leave command
    ws_stream.send(Message::Text("/leave lobby".into())).await.unwrap();
    if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
        assert!(text.contains("Left: lobby"));
    }

    // Clean up
    drop(ws_stream);
    server.abort();
}

#[tokio::test]
async fn test_handler_path_routing() {
    // Create registry with both handlers
    let mut registry = HandlerRegistry::new();
    registry.register(TestEchoHandler);
    registry.register(TestChatHandler);
    let registry = Arc::new(registry);

    // Start server
    let app = router_with_handlers(registry);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test echo endpoint (/ws)
    let echo_url = format!("ws://{}/ws", addr);
    let (mut echo_stream, _) = tokio_tungstenite::connect_async(echo_url).await.unwrap();

    if let Some(Ok(Message::Text(text))) = echo_stream.next().await {
        assert!(text.contains("Connected to echo handler"));
    }
    drop(echo_stream);

    // Test chat endpoint (/ws/chat)
    let chat_url = format!("ws://{}/ws/chat", addr);
    let (mut chat_stream, _) = tokio_tungstenite::connect_async(chat_url).await.unwrap();

    if let Some(Ok(Message::Text(text))) = chat_stream.next().await {
        assert!(text.contains("Welcome to chat"));
    }
    drop(chat_stream);

    server.abort();
}

#[tokio::test]
async fn test_multiple_clients_same_room() {
    // Create registry with chat handler
    let mut registry = HandlerRegistry::new();
    registry.register(TestChatHandler);
    let registry = Arc::new(registry);

    // Start server
    let app = router_with_handlers(registry);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Connect two clients
    let url1 = format!("ws://{}/ws/chat", addr);
    let (mut client1, _) = tokio_tungstenite::connect_async(url1).await.unwrap();

    let url2 = format!("ws://{}/ws/chat", addr);
    let (mut client2, _) = tokio_tungstenite::connect_async(url2).await.unwrap();

    // Consume welcome messages
    let _ = client1.next().await;
    let _ = client2.next().await;

    // Both clients should be in "general" room by default
    client1.send(Message::Text("/rooms".into())).await.unwrap();
    if let Some(Ok(Message::Text(text))) = client1.next().await {
        assert!(text.contains("general"));
    }

    client2.send(Message::Text("/rooms".into())).await.unwrap();
    if let Some(Ok(Message::Text(text))) = client2.next().await {
        assert!(text.contains("general"));
    }

    // Clean up
    drop(client1);
    drop(client2);
    server.abort();
}
