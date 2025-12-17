//! Data models for analytics

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Granularity level for aggregated metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Granularity {
    Minute,
    Hour,
    Day,
}

/// Aggregated metrics for a specific time window
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MetricsAggregate {
    pub id: Option<i64>,
    pub timestamp: i64,
    pub protocol: String,
    pub method: Option<String>,
    pub endpoint: Option<String>,
    pub status_code: Option<i32>,
    pub workspace_id: Option<String>,
    pub environment: Option<String>,
    pub request_count: i64,
    pub error_count: i64,
    pub latency_sum: f64,
    pub latency_min: Option<f64>,
    pub latency_max: Option<f64>,
    pub latency_p50: Option<f64>,
    pub latency_p95: Option<f64>,
    pub latency_p99: Option<f64>,
    pub bytes_sent: i64,
    pub bytes_received: i64,
    pub active_connections: Option<i64>,
    pub created_at: Option<i64>,
}

/// Hour-level aggregated metrics
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct HourMetricsAggregate {
    pub id: Option<i64>,
    pub timestamp: i64,
    pub protocol: String,
    pub method: Option<String>,
    pub endpoint: Option<String>,
    pub status_code: Option<i32>,
    pub workspace_id: Option<String>,
    pub environment: Option<String>,
    pub request_count: i64,
    pub error_count: i64,
    pub latency_sum: f64,
    pub latency_min: Option<f64>,
    pub latency_max: Option<f64>,
    pub latency_p50: Option<f64>,
    pub latency_p95: Option<f64>,
    pub latency_p99: Option<f64>,
    pub bytes_sent: i64,
    pub bytes_received: i64,
    pub active_connections_avg: Option<f64>,
    pub active_connections_max: Option<i64>,
    pub created_at: Option<i64>,
}

/// Day-level aggregated metrics
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DayMetricsAggregate {
    pub id: Option<i64>,
    pub date: String,
    pub timestamp: i64,
    pub protocol: String,
    pub method: Option<String>,
    pub endpoint: Option<String>,
    pub status_code: Option<i32>,
    pub workspace_id: Option<String>,
    pub environment: Option<String>,
    pub request_count: i64,
    pub error_count: i64,
    pub latency_sum: f64,
    pub latency_min: Option<f64>,
    pub latency_max: Option<f64>,
    pub latency_p50: Option<f64>,
    pub latency_p95: Option<f64>,
    pub latency_p99: Option<f64>,
    pub bytes_sent: i64,
    pub bytes_received: i64,
    pub active_connections_avg: Option<f64>,
    pub active_connections_max: Option<i64>,
    pub unique_clients: Option<i64>,
    pub peak_hour: Option<i32>,
    pub created_at: Option<i64>,
}

/// Statistics for a specific endpoint
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EndpointStats {
    pub id: Option<i64>,
    pub endpoint: String,
    pub protocol: String,
    pub method: Option<String>,
    pub workspace_id: Option<String>,
    pub environment: Option<String>,
    pub total_requests: i64,
    pub total_errors: i64,
    pub avg_latency_ms: Option<f64>,
    pub min_latency_ms: Option<f64>,
    pub max_latency_ms: Option<f64>,
    pub p95_latency_ms: Option<f64>,
    pub status_codes: Option<String>, // JSON
    pub total_bytes_sent: i64,
    pub total_bytes_received: i64,
    pub first_seen: i64,
    pub last_seen: i64,
    pub updated_at: Option<i64>,
}

/// Parsed status code breakdown from endpoint stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusCodeBreakdown {
    pub status_codes: HashMap<u16, i64>,
}

impl EndpointStats {
    /// Parse the status codes JSON field
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
    pub id: Option<i64>,
    pub timestamp: i64,
    pub protocol: String,
    pub method: Option<String>,
    pub endpoint: Option<String>,
    pub status_code: Option<i32>,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    pub error_category: Option<String>,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub workspace_id: Option<String>,
    pub environment: Option<String>,
    pub metadata: Option<String>, // JSON
    pub created_at: Option<i64>,
}

/// Error category enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    ClientError, // 4xx
    ServerError, // 5xx
    NetworkError,
    TimeoutError,
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
    pub id: Option<i64>,
    pub timestamp: i64,
    pub client_ip: String,
    pub user_agent: Option<String>,
    pub user_agent_family: Option<String>,
    pub user_agent_version: Option<String>,
    pub protocol: String,
    pub workspace_id: Option<String>,
    pub environment: Option<String>,
    pub request_count: i64,
    pub error_count: i64,
    pub avg_latency_ms: Option<f64>,
    pub bytes_sent: i64,
    pub bytes_received: i64,
    pub top_endpoints: Option<String>, // JSON array
    pub created_at: Option<i64>,
}

/// Traffic pattern data for heatmap visualization
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TrafficPattern {
    pub id: Option<i64>,
    pub date: String,
    pub hour: i32,
    pub day_of_week: i32,
    pub protocol: String,
    pub workspace_id: Option<String>,
    pub environment: Option<String>,
    pub request_count: i64,
    pub error_count: i64,
    pub avg_latency_ms: Option<f64>,
    pub unique_clients: Option<i64>,
    pub created_at: Option<i64>,
}

/// Analytics snapshot for comparison and trending
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AnalyticsSnapshot {
    pub id: Option<i64>,
    pub timestamp: i64,
    pub snapshot_type: String,
    pub total_requests: i64,
    pub total_errors: i64,
    pub avg_latency_ms: Option<f64>,
    pub active_connections: Option<i64>,
    pub protocol_stats: Option<String>, // JSON
    pub top_endpoints: Option<String>,  // JSON array
    pub memory_usage_bytes: Option<i64>,
    pub cpu_usage_percent: Option<f64>,
    pub thread_count: Option<i32>,
    pub uptime_seconds: Option<i64>,
    pub workspace_id: Option<String>,
    pub environment: Option<String>,
    pub created_at: Option<i64>,
}

/// Query filter for analytics queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalyticsFilter {
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub protocol: Option<String>,
    pub endpoint: Option<String>,
    pub method: Option<String>,
    pub status_code: Option<i32>,
    pub workspace_id: Option<String>,
    pub environment: Option<String>,
    pub limit: Option<i64>,
}

/// Overview metrics for the dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewMetrics {
    pub total_requests: i64,
    pub total_errors: i64,
    pub error_rate: f64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub active_connections: i64,
    pub total_bytes_sent: i64,
    pub total_bytes_received: i64,
    pub requests_per_second: f64,
    pub top_protocols: Vec<ProtocolStat>,
    pub top_endpoints: Vec<EndpointStat>,
}

/// Protocol statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolStat {
    pub protocol: String,
    pub request_count: i64,
    pub error_count: i64,
    pub avg_latency_ms: f64,
}

/// Endpoint statistics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointStat {
    pub endpoint: String,
    pub protocol: String,
    pub method: Option<String>,
    pub request_count: i64,
    pub error_count: i64,
    pub error_rate: f64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
}

/// Time series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: i64,
    pub value: f64,
}

/// Time series with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries {
    pub label: String,
    pub data: Vec<TimeSeriesPoint>,
}

/// Latency percentiles over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyTrend {
    pub timestamp: i64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub avg: f64,
    pub min: f64,
    pub max: f64,
}

/// Error summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSummary {
    pub error_type: String,
    pub error_category: String,
    pub count: i64,
    pub endpoints: Vec<String>,
    pub last_occurrence: DateTime<Utc>,
}

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Csv,
    Json,
}

// ============================================================================
// Coverage Metrics Models (MockOps)
// ============================================================================

/// Scenario usage metrics
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ScenarioUsageMetrics {
    pub id: Option<i64>,
    pub scenario_id: String,
    pub workspace_id: Option<String>,
    pub org_id: Option<String>,
    pub usage_count: i64,
    pub last_used_at: Option<i64>,
    pub usage_pattern: Option<String>, // JSON string
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

/// Persona CI hit record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PersonaCIHit {
    pub id: Option<i64>,
    pub persona_id: String,
    pub workspace_id: Option<String>,
    pub org_id: Option<String>,
    pub ci_run_id: Option<String>,
    pub hit_count: i64,
    pub hit_at: i64,
    pub created_at: Option<i64>,
}

/// Endpoint coverage metrics
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EndpointCoverage {
    pub id: Option<i64>,
    pub endpoint: String,
    pub method: Option<String>,
    pub protocol: String,
    pub workspace_id: Option<String>,
    pub org_id: Option<String>,
    pub test_count: i64,
    pub last_tested_at: Option<i64>,
    pub coverage_percentage: Option<f64>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

/// Reality level staleness record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RealityLevelStaleness {
    pub id: Option<i64>,
    pub workspace_id: String,
    pub org_id: Option<String>,
    pub endpoint: Option<String>,
    pub method: Option<String>,
    pub protocol: Option<String>,
    pub current_reality_level: Option<String>,
    pub last_updated_at: Option<i64>,
    pub staleness_days: Option<i32>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

/// Drift percentage metrics
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DriftPercentageMetrics {
    pub id: Option<i64>,
    pub workspace_id: String,
    pub org_id: Option<String>,
    pub total_mocks: i64,
    pub drifting_mocks: i64,
    pub drift_percentage: f64,
    pub measured_at: i64,
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
    fn test_granularity_clone() {
        let g = Granularity::Hour;
        let cloned = g.clone();
        assert_eq!(g, cloned);
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
            total_bytes_sent: 10000,
            total_bytes_received: 5000,
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
            total_bytes_sent: 100000,
            total_bytes_received: 50000,
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
            timestamp: 1234567890,
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
