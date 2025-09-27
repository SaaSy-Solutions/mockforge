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
        let max_events = max_events;
        let interval_duration = interval_duration;
        let initial_delay = initial_delay;

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
        assert!(event_data.timestamp.len() > 0);
    }
}
