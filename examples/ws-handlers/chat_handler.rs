//! Chat handler example with room support
//!
//! This example demonstrates a chat handler that supports rooms and broadcasting.

use async_trait::async_trait;
use mockforge_ws::{HandlerResult, MessagePattern, WsContext, WsHandler, WsMessage};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Chat message format
#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    #[serde(rename = "type")]
    msg_type: String,
    room: Option<String>,
    user: Option<String>,
    content: Option<String>,
}

/// A chat handler that supports rooms and user authentication
pub struct ChatHandler {
    default_room: String,
}

impl ChatHandler {
    pub fn new() -> Self {
        Self {
            default_room: "general".to_string(),
        }
    }

    pub fn with_default_room(room: &str) -> Self {
        Self {
            default_room: room.to_string(),
        }
    }
}

impl Default for ChatHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WsHandler for ChatHandler {
    async fn on_connect(&self, ctx: &mut WsContext) -> HandlerResult<()> {
        // Auto-join default room
        ctx.join_room(&self.default_room).await?;

        // Send welcome message
        let welcome = json!({
            "type": "system",
            "content": format!("Welcome to the chat! You've joined room: {}", self.default_room),
            "room": self.default_room
        });
        ctx.send_json(&welcome).await?;

        Ok(())
    }

    async fn on_message(&self, ctx: &mut WsContext, msg: WsMessage) -> HandlerResult<()> {
        if let WsMessage::Text(text) = msg {
            // Parse the message as JSON
            if let Ok(chat_msg) = serde_json::from_str::<ChatMessage>(&text) {
                match chat_msg.msg_type.as_str() {
                    "join" => {
                        // Join a room
                        if let Some(room) = chat_msg.room {
                            ctx.join_room(&room).await?;
                            let response = json!({
                                "type": "system",
                                "content": format!("You joined room: {}", room),
                                "room": room
                            });
                            ctx.send_json(&response).await?;
                        }
                    }
                    "leave" => {
                        // Leave a room
                        if let Some(room) = chat_msg.room {
                            ctx.leave_room(&room).await?;
                            let response = json!({
                                "type": "system",
                                "content": format!("You left room: {}", room)
                            });
                            ctx.send_json(&response).await?;
                        }
                    }
                    "message" => {
                        // Broadcast message to room
                        if let Some(room) = &chat_msg.room {
                            if let Some(content) = &chat_msg.content {
                                let broadcast_msg = json!({
                                    "type": "message",
                                    "room": room,
                                    "user": chat_msg.user.unwrap_or_else(|| "anonymous".to_string()),
                                    "content": content
                                });
                                ctx.broadcast_to_room(room, &broadcast_msg.to_string()).await?;
                            }
                        }
                    }
                    "rooms" => {
                        // List current rooms
                        let rooms = ctx.get_rooms().await;
                        let response = json!({
                            "type": "system",
                            "content": format!("You are in {} room(s)", rooms.len()),
                            "rooms": rooms
                        });
                        ctx.send_json(&response).await?;
                    }
                    _ => {
                        // Unknown message type
                        let response = json!({
                            "type": "error",
                            "content": "Unknown message type"
                        });
                        ctx.send_json(&response).await?;
                    }
                }
            } else {
                // Handle simple text commands
                if text.starts_with("/join ") {
                    let room = text.trim_start_matches("/join ").trim();
                    ctx.join_room(room).await?;
                    ctx.send_text(&format!("Joined room: {}", room)).await?;
                } else if text.starts_with("/leave ") {
                    let room = text.trim_start_matches("/leave ").trim();
                    ctx.leave_room(room).await?;
                    ctx.send_text(&format!("Left room: {}", room)).await?;
                } else if text == "/rooms" {
                    let rooms = ctx.get_rooms().await;
                    ctx.send_text(&format!("Current rooms: {:?}", rooms)).await?;
                } else {
                    // Echo back if not a command
                    ctx.send_text(&format!("Use JSON messages or commands like /join <room>, /leave <room>, /rooms")).await?;
                }
            }
        }
        Ok(())
    }

    async fn on_disconnect(&self, ctx: &mut WsContext) -> HandlerResult<()> {
        println!("Client {} disconnected from chat", ctx.connection_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_chat_handler_join() {
        let handler = ChatHandler::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let room_manager = mockforge_ws::RoomManager::new();
        let mut ctx = WsContext::new("test-conn".to_string(), "/ws".to_string(), room_manager, tx);

        // Test on_connect
        handler.on_connect(&mut ctx).await.unwrap();
        let _welcome_msg = rx.recv().await.unwrap();

        // Test joining a room
        let join_msg = json!({
            "type": "join",
            "room": "test-room"
        });
        let ws_msg = WsMessage::Text(join_msg.to_string());
        handler.on_message(&mut ctx, ws_msg).await.unwrap();

        let response = rx.recv().await.unwrap();
        // Should receive confirmation message
        assert!(matches!(response, mockforge_ws::handlers::Message::Text(_)));
    }

    #[tokio::test]
    async fn test_chat_handler_simple_commands() {
        let handler = ChatHandler::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let room_manager = mockforge_ws::RoomManager::new();
        let mut ctx = WsContext::new("test-conn".to_string(), "/ws".to_string(), room_manager, tx);

        handler.on_connect(&mut ctx).await.unwrap();
        let _welcome = rx.recv().await.unwrap();

        // Test /join command
        let join_cmd = WsMessage::Text("/join lobby".to_string());
        handler.on_message(&mut ctx, join_cmd).await.unwrap();

        let response = rx.recv().await.unwrap();
        if let mockforge_ws::handlers::Message::Text(text) = response {
            assert!(text.to_string().contains("lobby"));
        }
    }
}
