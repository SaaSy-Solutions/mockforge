//! Collaboration client for connecting to servers
//!
//! This module provides a client library for connecting to `MockForge` collaboration servers
//! via WebSocket. It handles connection management, automatic reconnection, message queuing,
//! and provides an event-driven API for workspace updates.

use crate::error::{CollabError, Result};
use crate::events::ChangeEvent;
use crate::sync::SyncMessage;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

/// Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Server WebSocket URL (e.g., <ws://localhost:8080/ws> or <wss://api.example.com/ws>)
    pub server_url: String,
    /// Authentication token (JWT)
    pub auth_token: String,
    /// Maximum reconnect attempts (None for unlimited)
    pub max_reconnect_attempts: Option<u32>,
    /// Maximum queue size for messages (when disconnected)
    pub max_queue_size: usize,
    /// Initial backoff delay in milliseconds (exponential backoff starts here)
    pub initial_backoff_ms: u64,
    /// Maximum backoff delay in milliseconds
    pub max_backoff_ms: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            auth_token: String::new(),
            max_reconnect_attempts: None,
            max_queue_size: 1000,
            initial_backoff_ms: 1000,
            max_backoff_ms: 30000,
        }
    }
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected and ready
    Connected,
    /// Reconnecting after error
    Reconnecting,
}

/// Callback function type for workspace updates
pub type WorkspaceUpdateCallback = Box<dyn Fn(ChangeEvent) + Send + Sync>;

/// Callback function type for connection state changes
pub type StateChangeCallback = Box<dyn Fn(ConnectionState) + Send + Sync>;

/// Collaboration client
pub struct CollabClient {
    /// Configuration
    config: ClientConfig,
    /// Client ID
    client_id: Uuid,
    /// Connection state
    state: Arc<RwLock<ConnectionState>>,
    /// Message queue for when disconnected
    message_queue: Arc<RwLock<Vec<SyncMessage>>>,
    /// WebSocket connection handle
    ws_sender: Arc<RwLock<Option<mpsc::UnboundedSender<SyncMessage>>>>,
    /// Connection task handle for cleanup
    connection_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    /// Workspace update callbacks
    workspace_callbacks: Arc<RwLock<Vec<WorkspaceUpdateCallback>>>,
    /// State change callbacks
    state_callbacks: Arc<RwLock<Vec<StateChangeCallback>>>,
    /// Reconnect attempt count
    reconnect_count: Arc<RwLock<u32>>,
    /// Stop signal
    stop_signal: Arc<RwLock<bool>>,
}

impl CollabClient {
    /// Create a new client and connect to server
    pub async fn connect(config: ClientConfig) -> Result<Self> {
        if config.server_url.is_empty() {
            return Err(CollabError::InvalidInput("server_url cannot be empty".to_string()));
        }

        let client = Self {
            config: config.clone(),
            client_id: Uuid::new_v4(),
            state: Arc::new(RwLock::new(ConnectionState::Connecting)),
            message_queue: Arc::new(RwLock::new(Vec::new())),
            ws_sender: Arc::new(RwLock::new(None)),
            connection_task: Arc::new(RwLock::new(None)),
            workspace_callbacks: Arc::new(RwLock::new(Vec::new())),
            state_callbacks: Arc::new(RwLock::new(Vec::new())),
            reconnect_count: Arc::new(RwLock::new(0)),
            stop_signal: Arc::new(RwLock::new(false)),
        };

        // Start connection process
        client.update_state(ConnectionState::Connecting).await;
        client.start_connection_loop().await?;

        Ok(client)
    }

    /// Internal: Start the connection loop with reconnection logic
    async fn start_connection_loop(&self) -> Result<()> {
        let config = self.config.clone();
        let state = self.state.clone();
        let message_queue = self.message_queue.clone();
        let ws_sender = self.ws_sender.clone();
        let stop_signal = self.stop_signal.clone();
        let reconnect_count = self.reconnect_count.clone();
        let workspace_callbacks = self.workspace_callbacks.clone();
        let state_callbacks = self.state_callbacks.clone();

        let task = tokio::spawn(async move {
            let mut backoff_ms = config.initial_backoff_ms;

            loop {
                // Check if we should stop
                if *stop_signal.read().await {
                    break;
                }

                // Attempt connection
                match Self::try_connect(
                    &config,
                    &state,
                    &ws_sender,
                    &workspace_callbacks,
                    &state_callbacks,
                    &stop_signal,
                )
                .await
                {
                    Ok(()) => {
                        // Connection successful, reset backoff
                        backoff_ms = config.initial_backoff_ms;
                        *reconnect_count.write().await = 0;

                        // Flush message queue
                        let mut queue = message_queue.write().await;
                        while let Some(msg) = queue.pop() {
                            if let Some(ref sender) = *ws_sender.read().await {
                                let _ = sender.send(msg);
                            }
                        }

                        // Wait for connection to close
                        // (This will happen when try_connect returns on error/disconnect)
                    }
                    Err(e) => {
                        tracing::warn!("Connection failed: {}, will retry", e);

                        // Check max reconnect attempts
                        let current_count = *reconnect_count.read().await;
                        if let Some(max) = config.max_reconnect_attempts {
                            if current_count >= max {
                                tracing::error!("Max reconnect attempts ({}) reached", max);
                                *state.write().await = ConnectionState::Disconnected;
                                Self::notify_state_change(
                                    &state_callbacks,
                                    ConnectionState::Disconnected,
                                )
                                .await;
                                break;
                            }
                        }

                        *reconnect_count.write().await += 1;
                        *state.write().await = ConnectionState::Reconnecting;
                        Self::notify_state_change(&state_callbacks, ConnectionState::Reconnecting)
                            .await;

                        // Exponential backoff
                        sleep(Duration::from_millis(backoff_ms)).await;
                        backoff_ms = (backoff_ms * 2).min(config.max_backoff_ms);
                    }
                }
            }
        });

        *self.connection_task.write().await = Some(task);
        Ok(())
    }

    /// Internal: Attempt to establish WebSocket connection
    async fn try_connect(
        config: &ClientConfig,
        state: &Arc<RwLock<ConnectionState>>,
        ws_sender: &Arc<RwLock<Option<mpsc::UnboundedSender<SyncMessage>>>>,
        workspace_callbacks: &Arc<RwLock<Vec<WorkspaceUpdateCallback>>>,
        state_callbacks: &Arc<RwLock<Vec<StateChangeCallback>>>,
        stop_signal: &Arc<RwLock<bool>>,
    ) -> Result<()> {
        // Build WebSocket URL with auth token
        let url = format!("{}?token={}", config.server_url, config.auth_token);
        tracing::info!("Connecting to WebSocket: {}", config.server_url);

        // Connect to WebSocket
        let (ws_stream, _) = connect_async(&url)
            .await
            .map_err(|e| CollabError::Internal(format!("WebSocket connection failed: {e}")))?;

        *state.write().await = ConnectionState::Connected;
        Self::notify_state_change(state_callbacks, ConnectionState::Connected).await;

        tracing::info!("WebSocket connected successfully");

        // Split stream into sender and receiver
        let (write, mut read) = ws_stream.split();

        // Create message channel for sending messages
        let (tx, mut rx) = mpsc::unbounded_channel();
        *ws_sender.write().await = Some(tx);

        // Spawn task to handle outgoing messages
        let mut write_handle = write;
        let write_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let json = match serde_json::to_string(&msg) {
                    Ok(json) => json,
                    Err(e) => {
                        tracing::error!("Failed to serialize message: {}", e);
                        continue;
                    }
                };

                if let Err(e) = write_handle.send(Message::Text(json)).await {
                    tracing::error!("Failed to send message: {}", e);
                    break;
                }
            }
        });

        // Handle incoming messages
        loop {
            // Check for stop signal first
            if *stop_signal.read().await {
                tracing::info!("Stop signal received, closing connection");
                break;
            }

            tokio::select! {
                // Receive message from server
                msg_opt = read.next() => {
                    match msg_opt {
                        Some(Ok(Message::Text(text))) => {
                            Self::handle_server_message(&text, workspace_callbacks).await;
                        }
                        Some(Ok(Message::Close(_))) => {
                            tracing::info!("Server closed connection");
                            *state.write().await = ConnectionState::Disconnected;
                            Self::notify_state_change(state_callbacks, ConnectionState::Disconnected).await;
                            break;
                        }
                        Some(Ok(Message::Ping(_))) => {
                            // Tungstenite handles pings automatically
                            tracing::debug!("Received ping");
                        }
                        Some(Ok(Message::Pong(_))) => {
                            tracing::debug!("Received pong");
                        }
                        Some(Err(e)) => {
                            tracing::error!("WebSocket error: {}", e);
                            *state.write().await = ConnectionState::Disconnected;
                            Self::notify_state_change(state_callbacks, ConnectionState::Disconnected).await;
                            return Err(CollabError::Internal(format!("WebSocket error: {e}")));
                        }
                        None => {
                            tracing::info!("WebSocket stream ended");
                            *state.write().await = ConnectionState::Disconnected;
                            Self::notify_state_change(state_callbacks, ConnectionState::Disconnected).await;
                            break;
                        }
                        _ => {}
                    }
                }

                // Periodic stop signal check
                () = sleep(Duration::from_millis(100)) => {
                    if *stop_signal.read().await {
                        tracing::info!("Stop signal received, closing connection");
                        break;
                    }
                }
            }
        }

        // Clean up
        write_task.abort();
        *ws_sender.write().await = None;

        Err(CollabError::Internal("Connection closed".to_string()))
    }

    /// Internal: Handle message from server
    async fn handle_server_message(
        text: &str,
        workspace_callbacks: &Arc<RwLock<Vec<WorkspaceUpdateCallback>>>,
    ) {
        match serde_json::from_str::<SyncMessage>(text) {
            Ok(SyncMessage::Change { event }) => {
                // Notify all workspace callbacks
                let callbacks = workspace_callbacks.read().await;
                for callback in callbacks.iter() {
                    callback(event.clone());
                }
            }
            Ok(SyncMessage::StateResponse {
                workspace_id,
                version,
                state,
            }) => {
                tracing::debug!(
                    "Received state response for workspace {} (version {})",
                    workspace_id,
                    version
                );
                // Could emit this as a separate event type if needed
            }
            Ok(SyncMessage::Error { message }) => {
                tracing::error!("Server error: {}", message);
            }
            Ok(SyncMessage::Pong) => {
                tracing::debug!("Received pong");
            }
            Ok(other) => {
                tracing::debug!("Received message: {:?}", other);
            }
            Err(e) => {
                tracing::warn!("Failed to parse server message: {} - {}", e, text);
            }
        }
    }

    /// Internal: Notify state change callbacks
    async fn notify_state_change(
        callbacks: &Arc<RwLock<Vec<StateChangeCallback>>>,
        new_state: ConnectionState,
    ) {
        let callbacks = callbacks.read().await;
        for callback in callbacks.iter() {
            callback(new_state);
        }
    }

    /// Internal: Update connection state and notify callbacks
    async fn update_state(&self, new_state: ConnectionState) {
        *self.state.write().await = new_state;
        let callbacks = self.state_callbacks.read().await;
        for callback in callbacks.iter() {
            callback(new_state);
        }
    }

    /// Internal: Send message (queue if disconnected)
    async fn send_message(&self, message: SyncMessage) -> Result<()> {
        let state = *self.state.read().await;

        if state == ConnectionState::Connected {
            // Try to send immediately
            if let Some(ref sender) = *self.ws_sender.read().await {
                sender.send(message).map_err(|_| {
                    CollabError::Internal("Failed to send message (channel closed)".to_string())
                })?;
                return Ok(());
            }
        }

        // Queue message if disconnected or sender unavailable
        let mut queue = self.message_queue.write().await;
        if queue.len() >= self.config.max_queue_size {
            return Err(CollabError::InvalidInput(format!(
                "Message queue full (max: {})",
                self.config.max_queue_size
            )));
        }

        queue.push(message);
        Ok(())
    }

    /// Subscribe to workspace updates
    ///
    /// # Arguments
    /// * `callback` - Function to call when workspace changes occur
    pub async fn on_workspace_update<F>(&self, callback: F)
    where
        F: Fn(ChangeEvent) + Send + Sync + 'static,
    {
        let mut callbacks = self.workspace_callbacks.write().await;
        callbacks.push(Box::new(callback));
    }

    /// Subscribe to connection state changes
    ///
    /// # Arguments
    /// * `callback` - Function to call when connection state changes
    pub async fn on_state_change<F>(&self, callback: F)
    where
        F: Fn(ConnectionState) + Send + Sync + 'static,
    {
        let mut callbacks = self.state_callbacks.write().await;
        callbacks.push(Box::new(callback));
    }

    /// Subscribe to a workspace
    pub async fn subscribe_to_workspace(&self, workspace_id: &str) -> Result<()> {
        let workspace_id = Uuid::parse_str(workspace_id)
            .map_err(|e| CollabError::InvalidInput(format!("Invalid workspace ID: {e}")))?;

        let message = SyncMessage::Subscribe { workspace_id };
        self.send_message(message).await?;

        Ok(())
    }

    /// Unsubscribe from a workspace
    pub async fn unsubscribe_from_workspace(&self, workspace_id: &str) -> Result<()> {
        let workspace_id = Uuid::parse_str(workspace_id)
            .map_err(|e| CollabError::InvalidInput(format!("Invalid workspace ID: {e}")))?;

        let message = SyncMessage::Unsubscribe { workspace_id };
        self.send_message(message).await?;

        Ok(())
    }

    /// Request state for a workspace
    pub async fn request_state(&self, workspace_id: &str, version: i64) -> Result<()> {
        let workspace_id = Uuid::parse_str(workspace_id)
            .map_err(|e| CollabError::InvalidInput(format!("Invalid workspace ID: {e}")))?;

        let message = SyncMessage::StateRequest {
            workspace_id,
            version,
        };
        self.send_message(message).await?;

        Ok(())
    }

    /// Send ping (heartbeat)
    pub async fn ping(&self) -> Result<()> {
        let message = SyncMessage::Ping;
        self.send_message(message).await?;
        Ok(())
    }

    /// Get connection state
    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Get queued message count
    pub async fn queued_message_count(&self) -> usize {
        self.message_queue.read().await.len()
    }

    /// Get reconnect attempt count
    pub async fn reconnect_count(&self) -> u32 {
        *self.reconnect_count.read().await
    }

    /// Disconnect from server
    pub async fn disconnect(&self) -> Result<()> {
        // Signal stop
        *self.stop_signal.write().await = true;

        // Update state
        *self.state.write().await = ConnectionState::Disconnected;
        Self::notify_state_change(&self.state_callbacks, ConnectionState::Disconnected).await;

        // Wait for connection task to finish
        if let Some(task) = self.connection_task.write().await.take() {
            task.abort();
        }

        Ok(())
    }
}

impl Drop for CollabClient {
    fn drop(&mut self) {
        // Ensure we disconnect when dropped
        let stop_signal = self.stop_signal.clone();
        let state = self.state.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                *stop_signal.write().await = true;
                *state.write().await = ConnectionState::Disconnected;
            });
        }
    }
}
