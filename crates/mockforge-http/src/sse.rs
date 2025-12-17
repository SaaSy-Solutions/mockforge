//! Server Sent Events (SSE) support for MockForge

use axum::{
    extract::{Query, State},
    response::{sse::Event, Sse},
    routing::get,
    Router,
};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::time::Duration;

use mockforge_core::templating;

/// SSE configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSEConfig {
    /// Event type to send
    pub event_type: Option<String>,
    /// Data template for events
    pub data_template: String,
    /// Interval between events in milliseconds
    pub interval_ms: u64,
    /// Maximum number of events to send (0 = unlimited)
    pub max_events: usize,
    /// Initial delay before first event in milliseconds
    pub initial_delay_ms: u64,
}

/// Query parameters for SSE endpoint
#[derive(Debug, Deserialize)]
pub struct SSEQueryParams {
    /// Event type override
    pub event: Option<String>,
    /// Data template override
    pub data: Option<String>,
    /// Interval override (milliseconds)
    pub interval: Option<u64>,
    /// Maximum events override
    pub max_events: Option<usize>,
}

/// SSE event data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSEEventData {
    /// Event ID
    pub id: Option<String>,
    /// Event type
    pub event: Option<String>,
    /// Event data
    pub data: serde_json::Value,
    /// Retry interval
    pub retry: Option<u64>,
    /// Timestamp
    pub timestamp: String,
}

/// SSE stream manager
#[derive(Clone)]
pub struct SSEStreamManager {
    config: SSEConfig,
}

impl SSEStreamManager {
    /// Create a new SSE stream manager
    pub fn new(config: SSEConfig) -> Self {
        Self { config }
    }

    /// Create default SSE configuration
    pub fn default_config() -> SSEConfig {
        SSEConfig {
            event_type: Some("message".to_string()),
            data_template: r#"{"message": "{{faker.sentence}}", "timestamp": "{{now}}"}"#
                .to_string(),
            interval_ms: 1000,
            max_events: 0, // unlimited
            initial_delay_ms: 0,
        }
    }

    /// Create SSE stream
    pub fn create_stream(
        &self,
        query_params: SSEQueryParams,
    ) -> impl Stream<Item = Result<Event, Infallible>> {
        let config = self.merge_config_with_params(query_params);

        let event_type = config.event_type.clone();
        let data_template = config.data_template.clone();
        let max_events = config.max_events;
        let interval_duration = Duration::from_millis(config.interval_ms);
        let initial_delay = config.initial_delay_ms;

        // Use a simpler approach to avoid pinning issues
        let event_type = event_type.clone();
        let data_template = data_template.clone();

        stream::unfold(0usize, move |count| {
            let event_type = event_type.clone();
            let data_template = data_template.clone();
            let max_events = max_events;
            let interval_duration = interval_duration;
            let initial_delay = initial_delay;

            Box::pin(async move {
                // Check if we've reached max events
                if max_events > 0 && count >= max_events {
                    return None;
                }

                // Wait for next interval
                if count > 0 || initial_delay > 0 {
                    tokio::time::sleep(interval_duration).await;
                }

                // Generate event data
                let event_data = Self::generate_event_data(&data_template, count);

                // Create SSE event
                let mut event = Event::default();

                if let Some(event_type) = &event_type {
                    event = event.event(event_type);
                }

                // Set event data
                let data_json =
                    serde_json::to_string(&event_data).unwrap_or_else(|_| "{}".to_string());
                event = event.data(data_json);

                // Add event ID
                event = event.id(count.to_string());

                Some((Ok(event), count + 1))
            })
        })
    }

    /// Generate event data from template
    fn generate_event_data(template: &str, count: usize) -> SSEEventData {
        // Expand template with MockForge templating
        let expanded_data = templating::expand_str(template);

        // Parse as JSON, fallback to string
        let data_value = serde_json::from_str(&expanded_data)
            .unwrap_or(serde_json::Value::String(expanded_data));

        SSEEventData {
            id: Some(count.to_string()),
            event: None, // Will be set from config
            data: data_value,
            retry: None,
            timestamp: templating::expand_str("{{now}}"),
        }
    }

    /// Merge configuration with query parameters
    fn merge_config_with_params(&self, params: SSEQueryParams) -> SSEConfig {
        let mut config = self.config.clone();

        if let Some(event) = params.event {
            config.event_type = Some(event);
        }

        if let Some(data) = params.data {
            config.data_template = data;
        }

        if let Some(interval) = params.interval {
            config.interval_ms = interval;
        }

        if let Some(max_events) = params.max_events {
            config.max_events = max_events;
        }

        config
    }
}

/// Create SSE router with default configuration
pub fn sse_router() -> Router {
    sse_router_with_config(SSEStreamManager::default_config())
}

/// Create SSE router with custom configuration
pub fn sse_router_with_config(config: SSEConfig) -> Router {
    let manager = SSEStreamManager::new(config);
    Router::new().route("/sse", get(sse_handler)).with_state(manager)
}

/// SSE handler
async fn sse_handler(
    State(manager): State<SSEStreamManager>,
    Query(params): Query<SSEQueryParams>,
) -> Sse<impl Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>> {
    let stream = manager.create_stream(params);

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keepalive"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::StreamExt;

    // ==================== SSEConfig Tests ====================

    #[test]
    fn test_sse_config_default_via_manager() {
        let config = SSEStreamManager::default_config();

        assert_eq!(config.event_type, Some("message".to_string()));
        assert!(config.data_template.contains("faker"));
        assert_eq!(config.interval_ms, 1000);
        assert_eq!(config.max_events, 0);
        assert_eq!(config.initial_delay_ms, 0);
    }

    #[test]
    fn test_sse_config_custom() {
        let config = SSEConfig {
            event_type: Some("custom".to_string()),
            data_template: "test data".to_string(),
            interval_ms: 500,
            max_events: 10,
            initial_delay_ms: 100,
        };

        assert_eq!(config.event_type, Some("custom".to_string()));
        assert_eq!(config.data_template, "test data");
        assert_eq!(config.interval_ms, 500);
        assert_eq!(config.max_events, 10);
        assert_eq!(config.initial_delay_ms, 100);
    }

    #[test]
    fn test_sse_config_clone() {
        let config = SSEConfig {
            event_type: Some("clone_test".to_string()),
            data_template: "clone data".to_string(),
            interval_ms: 250,
            max_events: 5,
            initial_delay_ms: 50,
        };

        let cloned = config.clone();

        assert_eq!(cloned.event_type, config.event_type);
        assert_eq!(cloned.data_template, config.data_template);
        assert_eq!(cloned.interval_ms, config.interval_ms);
        assert_eq!(cloned.max_events, config.max_events);
        assert_eq!(cloned.initial_delay_ms, config.initial_delay_ms);
    }

    #[test]
    fn test_sse_config_debug() {
        let config = SSEConfig {
            event_type: Some("debug".to_string()),
            data_template: "debug data".to_string(),
            interval_ms: 100,
            max_events: 1,
            initial_delay_ms: 0,
        };

        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("event_type"));
        assert!(debug_str.contains("data_template"));
        assert!(debug_str.contains("interval_ms"));
    }

    #[test]
    fn test_sse_config_serialization() {
        let config = SSEConfig {
            event_type: Some("serialize".to_string()),
            data_template: "{\"key\": \"value\"}".to_string(),
            interval_ms: 200,
            max_events: 3,
            initial_delay_ms: 10,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: SSEConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.event_type, config.event_type);
        assert_eq!(deserialized.interval_ms, config.interval_ms);
    }

    #[test]
    fn test_sse_config_no_event_type() {
        let config = SSEConfig {
            event_type: None,
            data_template: "data".to_string(),
            interval_ms: 100,
            max_events: 1,
            initial_delay_ms: 0,
        };

        assert!(config.event_type.is_none());
    }

    // ==================== SSEQueryParams Tests ====================

    #[test]
    fn test_sse_query_params_all_none() {
        let params = SSEQueryParams {
            event: None,
            data: None,
            interval: None,
            max_events: None,
        };

        assert!(params.event.is_none());
        assert!(params.data.is_none());
        assert!(params.interval.is_none());
        assert!(params.max_events.is_none());
    }

    #[test]
    fn test_sse_query_params_with_values() {
        let params = SSEQueryParams {
            event: Some("custom_event".to_string()),
            data: Some("{\"custom\": true}".to_string()),
            interval: Some(500),
            max_events: Some(10),
        };

        assert_eq!(params.event, Some("custom_event".to_string()));
        assert_eq!(params.data, Some("{\"custom\": true}".to_string()));
        assert_eq!(params.interval, Some(500));
        assert_eq!(params.max_events, Some(10));
    }

    #[test]
    fn test_sse_query_params_debug() {
        let params = SSEQueryParams {
            event: Some("test".to_string()),
            data: None,
            interval: Some(100),
            max_events: None,
        };

        let debug_str = format!("{:?}", params);

        assert!(debug_str.contains("event"));
        assert!(debug_str.contains("interval"));
    }

    // ==================== SSEEventData Tests ====================

    #[test]
    fn test_sse_event_data_creation() {
        let event_data = SSEEventData {
            id: Some("1".to_string()),
            event: Some("test_event".to_string()),
            data: serde_json::json!({"key": "value"}),
            retry: Some(5000),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        assert_eq!(event_data.id, Some("1".to_string()));
        assert_eq!(event_data.event, Some("test_event".to_string()));
        assert_eq!(event_data.data["key"], "value");
        assert_eq!(event_data.retry, Some(5000));
    }

    #[test]
    fn test_sse_event_data_clone() {
        let event_data = SSEEventData {
            id: Some("2".to_string()),
            event: None,
            data: serde_json::json!({"number": 42}),
            retry: None,
            timestamp: "now".to_string(),
        };

        let cloned = event_data.clone();

        assert_eq!(cloned.id, event_data.id);
        assert_eq!(cloned.event, event_data.event);
        assert_eq!(cloned.data, event_data.data);
    }

    #[test]
    fn test_sse_event_data_serialization() {
        let event_data = SSEEventData {
            id: Some("3".to_string()),
            event: Some("message".to_string()),
            data: serde_json::json!({"text": "hello"}),
            retry: Some(1000),
            timestamp: "test".to_string(),
        };

        let json = serde_json::to_string(&event_data).unwrap();

        assert!(json.contains("\"id\":\"3\""));
        assert!(json.contains("\"event\":\"message\""));
        assert!(json.contains("\"text\":\"hello\""));
    }

    // ==================== SSEStreamManager Tests ====================

    #[test]
    fn test_sse_stream_manager_creation() {
        let config = SSEConfig {
            event_type: Some("test".to_string()),
            data_template: "data".to_string(),
            interval_ms: 100,
            max_events: 5,
            initial_delay_ms: 0,
        };

        let manager = SSEStreamManager::new(config.clone());

        // Manager should store the config
        assert_eq!(manager.config.event_type, config.event_type);
        assert_eq!(manager.config.max_events, config.max_events);
    }

    #[test]
    fn test_merge_config_with_empty_params() {
        let config = SSEConfig {
            event_type: Some("original".to_string()),
            data_template: "original data".to_string(),
            interval_ms: 1000,
            max_events: 10,
            initial_delay_ms: 0,
        };

        let manager = SSEStreamManager::new(config);
        let params = SSEQueryParams {
            event: None,
            data: None,
            interval: None,
            max_events: None,
        };

        let merged = manager.merge_config_with_params(params);

        assert_eq!(merged.event_type, Some("original".to_string()));
        assert_eq!(merged.data_template, "original data");
        assert_eq!(merged.interval_ms, 1000);
        assert_eq!(merged.max_events, 10);
    }

    #[test]
    fn test_merge_config_with_all_params() {
        let config = SSEConfig {
            event_type: Some("original".to_string()),
            data_template: "original data".to_string(),
            interval_ms: 1000,
            max_events: 10,
            initial_delay_ms: 0,
        };

        let manager = SSEStreamManager::new(config);
        let params = SSEQueryParams {
            event: Some("overridden".to_string()),
            data: Some("overridden data".to_string()),
            interval: Some(500),
            max_events: Some(5),
        };

        let merged = manager.merge_config_with_params(params);

        assert_eq!(merged.event_type, Some("overridden".to_string()));
        assert_eq!(merged.data_template, "overridden data");
        assert_eq!(merged.interval_ms, 500);
        assert_eq!(merged.max_events, 5);
    }

    #[test]
    fn test_merge_config_partial_override() {
        let config = SSEConfig {
            event_type: Some("original".to_string()),
            data_template: "original data".to_string(),
            interval_ms: 1000,
            max_events: 10,
            initial_delay_ms: 0,
        };

        let manager = SSEStreamManager::new(config);
        let params = SSEQueryParams {
            event: Some("new_event".to_string()),
            data: None,
            interval: Some(2000),
            max_events: None,
        };

        let merged = manager.merge_config_with_params(params);

        assert_eq!(merged.event_type, Some("new_event".to_string()));
        assert_eq!(merged.data_template, "original data"); // Not overridden
        assert_eq!(merged.interval_ms, 2000);
        assert_eq!(merged.max_events, 10); // Not overridden
    }

    // ==================== Event Data Generation Tests ====================

    #[test]
    fn test_generate_event_data_simple_template() {
        let template = r#"{"message": "hello"}"#;
        let event_data = SSEStreamManager::generate_event_data(template, 0);

        assert_eq!(event_data.id, Some("0".to_string()));
        assert_eq!(event_data.event, None);
        assert!(event_data.data.is_object());
    }

    #[test]
    fn test_generate_event_data_string_fallback() {
        let template = "not json at all";
        let event_data = SSEStreamManager::generate_event_data(template, 5);

        assert_eq!(event_data.id, Some("5".to_string()));
        // Should fallback to string value
        assert!(event_data.data.is_string());
    }

    #[test]
    fn test_generate_event_data_incremental_count() {
        for count in 0..5 {
            let template = r#"{"test": true}"#;
            let event_data = SSEStreamManager::generate_event_data(template, count);

            assert_eq!(event_data.id, Some(count.to_string()));
        }
    }

    #[test]
    fn test_generate_event_data_timestamp_populated() {
        let template = "{}";
        let event_data = SSEStreamManager::generate_event_data(template, 0);

        assert!(!event_data.timestamp.is_empty());
    }

    // ==================== Async Stream Tests ====================

    #[tokio::test]
    async fn test_sse_stream_generation() {
        let config = SSEConfig {
            event_type: Some("test".to_string()),
            data_template: r#"{"count": {{count}}}"#.to_string(),
            interval_ms: 10,
            max_events: 3,
            initial_delay_ms: 0,
        };

        let manager = SSEStreamManager::new(config);
        let params = SSEQueryParams {
            event: None,
            data: None,
            interval: None,
            max_events: None,
        };

        let mut stream = manager.create_stream(params);
        let mut events = Vec::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => events.push(event),
                Err(_) => break,
            }
            if events.len() >= 3 {
                break;
            }
        }

        assert_eq!(events.len(), 3);
    }

    #[tokio::test]
    async fn test_event_data_generation() {
        let template = r#"{"message": "test", "timestamp": "{{now}}"}"#;
        let event_data = SSEStreamManager::generate_event_data(template, 1);

        assert_eq!(event_data.id, Some("1".to_string()));
        assert!(!event_data.timestamp.is_empty());
    }

    #[tokio::test]
    async fn test_sse_stream_with_max_events_1() {
        let config = SSEConfig {
            event_type: Some("single".to_string()),
            data_template: r#"{"single": true}"#.to_string(),
            interval_ms: 1,
            max_events: 1,
            initial_delay_ms: 0,
        };

        let manager = SSEStreamManager::new(config);
        let params = SSEQueryParams {
            event: None,
            data: None,
            interval: None,
            max_events: None,
        };

        let mut stream = manager.create_stream(params);
        let mut count = 0;

        while let Some(Ok(_)) = stream.next().await {
            count += 1;
            if count > 5 {
                break; // Safety limit
            }
        }

        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_sse_stream_with_query_param_override() {
        let config = SSEConfig {
            event_type: Some("original".to_string()),
            data_template: r#"{"original": true}"#.to_string(),
            interval_ms: 1000,
            max_events: 100,
            initial_delay_ms: 0,
        };

        let manager = SSEStreamManager::new(config);
        let params = SSEQueryParams {
            event: None,
            data: None,
            interval: Some(1),
            max_events: Some(2),
        };

        let mut stream = manager.create_stream(params);
        let mut count = 0;

        while let Some(Ok(_)) = stream.next().await {
            count += 1;
            if count > 10 {
                break; // Safety limit
            }
        }

        assert_eq!(count, 2);
    }
}
