# WebSocket Programmable Handlers

**Status:** ✅ Implemented

This document describes the programmable WebSocket handler system in MockForge, which allows you to move beyond static replay and create dynamic, scripted WebSocket event flows.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Handler API](#handler-api)
- [Room Management](#room-management)
- [Message Pattern Matching](#message-pattern-matching)
- [Passthrough Support](#passthrough-support)
- [Hot Reload](#hot-reload)
- [Examples](#examples)
- [Testing](#testing)
- [API Reference](#api-reference)

## Overview

The programmable handler system provides:

- **Connection Lifecycle Hooks**: Handle `on_connect`, `on_message`, and `on_disconnect` events
- **Pattern-Based Routing**: Route messages using regex or JSONPath patterns
- **Room/Broadcast Support**: Group connections and broadcast messages to rooms
- **Passthrough to Upstream**: Selectively forward messages to real WebSocket servers
- **Hot Reload**: Automatically reload handlers when code changes (via `MOCKFORGE_WS_HOTRELOAD=1`)
- **Coexistence with Replay**: Handlers and replay modes can work together

## Quick Start

### 1. Create a Simple Echo Handler

```rust
use async_trait::async_trait;
use mockforge_ws::{HandlerResult, WsContext, WsHandler, WsMessage};

struct EchoHandler;

#[async_trait]
impl WsHandler for EchoHandler {
    async fn on_connect(&self, ctx: &mut WsContext) -> HandlerResult<()> {
        ctx.send_text("Welcome!").await?;
        Ok(())
    }

    async fn on_message(&self, ctx: &mut WsContext, msg: WsMessage) -> HandlerResult<()> {
        if let WsMessage::Text(text) = msg {
            ctx.send_text(&format!("echo: {}", text)).await?;
        }
        Ok(())
    }
}
```

### 2. Register and Start the Server

```rust
use mockforge_ws::{router_with_handlers, HandlerRegistry};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = HandlerRegistry::new();
    registry.register(EchoHandler);

    let app = router_with_handlers(Arc::new(registry));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3030").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

### 3. Test with a WebSocket Client

```bash
# Using websocat
websocat ws://localhost:3030/ws
> hello
< Welcome!
< echo: hello
```

## Handler API

### Core Trait: `WsHandler`

The `WsHandler` trait defines the interface for all WebSocket handlers:

```rust
#[async_trait]
pub trait WsHandler: Send + Sync {
    /// Called when a new connection is established
    async fn on_connect(&self, ctx: &mut WsContext) -> HandlerResult<()> {
        Ok(())
    }

    /// Called when a message is received
    async fn on_message(&self, ctx: &mut WsContext, msg: WsMessage) -> HandlerResult<()>;

    /// Called when the connection is closed
    async fn on_disconnect(&self, ctx: &mut WsContext) -> HandlerResult<()> {
        Ok(())
    }

    /// Check if this handler should handle the given path
    fn handles_path(&self, path: &str) -> bool {
        true // Default: handle all paths
    }
}
```

### Context: `WsContext`

The `WsContext` provides methods for interacting with the WebSocket connection:

```rust
pub struct WsContext {
    pub connection_id: String,  // Unique connection identifier
    pub path: String,            // WebSocket path (e.g., "/ws/chat")
    // ... internal fields
}

impl WsContext {
    // Send messages
    async fn send_text(&self, text: &str) -> HandlerResult<()>;
    async fn send_binary(&self, data: Vec<u8>) -> HandlerResult<()>;
    async fn send_json(&self, value: &serde_json::Value) -> HandlerResult<()>;

    // Room management
    async fn join_room(&self, room: &str) -> HandlerResult<()>;
    async fn leave_room(&self, room: &str) -> HandlerResult<()>;
    async fn broadcast_to_room(&self, room: &str, text: &str) -> HandlerResult<()>;
    async fn get_rooms(&self) -> Vec<String>;

    // Metadata storage
    async fn set_metadata(&self, key: &str, value: serde_json::Value);
    async fn get_metadata(&self, key: &str) -> Option<serde_json::Value>;
}
```

### Message Types: `WsMessage`

```rust
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close,
}
```

## Room Management

Rooms allow you to group connections and broadcast messages to all members.

### Example: Chat Room Handler

```rust
use async_trait::async_trait;
use mockforge_ws::{HandlerResult, WsContext, WsHandler, WsMessage};
use serde_json::json;

struct ChatHandler;

#[async_trait]
impl WsHandler for ChatHandler {
    async fn on_connect(&self, ctx: &mut WsContext) -> HandlerResult<()> {
        // Auto-join "general" room
        ctx.join_room("general").await?;

        let welcome = json!({
            "type": "system",
            "message": "Welcome to chat! You're in room: general"
        });
        ctx.send_json(&welcome).await?;

        Ok(())
    }

    async fn on_message(&self, ctx: &mut WsContext, msg: WsMessage) -> HandlerResult<()> {
        if let WsMessage::Text(text) = msg {
            if text.starts_with("/join ") {
                let room = text.trim_start_matches("/join ").trim();
                ctx.join_room(room).await?;
                ctx.send_text(&format!("Joined room: {}", room)).await?;
            } else if text.starts_with("/msg ") {
                // Broadcast to current rooms
                let rooms = ctx.get_rooms().await;
                for room in rooms {
                    ctx.broadcast_to_room(&room, &text).await?;
                }
            }
        }
        Ok(())
    }

    fn handles_path(&self, path: &str) -> bool {
        path.starts_with("/ws/chat")
    }
}
```

### Usage

```bash
# Connect client 1
websocat ws://localhost:3030/ws/chat
< {"type":"system","message":"Welcome to chat! You're in room: general"}
> /join lobby
< Joined room: lobby
> /msg Hello everyone!
```

## Message Pattern Matching

Use `MessagePattern` to route messages based on content:

```rust
use mockforge_ws::MessagePattern;

// Regex matching
let pattern = MessagePattern::regex(r"^/join (.+)$")?;
if pattern.matches("/join lobby") {
    println!("Match!");
}

// JSONPath matching
let pattern = MessagePattern::jsonpath("$.type");
if pattern.matches(r#"{"type":"message","content":"hello"}"#) {
    println!("Has 'type' field!");
}

// Exact matching
let pattern = MessagePattern::exact("ping");
if pattern.matches("ping") {
    println!("Exact match!");
}
```

### Example: Pattern-Based Router

```rust
use mockforge_ws::{MessagePattern, MessageRouter};

let mut router = MessageRouter::new();

router
    .on(MessagePattern::exact("ping"), |_| Some("pong".to_string()))
    .on(MessagePattern::regex(r"^hello").unwrap(), |_| {
        Some("Hi there!".to_string())
    })
    .on(MessagePattern::jsonpath("$.action"), |msg| {
        // Handle JSON messages with 'action' field
        Some(format!("Received action: {}", msg))
    });

// Route a message
if let Some(response) = router.route("ping") {
    println!("Response: {}", response); // "pong"
}
```

## Passthrough Support

Forward messages to upstream WebSocket servers based on patterns:

```rust
use mockforge_ws::{PassthroughConfig, PassthroughHandler, MessagePattern};

#[async_trait]
impl WsHandler for MyHandler {
    async fn on_message(&self, ctx: &mut WsContext, msg: WsMessage) -> HandlerResult<()> {
        if let WsMessage::Text(text) = &msg {
            // Check if message should be forwarded to upstream
            if text.starts_with("FORWARD:") {
                // In practice, you'd use the WsProxyHandler from mockforge-core
                // for actual upstream forwarding
                ctx.send_text("Message forwarded to upstream").await?;
                return Ok(());
            }
        }

        // Handle locally
        ctx.send_text("Handled locally").await?;
        Ok(())
    }
}
```

### Passthrough Configuration

```rust
use mockforge_ws::{PassthroughConfig, PassthroughHandler, MessagePattern};

let config = PassthroughConfig::regex(
    r"^PROXY:",
    "wss://api.example.com/ws".to_string()
)?;

let handler = PassthroughHandler::new(config);
```

## Hot Reload

Enable hot-reload to automatically reload handlers when code changes:

### Enable via Environment Variable

```bash
export MOCKFORGE_WS_HOTRELOAD=1
cargo run
```

### Enable Programmatically

```rust
use mockforge_ws::HandlerRegistry;

let registry = HandlerRegistry::with_hot_reload();
```

### How It Works

When hot-reload is enabled:
1. The registry checks the `MOCKFORGE_WS_HOTRELOAD` environment variable
2. Handler changes are detected (implementation-specific)
3. The registry can be cleared and reloaded with `clear()` and re-registration

**Note:** Full hot-reload implementation requires a file watcher (e.g., `notify` crate) to detect handler file changes and trigger reloads. The foundation is in place via `HandlerRegistry::with_hot_reload()` and `clear()` methods.

## Examples

### Example 1: Echo Handler

See [examples/ws-handlers/echo_handler.rs](examples/ws-handlers/echo_handler.rs)

```bash
cargo run --example ws-handlers-demo
```

### Example 2: Chat Handler with Rooms

See [examples/ws-handlers/chat_handler.rs](examples/ws-handlers/chat_handler.rs)

**Features:**
- Auto-join default room on connect
- `/join <room>` to join a room
- `/leave <room>` to leave a room
- `/rooms` to list current rooms
- JSON message support

### Example 3: Combined Echo + Chat Server

```rust
let mut registry = HandlerRegistry::new();

// Echo handler for /ws
registry.register(EchoHandler);

// Chat handler for /ws/chat
registry.register(ChatHandler::new());

let app = router_with_handlers(Arc::new(registry));
```

## Testing

### Unit Testing

Test handlers in isolation:

```rust
#[tokio::test]
async fn test_my_handler() {
    let handler = MyHandler;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let room_manager = RoomManager::new();
    let mut ctx = WsContext::new(
        "test-conn".to_string(),
        "/ws".to_string(),
        room_manager,
        tx
    );

    // Test on_connect
    handler.on_connect(&mut ctx).await.unwrap();

    // Verify messages sent
    let msg = rx.recv().await.unwrap();
    assert!(matches!(msg, Message::Text(_)));
}
```

### End-to-End Testing

Test with real WebSocket connections:

```rust
#[tokio::test]
async fn test_handler_e2e() {
    let mut registry = HandlerRegistry::new();
    registry.register(EchoHandler);

    let app = router_with_handlers(Arc::new(registry));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Connect and test
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(
        format!("ws://{}/ws", addr)
    ).await.unwrap();

    // Send message
    ws_stream.send(Message::Text("hello".into())).await.unwrap();

    // Receive response
    if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
        assert_eq!(text, "echo: hello");
    }
}
```

See [crates/mockforge-ws/tests/ws_handlers_e2e.rs](crates/mockforge-ws/tests/ws_handlers_e2e.rs) for comprehensive examples.

## API Reference

### Core Types

- **`WsHandler`**: Main trait for implementing handlers
- **`WsContext`**: Connection context with send/room methods
- **`WsMessage`**: WebSocket message wrapper
- **`HandlerResult<T>`**: Result type for handler operations
- **`HandlerError`**: Error type for handler operations

### Pattern Matching

- **`MessagePattern`**: Pattern matching for messages
  - `MessagePattern::regex(pattern)` - Regex matching
  - `MessagePattern::jsonpath(query)` - JSONPath matching
  - `MessagePattern::exact(text)` - Exact text matching
  - `MessagePattern::any()` - Match everything

### Registry

- **`HandlerRegistry`**: Registry for managing handlers
  - `new()` - Create new registry
  - `with_hot_reload()` - Create with hot-reload enabled
  - `register(handler)` - Register a handler
  - `get_handlers(path)` - Get handlers for path
  - `clear()` - Clear all handlers

### Room Management

- **`RoomManager`**: Manages rooms and broadcasts
  - `join(conn_id, room)` - Join a room
  - `leave(conn_id, room)` - Leave a room
  - `leave_all(conn_id)` - Leave all rooms
  - `get_room_members(room)` - Get room members
  - `get_connection_rooms(conn_id)` - Get connection's rooms

### Routing

- **`MessageRouter`**: Pattern-based message routing
  - `on(pattern, handler)` - Add route
  - `route(text)` - Route message through handlers

### Passthrough

- **`PassthroughConfig`**: Configuration for upstream forwarding
  - `new(pattern, upstream_url)` - Create config
  - `regex(regex, upstream_url)` - Create with regex pattern

- **`PassthroughHandler`**: Handler for forwarding to upstream
  - `new(config)` - Create passthrough handler
  - `should_passthrough(text)` - Check if should forward
  - `upstream_url()` - Get upstream URL

## Environment Variables

- **`MOCKFORGE_WS_HOTRELOAD`**: Enable hot-reload (`1` or `true`)
- **`MOCKFORGE_WS_REPLAY_FILE`**: Use replay mode (coexists with handlers)
- **`MOCKFORGE_RESPONSE_TEMPLATE_EXPAND`**: Enable template expansion in replay

## Coexistence with Replay Mode

Handlers and replay modes can work together:

```bash
# Enable both handlers and replay
export MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl
cargo run --example ws-handlers-demo
```

When both are enabled:
1. Handlers process incoming messages
2. Replay sends scripted messages based on the JSONL file
3. Handlers can intercept and modify replay messages

## Best Practices

1. **Path Routing**: Use `handles_path()` to route handlers to specific paths
2. **Error Handling**: Always handle errors in `on_message` to prevent connection drops
3. **Room Cleanup**: Implement `on_disconnect` to clean up room memberships
4. **Testing**: Write both unit tests and E2E tests for handlers
5. **Metadata**: Use context metadata to store connection-specific state
6. **Logging**: Use tracing for debugging handler behavior

## Troubleshooting

### Handler Not Receiving Messages

- Check `handles_path()` implementation
- Verify handler is registered in the registry
- Check if another handler is handling all paths

### Room Broadcasts Not Working

- Ensure all clients have joined the room via `join_room()`
- Check that `broadcast_to_room()` is called with correct room name
- Verify room manager is shared across connections

### Hot Reload Not Working

- Check `MOCKFORGE_WS_HOTRELOAD` environment variable is set
- Implement file watching to trigger `clear()` and re-registration
- Ensure registry is wrapped in `Arc` for sharing

## Future Enhancements

Potential improvements to the handler system:

1. **Middleware Support**: Pre/post-processing of messages
2. **Handler Chains**: Compose multiple handlers
3. **Built-in File Watcher**: Automatic hot-reload on file changes
4. **Handler Metrics**: Track handler performance
5. **Rate Limiting**: Per-handler rate limiting
6. **Message Filtering**: Filter messages before handler processing

## Contributing

To contribute to the WebSocket handlers:

1. Add tests in `crates/mockforge-ws/tests/`
2. Update documentation in this file
3. Add examples in `examples/ws-handlers/`
4. Run `cargo test -p mockforge-ws` to verify

## License

Same as MockForge: MIT OR Apache-2.0
