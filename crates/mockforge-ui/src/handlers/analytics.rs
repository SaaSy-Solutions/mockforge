//! Analytics API handlers for querying metrics
//!
//! Provides REST endpoints for accessing Prometheus metrics in a UI-friendly format.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error};

use crate::models::ApiResponse;
use crate::prometheus_client::PrometheusClient;

/// Analytics state shared across handlers
#[derive(Clone)]
pub struct AnalyticsState {
    pub prometheus_client: PrometheusClient,
}

impl AnalyticsState {
    pub fn new(prometheus_url: String) -> Self {
        Self {
            prometheus_client: PrometheusClient::new(prometheus_url),
        }
    }
}

/// Time range parameter
#[derive(Debug, Deserialize)]
pub struct TimeRangeQuery {
    #[serde(default = "default_range")]
    pub range: String,
}

fn default_range() -> String {
    "1h".to_string()
}

/// Summary metrics response
#[derive(Debug, Serialize)]
pub struct SummaryMetrics {
    pub timestamp: String,
    pub request_rate: f64,
    pub p95_latency_ms: f64,
    pub error_rate_percent: f64,
    pub active_connections: f64,
}

/// Request metrics response
#[derive(Debug, Serialize)]
pub struct RequestMetrics {
    pub timestamps: Vec<i64>,
    pub series: Vec<SeriesData>,
}

#[derive(Debug, Serialize)]
pub struct SeriesData {
    pub name: String,
    pub values: Vec<f64>,
}

/// Endpoint metrics response
#[derive(Debug, Serialize)]
pub struct EndpointMetrics {
    pub path: String,
    pub method: String,
    pub request_rate: f64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub errors: f64,
    pub error_rate_percent: f64,
}

/// WebSocket metrics response
#[derive(Debug, Serialize)]
pub struct WebSocketMetrics {
    pub active_connections: f64,
    pub total_connections: f64,
    pub message_rate_sent: f64,
    pub message_rate_received: f64,
    pub error_rate: f64,
    pub avg_connection_duration_seconds: f64,
}

/// System metrics response
#[derive(Debug, Serialize)]
pub struct SystemMetrics {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub thread_count: f64,
    pub uptime_seconds: f64,
}

/// SMTP metrics response
#[derive(Debug, Serialize)]
pub struct SmtpMetrics {
    pub active_connections: f64,
    pub total_connections: f64,
    pub message_rate_received: f64,
    pub message_rate_stored: f64,
    pub error_rate: f64,
}

/// Get summary analytics
pub async fn get_summary(
    State(state): State<AnalyticsState>,
    Query(params): Query<TimeRangeQuery>,
) -> Result<Json<ApiResponse<SummaryMetrics>>, StatusCode> {
    debug!("Fetching analytics summary for range: {}", params.range);

    // Request rate
    let request_rate_query = "sum(rate(mockforge_requests_total[5m]))";
    let request_rate = match state.prometheus_client.query(request_rate_query).await {
        Ok(response) => PrometheusClient::extract_single_value(&response).unwrap_or(0.0),
        Err(e) => {
            error!("Failed to query request rate: {}", e);
            0.0
        }
    };

    // P95 latency
    let p95_query = "histogram_quantile(0.95, sum(rate(mockforge_request_duration_seconds_bucket[5m])) by (le)) * 1000";
    let p95_latency = match state.prometheus_client.query(p95_query).await {
        Ok(response) => PrometheusClient::extract_single_value(&response).unwrap_or(0.0),
        Err(e) => {
            error!("Failed to query P95 latency: {}", e);
            0.0
        }
    };

    // Error rate
    let error_rate_query = "(sum(rate(mockforge_errors_total[5m])) / sum(rate(mockforge_requests_total[5m]))) * 100";
    let error_rate = match state.prometheus_client.query(error_rate_query).await {
        Ok(response) => PrometheusClient::extract_single_value(&response).unwrap_or(0.0),
        Err(e) => {
            error!("Failed to query error rate: {}", e);
            0.0
        }
    };

    // Active connections
    let active_conn_query = "sum(mockforge_requests_in_flight)";
    let active_connections = match state.prometheus_client.query(active_conn_query).await {
        Ok(response) => PrometheusClient::extract_single_value(&response).unwrap_or(0.0),
        Err(e) => {
            error!("Failed to query active connections: {}", e);
            0.0
        }
    };

    let summary = SummaryMetrics {
        timestamp: Utc::now().to_rfc3339(),
        request_rate,
        p95_latency_ms: p95_latency,
        error_rate_percent: error_rate,
        active_connections,
    };

    Ok(Json(ApiResponse::success(summary)))
}

/// Get request metrics over time
pub async fn get_requests(
    State(state): State<AnalyticsState>,
    Query(params): Query<TimeRangeQuery>,
) -> Result<Json<ApiResponse<RequestMetrics>>, StatusCode> {
    debug!("Fetching request metrics for range: {}", params.range);

    let (start, end, step) = parse_time_range(&params.range);

    let query = "sum by (protocol) (rate(mockforge_requests_total[5m]))";

    match state
        .prometheus_client
        .query_range(query, start, end, &step)
        .await
    {
        Ok(response) => {
            let time_series = PrometheusClient::extract_time_series(&response);

            // Extract unique timestamps
            let mut timestamps: Vec<i64> = Vec::new();
            if let Some((_, values)) = time_series.first() {
                timestamps = values.iter().map(|(ts, _)| *ts).collect();
            }

            // Build series data
            let series: Vec<SeriesData> = time_series
                .into_iter()
                .map(|(name, values)| SeriesData {
                    name,
                    values: values.into_iter().map(|(_, v)| v).collect(),
                })
                .collect();

            let metrics = RequestMetrics { timestamps, series };

            Ok(Json(ApiResponse::success(metrics)))
        }
        Err(e) => {
            error!("Failed to query request metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get endpoint metrics (top endpoints)
pub async fn get_endpoints(
    State(state): State<AnalyticsState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<EndpointMetrics>>>, StatusCode> {
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);

    debug!("Fetching top {} endpoints", limit);

    // Get top endpoints by request rate
    let query = format!(
        "topk({}, sum by (path, method) (rate(mockforge_requests_by_path_total[5m])))",
        limit
    );

    match state.prometheus_client.query(&query).await {
        Ok(response) => {
            let mut endpoints = Vec::new();

            for result in &response.data.result {
                if let Some(metric) = result.metric.as_object() {
                    let path = metric
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let method = metric
                        .get("method")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let request_rate: f64 = result
                        .value
                        .as_ref()
                        .and_then(|(_, v)| v.parse().ok())
                        .unwrap_or(0.0);

                    // Query average latency for this endpoint
                    let avg_latency_query = format!(
                        "mockforge_average_latency_by_path_seconds{{path=\"{}\",method=\"{}\"}} * 1000",
                        path, method
                    );
                    let avg_latency = state
                        .prometheus_client
                        .query(&avg_latency_query)
                        .await
                        .ok()
                        .and_then(|r| PrometheusClient::extract_single_value(&r))
                        .unwrap_or(0.0);

                    // Query P95 latency
                    let p95_query = format!(
                        "histogram_quantile(0.95, sum(rate(mockforge_request_duration_by_path_seconds_bucket{{path=\"{}\",method=\"{}\"}}[5m])) by (le)) * 1000",
                        path, method
                    );
                    let p95_latency = state
                        .prometheus_client
                        .query(&p95_query)
                        .await
                        .ok()
                        .and_then(|r| PrometheusClient::extract_single_value(&r))
                        .unwrap_or(0.0);

                    endpoints.push(EndpointMetrics {
                        path,
                        method,
                        request_rate,
                        avg_latency_ms: avg_latency,
                        p95_latency_ms: p95_latency,
                        errors: 0.0, // TODO: Query error count
                        error_rate_percent: 0.0,
                    });
                }
            }

            Ok(Json(ApiResponse::success(endpoints)))
        }
        Err(e) => {
            error!("Failed to query endpoint metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get WebSocket metrics
pub async fn get_websocket(
    State(state): State<AnalyticsState>,
) -> Result<Json<ApiResponse<WebSocketMetrics>>, StatusCode> {
    debug!("Fetching WebSocket metrics");

    // Active connections
    let active_query = "mockforge_ws_connections_active";
    let active_connections = state
        .prometheus_client
        .query(active_query)
        .await
        .ok()
        .and_then(|r| PrometheusClient::extract_single_value(&r))
        .unwrap_or(0.0);

    // Total connections
    let total_query = "mockforge_ws_connections_total";
    let total_connections = state
        .prometheus_client
        .query(total_query)
        .await
        .ok()
        .and_then(|r| PrometheusClient::extract_single_value(&r))
        .unwrap_or(0.0);

    // Message rate sent
    let sent_query = "rate(mockforge_ws_messages_sent_total[5m])";
    let message_rate_sent = state
        .prometheus_client
        .query(sent_query)
        .await
        .ok()
        .and_then(|r| PrometheusClient::extract_single_value(&r))
        .unwrap_or(0.0);

    // Message rate received
    let received_query = "rate(mockforge_ws_messages_received_total[5m])";
    let message_rate_received = state
        .prometheus_client
        .query(received_query)
        .await
        .ok()
        .and_then(|r| PrometheusClient::extract_single_value(&r))
        .unwrap_or(0.0);

    // Error rate
    let error_query = "rate(mockforge_ws_errors_total[5m])";
    let error_rate = state
        .prometheus_client
        .query(error_query)
        .await
        .ok()
        .and_then(|r| PrometheusClient::extract_single_value(&r))
        .unwrap_or(0.0);

    // Average connection duration
    let duration_query =
        "rate(mockforge_ws_connection_duration_seconds_sum[5m]) / rate(mockforge_ws_connection_duration_seconds_count[5m])";
    let avg_duration = state
        .prometheus_client
        .query(duration_query)
        .await
        .ok()
        .and_then(|r| PrometheusClient::extract_single_value(&r))
        .unwrap_or(0.0);

    let metrics = WebSocketMetrics {
        active_connections,
        total_connections,
        message_rate_sent,
        message_rate_received,
        error_rate,
        avg_connection_duration_seconds: avg_duration,
    };

    Ok(Json(ApiResponse::success(metrics)))
}

/// Get SMTP metrics
pub async fn get_smtp(
    State(state): State<AnalyticsState>,
) -> Result<Json<ApiResponse<SmtpMetrics>>, StatusCode> {
    debug!("Fetching SMTP metrics");

    let active_query = "mockforge_smtp_connections_active";
    let active_connections = state
        .prometheus_client
        .query(active_query)
        .await
        .ok()
        .and_then(|r| PrometheusClient::extract_single_value(&r))
        .unwrap_or(0.0);

    let total_query = "mockforge_smtp_connections_total";
    let total_connections = state
        .prometheus_client
        .query(total_query)
        .await
        .ok()
        .and_then(|r| PrometheusClient::extract_single_value(&r))
        .unwrap_or(0.0);

    let received_query = "rate(mockforge_smtp_messages_received_total[5m])";
    let message_rate_received = state
        .prometheus_client
        .query(received_query)
        .await
        .ok()
        .and_then(|r| PrometheusClient::extract_single_value(&r))
        .unwrap_or(0.0);

    let stored_query = "rate(mockforge_smtp_messages_stored_total[5m])";
    let message_rate_stored = state
        .prometheus_client
        .query(stored_query)
        .await
        .ok()
        .and_then(|r| PrometheusClient::extract_single_value(&r))
        .unwrap_or(0.0);

    let error_query = "sum(rate(mockforge_smtp_errors_total[5m]))";
    let error_rate = state
        .prometheus_client
        .query(error_query)
        .await
        .ok()
        .and_then(|r| PrometheusClient::extract_single_value(&r))
        .unwrap_or(0.0);

    let metrics = SmtpMetrics {
        active_connections,
        total_connections,
        message_rate_received,
        message_rate_stored,
        error_rate,
    };

    Ok(Json(ApiResponse::success(metrics)))
}

/// Get system metrics
pub async fn get_system(
    State(state): State<AnalyticsState>,
) -> Result<Json<ApiResponse<SystemMetrics>>, StatusCode> {
    debug!("Fetching system metrics");

    let memory_query = "mockforge_memory_usage_bytes / 1024 / 1024";
    let memory_usage_mb = state
        .prometheus_client
        .query(memory_query)
        .await
        .ok()
        .and_then(|r| PrometheusClient::extract_single_value(&r))
        .unwrap_or(0.0);

    let cpu_query = "mockforge_cpu_usage_percent";
    let cpu_usage_percent = state
        .prometheus_client
        .query(cpu_query)
        .await
        .ok()
        .and_then(|r| PrometheusClient::extract_single_value(&r))
        .unwrap_or(0.0);

    let thread_query = "mockforge_thread_count";
    let thread_count = state
        .prometheus_client
        .query(thread_query)
        .await
        .ok()
        .and_then(|r| PrometheusClient::extract_single_value(&r))
        .unwrap_or(0.0);

    let uptime_query = "mockforge_uptime_seconds";
    let uptime_seconds = state
        .prometheus_client
        .query(uptime_query)
        .await
        .ok()
        .and_then(|r| PrometheusClient::extract_single_value(&r))
        .unwrap_or(0.0);

    let metrics = SystemMetrics {
        memory_usage_mb,
        cpu_usage_percent,
        thread_count,
        uptime_seconds,
    };

    Ok(Json(ApiResponse::success(metrics)))
}

/// Parse time range string to start, end, step
fn parse_time_range(range: &str) -> (i64, i64, String) {
    let now = Utc::now().timestamp();
    let duration_secs = match range {
        "5m" => 5 * 60,
        "15m" => 15 * 60,
        "1h" => 60 * 60,
        "6h" => 6 * 60 * 60,
        "24h" => 24 * 60 * 60,
        _ => 60 * 60, // Default to 1 hour
    };

    let start = now - duration_secs;
    let step = match range {
        "5m" => "15s",
        "15m" => "30s",
        "1h" => "1m",
        "6h" => "5m",
        "24h" => "15m",
        _ => "1m",
    }
    .to_string();

    (start, now, step)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_range() {
        let (start, end, step) = parse_time_range("1h");
        assert!(end - start == 3600);
        assert_eq!(step, "1m");
    }

    #[test]
    fn test_parse_time_range_5m() {
        let (start, end, step) = parse_time_range("5m");
        assert!(end - start == 300);
        assert_eq!(step, "15s");
    }
}
