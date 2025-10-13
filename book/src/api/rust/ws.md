# WebSocket Module

The `mockforge_ws` crate provides comprehensive WebSocket mocking with replay, proxy, and AI-powered event generation capabilities.

## Modules

### Core Functions

#### `router`

```rust
pub fn router() -> Router
```

Creates a basic WebSocket router with echo functionality.

**Returns:** Axum `Router` configured for WebSocket connections

**Example:**
```rust
use mockforge_ws::router;

let app = router();
// Routes WebSocket connections to /ws
```

#### `router_with_latency`

```rust
pub fn router_with_latency(latency_injector: LatencyInjector) -> Router
```

Creates a WebSocket router with latency simulation.

**Parameters:**
- `latency_injector`: Latency injection configuration

**Returns:** Axum `Router` with latency simulation

**Example:**
```rust
use mockforge_ws::router_with_latency;
use mockforge_core::{LatencyProfile, latency::LatencyInjector};

let latency = LatencyProfile::slow(); // 300-800ms
let injector = LatencyInjector::new(latency, Default::default());
let app = router_with_latency(injector);
```

#### `router_with_proxy`

```rust
pub fn router_with_proxy(proxy_handler: WsProxyHandler) -> Router
```

Creates a WebSocket router with proxy capabilities.

**Parameters:**
- `proxy_handler`: WebSocket proxy handler configuration

**Returns:** Axum `Router` with proxy functionality

**Example:**
```rust
use mockforge_ws::router_with_proxy;
use mockforge_core::{WsProxyConfig, WsProxyHandler};

let proxy_config = WsProxyConfig {
    upstream_url: "wss://api.example.com/ws".to_string(),
    should_proxy: true,
    ..Default::default()
};
let proxy = WsProxyHandler::new(proxy_config);
let app = router_with_proxy(proxy);
```

### Server Functions

#### `start_with_latency`

```rust
pub async fn start_with_latency(
    port: u16,
    latency: Option<LatencyProfile>,
) -> Result<(), Box<dyn std::error::Error>>
```

Starts a WebSocket server with optional latency simulation.

**Parameters:**
- `port`: Port number to bind to
- `latency`: Optional latency profile

**Returns:** `Result<(), Error>` indicating server startup success

**Example:**
```rust
use mockforge_ws::start_with_latency;
use mockforge_core::LatencyProfile;

start_with_latency(3001, Some(LatencyProfile::normal())).await?;
```

### AI Event Generation

#### `AiEventGenerator`

```rust
pub struct AiEventGenerator { /* fields omitted */ }
```

Generator for AI-powered WebSocket event streams.

**Methods:**
```rust
impl AiEventGenerator {
    pub fn new(config: ReplayAugmentationConfig) -> Result<Self>

    pub async fn stream_events(
        &self,
        socket: WebSocket,
        max_events: Option<usize>,
    )

    pub async fn stream_events_with_rate(
        &self,
        socket: WebSocket,
        max_events: Option<usize>,
        events_per_second: f64,
    )
}
```

**Example:**
```rust
use mockforge_ws::AiEventGenerator;
use mockforge_data::ReplayAugmentationConfig;

let config = ReplayAugmentationConfig {
    narrative: "Simulate stock market trading".to_string(),
    ..Default::default()
};

let generator = AiEventGenerator::new(config)?;
generator.stream_events(socket, Some(100)).await?;
```

#### `WebSocketAiConfig`

```rust
pub struct WebSocketAiConfig {
    pub enabled: bool,
    pub replay: Option<ReplayAugmentationConfig>,
    pub max_events: Option<usize>,
}
```

Configuration for WebSocket AI features.

**Fields:**
- `enabled`: Whether AI features are enabled
- `replay`: Optional replay augmentation configuration
- `max_events`: Maximum number of events to generate

### Tracing Functions

#### `create_ws_connection_span`

```rust
pub fn create_ws_connection_span(request: &Request) -> Span
```

Creates an OpenTelemetry span for WebSocket connection establishment.

**Parameters:**
- `request`: HTTP request that initiated the WebSocket connection

**Returns:** OpenTelemetry `Span` for connection tracking

#### `create_ws_message_span`

```rust
pub fn create_ws_message_span(message_size: usize, direction: &str) -> Span
```

Creates an OpenTelemetry span for WebSocket message processing.

**Parameters:**
- `message_size`: Size of the message in bytes
- `direction`: Message direction ("in" or "out")

**Returns:** OpenTelemetry `Span` for message tracking

#### `record_ws_connection_success`

```rust
pub fn record_ws_connection_success(span: &Span)
```

Records successful WebSocket connection establishment.

**Parameters:**
- `span`: Connection span to record success on

#### `record_ws_message_success`

```rust
pub fn record_ws_message_success(span: &Span, message_size: usize)
```

Records successful WebSocket message processing.

**Parameters:**
- `span`: Message span to record success on
- `message_size`: Size of processed message

#### `record_ws_error`

```rust
pub fn record_ws_error(span: &Span, error: &str)
```

Records WebSocket error.

**Parameters:**
- `span`: Span to record error on
- `error`: Error description

### Template Expansion

#### Token Expansion Functions

The crate includes internal template expansion functionality for replay files:

```rust
fn expand_tokens(text: &str) -> String
```

Expands template tokens in replay file content.

**Supported Tokens:**
- `{{uuid}}`: Generates random UUID
- `{{now}}`: Current timestamp in RFC3339 format
- `{{now+1m}}`: Timestamp 1 minute from now
- `{{now+1h}}`: Timestamp 1 hour from now
- `{{randInt min max}}`: Random integer between min and max

**Example:**
```rust
let text = "Hello {{uuid}} at {{now}}";
let expanded = expand_tokens(text);
// Result: "Hello 550e8400-e29b-41d4-a716-446655440000 at 2024-01-15T10:30:00Z"
```

### Internal Types

#### WebSocket Message Handling

The crate uses Axum's WebSocket types internally:

```rust
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
```

**Message Types:**
- `Message::Text(String)`: Text message
- `Message::Binary(Vec<u8>)`: Binary message
- `Message::Close(Option<CloseFrame>)`: Connection close
- `Message::Ping(Vec<u8>)`: Ping message
- `Message::Pong(Vec<u8>)`: Pong message

### Error Handling

All public functions return `Result<T, Box<dyn std::error::Error>>`. Common errors include:

- Server binding errors
- WebSocket protocol errors
- File I/O errors (for replay files)
- AI service errors
- Template expansion errors

### Constants

- Default WebSocket path: `/ws`
- Default server port: 3001

### Feature Flags

- `data-faker`: Enables rich data generation features

## Examples

### Basic WebSocket Server

```rust
use mockforge_ws::router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = router();

    let addr = "0.0.0.0:3001".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

### Server with Latency Simulation

```rust
use mockforge_ws::start_with_latency;
use mockforge_core::LatencyProfile;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Add 50-200ms latency to all messages
    start_with_latency(3001, Some(LatencyProfile::normal())).await?;
    Ok(())
}
```

### Proxy Server

```rust
use mockforge_ws::router_with_proxy;
use mockforge_core::{WsProxyConfig, WsProxyHandler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proxy_config = WsProxyConfig {
        upstream_url: "wss://echo.websocket.org".to_string(),
        should_proxy: true,
        message_transform: None,
    };

    let proxy = WsProxyHandler::new(proxy_config);
    let app = router_with_proxy(proxy);

    let addr = "0.0.0.0:3001".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

### Replay Mode

```rust
use mockforge_ws::router;

// Set replay file via environment variable
std::env::set_var("MOCKFORGE_WS_REPLAY_FILE", "./replay.jsonl");

// Enable template expansion
std::env::set_var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND", "1");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = router();

    let addr = "0.0.0.0:3001".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

### AI Event Generation

```rust
use mockforge_ws::AiEventGenerator;
use mockforge_data::ReplayAugmentationConfig;
use axum::extract::ws::WebSocket;

async fn handle_ai_events(mut socket: WebSocket) {
    let config = ReplayAugmentationConfig {
        narrative: "Simulate a live chat conversation with multiple users".to_string(),
        event_count: 50,
        provider: "openai".to_string(),
        ..Default::default()
    };

    let generator = AiEventGenerator::new(config)?;
    generator.stream_events_with_rate(socket, None, 2.0).await?; // 2 events/sec
}
```

### Custom WebSocket Handler

```rust
use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};

async fn custom_ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_custom_socket(socket))
}

async fn handle_custom_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                // Process text message
                let response = format!("Echo: {}", text);
                if socket.send(axum::extract::ws::Message::Text(response.into())).await.is_err() {
                    break;
                }
            }
            Ok(axum::extract::ws::Message::Close(_)) => {
                break;
            }
            Err(e) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
}
```

### Tracing Integration

```rust
use mockforge_ws::{create_ws_connection_span, record_ws_connection_success};
use axum::extract::ws::WebSocketUpgrade;

async fn traced_ws_handler(
    ws: WebSocketUpgrade,
    request: axum::http::Request<axum::body::Body>,
) -> impl IntoResponse {
    // Create connection span
    let span = create_ws_connection_span(&request);

    // Record successful connection
    record_ws_connection_success(&span);

    ws.on_upgrade(|socket| handle_socket_with_tracing(socket, span))
}

async fn handle_socket_with_tracing(mut socket: WebSocket, connection_span: tracing::Span) {
    let _guard = connection_span.enter();

    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                let message_span = create_ws_message_span(text.len(), "in");
                let _msg_guard = message_span.enter();

                // Process message...

                record_ws_message_success(&message_span, text.len());
            }
            // ... other message types
        }
    }
}
```

### Replay File Format

Replay files use JSON Lines format with the following structure:

```json
{"ts":0,"dir":"out","text":"HELLO {{uuid}}","waitFor":"^CLIENT_READY$"}
{"ts":10,"dir":"out","text":"{\"type\":\"welcome\",\"sessionId\":\"{{uuid}}\"}"}
{"ts":20,"dir":"out","text":"{\"data\":{{randInt 1 100}}}","waitFor":"^ACK$"}
```

**Fields:**
- `ts`: Timestamp offset in milliseconds
- `dir`: Direction ("in" for received, "out" for sent)
- `text`: Message content (supports template expansion)
- `waitFor`: Optional regex pattern to wait for before sending

### Environment Variables

- `MOCKFORGE_WS_REPLAY_FILE`: Path to replay file for replay mode
- `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND`: Enable template expansion ("1" or "true")

### Integration with MockForge Core

The WebSocket crate integrates with core MockForge functionality:

- **Latency Injection**: Uses `LatencyInjector` for network simulation
- **Proxy Handler**: Uses `WsProxyHandler` for upstream forwarding
- **Metrics**: Integrates with global metrics registry
- **Tracing**: Uses OpenTelemetry for distributed tracing
- **Data Generation**: Supports AI-powered content generation