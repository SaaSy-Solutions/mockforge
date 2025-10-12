//! AI-powered WebSocket event generation
//!
//! This module integrates LLM-powered replay augmentation into WebSocket
//! event streaming, allowing realistic event generation from narrative descriptions.

use axum::extract::ws::{Message, WebSocket};
use mockforge_data::{ReplayAugmentationConfig, ReplayAugmentationEngine};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info};

/// AI event generator for WebSocket connections
pub struct AiEventGenerator {
    /// Replay augmentation engine
    engine: Arc<RwLock<ReplayAugmentationEngine>>,
}

impl AiEventGenerator {
    /// Create a new AI event generator
    pub fn new(config: ReplayAugmentationConfig) -> mockforge_core::Result<Self> {
        debug!("Creating AI event generator");
        let engine = ReplayAugmentationEngine::new(config)?;
        Ok(Self {
            engine: Arc::new(RwLock::new(engine)),
        })
    }

    /// Stream AI-generated events to a WebSocket connection
    ///
    /// This method generates events using the configured AI engine and sends them
    /// to the client via WebSocket.
    pub async fn stream_events(&self, mut socket: WebSocket, max_events: Option<usize>) {
        info!("Starting AI event stream (max_events: {:?})", max_events);

        // Generate all events at once
        let events = match self.engine.write().await.generate_stream().await {
            Ok(events) => events,
            Err(e) => {
                error!("Failed to generate event stream: {}", e);
                return;
            }
        };

        info!("Generated {} events from AI engine", events.len());

        let max = max_events.unwrap_or(events.len());
        let events_to_send = events.into_iter().take(max);

        for event in events_to_send {
            // Convert event to JSON message
            let message_json = serde_json::json!({
                "type": event.event_type,
                "timestamp": event.timestamp.to_rfc3339(),
                "sequence": event.sequence,
                "data": event.data
            });

            let message_str = match serde_json::to_string(&message_json) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to serialize event: {}", e);
                    continue;
                }
            };

            debug!("Sending AI-generated event: {}", message_str);

            // Send event to client
            if socket.send(Message::Text(message_str.into())).await.is_err() {
                info!("Client disconnected, stopping event stream");
                break;
            }

            // Small delay between events (configurable event rate would be better)
            sleep(Duration::from_millis(100)).await;
        }

        info!("AI event stream completed");
    }

    /// Stream events with custom event rate
    pub async fn stream_events_with_rate(
        &self,
        mut socket: WebSocket,
        max_events: Option<usize>,
        events_per_second: f64,
    ) {
        info!(
            "Starting AI event stream (max_events: {:?}, rate: {} events/sec)",
            max_events, events_per_second
        );

        // Generate all events at once
        let events = match self.engine.write().await.generate_stream().await {
            Ok(events) => events,
            Err(e) => {
                error!("Failed to generate event stream: {}", e);
                return;
            }
        };

        info!("Generated {} events from AI engine", events.len());

        let delay_ms = (1000.0 / events_per_second) as u64;
        let max = max_events.unwrap_or(events.len());
        let events_to_send = events.into_iter().take(max);

        for event in events_to_send {
            // Convert event to JSON message
            let message_json = serde_json::json!({
                "type": event.event_type,
                "timestamp": event.timestamp.to_rfc3339(),
                "sequence": event.sequence,
                "data": event.data
            });

            let message_str = match serde_json::to_string(&message_json) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to serialize event: {}", e);
                    continue;
                }
            };

            debug!("Sending AI-generated event: {}", message_str);

            // Send event to client
            if socket.send(Message::Text(message_str.into())).await.is_err() {
                info!("Client disconnected, stopping event stream");
                break;
            }

            // Delay based on configured rate
            sleep(Duration::from_millis(delay_ms)).await;
        }

        info!("AI event stream completed");
    }
}

/// Configuration for WebSocket AI event generation
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct WebSocketAiConfig {
    /// Whether AI event generation is enabled
    pub enabled: bool,
    /// Replay augmentation configuration
    pub replay: Option<ReplayAugmentationConfig>,
    /// Maximum number of events to generate
    pub max_events: Option<usize>,
    /// Events per second
    pub event_rate: Option<f64>,
}

impl Default for WebSocketAiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            replay: None,
            max_events: Some(100),
            event_rate: Some(1.0),
        }
    }
}

impl WebSocketAiConfig {
    /// Check if AI features are enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled && self.replay.is_some()
    }

    /// Create an AI event generator from this configuration
    pub fn create_generator(&self) -> mockforge_core::Result<Option<AiEventGenerator>> {
        if let Some(replay_config) = &self.replay {
            Ok(Some(AiEventGenerator::new(replay_config.clone())?))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_data::{EventStrategy, ReplayMode};

    #[test]
    fn test_websocket_ai_config_default() {
        let config = WebSocketAiConfig::default();
        assert!(!config.is_enabled());
        assert_eq!(config.max_events, Some(100));
        assert_eq!(config.event_rate, Some(1.0));
    }

    #[test]
    fn test_websocket_ai_config_is_enabled() {
        let mut config = WebSocketAiConfig {
            enabled: true,
            ..Default::default()
        };

        // Still not enabled without replay config
        assert!(!config.is_enabled());

        // Now enabled with replay config
        config.replay = Some(ReplayAugmentationConfig {
            mode: ReplayMode::Generated,
            strategy: EventStrategy::CountBased,
            ..Default::default()
        });
        assert!(config.is_enabled());
    }
}
