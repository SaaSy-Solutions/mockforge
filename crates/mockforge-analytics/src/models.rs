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
    pub fn from_status_code(status_code: u16) -> Self {
        match status_code {
            400..=499 => ErrorCategory::ClientError,
            500..=599 => ErrorCategory::ServerError,
            _ => ErrorCategory::Other,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCategory::ClientError => "client_error",
            ErrorCategory::ServerError => "server_error",
            ErrorCategory::NetworkError => "network_error",
            ErrorCategory::TimeoutError => "timeout_error",
            ErrorCategory::Other => "other",
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
