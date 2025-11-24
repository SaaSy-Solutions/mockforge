//! WebSocket handler for real-time collaboration

use crate::auth::AuthService;
use crate::error::{CollabError, Result};
use crate::events::EventBus;
use crate::sync::{SyncEngine, SyncMessage};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::select;
use uuid::Uuid;

/// WebSocket state
#[derive(Clone)]
pub struct WsState {
    pub auth: Arc<AuthService>,
    pub sync: Arc<SyncEngine>,
    pub event_bus: Arc<EventBus>,
}

/// Handle WebSocket upgrade
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<WsState>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: WsState) {
    let (mut sender, mut receiver) = socket.split();

    // Generate client ID
    let client_id = Uuid::new_v4();
    tracing::info!("WebSocket client connected: {}", client_id);

    // Track subscribed workspaces
    let mut subscriptions: Vec<Uuid> = Vec::new();

    // Subscribe to event bus
    let mut event_rx = state.event_bus.subscribe();

    loop {
        select! {
            // Handle incoming messages from client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = handle_client_message(&text, client_id, &state, &mut subscriptions, &mut sender).await {
                            tracing::error!("Error handling client message: {}", e);
                            let _ = sender.send(Message::Text(
                                serde_json::to_string(&SyncMessage::Error {
                                    message: e.to_string(),
                                }).unwrap().into()
                            )).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::info!("Client {} requested close", client_id);
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = sender.send(Message::Pong(data)).await;
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        tracing::info!("Client {} disconnected", client_id);
                        break;
                    }
                    _ => {}
                }
            }

            // Handle broadcast events
            event = event_rx.recv() => {
                match event {
                    Ok(change_event) => {
                        // Only send events for subscribed workspaces
                        if subscriptions.contains(&change_event.workspace_id) {
                            let msg = SyncMessage::Change { event: change_event };
                            if let Ok(json) = serde_json::to_string(&msg) {
                                let _ = sender.send(Message::Text(json.into())).await;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Client {} lagged {} messages", client_id, n);
                    }
                    Err(_) => {
                        tracing::error!("Event channel closed");
                        break;
                    }
                }
            }
        }
    }

    // Cleanup: unsubscribe from all workspaces
    for workspace_id in subscriptions {
        let _ = state.sync.unsubscribe(workspace_id, client_id);
    }

    tracing::info!("Client {} connection closed", client_id);
}

/// Handle a message from the client
async fn handle_client_message(
    text: &str,
    client_id: Uuid,
    state: &WsState,
    subscriptions: &mut Vec<Uuid>,
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
) -> Result<()> {
    let message: SyncMessage = serde_json::from_str(text)
        .map_err(|e| CollabError::InvalidInput(format!("Invalid JSON: {e}")))?;

    match message {
        SyncMessage::Subscribe { workspace_id } => {
            // TODO: Verify user has access to workspace

            // Subscribe to workspace
            state.sync.subscribe(workspace_id, client_id)?;
            subscriptions.push(workspace_id);

            tracing::info!("Client {} subscribed to workspace {}", client_id, workspace_id);

            // Send current state
            if let Some(sync_state) = state.sync.get_state(workspace_id) {
                let response = SyncMessage::StateResponse {
                    workspace_id,
                    version: sync_state.version,
                    state: sync_state.state,
                };
                let json = serde_json::to_string(&response)?;
                sender
                    .send(Message::Text(json.into()))
                    .await
                    .map_err(|e| CollabError::Internal(format!("Failed to send: {e}")))?;
            }
        }

        SyncMessage::Unsubscribe { workspace_id } => {
            state.sync.unsubscribe(workspace_id, client_id)?;
            subscriptions.retain(|id| *id != workspace_id);

            tracing::info!("Client {} unsubscribed from workspace {}", client_id, workspace_id);
        }

        SyncMessage::StateRequest {
            workspace_id,
            version,
        } => {
            // Check if client needs update
            if let Some(sync_state) = state.sync.get_state(workspace_id) {
                if sync_state.version > version {
                    let response = SyncMessage::StateResponse {
                        workspace_id,
                        version: sync_state.version,
                        state: sync_state.state,
                    };
                    let json = serde_json::to_string(&response)?;
                    sender
                        .send(Message::Text(json.into()))
                        .await
                        .map_err(|e| CollabError::Internal(format!("Failed to send: {e}")))?;
                }
            }
        }

        SyncMessage::Ping => {
            let pong = SyncMessage::Pong;
            let json = serde_json::to_string(&pong)?;
            sender
                .send(Message::Text(json.into()))
                .await
                .map_err(|e| CollabError::Internal(format!("Failed to send: {e}")))?;
        }

        _ => {
            tracing::warn!("Unexpected message type from client {}", client_id);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_message_serialization() {
        let msg = SyncMessage::Subscribe {
            workspace_id: Uuid::new_v4(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("subscribe"));

        let deserialized: SyncMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SyncMessage::Subscribe { .. } => {}
            _ => panic!("Wrong message type"),
        }
    }
}
