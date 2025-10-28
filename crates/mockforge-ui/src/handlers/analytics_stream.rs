//! Real-time analytics streaming via WebSocket
//!
//! Provides live updates of analytics metrics to connected clients

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use mockforge_analytics::AnalyticsDatabase;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// WebSocket analytics state
#[derive(Clone)]
pub struct AnalyticsStreamState {
    pub db: Arc<AnalyticsDatabase>,
}

impl AnalyticsStreamState {
    pub fn new(db: AnalyticsDatabase) -> Self {
        Self { db: Arc::new(db) }
    }
}

/// Client subscription configuration
#[derive(Debug, Clone, Deserialize)]
pub struct StreamConfig {
    /// Update interval in seconds (default: 5)
    #[serde(default = "default_interval")]
    pub interval_seconds: u64,
    /// Metrics duration window (default: 3600)
    #[serde(default = "default_duration")]
    pub duration_seconds: i64,
    /// Protocol filter
    pub protocol: Option<String>,
    /// Endpoint filter
    pub endpoint: Option<String>,
    /// Workspace ID filter
    pub workspace_id: Option<String>,
}

fn default_interval() -> u64 {
    5
}

fn default_duration() -> i64 {
    3600
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            interval_seconds: default_interval(),
            duration_seconds: default_duration(),
            protocol: None,
            endpoint: None,
            workspace_id: None,
        }
    }
}

/// Streamed metrics update
#[derive(Debug, Serialize)]
pub struct MetricsUpdate {
    pub timestamp: i64,
    pub total_requests: i64,
    pub total_errors: i64,
    pub error_rate: f64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub active_connections: i64,
    pub requests_per_second: f64,
}

/// WebSocket handler for analytics streaming
pub async fn analytics_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AnalyticsStreamState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_analytics_socket(socket, state))
}

async fn handle_analytics_socket(socket: WebSocket, state: AnalyticsStreamState) {
    let (mut sender, mut receiver) = socket.split();

    info!("Analytics WebSocket client connected");

    // Default configuration
    let mut config = StreamConfig::default();

    // Spawn a task to handle incoming messages (config updates, ping/pong)
    let config_clone = Arc::new(tokio::sync::Mutex::new(config.clone()));
    let config_update_handle = {
        let config_clone = Arc::clone(&config_clone);
        tokio::spawn(async move {
            while let Some(msg) = receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        // Try to parse as config update
                        if let Ok(new_config) = serde_json::from_str::<StreamConfig>(&text) {
                            debug!("Received config update: {:?}", new_config);
                            let mut cfg = config_clone.lock().await;
                            *cfg = new_config;
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        debug!("Received ping");
                        // Pong is handled automatically by axum
                    }
                    Ok(Message::Close(_)) => {
                        info!("Client requested close");
                        break;
                    }
                    Err(e) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        })
    };

    // Spawn a task to send periodic updates
    let update_task = tokio::spawn(async move {
        // Wait for initial config
        tokio::time::sleep(Duration::from_millis(100)).await;

        loop {
            // Get current config
            let current_config = {
                let cfg = config_clone.lock().await;
                cfg.clone()
            };

            // Create interval based on config
            let mut tick_interval = interval(Duration::from_secs(current_config.interval_seconds));
            tick_interval.tick().await; // First tick completes immediately

            // Fetch and send metrics
            match state.db.get_overview_metrics(current_config.duration_seconds).await {
                Ok(overview) => {
                    let update = MetricsUpdate {
                        timestamp: chrono::Utc::now().timestamp(),
                        total_requests: overview.total_requests,
                        total_errors: overview.total_errors,
                        error_rate: overview.error_rate,
                        avg_latency_ms: overview.avg_latency_ms,
                        p95_latency_ms: overview.p95_latency_ms,
                        p99_latency_ms: overview.p99_latency_ms,
                        active_connections: overview.active_connections,
                        requests_per_second: overview.requests_per_second,
                    };

                    if let Ok(json) = serde_json::to_string(&update) {
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            error!("Failed to send update to client");
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to get overview metrics: {}", e);
                    // Continue trying
                }
            }

            // Wait for next interval
            tick_interval.tick().await;
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = config_update_handle => {
            debug!("Config update handler completed");
        }
        _ = update_task => {
            debug!("Update task completed");
        }
    }

    info!("Analytics WebSocket client disconnected");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_stream_config() {
        let config = StreamConfig::default();
        assert_eq!(config.interval_seconds, 5);
        assert_eq!(config.duration_seconds, 3600);
    }

    #[test]
    fn test_stream_config_parsing() {
        let json = r#"{
            "interval_seconds": 10,
            "duration_seconds": 7200,
            "protocol": "HTTP"
        }"#;

        let config: StreamConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.interval_seconds, 10);
        assert_eq!(config.duration_seconds, 7200);
        assert_eq!(config.protocol, Some("HTTP".to_string()));
    }
}
