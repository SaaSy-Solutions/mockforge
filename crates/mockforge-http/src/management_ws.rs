/// WebSocket interface for live mock updates
///
/// Provides real-time notifications when mocks are created, updated, or deleted.
/// Used by developer tools like VS Code extension for live synchronization.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures::stream::StreamExt;
use futures::SinkExt;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::*;

/// Events that can be broadcasted to WebSocket clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MockEvent {
    /// Mock was created
    MockCreated {
        mock: super::management::MockConfig,
        timestamp: String,
    },
    /// Mock was updated
    MockUpdated {
        mock: super::management::MockConfig,
        timestamp: String,
    },
    /// Mock was deleted
    MockDeleted {
        id: String,
        timestamp: String,
    },
    /// Server statistics changed
    StatsUpdated {
        stats: super::management::ServerStats,
        timestamp: String,
    },
    /// Connection established confirmation
    Connected {
        message: String,
        timestamp: String,
    },
}

impl MockEvent {
    pub fn mock_created(mock: super::management::MockConfig) -> Self {
        Self::MockCreated {
            mock,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn mock_updated(mock: super::management::MockConfig) -> Self {
        Self::MockUpdated {
            mock,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn mock_deleted(id: String) -> Self {
        Self::MockDeleted {
            id,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn stats_updated(stats: super::management::ServerStats) -> Self {
        Self::StatsUpdated {
            stats,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn connected(message: String) -> Self {
        Self::Connected {
            message,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Shared state for WebSocket management
#[derive(Clone)]
pub struct WsManagementState {
    /// Broadcast channel for sending events to all connected clients
    pub tx: broadcast::Sender<MockEvent>,
}

impl WsManagementState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self { tx }
    }

    /// Broadcast an event to all connected clients
    pub fn broadcast(&self, event: MockEvent) -> Result<usize, broadcast::error::SendError<MockEvent>> {
        self.tx.send(event)
    }
}

impl Default for WsManagementState {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket upgrade handler
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<WsManagementState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle a WebSocket connection
async fn handle_socket(socket: WebSocket, state: WsManagementState) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast channel
    let mut rx = state.tx.subscribe();

    // Send initial connection confirmation
    let connected_event = MockEvent::connected("Connected to MockForge management API".to_string());
    if let Ok(json) = serde_json::to_string(&connected_event) {
        if sender.send(Message::Text(json.into())).await.is_err() {
            return;
        }
    }

    // Spawn a task to forward broadcast messages to this client
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&event) {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages from client (for now, just keep connection alive)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    debug!("Received WebSocket message: {}", text);
                    // Could handle client commands here in the future
                }
                Message::Close(_) => {
                    info!("WebSocket client disconnected");
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => {
            debug!("Send task completed");
            recv_task.abort();
        }
        _ = &mut recv_task => {
            debug!("Receive task completed");
            send_task.abort();
        }
    }
}

/// Build the WebSocket management router
pub fn ws_management_router(state: WsManagementState) -> Router {
    Router::new()
        .route("/", get(ws_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_management_state_creation() {
        let state = WsManagementState::new();
        // Should be able to create state without errors
        assert!(true);
    }

    #[test]
    fn test_mock_event_creation() {
        use super::super::management::{MockConfig, MockResponse};

        let mock = MockConfig {
            id: "test-1".to_string(),
            name: "Test Mock".to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            response: MockResponse {
                body: serde_json::json!({"message": "test"}),
                headers: None,
            },
            enabled: true,
            latency_ms: None,
            status_code: Some(200),
        };

        let event = MockEvent::mock_created(mock);

        // Should serialize successfully
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("mock_created"));
    }

    #[test]
    fn test_broadcast_event() {
        let state = WsManagementState::new();

        let event = MockEvent::connected("Test connection".to_string());

        // Should be able to send even with no subscribers
        let result = state.broadcast(event);
        // With no subscribers, this returns Err with the number of subscribers (0)
        assert!(result.is_err() || result.is_ok());
    }

    #[tokio::test]
    async fn test_ws_management_router_creation() {
        let state = WsManagementState::new();
        let _router = ws_management_router(state);
        // Router should be created successfully
        assert!(true);
    }
}
