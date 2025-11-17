//! # MockForge WebSocket
//!
//! WebSocket mocking library for MockForge with replay, proxy, and AI-powered event generation.
//!
//! This crate provides comprehensive WebSocket mocking capabilities, including:
//!
//! - **Replay Mode**: Script and replay WebSocket message sequences
//! - **Interactive Mode**: Dynamic responses based on client messages
//! - **AI Event Streams**: Generate narrative-driven event sequences
//! - **Proxy Mode**: Forward messages to real WebSocket backends
//! - **JSONPath Matching**: Sophisticated message matching with JSONPath queries
//!
//! ## Overview
//!
//! MockForge WebSocket supports multiple operational modes:
//!
//! ### 1. Replay Mode
//! Play back pre-recorded WebSocket interactions from JSONL files with template expansion.
//!
//! ### 2. Proxy Mode
//! Forward WebSocket messages to upstream servers with optional message transformation.
//!
//! ### 3. AI Event Generation
//! Generate realistic event streams using LLMs based on narrative descriptions.
//!
//! ## Quick Start
//!
//! ### Basic WebSocket Server
//!
//! ```rust,no_run
//! use mockforge_ws::router;
//! use std::net::SocketAddr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create WebSocket router
//!     let app = router();
//!
//!     // Start server
//!     let addr: SocketAddr = "0.0.0.0:3001".parse()?;
//!     let listener = tokio::net::TcpListener::bind(addr).await?;
//!     axum::serve(listener, app).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### With Latency Simulation
//!
//! ```rust,no_run
//! use mockforge_ws::router_with_latency;
//! use mockforge_core::latency::{FaultConfig, LatencyInjector};
//! use mockforge_core::LatencyProfile;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let latency = LatencyProfile::with_normal_distribution(250, 75.0)
//!     .with_min_ms(100)
//!     .with_max_ms(500);
//! let injector = LatencyInjector::new(latency, FaultConfig::default());
//! let app = router_with_latency(injector);
//! # Ok(())
//! # }
//! ```
//!
//! ### With Proxy Support
//!
//! ```rust,no_run
//! use mockforge_ws::router_with_proxy;
//! use mockforge_core::{WsProxyHandler, WsProxyConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let proxy_config = WsProxyConfig {
//!     upstream_url: "wss://api.example.com/ws".to_string(),
//!     ..Default::default()
//! };
//! let proxy = WsProxyHandler::new(proxy_config);
//! let app = router_with_proxy(proxy);
//! # Ok(())
//! # }
//! ```
//!
//! ### AI Event Generation
//!
//! Generate realistic event streams from narrative descriptions:
//!
//! ```rust,no_run
//! use mockforge_ws::{AiEventGenerator, WebSocketAiConfig};
//! use mockforge_data::replay_augmentation::{scenarios, ReplayMode};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let ai_config = WebSocketAiConfig {
//!     enabled: true,
//!     replay: Some(scenarios::stock_market_scenario()),
//!     max_events: Some(30),
//!     event_rate: Some(1.5),
//! };
//!
//! let generator = AiEventGenerator::new(ai_config.replay.clone().unwrap())?;
//! let _events = generator; // use the generator with `stream_events` in your handler
//! # Ok(())
//! # }
//! ```
//!
//! ## Replay File Format
//!
//! WebSocket replay files use JSON Lines (JSONL) format:
//!
//! ```json
//! {"ts":0,"dir":"out","text":"HELLO {{uuid}}","waitFor":"^CLIENT_READY$"}
//! {"ts":10,"dir":"out","text":"{\"type\":\"welcome\",\"sessionId\":\"{{uuid}}\"}"}
//! {"ts":20,"dir":"out","text":"{\"data\":{{randInt 1 100}}}","waitFor":"^ACK$"}
//! ```
//!
//! Fields:
//! - `ts`: Timestamp in milliseconds
//! - `dir`: Direction ("in" = received, "out" = sent)
//! - `text`: Message content (supports template expansion)
//! - `waitFor`: Optional regex/JSONPath pattern to wait for
//!
//! ## JSONPath Message Matching
//!
//! Match messages using JSONPath queries:
//!
//! ```json
//! {"waitFor": "$.type", "text": "Type received"}
//! {"waitFor": "$.user.id", "text": "User authenticated"}
//! {"waitFor": "$.order.status", "text": "Order updated"}
//! ```
//!
//! ## Key Modules
//!
//! - [`ai_event_generator`]: AI-powered event stream generation
//! - [`ws_tracing`]: Distributed tracing integration
//!
//! ## Examples
//!
//! See the [examples directory](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples)
//! for complete working examples.
//!
//! ## Related Crates
//!
//! - [`mockforge-core`](https://docs.rs/mockforge-core): Core mocking functionality
//! - [`mockforge-data`](https://docs.rs/mockforge-data): Synthetic data generation
//!
//! ## Documentation
//!
//! - [MockForge Book](https://docs.mockforge.dev/)
//! - [WebSocket Mocking Guide](https://docs.mockforge.dev/user-guide/websocket-mocking.html)
//! - [API Reference](https://docs.rs/mockforge-ws)

pub mod ai_event_generator;
pub mod handlers;
pub mod ws_tracing;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::{response::IntoResponse, routing::get, Router};
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use mockforge_core::{latency::LatencyInjector, LatencyProfile, WsProxyHandler};
#[cfg(feature = "data-faker")]
use mockforge_data::provider::register_core_faker_provider;
use mockforge_observability::get_global_registry;
use serde_json::Value;
use tokio::fs;
use tokio::time::{sleep, Duration};
use tracing::*;

// Re-export AI event generator utilities
pub use ai_event_generator::{AiEventGenerator, WebSocketAiConfig};

// Re-export tracing utilities
pub use ws_tracing::{
    create_ws_connection_span, create_ws_message_span, record_ws_connection_success,
    record_ws_error, record_ws_message_success,
};

// Re-export handler utilities
pub use handlers::{
    HandlerError, HandlerRegistry, HandlerResult, MessagePattern, MessageRouter, PassthroughConfig,
    PassthroughHandler, RoomManager, WsContext, WsHandler, WsMessage,
};

/// Build the WebSocket router (exposed for tests and embedding)
pub fn router() -> Router {
    #[cfg(feature = "data-faker")]
    register_core_faker_provider();

    Router::new().route("/ws", get(ws_handler_no_state))
}

/// Build the WebSocket router with latency injector state
pub fn router_with_latency(latency_injector: LatencyInjector) -> Router {
    #[cfg(feature = "data-faker")]
    register_core_faker_provider();

    Router::new()
        .route("/ws", get(ws_handler_with_state))
        .with_state(latency_injector)
}

/// Build the WebSocket router with proxy handler
pub fn router_with_proxy(proxy_handler: WsProxyHandler) -> Router {
    #[cfg(feature = "data-faker")]
    register_core_faker_provider();

    Router::new()
        .route("/ws", get(ws_handler_with_proxy))
        .route("/ws/{*path}", get(ws_handler_with_proxy_path))
        .with_state(proxy_handler)
}

/// Build the WebSocket router with handler registry
pub fn router_with_handlers(registry: std::sync::Arc<HandlerRegistry>) -> Router {
    #[cfg(feature = "data-faker")]
    register_core_faker_provider();

    Router::new()
        .route("/ws", get(ws_handler_with_registry))
        .route("/ws/{*path}", get(ws_handler_with_registry_path))
        .with_state(registry)
}

/// Start WebSocket server with latency simulation
pub async fn start_with_latency(
    port: u16,
    latency: Option<LatencyProfile>,
) -> Result<(), Box<dyn std::error::Error>> {
    start_with_latency_and_host(port, "0.0.0.0", latency).await
}

/// Start WebSocket server with latency simulation and custom host
pub async fn start_with_latency_and_host(
    port: u16,
    host: &str,
    latency: Option<LatencyProfile>,
) -> Result<(), Box<dyn std::error::Error>> {
    let latency_injector = latency.map(|profile| LatencyInjector::new(profile, Default::default()));
    let router = if let Some(injector) = latency_injector {
        router_with_latency(injector)
    } else {
        router()
    };

    let addr: std::net::SocketAddr = format!("{}:{}", host, port).parse()?;
    info!("WebSocket server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        format!(
            "Failed to bind WebSocket server to port {}: {}\n\
             Hint: The port may already be in use. Try using a different port with --ws-port or check if another process is using this port with: lsof -i :{} or netstat -tulpn | grep {}",
            port, e, port, port
        )
    })?;

    axum::serve(listener, router).await?;
    Ok(())
}

// WebSocket handlers
async fn ws_handler_no_state(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn ws_handler_with_state(
    ws: WebSocketUpgrade,
    axum::extract::State(_latency): axum::extract::State<LatencyInjector>,
) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn ws_handler_with_proxy(
    ws: WebSocketUpgrade,
    State(proxy): State<WsProxyHandler>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket_with_proxy(socket, proxy, "/ws".to_string()))
}

async fn ws_handler_with_proxy_path(
    Path(path): Path<String>,
    ws: WebSocketUpgrade,
    State(proxy): State<WsProxyHandler>,
) -> impl IntoResponse {
    let full_path = format!("/ws/{}", path);
    ws.on_upgrade(move |socket| handle_socket_with_proxy(socket, proxy, full_path))
}

async fn ws_handler_with_registry(
    ws: WebSocketUpgrade,
    State(registry): State<std::sync::Arc<HandlerRegistry>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket_with_handlers(socket, registry, "/ws".to_string()))
}

async fn ws_handler_with_registry_path(
    Path(path): Path<String>,
    ws: WebSocketUpgrade,
    State(registry): State<std::sync::Arc<HandlerRegistry>>,
) -> impl IntoResponse {
    let full_path = format!("/ws/{}", path);
    ws.on_upgrade(move |socket| handle_socket_with_handlers(socket, registry, full_path))
}

async fn handle_socket(mut socket: WebSocket) {
    use std::time::Instant;

    // Track WebSocket connection
    let registry = get_global_registry();
    let connection_start = Instant::now();
    registry.record_ws_connection_established();
    debug!("WebSocket connection established, tracking metrics");

    // Track connection status (for metrics reporting)
    let mut status = "normal";

    // Check if replay mode is enabled
    if let Ok(replay_file) = std::env::var("MOCKFORGE_WS_REPLAY_FILE") {
        info!("WebSocket replay mode enabled with file: {}", replay_file);
        handle_socket_with_replay(socket, &replay_file).await;
    } else {
        // Normal echo mode
        while let Some(msg) = socket.recv().await {
            match msg {
                Ok(Message::Text(text)) => {
                    registry.record_ws_message_received();

                    // Echo the message back with "echo: " prefix
                    let response = format!("echo: {}", text);
                    if socket.send(Message::Text(response.into())).await.is_err() {
                        status = "send_error";
                        break;
                    }
                    registry.record_ws_message_sent();
                }
                Ok(Message::Close(_)) => {
                    status = "client_close";
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    registry.record_ws_error();
                    status = "error";
                    break;
                }
                _ => {}
            }
        }
    }

    // Connection closed - record duration
    let duration = connection_start.elapsed().as_secs_f64();
    registry.record_ws_connection_closed(duration, status);
    debug!("WebSocket connection closed (status: {}, duration: {:.2}s)", status, duration);
}

async fn handle_socket_with_replay(mut socket: WebSocket, replay_file: &str) {
    let _registry = get_global_registry(); // Available for future message tracking

    // Read the replay file
    let content = match fs::read_to_string(replay_file).await {
        Ok(content) => content,
        Err(e) => {
            error!("Failed to read replay file {}: {}", replay_file, e);
            return;
        }
    };

    // Parse JSONL file
    let mut replay_entries = Vec::new();
    for line in content.lines() {
        if let Ok(entry) = serde_json::from_str::<Value>(line) {
            replay_entries.push(entry);
        }
    }

    info!("Loaded {} replay entries", replay_entries.len());

    // Process replay entries
    for entry in replay_entries {
        // Check if we need to wait for a specific message
        if let Some(wait_for) = entry.get("waitFor") {
            if let Some(wait_pattern) = wait_for.as_str() {
                info!("Waiting for pattern: {}", wait_pattern);
                // Wait for matching message from client
                let mut found = false;
                while let Some(msg) = socket.recv().await {
                    if let Ok(Message::Text(text)) = msg {
                        if text.contains(wait_pattern) || wait_pattern == "^CLIENT_READY$" {
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    break;
                }
            }
        }

        // Get the message text
        if let Some(text) = entry.get("text").and_then(|v| v.as_str()) {
            // Expand tokens if enabled
            let expanded_text = if std::env::var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false)
            {
                expand_tokens(text)
            } else {
                text.to_string()
            };

            info!("Sending replay message: {}", expanded_text);
            if socket.send(Message::Text(expanded_text.into())).await.is_err() {
                break;
            }
        }

        // Wait for the specified time
        if let Some(ts) = entry.get("ts").and_then(|v| v.as_u64()) {
            sleep(Duration::from_millis(ts * 10)).await; // Convert to milliseconds
        }
    }
}

fn expand_tokens(text: &str) -> String {
    let mut result = text.to_string();

    // Expand {{uuid}}
    result = result.replace("{{uuid}}", &uuid::Uuid::new_v4().to_string());

    // Expand {{now}}
    result = result.replace("{{now}}", &chrono::Utc::now().to_rfc3339());

    // Expand {{now+1m}} (add 1 minute)
    if result.contains("{{now+1m}}") {
        let now_plus_1m = chrono::Utc::now() + chrono::Duration::minutes(1);
        result = result.replace("{{now+1m}}", &now_plus_1m.to_rfc3339());
    }

    // Expand {{now+1h}} (add 1 hour)
    if result.contains("{{now+1h}}") {
        let now_plus_1h = chrono::Utc::now() + chrono::Duration::hours(1);
        result = result.replace("{{now+1h}}", &now_plus_1h.to_rfc3339());
    }

    // Expand {{randInt min max}}
    while result.contains("{{randInt") {
        if let Some(start) = result.find("{{randInt") {
            if let Some(end) = result[start..].find("}}") {
                let full_match = &result[start..start + end + 2];
                let content = &result[start + 9..start + end]; // Skip "{{randInt"

                if let Some(space_pos) = content.find(' ') {
                    let min_str = &content[..space_pos];
                    let max_str = &content[space_pos + 1..];

                    if let (Ok(min), Ok(max)) = (min_str.parse::<i32>(), max_str.parse::<i32>()) {
                        let random_value = fastrand::i32(min..=max);
                        result = result.replace(full_match, &random_value.to_string());
                    } else {
                        result = result.replace(full_match, "0");
                    }
                } else {
                    result = result.replace(full_match, "0");
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    result
}

async fn handle_socket_with_proxy(socket: WebSocket, proxy: WsProxyHandler, path: String) {
    use std::time::Instant;

    let registry = get_global_registry();
    let connection_start = Instant::now();
    registry.record_ws_connection_established();

    let mut status = "normal";

    // Check if this connection should be proxied
    if proxy.config.should_proxy(&path) {
        info!("Proxying WebSocket connection for path: {}", path);
        if let Err(e) = proxy.proxy_connection(&path, socket).await {
            error!("Failed to proxy WebSocket connection: {}", e);
            registry.record_ws_error();
            status = "proxy_error";
        }
    } else {
        info!("Handling WebSocket connection locally for path: {}", path);
        // Handle locally by echoing messages
        // Note: handle_socket already tracks its own connection metrics,
        // so we need to avoid double-counting
        registry.record_ws_connection_closed(0.0, ""); // Decrement the one we just added
        handle_socket(socket).await;
        return; // Early return to avoid double-tracking
    }

    let duration = connection_start.elapsed().as_secs_f64();
    registry.record_ws_connection_closed(duration, status);
    debug!(
        "Proxied WebSocket connection closed (status: {}, duration: {:.2}s)",
        status, duration
    );
}

async fn handle_socket_with_handlers(
    socket: WebSocket,
    registry: std::sync::Arc<HandlerRegistry>,
    path: String,
) {
    use std::time::Instant;

    let metrics_registry = get_global_registry();
    let connection_start = Instant::now();
    metrics_registry.record_ws_connection_established();

    let mut status = "normal";

    // Generate unique connection ID
    let connection_id = uuid::Uuid::new_v4().to_string();

    // Get handlers for this path
    let handlers = registry.get_handlers(&path);
    if handlers.is_empty() {
        info!("No handlers found for path: {}, falling back to echo mode", path);
        metrics_registry.record_ws_connection_closed(0.0, "");
        handle_socket(socket).await;
        return;
    }

    info!(
        "Handling WebSocket connection with {} handler(s) for path: {}",
        handlers.len(),
        path
    );

    // Create room manager
    let room_manager = RoomManager::new();

    // Split socket for concurrent send/receive
    let (mut socket_sender, mut socket_receiver) = socket.split();

    // Create message channel for handlers to send messages
    let (message_tx, mut message_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    // Create context
    let mut ctx =
        WsContext::new(connection_id.clone(), path.clone(), room_manager.clone(), message_tx);

    // Call on_connect for all handlers
    for handler in &handlers {
        if let Err(e) = handler.on_connect(&mut ctx).await {
            error!("Handler on_connect error: {}", e);
            status = "handler_error";
        }
    }

    // Spawn task to send messages from handlers to the socket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = message_rx.recv().await {
            if socket_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(msg) = socket_receiver.next().await {
        match msg {
            Ok(axum_msg) => {
                metrics_registry.record_ws_message_received();

                let ws_msg: WsMessage = axum_msg.into();

                // Check for close message
                if matches!(ws_msg, WsMessage::Close) {
                    status = "client_close";
                    break;
                }

                // Pass message through all handlers
                for handler in &handlers {
                    if let Err(e) = handler.on_message(&mut ctx, ws_msg.clone()).await {
                        error!("Handler on_message error: {}", e);
                        status = "handler_error";
                    }
                }

                metrics_registry.record_ws_message_sent();
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                metrics_registry.record_ws_error();
                status = "error";
                break;
            }
        }
    }

    // Call on_disconnect for all handlers
    for handler in &handlers {
        if let Err(e) = handler.on_disconnect(&mut ctx).await {
            error!("Handler on_disconnect error: {}", e);
        }
    }

    // Clean up room memberships
    let _ = room_manager.leave_all(&connection_id).await;

    // Abort send task
    send_task.abort();

    let duration = connection_start.elapsed().as_secs_f64();
    metrics_registry.record_ws_connection_closed(duration, status);
    debug!(
        "Handler-based WebSocket connection closed (status: {}, duration: {:.2}s)",
        status, duration
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let _router = router();
        // Router should be created successfully
    }

    #[test]
    fn test_router_with_latency_creation() {
        let latency_profile = LatencyProfile::default();
        let latency_injector = LatencyInjector::new(latency_profile, Default::default());
        let _router = router_with_latency(latency_injector);
        // Router should be created successfully
    }

    #[test]
    fn test_router_with_proxy_creation() {
        let config = mockforge_core::WsProxyConfig {
            upstream_url: "ws://localhost:8080".to_string(),
            ..Default::default()
        };
        let proxy_handler = WsProxyHandler::new(config);
        let _router = router_with_proxy(proxy_handler);
        // Router should be created successfully
    }

    #[tokio::test]
    async fn test_start_with_latency_config_none() {
        // Test that we can create the router without latency
        let result = std::panic::catch_unwind(|| {
            let _router = router();
        });
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_with_latency_config_some() {
        // Test that we can create the router with latency
        let latency_profile = LatencyProfile::default();
        let latency_injector = LatencyInjector::new(latency_profile, Default::default());

        let result = std::panic::catch_unwind(|| {
            let _router = router_with_latency(latency_injector);
        });
        assert!(result.is_ok());
    }
}
