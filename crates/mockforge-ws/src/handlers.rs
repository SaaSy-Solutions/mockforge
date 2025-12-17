//! # Programmable WebSocket Handlers
//!
//! This module provides a flexible handler API for scripting WebSocket event flows.
//! Unlike static replay, handlers allow you to write custom logic for responding to
//! WebSocket events, manage rooms, and route messages dynamically.
//!
//! ## Features
//!
//! - **Connection Lifecycle**: `on_connect` and `on_disconnect` hooks
//! - **Pattern Matching**: Route messages with regex or JSONPath patterns
//! - **Room Management**: Broadcast messages to groups of connections
//! - **Passthrough**: Selectively forward messages to upstream servers
//! - **Hot Reload**: Automatically reload handlers when code changes (via `MOCKFORGE_WS_HOTRELOAD`)
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use mockforge_ws::handlers::{WsHandler, WsContext, WsMessage, HandlerResult};
//! use async_trait::async_trait;
//!
//! struct EchoHandler;
//!
//! #[async_trait]
//! impl WsHandler for EchoHandler {
//!     async fn on_connect(&self, ctx: &mut WsContext) -> HandlerResult<()> {
//!         ctx.send_text("Welcome to the echo server!").await?;
//!         Ok(())
//!     }
//!
//!     async fn on_message(&self, ctx: &mut WsContext, msg: WsMessage) -> HandlerResult<()> {
//!         if let WsMessage::Text(text) = msg {
//!             ctx.send_text(&format!("echo: {}", text)).await?;
//!         }
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Message Pattern Matching
//!
//! ```rust,no_run
//! use mockforge_ws::handlers::{WsHandler, WsContext, WsMessage, HandlerResult, MessagePattern};
//! use async_trait::async_trait;
//!
//! struct ChatHandler;
//!
//! #[async_trait]
//! impl WsHandler for ChatHandler {
//!     async fn on_message(&self, ctx: &mut WsContext, msg: WsMessage) -> HandlerResult<()> {
//!         if let WsMessage::Text(text) = msg {
//!             // Use pattern matching to route messages
//!             if let Ok(pattern) = MessagePattern::regex(r"^/join (.+)$") {
//!                 if pattern.matches(&text) {
//!                     // Extract room name and join
//!                     if let Some(room) = text.strip_prefix("/join ") {
//!                         ctx.join_room(room).await?;
//!                         ctx.send_text(&format!("Joined room: {}", room)).await?;
//!                     }
//!                 }
//!             }
//!             // Handle JSON chat messages
//!             let jsonpath_pattern = MessagePattern::jsonpath("$.type");
//!             if jsonpath_pattern.matches(&text) {
//!                 ctx.broadcast_to_room("general", &text).await?;
//!             }
//!         }
//!         Ok(())
//!     }
//! }
//! ```

use async_trait::async_trait;
use axum::extract::ws::Message;
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Result type for handler operations
pub type HandlerResult<T> = Result<T, HandlerError>;

/// Error type for handler operations
#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    /// Failed to send WebSocket message
    #[error("Failed to send message: {0}")]
    SendError(String),

    /// JSON parsing/serialization error
    #[error("Failed to parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Pattern matching failure (e.g., route pattern)
    #[error("Pattern matching error: {0}")]
    PatternError(String),

    /// Room/group operation failure
    #[error("Room operation failed: {0}")]
    RoomError(String),

    /// WebSocket connection error
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Generic handler error
    #[error("Handler error: {0}")]
    Generic(String),
}

/// WebSocket message wrapper for different message types
#[derive(Debug, Clone)]
pub enum WsMessage {
    /// Text message (UTF-8 string)
    Text(String),
    /// Binary message (raw bytes)
    Binary(Vec<u8>),
    /// Ping frame (connection keepalive)
    Ping(Vec<u8>),
    /// Pong frame (response to ping)
    Pong(Vec<u8>),
    /// Close frame (connection termination)
    Close,
}

impl From<Message> for WsMessage {
    fn from(msg: Message) -> Self {
        match msg {
            Message::Text(text) => WsMessage::Text(text.to_string()),
            Message::Binary(data) => WsMessage::Binary(data.to_vec()),
            Message::Ping(data) => WsMessage::Ping(data.to_vec()),
            Message::Pong(data) => WsMessage::Pong(data.to_vec()),
            Message::Close(_) => WsMessage::Close,
        }
    }
}

impl From<WsMessage> for Message {
    fn from(msg: WsMessage) -> Self {
        match msg {
            WsMessage::Text(text) => Message::Text(text.into()),
            WsMessage::Binary(data) => Message::Binary(data.into()),
            WsMessage::Ping(data) => Message::Ping(data.into()),
            WsMessage::Pong(data) => Message::Pong(data.into()),
            WsMessage::Close => Message::Close(None),
        }
    }
}

/// Pattern for matching WebSocket messages
#[derive(Debug, Clone)]
pub enum MessagePattern {
    /// Match using regular expression
    Regex(Regex),
    /// Match using JSONPath query
    JsonPath(String),
    /// Match exact text
    Exact(String),
    /// Always matches
    Any,
}

impl MessagePattern {
    /// Create a regex pattern
    pub fn regex(pattern: &str) -> HandlerResult<Self> {
        Ok(MessagePattern::Regex(
            Regex::new(pattern).map_err(|e| HandlerError::PatternError(e.to_string()))?,
        ))
    }

    /// Create a JSONPath pattern
    pub fn jsonpath(query: &str) -> Self {
        MessagePattern::JsonPath(query.to_string())
    }

    /// Create an exact match pattern
    pub fn exact(text: &str) -> Self {
        MessagePattern::Exact(text.to_string())
    }

    /// Create a pattern that matches everything
    pub fn any() -> Self {
        MessagePattern::Any
    }

    /// Check if the pattern matches the message
    pub fn matches(&self, text: &str) -> bool {
        match self {
            MessagePattern::Regex(re) => re.is_match(text),
            MessagePattern::JsonPath(query) => {
                // Try to parse as JSON and check if path exists
                if let Ok(json) = serde_json::from_str::<Value>(text) {
                    // Use jsonpath crate's Selector
                    if let Ok(selector) = jsonpath::Selector::new(query) {
                        let results: Vec<_> = selector.find(&json).collect();
                        !results.is_empty()
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            MessagePattern::Exact(expected) => text == expected,
            MessagePattern::Any => true,
        }
    }

    /// Check if pattern matches and extract value using JSONPath
    pub fn extract(&self, text: &str, query: &str) -> Option<Value> {
        if let Ok(json) = serde_json::from_str::<Value>(text) {
            if let Ok(selector) = jsonpath::Selector::new(query) {
                let results: Vec<_> = selector.find(&json).collect();
                results.first().cloned().cloned()
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Connection ID type
pub type ConnectionId = String;

/// Room manager for broadcasting messages to groups of connections
#[derive(Clone)]
pub struct RoomManager {
    rooms: Arc<RwLock<HashMap<String, HashSet<ConnectionId>>>>,
    connections: Arc<RwLock<HashMap<ConnectionId, HashSet<String>>>>,
    broadcasters: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
}

impl RoomManager {
    /// Create a new room manager
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            broadcasters: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Join a room
    pub async fn join(&self, conn_id: &str, room: &str) -> HandlerResult<()> {
        let mut rooms = self.rooms.write().await;
        let mut connections = self.connections.write().await;

        rooms
            .entry(room.to_string())
            .or_insert_with(HashSet::new)
            .insert(conn_id.to_string());

        connections
            .entry(conn_id.to_string())
            .or_insert_with(HashSet::new)
            .insert(room.to_string());

        Ok(())
    }

    /// Leave a room
    pub async fn leave(&self, conn_id: &str, room: &str) -> HandlerResult<()> {
        let mut rooms = self.rooms.write().await;
        let mut connections = self.connections.write().await;

        if let Some(room_members) = rooms.get_mut(room) {
            room_members.remove(conn_id);
            if room_members.is_empty() {
                rooms.remove(room);
            }
        }

        if let Some(conn_rooms) = connections.get_mut(conn_id) {
            conn_rooms.remove(room);
            if conn_rooms.is_empty() {
                connections.remove(conn_id);
            }
        }

        Ok(())
    }

    /// Leave all rooms for a connection
    pub async fn leave_all(&self, conn_id: &str) -> HandlerResult<()> {
        let mut connections = self.connections.write().await;
        if let Some(conn_rooms) = connections.remove(conn_id) {
            let mut rooms = self.rooms.write().await;
            for room in conn_rooms {
                if let Some(room_members) = rooms.get_mut(&room) {
                    room_members.remove(conn_id);
                    if room_members.is_empty() {
                        rooms.remove(&room);
                    }
                }
            }
        }
        Ok(())
    }

    /// Get all connections in a room
    pub async fn get_room_members(&self, room: &str) -> Vec<ConnectionId> {
        let rooms = self.rooms.read().await;
        rooms
            .get(room)
            .map(|members| members.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all rooms for a connection
    pub async fn get_connection_rooms(&self, conn_id: &str) -> Vec<String> {
        let connections = self.connections.read().await;
        connections
            .get(conn_id)
            .map(|rooms| rooms.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get broadcast sender for a room (creates if doesn't exist)
    pub async fn get_broadcaster(&self, room: &str) -> broadcast::Sender<String> {
        let mut broadcasters = self.broadcasters.write().await;
        broadcasters
            .entry(room.to_string())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(1024);
                tx
            })
            .clone()
    }
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Context provided to handlers for each connection
pub struct WsContext {
    /// Unique connection ID
    pub connection_id: ConnectionId,
    /// WebSocket path
    pub path: String,
    /// Room manager for broadcasting
    room_manager: RoomManager,
    /// Sender for outgoing messages
    message_tx: tokio::sync::mpsc::UnboundedSender<Message>,
    /// Metadata storage
    metadata: Arc<RwLock<HashMap<String, Value>>>,
}

impl WsContext {
    /// Create a new WebSocket context
    pub fn new(
        connection_id: ConnectionId,
        path: String,
        room_manager: RoomManager,
        message_tx: tokio::sync::mpsc::UnboundedSender<Message>,
    ) -> Self {
        Self {
            connection_id,
            path,
            room_manager,
            message_tx,
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Send a text message
    pub async fn send_text(&self, text: &str) -> HandlerResult<()> {
        self.message_tx
            .send(Message::Text(text.to_string().into()))
            .map_err(|e| HandlerError::SendError(e.to_string()))
    }

    /// Send a binary message
    pub async fn send_binary(&self, data: Vec<u8>) -> HandlerResult<()> {
        self.message_tx
            .send(Message::Binary(data.into()))
            .map_err(|e| HandlerError::SendError(e.to_string()))
    }

    /// Send a JSON message
    pub async fn send_json(&self, value: &Value) -> HandlerResult<()> {
        let text = serde_json::to_string(value)?;
        self.send_text(&text).await
    }

    /// Join a room
    pub async fn join_room(&self, room: &str) -> HandlerResult<()> {
        self.room_manager.join(&self.connection_id, room).await
    }

    /// Leave a room
    pub async fn leave_room(&self, room: &str) -> HandlerResult<()> {
        self.room_manager.leave(&self.connection_id, room).await
    }

    /// Broadcast text to all members in a room
    pub async fn broadcast_to_room(&self, room: &str, text: &str) -> HandlerResult<()> {
        let broadcaster = self.room_manager.get_broadcaster(room).await;
        broadcaster
            .send(text.to_string())
            .map_err(|e| HandlerError::RoomError(e.to_string()))?;
        Ok(())
    }

    /// Get all rooms this connection is in
    pub async fn get_rooms(&self) -> Vec<String> {
        self.room_manager.get_connection_rooms(&self.connection_id).await
    }

    /// Set metadata value
    pub async fn set_metadata(&self, key: &str, value: Value) {
        let mut metadata = self.metadata.write().await;
        metadata.insert(key.to_string(), value);
    }

    /// Get metadata value
    pub async fn get_metadata(&self, key: &str) -> Option<Value> {
        let metadata = self.metadata.read().await;
        metadata.get(key).cloned()
    }
}

/// Trait for WebSocket message handlers
#[async_trait]
pub trait WsHandler: Send + Sync {
    /// Called when a new WebSocket connection is established
    async fn on_connect(&self, _ctx: &mut WsContext) -> HandlerResult<()> {
        Ok(())
    }

    /// Called when a message is received
    async fn on_message(&self, ctx: &mut WsContext, msg: WsMessage) -> HandlerResult<()>;

    /// Called when the connection is closed
    async fn on_disconnect(&self, _ctx: &mut WsContext) -> HandlerResult<()> {
        Ok(())
    }

    /// Check if this handler should handle the given path
    fn handles_path(&self, _path: &str) -> bool {
        true // Default: handle all paths
    }
}

/// Pattern-based message router
pub struct MessageRouter {
    routes: Vec<(MessagePattern, Box<dyn Fn(String) -> Option<String> + Send + Sync>)>,
}

impl MessageRouter {
    /// Create a new message router
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Add a route with a pattern and handler function
    pub fn on<F>(&mut self, pattern: MessagePattern, handler: F) -> &mut Self
    where
        F: Fn(String) -> Option<String> + Send + Sync + 'static,
    {
        self.routes.push((pattern, Box::new(handler)));
        self
    }

    /// Route a message through the registered handlers
    pub fn route(&self, text: &str) -> Option<String> {
        for (pattern, handler) in &self.routes {
            if pattern.matches(text) {
                if let Some(response) = handler(text.to_string()) {
                    return Some(response);
                }
            }
        }
        None
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Handler registry for managing multiple handlers
pub struct HandlerRegistry {
    handlers: Vec<Arc<dyn WsHandler>>,
    hot_reload_enabled: bool,
}

impl HandlerRegistry {
    /// Create a new handler registry
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            hot_reload_enabled: std::env::var("MOCKFORGE_WS_HOTRELOAD")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
        }
    }

    /// Create a registry with hot-reload enabled
    pub fn with_hot_reload() -> Self {
        Self {
            handlers: Vec::new(),
            hot_reload_enabled: true,
        }
    }

    /// Check if hot-reload is enabled
    pub fn is_hot_reload_enabled(&self) -> bool {
        self.hot_reload_enabled
    }

    /// Register a handler
    pub fn register<H: WsHandler + 'static>(&mut self, handler: H) -> &mut Self {
        self.handlers.push(Arc::new(handler));
        self
    }

    /// Get handlers for a specific path
    pub fn get_handlers(&self, path: &str) -> Vec<Arc<dyn WsHandler>> {
        self.handlers.iter().filter(|h| h.handles_path(path)).cloned().collect()
    }

    /// Check if any handler handles the given path
    pub fn has_handler_for(&self, path: &str) -> bool {
        self.handlers.iter().any(|h| h.handles_path(path))
    }

    /// Clear all handlers (useful for hot-reload)
    pub fn clear(&mut self) {
        self.handlers.clear();
    }

    /// Get the number of registered handlers
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Passthrough handler configuration for forwarding messages to upstream servers
#[derive(Clone)]
pub struct PassthroughConfig {
    /// Pattern to match paths for passthrough
    pub pattern: MessagePattern,
    /// Upstream URL to forward to
    pub upstream_url: String,
}

impl PassthroughConfig {
    /// Create a new passthrough configuration
    pub fn new(pattern: MessagePattern, upstream_url: String) -> Self {
        Self {
            pattern,
            upstream_url,
        }
    }

    /// Create a passthrough for all messages matching a regex
    pub fn regex(regex: &str, upstream_url: String) -> HandlerResult<Self> {
        Ok(Self {
            pattern: MessagePattern::regex(regex)?,
            upstream_url,
        })
    }
}

/// Passthrough handler that forwards messages to an upstream server
pub struct PassthroughHandler {
    config: PassthroughConfig,
}

impl PassthroughHandler {
    /// Create a new passthrough handler
    pub fn new(config: PassthroughConfig) -> Self {
        Self { config }
    }

    /// Check if a message should be passed through
    pub fn should_passthrough(&self, text: &str) -> bool {
        self.config.pattern.matches(text)
    }

    /// Get the upstream URL
    pub fn upstream_url(&self) -> &str {
        &self.config.upstream_url
    }
}

#[async_trait]
impl WsHandler for PassthroughHandler {
    async fn on_message(&self, ctx: &mut WsContext, msg: WsMessage) -> HandlerResult<()> {
        if let WsMessage::Text(text) = &msg {
            if self.should_passthrough(text) {
                // In a real implementation, this would forward to upstream
                // For now, we'll just log and echo back
                ctx.send_text(&format!("PASSTHROUGH({}): {}", self.config.upstream_url, text))
                    .await?;
                return Ok(());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== WsMessage Tests ====================

    #[test]
    fn test_ws_message_text_from_axum() {
        let axum_msg = Message::Text("hello".to_string().into());
        let ws_msg: WsMessage = axum_msg.into();
        match ws_msg {
            WsMessage::Text(text) => assert_eq!(text, "hello"),
            _ => panic!("Expected Text message"),
        }
    }

    #[test]
    fn test_ws_message_binary_from_axum() {
        let data = vec![1, 2, 3, 4];
        let axum_msg = Message::Binary(data.clone().into());
        let ws_msg: WsMessage = axum_msg.into();
        match ws_msg {
            WsMessage::Binary(bytes) => assert_eq!(bytes, data),
            _ => panic!("Expected Binary message"),
        }
    }

    #[test]
    fn test_ws_message_ping_from_axum() {
        let data = vec![1, 2];
        let axum_msg = Message::Ping(data.clone().into());
        let ws_msg: WsMessage = axum_msg.into();
        match ws_msg {
            WsMessage::Ping(bytes) => assert_eq!(bytes, data),
            _ => panic!("Expected Ping message"),
        }
    }

    #[test]
    fn test_ws_message_pong_from_axum() {
        let data = vec![3, 4];
        let axum_msg = Message::Pong(data.clone().into());
        let ws_msg: WsMessage = axum_msg.into();
        match ws_msg {
            WsMessage::Pong(bytes) => assert_eq!(bytes, data),
            _ => panic!("Expected Pong message"),
        }
    }

    #[test]
    fn test_ws_message_close_from_axum() {
        let axum_msg = Message::Close(None);
        let ws_msg: WsMessage = axum_msg.into();
        assert!(matches!(ws_msg, WsMessage::Close));
    }

    #[test]
    fn test_ws_message_text_to_axum() {
        let ws_msg = WsMessage::Text("hello".to_string());
        let axum_msg: Message = ws_msg.into();
        assert!(matches!(axum_msg, Message::Text(_)));
    }

    #[test]
    fn test_ws_message_binary_to_axum() {
        let ws_msg = WsMessage::Binary(vec![1, 2, 3]);
        let axum_msg: Message = ws_msg.into();
        assert!(matches!(axum_msg, Message::Binary(_)));
    }

    #[test]
    fn test_ws_message_close_to_axum() {
        let ws_msg = WsMessage::Close;
        let axum_msg: Message = ws_msg.into();
        assert!(matches!(axum_msg, Message::Close(_)));
    }

    // ==================== MessagePattern Tests ====================

    #[test]
    fn test_message_pattern_regex() {
        let pattern = MessagePattern::regex(r"^hello").unwrap();
        assert!(pattern.matches("hello world"));
        assert!(!pattern.matches("goodbye world"));
    }

    #[test]
    fn test_message_pattern_regex_invalid() {
        let result = MessagePattern::regex(r"[invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_message_pattern_exact() {
        let pattern = MessagePattern::exact("hello");
        assert!(pattern.matches("hello"));
        assert!(!pattern.matches("hello world"));
    }

    #[test]
    fn test_message_pattern_jsonpath() {
        let pattern = MessagePattern::jsonpath("$.type");
        assert!(pattern.matches(r#"{"type": "message"}"#));
        assert!(!pattern.matches(r#"{"name": "test"}"#));
    }

    #[test]
    fn test_message_pattern_jsonpath_nested() {
        let pattern = MessagePattern::jsonpath("$.user.name");
        assert!(pattern.matches(r#"{"user": {"name": "John"}}"#));
        assert!(!pattern.matches(r#"{"user": {"email": "john@example.com"}}"#));
    }

    #[test]
    fn test_message_pattern_jsonpath_invalid_json() {
        let pattern = MessagePattern::jsonpath("$.type");
        assert!(!pattern.matches("not json"));
    }

    #[test]
    fn test_message_pattern_any() {
        let pattern = MessagePattern::any();
        assert!(pattern.matches("anything"));
        assert!(pattern.matches(""));
        assert!(pattern.matches(r#"{"json": true}"#));
    }

    #[test]
    fn test_message_pattern_extract() {
        let pattern = MessagePattern::jsonpath("$.type");
        let result = pattern.extract(r#"{"type": "greeting", "data": "hello"}"#, "$.type");
        assert_eq!(result, Some(serde_json::json!("greeting")));
    }

    #[test]
    fn test_message_pattern_extract_nested() {
        let pattern = MessagePattern::any();
        let result = pattern.extract(r#"{"user": {"id": 123}}"#, "$.user.id");
        assert_eq!(result, Some(serde_json::json!(123)));
    }

    #[test]
    fn test_message_pattern_extract_not_found() {
        let pattern = MessagePattern::any();
        let result = pattern.extract(r#"{"type": "message"}"#, "$.nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_message_pattern_extract_invalid_json() {
        let pattern = MessagePattern::any();
        let result = pattern.extract("not json", "$.type");
        assert!(result.is_none());
    }

    // ==================== RoomManager Tests ====================

    #[tokio::test]
    async fn test_room_manager() {
        let manager = RoomManager::new();

        // Join rooms
        manager.join("conn1", "room1").await.unwrap();
        manager.join("conn1", "room2").await.unwrap();
        manager.join("conn2", "room1").await.unwrap();

        // Check room members
        let room1_members = manager.get_room_members("room1").await;
        assert_eq!(room1_members.len(), 2);
        assert!(room1_members.contains(&"conn1".to_string()));
        assert!(room1_members.contains(&"conn2".to_string()));

        // Check connection rooms
        let conn1_rooms = manager.get_connection_rooms("conn1").await;
        assert_eq!(conn1_rooms.len(), 2);
        assert!(conn1_rooms.contains(&"room1".to_string()));
        assert!(conn1_rooms.contains(&"room2".to_string()));

        // Leave room
        manager.leave("conn1", "room1").await.unwrap();
        let room1_members = manager.get_room_members("room1").await;
        assert_eq!(room1_members.len(), 1);
        assert!(room1_members.contains(&"conn2".to_string()));

        // Leave all rooms
        manager.leave_all("conn1").await.unwrap();
        let conn1_rooms = manager.get_connection_rooms("conn1").await;
        assert_eq!(conn1_rooms.len(), 0);
    }

    #[tokio::test]
    async fn test_room_manager_default() {
        let manager = RoomManager::default();
        // Should work the same as new()
        manager.join("conn1", "room1").await.unwrap();
        let members = manager.get_room_members("room1").await;
        assert_eq!(members.len(), 1);
    }

    #[tokio::test]
    async fn test_room_manager_empty_room() {
        let manager = RoomManager::new();
        let members = manager.get_room_members("nonexistent").await;
        assert!(members.is_empty());
    }

    #[tokio::test]
    async fn test_room_manager_empty_connection() {
        let manager = RoomManager::new();
        let rooms = manager.get_connection_rooms("nonexistent").await;
        assert!(rooms.is_empty());
    }

    #[tokio::test]
    async fn test_room_manager_leave_nonexistent() {
        let manager = RoomManager::new();
        // Should not error when leaving a room we're not in
        let result = manager.leave("conn1", "room1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_room_manager_broadcaster() {
        let manager = RoomManager::new();
        manager.join("conn1", "room1").await.unwrap();

        let broadcaster = manager.get_broadcaster("room1").await;
        let mut receiver = broadcaster.subscribe();

        // Send a message
        broadcaster.send("hello".to_string()).unwrap();

        // Receive it
        let msg = receiver.recv().await.unwrap();
        assert_eq!(msg, "hello");
    }

    #[tokio::test]
    async fn test_room_manager_room_cleanup_on_last_leave() {
        let manager = RoomManager::new();
        manager.join("conn1", "room1").await.unwrap();
        manager.leave("conn1", "room1").await.unwrap();

        // Room should be cleaned up
        let members = manager.get_room_members("room1").await;
        assert!(members.is_empty());
    }

    // ==================== MessageRouter Tests ====================

    #[test]
    fn test_message_router() {
        let mut router = MessageRouter::new();

        router
            .on(MessagePattern::exact("ping"), |_| Some("pong".to_string()))
            .on(MessagePattern::regex(r"^hello").unwrap(), |_| Some("hi there!".to_string()));

        assert_eq!(router.route("ping"), Some("pong".to_string()));
        assert_eq!(router.route("hello world"), Some("hi there!".to_string()));
        assert_eq!(router.route("goodbye"), None);
    }

    #[test]
    fn test_message_router_default() {
        let router = MessageRouter::default();
        // Empty router returns None for all messages
        assert_eq!(router.route("anything"), None);
    }

    #[test]
    fn test_message_router_first_match_wins() {
        let mut router = MessageRouter::new();
        router
            .on(MessagePattern::any(), |_| Some("first".to_string()))
            .on(MessagePattern::any(), |_| Some("second".to_string()));

        assert_eq!(router.route("test"), Some("first".to_string()));
    }

    #[test]
    fn test_message_router_handler_returns_none() {
        let mut router = MessageRouter::new();
        router
            .on(MessagePattern::exact("skip"), |_| None)
            .on(MessagePattern::any(), |_| Some("fallback".to_string()));

        // First pattern matches but returns None, so it continues to next
        assert_eq!(router.route("skip"), Some("fallback".to_string()));
    }

    // ==================== HandlerRegistry Tests ====================

    struct TestHandler;

    #[async_trait]
    impl WsHandler for TestHandler {
        async fn on_message(&self, _ctx: &mut WsContext, _msg: WsMessage) -> HandlerResult<()> {
            Ok(())
        }
    }

    struct PathSpecificHandler {
        path: String,
    }

    #[async_trait]
    impl WsHandler for PathSpecificHandler {
        async fn on_message(&self, _ctx: &mut WsContext, _msg: WsMessage) -> HandlerResult<()> {
            Ok(())
        }

        fn handles_path(&self, path: &str) -> bool {
            path == self.path
        }
    }

    #[test]
    fn test_handler_registry_new() {
        let registry = HandlerRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_handler_registry_default() {
        let registry = HandlerRegistry::default();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_handler_registry_register() {
        let mut registry = HandlerRegistry::new();
        registry.register(TestHandler);
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_handler_registry_get_handlers() {
        let mut registry = HandlerRegistry::new();
        registry.register(TestHandler);

        let handlers = registry.get_handlers("/any/path");
        assert_eq!(handlers.len(), 1);
    }

    #[test]
    fn test_handler_registry_path_filtering() {
        let mut registry = HandlerRegistry::new();
        registry.register(PathSpecificHandler {
            path: "/ws/chat".to_string(),
        });
        registry.register(PathSpecificHandler {
            path: "/ws/events".to_string(),
        });

        let chat_handlers = registry.get_handlers("/ws/chat");
        assert_eq!(chat_handlers.len(), 1);

        let events_handlers = registry.get_handlers("/ws/events");
        assert_eq!(events_handlers.len(), 1);

        let other_handlers = registry.get_handlers("/ws/other");
        assert!(other_handlers.is_empty());
    }

    #[test]
    fn test_handler_registry_has_handler_for() {
        let mut registry = HandlerRegistry::new();
        registry.register(PathSpecificHandler {
            path: "/ws/chat".to_string(),
        });

        assert!(registry.has_handler_for("/ws/chat"));
        assert!(!registry.has_handler_for("/ws/other"));
    }

    #[test]
    fn test_handler_registry_clear() {
        let mut registry = HandlerRegistry::new();
        registry.register(TestHandler);
        registry.register(TestHandler);
        assert_eq!(registry.len(), 2);

        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_handler_registry_with_hot_reload() {
        let registry = HandlerRegistry::with_hot_reload();
        assert!(registry.is_hot_reload_enabled());
    }

    // ==================== PassthroughConfig Tests ====================

    #[test]
    fn test_passthrough_config_new() {
        let config =
            PassthroughConfig::new(MessagePattern::any(), "ws://upstream:8080".to_string());
        assert_eq!(config.upstream_url, "ws://upstream:8080");
    }

    #[test]
    fn test_passthrough_config_regex() {
        let config =
            PassthroughConfig::regex(r"^forward", "ws://upstream:8080".to_string()).unwrap();
        assert!(config.pattern.matches("forward this"));
        assert!(!config.pattern.matches("don't forward"));
    }

    #[test]
    fn test_passthrough_config_regex_invalid() {
        let result = PassthroughConfig::regex(r"[invalid", "ws://upstream:8080".to_string());
        assert!(result.is_err());
    }

    // ==================== PassthroughHandler Tests ====================

    #[test]
    fn test_passthrough_handler_should_passthrough() {
        let config =
            PassthroughConfig::regex(r"^proxy:", "ws://upstream:8080".to_string()).unwrap();
        let handler = PassthroughHandler::new(config);

        assert!(handler.should_passthrough("proxy:hello"));
        assert!(!handler.should_passthrough("hello"));
    }

    #[test]
    fn test_passthrough_handler_upstream_url() {
        let config =
            PassthroughConfig::new(MessagePattern::any(), "ws://upstream:8080".to_string());
        let handler = PassthroughHandler::new(config);

        assert_eq!(handler.upstream_url(), "ws://upstream:8080");
    }

    // ==================== HandlerError Tests ====================

    #[test]
    fn test_handler_error_send_error() {
        let err = HandlerError::SendError("connection closed".to_string());
        assert!(err.to_string().contains("send message"));
        assert!(err.to_string().contains("connection closed"));
    }

    #[test]
    fn test_handler_error_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let err = HandlerError::JsonError(json_err);
        assert!(err.to_string().contains("JSON"));
    }

    #[test]
    fn test_handler_error_pattern_error() {
        let err = HandlerError::PatternError("invalid regex".to_string());
        assert!(err.to_string().contains("Pattern"));
    }

    #[test]
    fn test_handler_error_room_error() {
        let err = HandlerError::RoomError("room full".to_string());
        assert!(err.to_string().contains("Room"));
    }

    #[test]
    fn test_handler_error_connection_error() {
        let err = HandlerError::ConnectionError("timeout".to_string());
        assert!(err.to_string().contains("Connection"));
    }

    #[test]
    fn test_handler_error_generic() {
        let err = HandlerError::Generic("something went wrong".to_string());
        assert!(err.to_string().contains("something went wrong"));
    }

    // ==================== WsContext Tests ====================

    #[tokio::test]
    async fn test_ws_context_metadata() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let ctx = WsContext::new("conn123".to_string(), "/ws".to_string(), RoomManager::new(), tx);

        // Set and get metadata
        ctx.set_metadata("user", serde_json::json!({"id": 1})).await;
        let value = ctx.get_metadata("user").await;
        assert_eq!(value, Some(serde_json::json!({"id": 1})));

        // Get nonexistent key
        let missing = ctx.get_metadata("nonexistent").await;
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_ws_context_send_text() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let ctx = WsContext::new("conn123".to_string(), "/ws".to_string(), RoomManager::new(), tx);

        ctx.send_text("hello").await.unwrap();

        let msg = rx.recv().await.unwrap();
        assert!(matches!(msg, Message::Text(_)));
    }

    #[tokio::test]
    async fn test_ws_context_send_binary() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let ctx = WsContext::new("conn123".to_string(), "/ws".to_string(), RoomManager::new(), tx);

        ctx.send_binary(vec![1, 2, 3]).await.unwrap();

        let msg = rx.recv().await.unwrap();
        assert!(matches!(msg, Message::Binary(_)));
    }

    #[tokio::test]
    async fn test_ws_context_send_json() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let ctx = WsContext::new("conn123".to_string(), "/ws".to_string(), RoomManager::new(), tx);

        ctx.send_json(&serde_json::json!({"type": "test"})).await.unwrap();

        let msg = rx.recv().await.unwrap();
        assert!(matches!(msg, Message::Text(_)));
    }

    #[tokio::test]
    async fn test_ws_context_rooms() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let ctx = WsContext::new("conn123".to_string(), "/ws".to_string(), RoomManager::new(), tx);

        // Join rooms
        ctx.join_room("chat").await.unwrap();
        ctx.join_room("notifications").await.unwrap();

        let rooms = ctx.get_rooms().await;
        assert_eq!(rooms.len(), 2);

        // Leave room
        ctx.leave_room("chat").await.unwrap();
        let rooms = ctx.get_rooms().await;
        assert_eq!(rooms.len(), 1);
    }
}
