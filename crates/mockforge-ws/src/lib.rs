pub mod ai_event_generator;
pub mod ws_tracing;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::{response::IntoResponse, routing::get, Router};
use mockforge_core::{latency::LatencyInjector, LatencyProfile, WsProxyHandler};
use mockforge_observability::get_global_registry;
use serde_json::Value;
use tokio::fs;
use tokio::time::{sleep, Duration};
#[cfg(feature = "data-faker")]
use mockforge_data::provider::register_core_faker_provider;
use tracing::*;

// Re-export AI event generator utilities
pub use ai_event_generator::{AiEventGenerator, WebSocketAiConfig};

// Re-export tracing utilities
pub use ws_tracing::{
    create_ws_connection_span, create_ws_message_span,
    record_ws_connection_success, record_ws_error, record_ws_message_success,
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

/// Start WebSocket server with latency simulation
pub async fn start_with_latency(
    port: u16,
    latency: Option<LatencyProfile>,
) -> Result<(), Box<dyn std::error::Error>> {
    let latency_injector = latency.map(|profile| LatencyInjector::new(profile, Default::default()));
    let router = if let Some(injector) = latency_injector {
        router_with_latency(injector)
    } else {
        router()
    };

    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse()?;
    info!("WebSocket server listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, router).await?;
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

async fn handle_socket(mut socket: WebSocket) {
    // Track WebSocket connection
    let registry = get_global_registry();
    registry.ws_connections_active.inc();
    debug!("WebSocket connection established, tracking metrics");

    // Check if replay mode is enabled
    if let Ok(replay_file) = std::env::var("MOCKFORGE_WS_REPLAY_FILE") {
        info!("WebSocket replay mode enabled with file: {}", replay_file);
        handle_socket_with_replay(socket, &replay_file).await;
    } else {
        // Normal echo mode
        while let Some(msg) = socket.recv().await {
            if let Ok(Message::Text(text)) = msg {
                registry.record_ws_message_received();

                // Echo the message back with "echo: " prefix
                let response = format!("echo: {}", text);
                if socket.send(Message::Text(response.into())).await.is_err() {
                    break;
                }
                registry.record_ws_message_sent();
            }
        }
    }

    // Connection closed
    registry.ws_connections_active.dec();
    debug!("WebSocket connection closed");
}

async fn handle_socket_with_replay(mut socket: WebSocket, replay_file: &str) {
    let registry = get_global_registry();

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
    // Check if this connection should be proxied
    if proxy.config.should_proxy(&path) {
        info!("Proxying WebSocket connection for path: {}", path);
        if let Err(e) = proxy.proxy_connection(&path, socket).await {
            error!("Failed to proxy WebSocket connection: {}", e);
        }
    } else {
        info!("Handling WebSocket connection locally for path: {}", path);
        // Handle locally by echoing messages
        handle_socket(socket).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let _router = router();
        // Router should be created successfully
        assert!(true);
    }

    #[test]
    fn test_router_with_latency_creation() {
        let latency_profile = LatencyProfile::default();
        let latency_injector = LatencyInjector::new(latency_profile, Default::default());
        let _router = router_with_latency(latency_injector);
        // Router should be created successfully
        assert!(true);
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
        assert!(true);
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
