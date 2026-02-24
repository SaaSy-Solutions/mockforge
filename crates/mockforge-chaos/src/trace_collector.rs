//! Trace collector for querying traces from Jaeger and OTLP backends

use mockforge_tracing::exporter::ExporterType;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Trace collector errors
#[derive(Error, Debug)]
pub enum TraceCollectorError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("Trace backend unavailable: {0}")]
    BackendUnavailable(String),
}

/// Configuration for trace collection
#[derive(Debug, Clone)]
pub struct TraceCollectorConfig {
    /// Backend type
    pub backend_type: ExporterType,
    /// Jaeger endpoint (for Jaeger backend)
    pub jaeger_endpoint: Option<String>,
    /// OTLP endpoint (for OTLP backend)
    pub otlp_endpoint: Option<String>,
    /// Query timeout
    pub timeout: Duration,
    /// Maximum number of traces to return
    pub max_traces: usize,
}

impl Default for TraceCollectorConfig {
    fn default() -> Self {
        Self {
            backend_type: ExporterType::Jaeger,
            jaeger_endpoint: Some("http://localhost:16686".to_string()), // Jaeger UI endpoint
            otlp_endpoint: None,
            timeout: Duration::from_secs(30),
            max_traces: 100,
        }
    }
}

/// Collected trace data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectedTrace {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub start_time: String,
    pub end_time: String,
    pub duration_ms: u64,
    pub attributes: std::collections::HashMap<String, serde_json::Value>,
}

/// Trace collector for querying backend systems
pub struct TraceCollector {
    client: Client,
    config: TraceCollectorConfig,
}

impl TraceCollector {
    /// Create a new trace collector
    pub fn new(config: TraceCollectorConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    /// Collect traces from the configured backend
    pub async fn collect_traces(&self) -> Result<Vec<CollectedTrace>, TraceCollectorError> {
        match self.config.backend_type {
            ExporterType::Jaeger => self.collect_from_jaeger().await,
            ExporterType::Otlp => self.collect_from_otlp().await,
        }
    }

    /// Get a specific trace by ID from the configured backend
    pub async fn get_trace_by_id(
        &self,
        trace_id: &str,
    ) -> Result<Vec<CollectedTrace>, TraceCollectorError> {
        match self.config.backend_type {
            ExporterType::Jaeger => self.get_trace_from_jaeger(trace_id).await,
            ExporterType::Otlp => self.get_trace_from_otlp(trace_id).await,
        }
    }

    /// Collect traces from Jaeger backend
    async fn collect_from_jaeger(&self) -> Result<Vec<CollectedTrace>, TraceCollectorError> {
        let endpoint = self.config.jaeger_endpoint.as_ref().ok_or_else(|| {
            TraceCollectorError::ConfigError("Jaeger endpoint not configured".to_string())
        })?;

        // Query recent traces from Jaeger API
        let url = format!("{}/api/traces", endpoint);

        // For now, query traces from the last hour
        let start_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            - 3600000; // 1 hour ago

        let params = [
            ("start", start_time.to_string()),
            ("limit", self.config.max_traces.to_string()),
        ];

        let response = self.client.get(&url).query(&params).send().await?;

        if !response.status().is_success() {
            return Err(TraceCollectorError::BackendUnavailable(format!(
                "Jaeger API returned status: {}",
                response.status()
            )));
        }

        let jaeger_response: JaegerTracesResponse = response.json().await?;

        // Convert Jaeger format to our format
        let mut traces = Vec::new();
        for trace_data in jaeger_response.data {
            for span in trace_data.spans {
                let trace = CollectedTrace {
                    trace_id: span.trace_id,
                    span_id: span.span_id,
                    parent_span_id: span.parent_span_id,
                    name: span.operation_name,
                    start_time: format!(
                        "{:?}",
                        UNIX_EPOCH + Duration::from_micros(span.start_time)
                    ),
                    end_time: {
                        let end_micros = span.start_time.saturating_add(span.duration);
                        format!("{:?}", UNIX_EPOCH + Duration::from_micros(end_micros))
                    },
                    duration_ms: span.duration / 1000, // Convert microseconds to milliseconds
                    attributes: {
                        let mut attrs = std::collections::HashMap::new();
                        for tag in &span.tags {
                            attrs.insert(tag.key.clone(), tag.value.clone());
                        }
                        attrs
                    },
                };
                traces.push(trace);
            }
        }

        Ok(traces)
    }

    /// Get a specific trace from Jaeger backend
    async fn get_trace_from_jaeger(
        &self,
        trace_id: &str,
    ) -> Result<Vec<CollectedTrace>, TraceCollectorError> {
        let endpoint = self.config.jaeger_endpoint.as_ref().ok_or_else(|| {
            TraceCollectorError::ConfigError("Jaeger endpoint not configured".to_string())
        })?;

        // Query specific trace from Jaeger API
        let url = format!("{}/api/traces/{}", endpoint, trace_id);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(TraceCollectorError::BackendUnavailable(format!(
                "Jaeger API returned status: {}",
                response.status()
            )));
        }

        let jaeger_response: JaegerTracesResponse = response.json().await?;

        // Convert Jaeger format to our format
        let mut traces = Vec::new();
        for trace_data in jaeger_response.data {
            for span in trace_data.spans {
                let trace = CollectedTrace {
                    trace_id: span.trace_id,
                    span_id: span.span_id,
                    parent_span_id: span.parent_span_id,
                    name: span.operation_name,
                    start_time: format!(
                        "{:?}",
                        UNIX_EPOCH + Duration::from_micros(span.start_time)
                    ),
                    end_time: {
                        let end_micros = span.start_time.saturating_add(span.duration);
                        format!("{:?}", UNIX_EPOCH + Duration::from_micros(end_micros))
                    },
                    duration_ms: span.duration / 1000, // Convert microseconds to milliseconds
                    attributes: {
                        let mut attrs = std::collections::HashMap::new();
                        for tag in &span.tags {
                            attrs.insert(tag.key.clone(), tag.value.clone());
                        }
                        attrs
                    },
                };
                traces.push(trace);
            }
        }

        Ok(traces)
    }

    /// Collect traces from OTLP backend.
    ///
    /// OTLP is an export protocol (push-based), not a query API. Trace retrieval
    /// requires a separate backend (e.g., Jaeger, Tempo). Returns empty when
    /// no compatible query backend is configured.
    async fn collect_from_otlp(&self) -> Result<Vec<CollectedTrace>, TraceCollectorError> {
        Ok(Vec::new())
    }

    /// Get a specific trace from OTLP backend.
    ///
    /// See [`collect_from_otlp`] — OTLP has no query API, so this always
    /// returns empty unless a compatible trace store is configured.
    async fn get_trace_from_otlp(
        &self,
        _trace_id: &str,
    ) -> Result<Vec<CollectedTrace>, TraceCollectorError> {
        Ok(Vec::new())
    }
}

/// Jaeger API response structures
#[derive(Deserialize)]
struct JaegerTracesResponse {
    data: Vec<JaegerTraceData>,
}

#[derive(Deserialize)]
struct JaegerTraceData {
    spans: Vec<JaegerSpan>,
}

#[derive(Deserialize)]
struct JaegerSpan {
    trace_id: String,
    span_id: String,
    parent_span_id: Option<String>,
    operation_name: String,
    start_time: u64, // microseconds since epoch
    duration: u64,   // microseconds
    tags: Vec<JaegerTag>,
}

#[derive(Deserialize, Serialize)]
struct JaegerTag {
    key: String,
    value: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TraceCollectorConfig::default();
        assert_eq!(config.backend_type, ExporterType::Jaeger);
        assert_eq!(config.max_traces, 100);
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_collect_traces_jaeger_unavailable() {
        let config = TraceCollectorConfig {
            backend_type: ExporterType::Jaeger,
            jaeger_endpoint: Some("http://nonexistent:16686".to_string()),
            ..Default::default()
        };

        let collector = TraceCollector::new(config);
        let result = collector.collect_traces().await;

        // Should fail due to unreachable endpoint
        assert!(result.is_err());
    }
}
