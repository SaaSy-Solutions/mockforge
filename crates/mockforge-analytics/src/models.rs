//! Data models for analytics

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Granularity level for aggregated metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Granularity {
    /// Minute-level granularity (60 seconds)
    Minute,
    /// Hour-level granularity (3600 seconds)
    Hour,
    /// Day-level granularity (86400 seconds)
    Day,
}

/// Aggregated metrics for a specific time window
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct MetricsAggregate {
    /// Row ID
    pub id: Option<i64>,
    /// Unix timestamp for the aggregation window
    pub timestamp: i64,
    /// Protocol name (e.g., "http", "grpc")
    pub protocol: String,
    /// HTTP method (e.g., "GET", "POST")
    pub method: Option<String>,
    /// Endpoint path
    pub endpoint: Option<String>,
    /// HTTP status code
    pub status_code: Option<i32>,
    /// Workspace identifier
    pub workspace_id: Option<String>,
    /// Deployment environment
    pub environment: Option<String>,
    /// Total request count in this window
    pub request_count: i64,
    /// Total error count in this window
    pub error_count: i64,
    /// Sum of all latencies in milliseconds
    pub latency_sum: f64,
    /// Minimum latency in milliseconds
    pub latency_min: Option<f64>,
    /// Maximum latency in milliseconds
    pub latency_max: Option<f64>,
    /// 50th percentile latency
    pub latency_p50: Option<f64>,
    /// 95th percentile latency
    pub latency_p95: Option<f64>,
    /// 99th percentile latency
    pub latency_p99: Option<f64>,
    /// Total bytes sent
    pub bytes_sent: i64,
    /// Total bytes received
    pub bytes_received: i64,
    /// Number of active connections
    pub active_connections: Option<i64>,
    /// Row creation timestamp
    pub created_at: Option<i64>,
}

/// Hour-level aggregated metrics
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct HourMetricsAggregate {
    /// Row ID
    pub id: Option<i64>,
    /// Unix timestamp for the hour window
    pub timestamp: i64,
    /// Protocol name
    pub protocol: String,
    /// HTTP method
    pub method: Option<String>,
    /// Endpoint path
    pub endpoint: Option<String>,
    /// HTTP status code
    pub status_code: Option<i32>,
    /// Workspace identifier
    pub workspace_id: Option<String>,
    /// Deployment environment
    pub environment: Option<String>,
    /// Total request count
    pub request_count: i64,
    /// Total error count
    pub error_count: i64,
    /// Sum of all latencies
    pub latency_sum: f64,
    /// Minimum latency
    pub latency_min: Option<f64>,
    /// Maximum latency
    pub latency_max: Option<f64>,
    /// 50th percentile latency
    pub latency_p50: Option<f64>,
    /// 95th percentile latency
    pub latency_p95: Option<f64>,
    /// 99th percentile latency
    pub latency_p99: Option<f64>,
    /// Total bytes sent
    pub bytes_sent: i64,
    /// Total bytes received
    pub bytes_received: i64,
    /// Average active connections during the hour
    pub active_connections_avg: Option<f64>,
    /// Maximum active connections during the hour
    pub active_connections_max: Option<i64>,
    /// Row creation timestamp
    pub created_at: Option<i64>,
}

/// Day-level aggregated metrics
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DayMetricsAggregate {
    /// Row ID
    pub id: Option<i64>,
    /// Date string (YYYY-MM-DD)
    pub date: String,
    /// Unix timestamp for the day
    pub timestamp: i64,
    /// Protocol name
    pub protocol: String,
    /// HTTP method
    pub method: Option<String>,
    /// Endpoint path
    pub endpoint: Option<String>,
    /// HTTP status code
    pub status_code: Option<i32>,
    /// Workspace identifier
    pub workspace_id: Option<String>,
    /// Deployment environment
    pub environment: Option<String>,
    /// Total request count
    pub request_count: i64,
    /// Total error count
    pub error_count: i64,
    /// Sum of all latencies
    pub latency_sum: f64,
    /// Minimum latency
    pub latency_min: Option<f64>,
    /// Maximum latency
    pub latency_max: Option<f64>,
    /// 50th percentile latency
    pub latency_p50: Option<f64>,
    /// 95th percentile latency
    pub latency_p95: Option<f64>,
    /// 99th percentile latency
    pub latency_p99: Option<f64>,
    /// Total bytes sent
    pub bytes_sent: i64,
    /// Total bytes received
    pub bytes_received: i64,
    /// Average active connections
    pub active_connections_avg: Option<f64>,
    /// Maximum active connections
    pub active_connections_max: Option<i64>,
    /// Number of unique clients
    pub unique_clients: Option<i64>,
    /// Hour with the most requests (0-23)
    pub peak_hour: Option<i32>,
    /// Row creation timestamp
    pub created_at: Option<i64>,
}

/// Statistics for a specific endpoint
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EndpointStats {
    /// Row ID
    pub id: Option<i64>,
    /// Endpoint path
    pub endpoint: String,
    /// Protocol name
    pub protocol: String,
    /// HTTP method
    pub method: Option<String>,
    /// Workspace identifier
    pub workspace_id: Option<String>,
    /// Deployment environment
    pub environment: Option<String>,
    /// Total number of requests
    pub total_requests: i64,
    /// Total number of errors
    pub total_errors: i64,
    /// Average latency in milliseconds
    pub avg_latency_ms: Option<f64>,
    /// Minimum latency in milliseconds
    pub min_latency_ms: Option<f64>,
    /// Maximum latency in milliseconds
    pub max_latency_ms: Option<f64>,
    /// 95th percentile latency in milliseconds
    pub p95_latency_ms: Option<f64>,
    /// JSON-encoded status code breakdown
    pub status_codes: Option<String>,
    /// Total bytes sent
    pub total_bytes_sent: i64,
    /// Total bytes received
    pub total_bytes_received: i64,
    /// Unix timestamp of first request
    pub first_seen: i64,
    /// Unix timestamp of most recent request
    pub last_seen: i64,
    /// Last update timestamp
    pub updated_at: Option<i64>,
}

/// Parsed status code breakdown from endpoint stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusCodeBreakdown {
    /// Map of HTTP status codes to their occurrence counts
    pub status_codes: HashMap<u16, i64>,
}

impl EndpointStats {
    /// Parse the status codes JSON field
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON status codes field cannot be deserialized.
    pub fn get_status_code_breakdown(&self) -> Result<StatusCodeBreakdown, serde_json::Error> {
        if let Some(ref json) = self.status_codes {
            let map: HashMap<String, i64> = serde_json::from_str(json)?;
            let status_codes = map
                .into_iter()
                .filter_map(|(k, v)| k.parse::<u16>().ok().map(|code| (code, v)))
                .collect();
            Ok(StatusCodeBreakdown { status_codes })
        } else {
            Ok(StatusCodeBreakdown {
                status_codes: HashMap::new(),
            })
        }
    }

    /// Set the status codes from a breakdown
    ///
    /// # Errors
    ///
    /// Returns an error if the status codes cannot be serialized to JSON.
    pub fn set_status_code_breakdown(
        &mut self,
        breakdown: &StatusCodeBreakdown,
    ) -> Result<(), serde_json::Error> {
        let map: HashMap<String, i64> =
            breakdown.status_codes.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        self.status_codes = Some(serde_json::to_string(&map)?);
        Ok(())
    }
}

/// Individual error event
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ErrorEvent {
    /// Row ID
    pub id: Option<i64>,
    /// Unix timestamp of the error
    pub timestamp: i64,
    /// Protocol name
    pub protocol: String,
    /// HTTP method
    pub method: Option<String>,
    /// Endpoint path
    pub endpoint: Option<String>,
    /// HTTP status code
    pub status_code: Option<i32>,
    /// Error type classification
    pub error_type: Option<String>,
    /// Human-readable error message
    pub error_message: Option<String>,
    /// Error category (`client_error`, `server_error`, etc.)
    pub error_category: Option<String>,
    /// Unique request identifier
    pub request_id: Option<String>,
    /// Distributed trace identifier
    pub trace_id: Option<String>,
    /// Distributed span identifier
    pub span_id: Option<String>,
    /// Client IP address
    pub client_ip: Option<String>,
    /// Client user agent string
    pub user_agent: Option<String>,
    /// Workspace identifier
    pub workspace_id: Option<String>,
    /// Deployment environment
    pub environment: Option<String>,
    /// JSON-encoded additional metadata
    pub metadata: Option<String>,
    /// Row creation timestamp
    pub created_at: Option<i64>,
}

/// Error category enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// HTTP 4xx client errors
    ClientError,
    /// HTTP 5xx server errors
    ServerError,
    /// Network-level errors
    NetworkError,
    /// Timeout errors
    TimeoutError,
    /// Uncategorized errors
    Other,
}

impl ErrorCategory {
    /// Get the category from a status code
    #[must_use]
    pub const fn from_status_code(status_code: u16) -> Self {
        match status_code {
            400..=499 => Self::ClientError,
            500..=599 => Self::ServerError,
            _ => Self::Other,
        }
    }

    /// Convert to string representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::ClientError => "client_error",
            Self::ServerError => "server_error",
            Self::NetworkError => "network_error",
            Self::TimeoutError => "timeout_error",
            Self::Other => "other",
        }
    }
}

/// Client analytics data
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ClientAnalytics {
    /// Row ID
    pub id: Option<i64>,
    /// Unix timestamp
    pub timestamp: i64,
    /// Client IP address
    pub client_ip: String,
    /// Full user agent string
    pub user_agent: Option<String>,
    /// Parsed user agent family
    pub user_agent_family: Option<String>,
    /// Parsed user agent version
    pub user_agent_version: Option<String>,
    /// Protocol name
    pub protocol: String,
    /// Workspace identifier
    pub workspace_id: Option<String>,
    /// Deployment environment
    pub environment: Option<String>,
    /// Total request count
    pub request_count: i64,
    /// Total error count
    pub error_count: i64,
    /// Average latency in milliseconds
    pub avg_latency_ms: Option<f64>,
    /// Total bytes sent
    pub bytes_sent: i64,
    /// Total bytes received
    pub bytes_received: i64,
    /// JSON array of most-accessed endpoints
    pub top_endpoints: Option<String>,
    /// Row creation timestamp
    pub created_at: Option<i64>,
}

/// Traffic pattern data for heatmap visualization
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TrafficPattern {
    /// Row ID
    pub id: Option<i64>,
    /// Date string (YYYY-MM-DD)
    pub date: String,
    /// Hour of day (0-23)
    pub hour: i32,
    /// Day of week (0-6, Monday=0)
    pub day_of_week: i32,
    /// Protocol name
    pub protocol: String,
    /// Workspace identifier
    pub workspace_id: Option<String>,
    /// Deployment environment
    pub environment: Option<String>,
    /// Total request count
    pub request_count: i64,
    /// Total error count
    pub error_count: i64,
    /// Average latency in milliseconds
    pub avg_latency_ms: Option<f64>,
    /// Number of unique clients
    pub unique_clients: Option<i64>,
    /// Row creation timestamp
    pub created_at: Option<i64>,
}

/// Analytics snapshot for comparison and trending
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AnalyticsSnapshot {
    /// Row ID
    pub id: Option<i64>,
    /// Unix timestamp of the snapshot
    pub timestamp: i64,
    /// Snapshot type identifier
    pub snapshot_type: String,
    /// Total requests at snapshot time
    pub total_requests: i64,
    /// Total errors at snapshot time
    pub total_errors: i64,
    /// Average latency in milliseconds
    pub avg_latency_ms: Option<f64>,
    /// Number of active connections
    pub active_connections: Option<i64>,
    /// JSON-encoded protocol statistics
    pub protocol_stats: Option<String>,
    /// JSON array of top endpoints
    pub top_endpoints: Option<String>,
    /// Memory usage in bytes
    pub memory_usage_bytes: Option<i64>,
    /// CPU usage percentage
    pub cpu_usage_percent: Option<f64>,
    /// Number of active threads
    pub thread_count: Option<i32>,
    /// Server uptime in seconds
    pub uptime_seconds: Option<i64>,
    /// Workspace identifier
    pub workspace_id: Option<String>,
    /// Deployment environment
    pub environment: Option<String>,
    /// Row creation timestamp
    pub created_at: Option<i64>,
}

/// Query filter for analytics queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalyticsFilter {
    /// Start of time range (unix timestamp)
    pub start_time: Option<i64>,
    /// End of time range (unix timestamp)
    pub end_time: Option<i64>,
    /// Filter by protocol
    pub protocol: Option<String>,
    /// Filter by endpoint path
    pub endpoint: Option<String>,
    /// Filter by HTTP method
    pub method: Option<String>,
    /// Filter by HTTP status code
    pub status_code: Option<i32>,
    /// Filter by workspace
    pub workspace_id: Option<String>,
    /// Filter by environment
    pub environment: Option<String>,
    /// Maximum number of results
    pub limit: Option<i64>,
}

/// Overview metrics for the dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewMetrics {
    /// Total number of requests
    pub total_requests: i64,
    /// Total number of errors
    pub total_errors: i64,
    /// Error rate as a percentage
    pub error_rate: f64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// 95th percentile latency in milliseconds
    pub p95_latency_ms: f64,
    /// 99th percentile latency in milliseconds
    pub p99_latency_ms: f64,
    /// Number of active connections
    pub active_connections: i64,
    /// Total bytes sent
    pub total_bytes_sent: i64,
    /// Total bytes received
    pub total_bytes_received: i64,
    /// Requests per second
    pub requests_per_second: f64,
    /// Top protocols by request count
    pub top_protocols: Vec<ProtocolStat>,
    /// Top endpoints by request count
    pub top_endpoints: Vec<EndpointStat>,
}

/// Protocol statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolStat {
    /// Protocol name
    pub protocol: String,
    /// Total request count
    pub request_count: i64,
    /// Total error count
    pub error_count: i64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
}

/// Endpoint statistics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointStat {
    /// Endpoint path
    pub endpoint: String,
    /// Protocol name
    pub protocol: String,
    /// HTTP method
    pub method: Option<String>,
    /// Total request count
    pub request_count: i64,
    /// Total error count
    pub error_count: i64,
    /// Error rate as a percentage
    pub error_rate: f64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// 95th percentile latency in milliseconds
    pub p95_latency_ms: f64,
}

/// Time series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    /// Unix timestamp
    pub timestamp: i64,
    /// Metric value at this point
    pub value: f64,
}

/// Time series with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries {
    /// Label identifying this series
    pub label: String,
    /// Ordered data points
    pub data: Vec<TimeSeriesPoint>,
}

/// Latency percentiles over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyTrend {
    /// Unix timestamp
    pub timestamp: i64,
    /// 50th percentile latency
    pub p50: f64,
    /// 95th percentile latency
    pub p95: f64,
    /// 99th percentile latency
    pub p99: f64,
    /// Average latency
    pub avg: f64,
    /// Minimum latency
    pub min: f64,
    /// Maximum latency
    pub max: f64,
}

/// Error summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSummary {
    /// Error type classification
    pub error_type: String,
    /// Error category
    pub error_category: String,
    /// Number of occurrences
    pub count: i64,
    /// Endpoints affected
    pub endpoints: Vec<String>,
    /// Timestamp of most recent occurrence
    pub last_occurrence: DateTime<Utc>,
}

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// Comma-separated values format
    Csv,
    /// JSON format
    Json,
}

// ============================================================================
// Coverage Metrics Models (MockOps)
// ============================================================================

/// Scenario usage metrics
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ScenarioUsageMetrics {
    /// Row ID
    pub id: Option<i64>,
    /// Scenario identifier
    pub scenario_id: String,
    /// Workspace identifier
    pub workspace_id: Option<String>,
    /// Organization identifier
    pub org_id: Option<String>,
    /// Number of times this scenario has been used
    pub usage_count: i64,
    /// Unix timestamp of last usage
    pub last_used_at: Option<i64>,
    /// JSON-encoded usage pattern data
    pub usage_pattern: Option<String>,
    /// Row creation timestamp
    pub created_at: Option<i64>,
    /// Last update timestamp
    pub updated_at: Option<i64>,
}

/// Persona CI hit record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PersonaCIHit {
    /// Row ID
    pub id: Option<i64>,
    /// Persona identifier
    pub persona_id: String,
    /// Workspace identifier
    pub workspace_id: Option<String>,
    /// Organization identifier
    pub org_id: Option<String>,
    /// CI run identifier
    pub ci_run_id: Option<String>,
    /// Number of hits in this CI run
    pub hit_count: i64,
    /// Unix timestamp of the hit
    pub hit_at: i64,
    /// Row creation timestamp
    pub created_at: Option<i64>,
}

/// Endpoint coverage metrics
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EndpointCoverage {
    /// Row ID
    pub id: Option<i64>,
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: Option<String>,
    /// Protocol name
    pub protocol: String,
    /// Workspace identifier
    pub workspace_id: Option<String>,
    /// Organization identifier
    pub org_id: Option<String>,
    /// Number of tests covering this endpoint
    pub test_count: i64,
    /// Unix timestamp of last test run
    pub last_tested_at: Option<i64>,
    /// Coverage percentage (0-100)
    pub coverage_percentage: Option<f64>,
    /// Row creation timestamp
    pub created_at: Option<i64>,
    /// Last update timestamp
    pub updated_at: Option<i64>,
}

/// Reality level staleness record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RealityLevelStaleness {
    /// Row ID
    pub id: Option<i64>,
    /// Workspace identifier
    pub workspace_id: String,
    /// Organization identifier
    pub org_id: Option<String>,
    /// Endpoint path
    pub endpoint: Option<String>,
    /// HTTP method
    pub method: Option<String>,
    /// Protocol name
    pub protocol: Option<String>,
    /// Current reality level setting
    pub current_reality_level: Option<String>,
    /// Unix timestamp of last reality level update
    pub last_updated_at: Option<i64>,
    /// Number of days since last update
    pub staleness_days: Option<i32>,
    /// Row creation timestamp
    pub created_at: Option<i64>,
    /// Last update timestamp
    pub updated_at: Option<i64>,
}

/// Drift percentage metrics
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DriftPercentageMetrics {
    /// Row ID
    pub id: Option<i64>,
    /// Workspace identifier
    pub workspace_id: String,
    /// Organization identifier
    pub org_id: Option<String>,
    /// Total number of mock endpoints
    pub total_mocks: i64,
    /// Number of mocks that have drifted from spec
    pub drifting_mocks: i64,
    /// Drift percentage (0-100)
    pub drift_percentage: f64,
    /// Unix timestamp of measurement
    pub measured_at: i64,
    /// Row creation timestamp
    pub created_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Granularity Tests ====================

    #[test]
    fn test_granularity_serialize() {
        let minute = Granularity::Minute;
        let hour = Granularity::Hour;
        let day = Granularity::Day;

        assert_eq!(serde_json::to_string(&minute).unwrap(), "\"minute\"");
        assert_eq!(serde_json::to_string(&hour).unwrap(), "\"hour\"");
        assert_eq!(serde_json::to_string(&day).unwrap(), "\"day\"");
    }

    #[test]
    fn test_granularity_deserialize() {
        let minute: Granularity = serde_json::from_str("\"minute\"").unwrap();
        let hour: Granularity = serde_json::from_str("\"hour\"").unwrap();
        let day: Granularity = serde_json::from_str("\"day\"").unwrap();

        assert_eq!(minute, Granularity::Minute);
        assert_eq!(hour, Granularity::Hour);
        assert_eq!(day, Granularity::Day);
    }

    #[test]
    fn test_granularity_copy() {
        let g = Granularity::Hour;
        let copied = g;
        assert_eq!(g, copied);
    }

    // ==================== ErrorCategory Tests ====================

    #[test]
    fn test_error_category_from_status_code_client() {
        assert_eq!(ErrorCategory::from_status_code(400), ErrorCategory::ClientError);
        assert_eq!(ErrorCategory::from_status_code(401), ErrorCategory::ClientError);
        assert_eq!(ErrorCategory::from_status_code(403), ErrorCategory::ClientError);
        assert_eq!(ErrorCategory::from_status_code(404), ErrorCategory::ClientError);
        assert_eq!(ErrorCategory::from_status_code(499), ErrorCategory::ClientError);
    }

    #[test]
    fn test_error_category_from_status_code_server() {
        assert_eq!(ErrorCategory::from_status_code(500), ErrorCategory::ServerError);
        assert_eq!(ErrorCategory::from_status_code(502), ErrorCategory::ServerError);
        assert_eq!(ErrorCategory::from_status_code(503), ErrorCategory::ServerError);
        assert_eq!(ErrorCategory::from_status_code(504), ErrorCategory::ServerError);
        assert_eq!(ErrorCategory::from_status_code(599), ErrorCategory::ServerError);
    }

    #[test]
    fn test_error_category_from_status_code_other() {
        assert_eq!(ErrorCategory::from_status_code(200), ErrorCategory::Other);
        assert_eq!(ErrorCategory::from_status_code(301), ErrorCategory::Other);
        assert_eq!(ErrorCategory::from_status_code(0), ErrorCategory::Other);
    }

    #[test]
    fn test_error_category_as_str() {
        assert_eq!(ErrorCategory::ClientError.as_str(), "client_error");
        assert_eq!(ErrorCategory::ServerError.as_str(), "server_error");
        assert_eq!(ErrorCategory::NetworkError.as_str(), "network_error");
        assert_eq!(ErrorCategory::TimeoutError.as_str(), "timeout_error");
        assert_eq!(ErrorCategory::Other.as_str(), "other");
    }

    #[test]
    fn test_error_category_serialize() {
        assert_eq!(serde_json::to_string(&ErrorCategory::ClientError).unwrap(), "\"client_error\"");
        assert_eq!(serde_json::to_string(&ErrorCategory::ServerError).unwrap(), "\"server_error\"");
    }

    // ==================== ExportFormat Tests ====================

    #[test]
    fn test_export_format_serialize() {
        assert_eq!(serde_json::to_string(&ExportFormat::Csv).unwrap(), "\"csv\"");
        assert_eq!(serde_json::to_string(&ExportFormat::Json).unwrap(), "\"json\"");
    }

    #[test]
    fn test_export_format_deserialize() {
        let csv: ExportFormat = serde_json::from_str("\"csv\"").unwrap();
        let json_fmt: ExportFormat = serde_json::from_str("\"json\"").unwrap();

        assert_eq!(csv, ExportFormat::Csv);
        assert_eq!(json_fmt, ExportFormat::Json);
    }

    // ==================== EndpointStats Tests ====================

    #[test]
    fn test_endpoint_stats_get_status_code_breakdown() {
        let stats = EndpointStats {
            id: Some(1),
            endpoint: "/api/users".to_string(),
            protocol: "http".to_string(),
            method: Some("GET".to_string()),
            workspace_id: None,
            environment: None,
            total_requests: 100,
            total_errors: 5,
            avg_latency_ms: Some(50.0),
            min_latency_ms: Some(10.0),
            max_latency_ms: Some(200.0),
            p95_latency_ms: Some(150.0),
            status_codes: Some(r#"{"200": 90, "404": 5, "500": 5}"#.to_string()),
            total_bytes_sent: 10_000,
            total_bytes_received: 5_000,
            first_seen: 1000,
            last_seen: 2000,
            updated_at: None,
        };

        let breakdown = stats.get_status_code_breakdown().unwrap();
        assert_eq!(breakdown.status_codes.get(&200), Some(&90));
        assert_eq!(breakdown.status_codes.get(&404), Some(&5));
        assert_eq!(breakdown.status_codes.get(&500), Some(&5));
    }

    #[test]
    fn test_endpoint_stats_get_status_code_breakdown_none() {
        let stats = EndpointStats {
            id: None,
            endpoint: "/api/test".to_string(),
            protocol: "http".to_string(),
            method: None,
            workspace_id: None,
            environment: None,
            total_requests: 0,
            total_errors: 0,
            avg_latency_ms: None,
            min_latency_ms: None,
            max_latency_ms: None,
            p95_latency_ms: None,
            status_codes: None,
            total_bytes_sent: 0,
            total_bytes_received: 0,
            first_seen: 0,
            last_seen: 0,
            updated_at: None,
        };

        let breakdown = stats.get_status_code_breakdown().unwrap();
        assert!(breakdown.status_codes.is_empty());
    }

    #[test]
    fn test_endpoint_stats_set_status_code_breakdown() {
        let mut stats = EndpointStats {
            id: None,
            endpoint: "/api/test".to_string(),
            protocol: "http".to_string(),
            method: None,
            workspace_id: None,
            environment: None,
            total_requests: 0,
            total_errors: 0,
            avg_latency_ms: None,
            min_latency_ms: None,
            max_latency_ms: None,
            p95_latency_ms: None,
            status_codes: None,
            total_bytes_sent: 0,
            total_bytes_received: 0,
            first_seen: 0,
            last_seen: 0,
            updated_at: None,
        };

        let breakdown = StatusCodeBreakdown {
            status_codes: HashMap::from([(200, 100), (500, 10)]),
        };

        stats.set_status_code_breakdown(&breakdown).unwrap();
        assert!(stats.status_codes.is_some());

        // Verify roundtrip
        let restored = stats.get_status_code_breakdown().unwrap();
        assert_eq!(restored.status_codes.get(&200), Some(&100));
        assert_eq!(restored.status_codes.get(&500), Some(&10));
    }

    // ==================== AnalyticsFilter Tests ====================

    #[test]
    fn test_analytics_filter_default() {
        let filter = AnalyticsFilter::default();
        assert!(filter.start_time.is_none());
        assert!(filter.end_time.is_none());
        assert!(filter.protocol.is_none());
        assert!(filter.endpoint.is_none());
        assert!(filter.limit.is_none());
    }

    #[test]
    fn test_analytics_filter_serialize() {
        let filter = AnalyticsFilter {
            start_time: Some(1000),
            end_time: Some(2000),
            protocol: Some("http".to_string()),
            endpoint: Some("/api/users".to_string()),
            method: Some("GET".to_string()),
            status_code: Some(200),
            workspace_id: None,
            environment: None,
            limit: Some(100),
        };

        let json = serde_json::to_string(&filter).unwrap();
        assert!(json.contains("1000"));
        assert!(json.contains("http"));
        assert!(json.contains("/api/users"));
    }

    #[test]
    fn test_analytics_filter_clone() {
        let filter = AnalyticsFilter {
            start_time: Some(1000),
            protocol: Some("grpc".to_string()),
            ..Default::default()
        };

        let cloned = filter.clone();
        assert_eq!(filter.start_time, cloned.start_time);
        assert_eq!(filter.protocol, cloned.protocol);
    }

    // ==================== OverviewMetrics Tests ====================

    #[test]
    fn test_overview_metrics_serialize() {
        let metrics = OverviewMetrics {
            total_requests: 1000,
            total_errors: 50,
            error_rate: 0.05,
            avg_latency_ms: 100.0,
            p95_latency_ms: 250.0,
            p99_latency_ms: 500.0,
            active_connections: 10,
            total_bytes_sent: 100_000,
            total_bytes_received: 50_000,
            requests_per_second: 10.5,
            top_protocols: vec![],
            top_endpoints: vec![],
        };

        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("1000"));
        assert!(json.contains("0.05"));
    }

    // ==================== ProtocolStat Tests ====================

    #[test]
    fn test_protocol_stat_serialize() {
        let stat = ProtocolStat {
            protocol: "http".to_string(),
            request_count: 1000,
            error_count: 10,
            avg_latency_ms: 50.0,
        };

        let json = serde_json::to_string(&stat).unwrap();
        assert!(json.contains("http"));
        assert!(json.contains("1000"));
    }

    // ==================== EndpointStat Tests ====================

    #[test]
    fn test_endpoint_stat_serialize() {
        let stat = EndpointStat {
            endpoint: "/api/users".to_string(),
            protocol: "http".to_string(),
            method: Some("GET".to_string()),
            request_count: 500,
            error_count: 5,
            error_rate: 0.01,
            avg_latency_ms: 75.0,
            p95_latency_ms: 150.0,
        };

        let json = serde_json::to_string(&stat).unwrap();
        assert!(json.contains("/api/users"));
        assert!(json.contains("GET"));
    }

    // ==================== TimeSeriesPoint Tests ====================

    #[test]
    fn test_time_series_point_serialize() {
        let point = TimeSeriesPoint {
            timestamp: 1_234_567_890,
            value: 42.5,
        };

        let json = serde_json::to_string(&point).unwrap();
        assert!(json.contains("1234567890"));
        assert!(json.contains("42.5"));
    }

    // ==================== TimeSeries Tests ====================

    #[test]
    fn test_time_series_serialize() {
        let series = TimeSeries {
            label: "requests".to_string(),
            data: vec![
                TimeSeriesPoint {
                    timestamp: 1000,
                    value: 10.0,
                },
                TimeSeriesPoint {
                    timestamp: 2000,
                    value: 20.0,
                },
            ],
        };

        let json = serde_json::to_string(&series).unwrap();
        assert!(json.contains("requests"));
        assert!(json.contains("1000"));
    }

    // ==================== LatencyTrend Tests ====================

    #[test]
    fn test_latency_trend_serialize() {
        let trend = LatencyTrend {
            timestamp: 1000,
            p50: 50.0,
            p95: 150.0,
            p99: 250.0,
            avg: 75.0,
            min: 10.0,
            max: 500.0,
        };

        let json = serde_json::to_string(&trend).unwrap();
        assert!(json.contains("p50"));
        assert!(json.contains("p95"));
        assert!(json.contains("p99"));
    }

    // ==================== MetricsAggregate Tests ====================

    #[test]
    fn test_metrics_aggregate_clone() {
        let agg = MetricsAggregate {
            id: Some(1),
            timestamp: 1000,
            protocol: "http".to_string(),
            method: Some("GET".to_string()),
            endpoint: Some("/api/users".to_string()),
            status_code: Some(200),
            workspace_id: None,
            environment: None,
            request_count: 100,
            error_count: 5,
            latency_sum: 5000.0,
            latency_min: Some(10.0),
            latency_max: Some(200.0),
            latency_p50: Some(50.0),
            latency_p95: Some(150.0),
            latency_p99: Some(180.0),
            bytes_sent: 10000,
            bytes_received: 5000,
            active_connections: Some(10),
            created_at: None,
        };

        let cloned = agg.clone();
        assert_eq!(agg.timestamp, cloned.timestamp);
        assert_eq!(agg.request_count, cloned.request_count);
    }

    // ==================== ErrorEvent Tests ====================

    #[test]
    fn test_error_event_serialize() {
        let event = ErrorEvent {
            id: Some(1),
            timestamp: 1000,
            protocol: "http".to_string(),
            method: Some("POST".to_string()),
            endpoint: Some("/api/orders".to_string()),
            status_code: Some(500),
            error_type: Some("InternalServerError".to_string()),
            error_message: Some("Database connection failed".to_string()),
            error_category: Some("server_error".to_string()),
            request_id: Some("req-123".to_string()),
            trace_id: None,
            span_id: None,
            client_ip: Some("192.168.1.1".to_string()),
            user_agent: Some("TestClient/1.0".to_string()),
            workspace_id: None,
            environment: None,
            metadata: None,
            created_at: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("InternalServerError"));
        assert!(json.contains("Database connection failed"));
    }

    // ==================== TrafficPattern Tests ====================

    #[test]
    fn test_traffic_pattern_serialize() {
        let pattern = TrafficPattern {
            id: Some(1),
            date: "2024-01-15".to_string(),
            hour: 14,
            day_of_week: 1,
            protocol: "http".to_string(),
            workspace_id: None,
            environment: None,
            request_count: 500,
            error_count: 10,
            avg_latency_ms: Some(50.0),
            unique_clients: Some(25),
            created_at: None,
        };

        let json = serde_json::to_string(&pattern).unwrap();
        assert!(json.contains("2024-01-15"));
        assert!(json.contains("14"));
    }

    // ==================== DriftPercentageMetrics Tests ====================

    #[test]
    fn test_drift_percentage_metrics_serialize() {
        let metrics = DriftPercentageMetrics {
            id: Some(1),
            workspace_id: "ws-123".to_string(),
            org_id: Some("org-456".to_string()),
            total_mocks: 100,
            drifting_mocks: 15,
            drift_percentage: 15.0,
            measured_at: 1000,
            created_at: None,
        };

        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("ws-123"));
        assert!(json.contains("15.0"));
    }
}
