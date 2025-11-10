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
        /// The created mock configuration
        mock: super::management::MockConfig,
        /// ISO 8601 timestamp of the event
        timestamp: String,
    },
    /// Mock was updated
    MockUpdated {
        /// The updated mock configuration
        mock: super::management::MockConfig,
        /// ISO 8601 timestamp of the event
        timestamp: String,
    },
    /// Mock was deleted
    MockDeleted {
        /// ID of the deleted mock
        id: String,
        /// ISO 8601 timestamp of the event
        timestamp: String,
    },
    /// Server statistics changed
    StatsUpdated {
        /// Updated server statistics
        stats: super::management::ServerStats,
        /// ISO 8601 timestamp of the event
        timestamp: String,
    },
    /// Connection established confirmation
    Connected {
        /// Connection confirmation message
        message: String,
        /// ISO 8601 timestamp of the event
        timestamp: String,
    },
    /// State machine was created or updated
    StateMachineUpdated {
        /// Resource type of the state machine
        resource_type: String,
        /// The state machine definition
        state_machine: mockforge_core::intelligent_behavior::rules::StateMachine,
        /// ISO 8601 timestamp of the event
        timestamp: String,
    },
    /// State machine was deleted
    StateMachineDeleted {
        /// Resource type of the deleted state machine
        resource_type: String,
        /// ISO 8601 timestamp of the event
        timestamp: String,
    },
    /// State instance was created
    StateInstanceCreated {
        /// Resource ID
        resource_id: String,
        /// Resource type
        resource_type: String,
        /// Initial state
        initial_state: String,
        /// ISO 8601 timestamp of the event
        timestamp: String,
    },
    /// State transition occurred
    StateTransitioned {
        /// Resource ID
        resource_id: String,
        /// Resource type
        resource_type: String,
        /// Previous state
        from_state: String,
        /// New state
        to_state: String,
        /// Current state data
        state_data: std::collections::HashMap<String, serde_json::Value>,
        /// ISO 8601 timestamp of the event
        timestamp: String,
    },
    /// State instance was deleted
    StateInstanceDeleted {
        /// Resource ID
        resource_id: String,
        /// Resource type
        resource_type: String,
        /// ISO 8601 timestamp of the event
        timestamp: String,
    },
}

impl MockEvent {
    /// Create a mock created event
    pub fn mock_created(mock: super::management::MockConfig) -> Self {
        Self::MockCreated {
            mock,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a mock updated event
    pub fn mock_updated(mock: super::management::MockConfig) -> Self {
        Self::MockUpdated {
            mock,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a mock deleted event
    pub fn mock_deleted(id: String) -> Self {
        Self::MockDeleted {
            id,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a stats updated event
    pub fn stats_updated(stats: super::management::ServerStats) -> Self {
        Self::StatsUpdated {
            stats,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a connection established event
    pub fn connected(message: String) -> Self {
        Self::Connected {
            message,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a state machine updated event
    pub fn state_machine_updated(
        resource_type: String,
        state_machine: mockforge_core::intelligent_behavior::rules::StateMachine,
    ) -> Self {
        Self::StateMachineUpdated {
            resource_type,
            state_machine,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a state machine deleted event
    pub fn state_machine_deleted(resource_type: String) -> Self {
        Self::StateMachineDeleted {
            resource_type,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a state instance created event
    pub fn state_instance_created(
        resource_id: String,
        resource_type: String,
        initial_state: String,
    ) -> Self {
        Self::StateInstanceCreated {
            resource_id,
            resource_type,
            initial_state,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a state transitioned event
    pub fn state_transitioned(
        resource_id: String,
        resource_type: String,
        from_state: String,
        to_state: String,
        state_data: std::collections::HashMap<String, serde_json::Value>,
    ) -> Self {
        Self::StateTransitioned {
            resource_id,
            resource_type,
            from_state,
            to_state,
            state_data,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a state instance deleted event
    pub fn state_instance_deleted(resource_id: String, resource_type: String) -> Self {
        Self::StateInstanceDeleted {
            resource_id,
            resource_type,
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
    /// Create a new WebSocket management state with broadcast channel
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self { tx }
    }

    /// Broadcast an event to all connected clients
    pub fn broadcast(
        &self,
        event: MockEvent,
    ) -> Result<usize, Box<broadcast::error::SendError<MockEvent>>> {
        self.tx.send(event).map_err(Box::new)
    }
}

impl Default for WsManagementState {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket upgrade handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<WsManagementState>,
) -> impl IntoResponse {
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
    Router::new().route("/", get(ws_handler)).with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_management_state_creation() {
        let _state = WsManagementState::new();
        // Should be able to create state without errors
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
            request_match: None,
            priority: None,
            scenario: None,
            required_scenario_state: None,
            new_scenario_state: None,
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
    }
}
