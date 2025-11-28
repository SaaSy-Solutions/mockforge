//! Echo handler example
//!
//! This example demonstrates a simple echo handler that responds to all messages.

use async_trait::async_trait;
use mockforge_ws::{HandlerResult, WsContext, WsHandler, WsMessage};

/// A simple echo handler that echoes all text messages back
pub struct EchoHandler;

#[async_trait]
impl WsHandler for EchoHandler {
    async fn on_connect(&self, ctx: &mut WsContext) -> HandlerResult<()> {
        ctx.send_text("Welcome! This is the echo handler. Send me a message and I'll echo it back.").await?;
        Ok(())
    }

    async fn on_message(&self, ctx: &mut WsContext, msg: WsMessage) -> HandlerResult<()> {
        if let WsMessage::Text(text) = msg {
            // Echo the message back with a prefix
            ctx.send_text(&format!("echo: {}", text)).await?;
        }
        Ok(())
    }

    async fn on_disconnect(&self, ctx: &mut WsContext) -> HandlerResult<()> {
        println!("Client {} disconnected from echo handler", ctx.connection_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_echo_handler() {
        let handler = EchoHandler;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let room_manager = mockforge_ws::RoomManager::new();
        let mut ctx = WsContext::new("test-conn".to_string(), "/ws".to_string(), room_manager, tx);

        // Test on_connect
        handler.on_connect(&mut ctx).await.unwrap();
        let msg = rx.recv().await.unwrap();
        assert!(matches!(msg, mockforge_ws::handlers::Message::Text(_)));

        // Test on_message
        let test_msg = WsMessage::Text("hello".to_string());
        handler.on_message(&mut ctx, test_msg).await.unwrap();
        let response = rx.recv().await.unwrap();
        if let mockforge_ws::handlers::Message::Text(text) = response {
            assert_eq!(text.to_string(), "echo: hello");
        } else {
            panic!("Expected text message");
        }
    }
}
