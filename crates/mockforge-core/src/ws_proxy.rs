//! WebSocket proxy functionality for tunneling connections to upstream services

use crate::{Error, Result};
use axum::extract::ws::{Message as AxumMessage, WebSocket};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use tracing::*;

/// WebSocket proxy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsProxyRule {
    /// Path pattern (supports wildcards)
    pub pattern: String,
    /// Upstream WebSocket URL for this path
    pub upstream_url: String,
    /// Whether this rule is enabled
    pub enabled: bool,
}

/// WebSocket proxy configuration
/// Environment variables:
/// - MOCKFORGE_WS_PROXY_UPSTREAM_URL: Default upstream WebSocket URL for proxy (default: ws://localhost:8080)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsProxyConfig {
    /// Default upstream WebSocket URL
    pub upstream_url: String,
    /// Whether to enable proxy mode
    pub enabled: bool,
    /// Per-path proxy rules
    #[serde(default)]
    pub rules: Vec<WsProxyRule>,
    /// Passthrough by default unless an override applies
    #[serde(default = "default_passthrough")]
    pub passthrough_by_default: bool,
}

fn default_passthrough() -> bool {
    true
}

impl Default for WsProxyConfig {
    fn default() -> Self {
        Self {
            upstream_url: std::env::var("MOCKFORGE_WS_PROXY_UPSTREAM_URL")
                .unwrap_or_else(|_| "ws://localhost:8080".to_string()),
            enabled: false,
            rules: Vec::new(),
            passthrough_by_default: true,
        }
    }
}

impl WsProxyConfig {
    /// Create a new WebSocket proxy configuration
    pub fn new(upstream_url: String) -> Self {
        Self {
            upstream_url,
            ..Default::default()
        }
    }

    /// Check if a WebSocket connection should be proxied
    pub fn should_proxy(&self, path: &str) -> bool {
        if !self.enabled {
            return false;
        }

        // Check per-path rules first
        for rule in &self.rules {
            if rule.enabled && self.matches_path(&rule.pattern, path) {
                return true;
            }
        }

        // If no specific rule matches, use passthrough behavior
        self.passthrough_by_default
    }

    /// Get the upstream URL for a specific path
    pub fn get_upstream_url(&self, path: &str) -> String {
        // Check per-path rules first
        for rule in &self.rules {
            if rule.enabled && self.matches_path(&rule.pattern, path) {
                return rule.upstream_url.clone();
            }
        }

        // Return default upstream URL
        self.upstream_url.clone()
    }

    /// Check if a path matches a pattern
    fn matches_path(&self, pattern: &str, path: &str) -> bool {
        if pattern == path {
            return true;
        }

        // Simple wildcard matching (* matches any segment)
        if pattern.contains('*') {
            let pattern_parts: Vec<&str> = pattern.split('/').collect();
            let path_parts: Vec<&str> = path.split('/').collect();

            if pattern_parts.len() != path_parts.len() {
                return false;
            }

            for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
                if *pattern_part != "*" && *pattern_part != *path_part {
                    return false;
                }
            }
            return true;
        }

        false
    }
}

/// Convert Axum WebSocket message to Tungstenite message
fn axum_to_tungstenite(msg: AxumMessage) -> TungsteniteMessage {
    match msg {
        AxumMessage::Text(text) => TungsteniteMessage::Text(text.to_string().into()),
        AxumMessage::Binary(data) => TungsteniteMessage::Binary(data),
        AxumMessage::Ping(data) => TungsteniteMessage::Ping(data),
        AxumMessage::Pong(data) => TungsteniteMessage::Pong(data),
        AxumMessage::Close(frame) => TungsteniteMessage::Close(frame.map(|f| {
            tokio_tungstenite::tungstenite::protocol::CloseFrame {
                code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::from(
                    f.code,
                ),
                reason: f.reason.to_string().into(),
            }
        })),
    }
}

/// Convert Tungstenite WebSocket message to Axum message
fn tungstenite_to_axum(msg: TungsteniteMessage) -> AxumMessage {
    match msg {
        TungsteniteMessage::Text(text) => AxumMessage::Text(text.to_string().into()),
        TungsteniteMessage::Binary(data) => AxumMessage::Binary(data),
        TungsteniteMessage::Ping(data) => AxumMessage::Ping(data),
        TungsteniteMessage::Pong(data) => AxumMessage::Pong(data),
        TungsteniteMessage::Close(frame) => {
            AxumMessage::Close(frame.map(|f| axum::extract::ws::CloseFrame {
                code: axum::extract::ws::CloseCode::from(u16::from(f.code)),
                reason: f.reason.to_string().into(),
            }))
        }
        TungsteniteMessage::Frame(_) => AxumMessage::Text("".to_string().into()), // Should not happen in normal operation
    }
}

/// WebSocket proxy handler for tunneling connections to upstream
#[derive(Clone)]
pub struct WsProxyHandler {
    pub config: WsProxyConfig,
}

impl WsProxyHandler {
    /// Create a new WebSocket proxy handler
    pub fn new(config: WsProxyConfig) -> Self {
        Self { config }
    }

    /// Proxy a WebSocket connection to the upstream service
    pub async fn proxy_connection(&self, path: &str, client_socket: WebSocket) -> Result<()> {
        if !self.config.should_proxy(path) {
            return Err(Error::generic("WebSocket connection should not be proxied".to_string()));
        }

        // Get the upstream URL for this path
        let upstream_url = self.config.get_upstream_url(path);

        // Connect to upstream WebSocket server
        let (upstream_socket, _) =
            tokio_tungstenite::connect_async(&upstream_url).await.map_err(|e| {
                Error::generic(format!("Failed to connect to upstream WebSocket: {}", e))
            })?;

        info!("Connected to upstream WebSocket at {}", upstream_url);

        // Use a simpler approach without shared mutexes
        let (mut client_sink, mut client_stream) = client_socket.split();
        let (mut upstream_sink, mut upstream_stream) = upstream_socket.split();

        // Forward messages from client to upstream
        let forward_client_to_upstream = tokio::spawn(async move {
            while let Some(msg) = client_stream.next().await {
                match msg {
                    Ok(message) => {
                        let tungstenite_msg = axum_to_tungstenite(message);
                        if let Err(e) = upstream_sink.send(tungstenite_msg).await {
                            error!("Failed to send message to upstream: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Error receiving message from client: {}", e);
                        break;
                    }
                }
            }
        });

        // Forward messages from upstream to client
        let forward_upstream_to_client = tokio::spawn(async move {
            while let Some(msg) = upstream_stream.next().await {
                match msg {
                    Ok(message) => {
                        let axum_msg = tungstenite_to_axum(message);
                        if let Err(e) = client_sink.send(axum_msg).await {
                            error!("Failed to send message to client: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Error receiving message from upstream: {}", e);
                        break;
                    }
                }
            }
        });

        // Wait for either task to complete
        tokio::select! {
            _ = forward_client_to_upstream => {
                info!("Client to upstream forwarding completed");
            }
            _ = forward_upstream_to_client => {
                info!("Upstream to client forwarding completed");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_proxy_config() {
        let mut config = WsProxyConfig::new("ws://default.example.com".to_string());
        config.enabled = true;
        config.rules.push(WsProxyRule {
            pattern: "/ws/users/*".to_string(),
            upstream_url: "ws://users.example.com".to_string(),
            enabled: true,
        });
        config.rules.push(WsProxyRule {
            pattern: "/ws/orders/*".to_string(),
            upstream_url: "ws://orders.example.com".to_string(),
            enabled: true,
        });

        assert!(config.should_proxy("/ws/users/123"));
        assert!(config.should_proxy("/ws/orders/456"));

        assert_eq!(config.get_upstream_url("/ws/users/123"), "ws://users.example.com");
        assert_eq!(config.get_upstream_url("/ws/orders/456"), "ws://orders.example.com");
        assert_eq!(config.get_upstream_url("/ws/products"), "ws://default.example.com");
    }

    #[test]
    fn test_ws_proxy_config_passthrough() {
        let mut config = WsProxyConfig::new("ws://default.example.com".to_string());
        config.passthrough_by_default = true;
        config.enabled = true;

        // With passthrough enabled, all connections should be proxied
        assert!(config.should_proxy("/ws/users"));
        assert!(config.should_proxy("/ws/orders"));

        // Disable passthrough
        config.passthrough_by_default = false;

        // Now only connections with matching rules should be proxied
        assert!(!config.should_proxy("/ws/users"));
        assert!(!config.should_proxy("/ws/orders"));
    }
}
