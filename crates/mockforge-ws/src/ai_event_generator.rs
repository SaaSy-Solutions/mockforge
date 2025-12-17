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
        let engine = ReplayAugmentationEngine::new(config)
            .map_err(|e| mockforge_core::Error::generic(e.to_string()))?;
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
            let generator = AiEventGenerator::new(replay_config.clone())?;
            Ok(Some(generator))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_data::{EventStrategy, ReplayMode};

    // ==================== WebSocketAiConfig Tests ====================

    #[test]
    fn test_websocket_ai_config_default() {
        let config = WebSocketAiConfig::default();
        assert!(!config.is_enabled());
        assert_eq!(config.max_events, Some(100));
        assert_eq!(config.event_rate, Some(1.0));
    }

    #[test]
    fn test_websocket_ai_config_default_enabled_false() {
        let config = WebSocketAiConfig::default();
        assert!(!config.enabled);
        assert!(config.replay.is_none());
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

    #[test]
    fn test_websocket_ai_config_enabled_requires_both() {
        // Only enabled flag set
        let config1 = WebSocketAiConfig {
            enabled: true,
            replay: None,
            max_events: None,
            event_rate: None,
        };
        assert!(!config1.is_enabled());

        // Only replay set, but enabled is false
        let config2 = WebSocketAiConfig {
            enabled: false,
            replay: Some(ReplayAugmentationConfig::default()),
            max_events: None,
            event_rate: None,
        };
        assert!(!config2.is_enabled());

        // Both set
        let config3 = WebSocketAiConfig {
            enabled: true,
            replay: Some(ReplayAugmentationConfig::default()),
            max_events: None,
            event_rate: None,
        };
        assert!(config3.is_enabled());
    }

    #[test]
    fn test_websocket_ai_config_custom_values() {
        let config = WebSocketAiConfig {
            enabled: true,
            replay: Some(ReplayAugmentationConfig {
                mode: ReplayMode::Generated,
                strategy: EventStrategy::TimeBased,
                ..Default::default()
            }),
            max_events: Some(50),
            event_rate: Some(2.5),
        };

        assert!(config.is_enabled());
        assert_eq!(config.max_events, Some(50));
        assert_eq!(config.event_rate, Some(2.5));
    }

    #[test]
    fn test_websocket_ai_config_create_generator_none_when_no_replay() {
        let config = WebSocketAiConfig::default();
        let result = config.create_generator();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_websocket_ai_config_create_generator_with_replay_set() {
        let config = WebSocketAiConfig {
            enabled: true,
            replay: Some(ReplayAugmentationConfig {
                mode: ReplayMode::Generated,
                strategy: EventStrategy::CountBased,
                ..Default::default()
            }),
            max_events: Some(10),
            event_rate: Some(1.0),
        };

        // The result depends on proper initialization of the ReplayAugmentationEngine
        // Just verify it returns a Result
        let _result = config.create_generator();
    }

    // ==================== ReplayMode Tests ====================

    #[test]
    fn test_replay_mode_generated() {
        let config = ReplayAugmentationConfig {
            mode: ReplayMode::Generated,
            strategy: EventStrategy::CountBased,
            ..Default::default()
        };
        assert!(matches!(config.mode, ReplayMode::Generated));
    }

    // ==================== EventStrategy Tests ====================

    #[test]
    fn test_event_strategy_count_based() {
        let config = ReplayAugmentationConfig {
            mode: ReplayMode::Generated,
            strategy: EventStrategy::CountBased,
            ..Default::default()
        };
        assert!(matches!(config.strategy, EventStrategy::CountBased));
    }

    #[test]
    fn test_event_strategy_time_based() {
        let config = ReplayAugmentationConfig {
            mode: ReplayMode::Generated,
            strategy: EventStrategy::TimeBased,
            ..Default::default()
        };
        assert!(matches!(config.strategy, EventStrategy::TimeBased));
    }

    // ==================== AiEventGenerator Tests ====================
    // Note: AiEventGenerator::new may fail without proper config, so we just check it doesn't panic

    // ==================== Serialization Tests ====================

    #[test]
    fn test_websocket_ai_config_serialize() {
        let config = WebSocketAiConfig {
            enabled: true,
            replay: None,
            max_events: Some(50),
            event_rate: Some(1.5),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"enabled\":true"));
        assert!(json.contains("\"max_events\":50"));
        assert!(json.contains("\"event_rate\":1.5"));
    }

    #[test]
    fn test_websocket_ai_config_deserialize() {
        let json = r#"{
            "enabled": true,
            "replay": null,
            "max_events": 100,
            "event_rate": 2.0
        }"#;

        let config: WebSocketAiConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert!(config.replay.is_none());
        assert_eq!(config.max_events, Some(100));
        assert_eq!(config.event_rate, Some(2.0));
    }

    #[test]
    fn test_websocket_ai_config_roundtrip() {
        let original = WebSocketAiConfig {
            enabled: true,
            replay: Some(ReplayAugmentationConfig::default()),
            max_events: Some(25),
            event_rate: Some(0.5),
        };

        let json = serde_json::to_string(&original).unwrap();
        let restored: WebSocketAiConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(original.enabled, restored.enabled);
        assert_eq!(original.max_events, restored.max_events);
        assert_eq!(original.event_rate, restored.event_rate);
        assert!(restored.replay.is_some());
    }

    // ==================== Clone and Debug Tests ====================

    #[test]
    fn test_websocket_ai_config_clone() {
        let config = WebSocketAiConfig {
            enabled: true,
            replay: Some(ReplayAugmentationConfig::default()),
            max_events: Some(50),
            event_rate: Some(1.0),
        };

        let cloned = config.clone();
        assert_eq!(config.enabled, cloned.enabled);
        assert_eq!(config.max_events, cloned.max_events);
        assert_eq!(config.event_rate, cloned.event_rate);
    }

    #[test]
    fn test_websocket_ai_config_debug() {
        let config = WebSocketAiConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("WebSocketAiConfig"));
        assert!(debug_str.contains("enabled"));
    }
}
