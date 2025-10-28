//! Enhanced Analytics API handlers with persistent storage
//!
//! This module provides comprehensive analytics endpoints that combine:
//! - Real-time metrics from Prometheus
//! - Historical data from the analytics database
//! - Advanced queries (time-series, trends, patterns)

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use mockforge_analytics::{
    AnalyticsDatabase, AnalyticsFilter, ErrorCategory, Granularity, OverviewMetrics,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error};

use crate::models::ApiResponse;

/// Enhanced analytics state with both Prometheus and database access
#[derive(Clone)]
pub struct AnalyticsV2State {
    pub db: Arc<AnalyticsDatabase>,
}

impl AnalyticsV2State {
    pub fn new(db: AnalyticsDatabase) -> Self {
        Self { db: Arc::new(db) }
    }
}

/// Query parameters for analytics endpoints
#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    /// Start time (Unix timestamp)
    pub start_time: Option<i64>,
    /// End time (Unix timestamp)
    pub end_time: Option<i64>,
    /// Duration in seconds (alternative to start/end)
    #[serde(default = "default_duration")]
    pub duration: i64,
    /// Protocol filter (HTTP, gRPC, WebSocket, etc.)
    pub protocol: Option<String>,
    /// Endpoint filter
    pub endpoint: Option<String>,
    /// Method filter (GET, POST, etc.)
    pub method: Option<String>,
    /// Status code filter
    pub status_code: Option<i32>,
    /// Workspace ID filter
    pub workspace_id: Option<String>,
    /// Environment filter (dev, staging, prod)
    pub environment: Option<String>,
    /// Limit results
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Granularity for time-series data
    #[serde(default = "default_granularity")]
    pub granularity: String,
}

fn default_duration() -> i64 {
    3600 // 1 hour
}

fn default_limit() -> i64 {
    100
}

fn default_granularity() -> String {
    "minute".to_string()
}

impl AnalyticsQuery {
    /// Convert to AnalyticsFilter
    fn to_filter(&self) -> AnalyticsFilter {
        let (start_time, end_time) =
            if let (Some(start), Some(end)) = (self.start_time, self.end_time) {
                (Some(start), Some(end))
            } else {
                let end = Utc::now().timestamp();
                let start = end - self.duration;
                (Some(start), Some(end))
            };

        AnalyticsFilter {
            start_time,
            end_time,
            protocol: self.protocol.clone(),
            endpoint: self.endpoint.clone(),
            method: self.method.clone(),
            status_code: self.status_code,
            workspace_id: self.workspace_id.clone(),
            environment: self.environment.clone(),
            limit: Some(self.limit),
        }
    }

    /// Parse granularity string
    fn get_granularity(&self) -> Granularity {
        match self.granularity.as_str() {
            "minute" => Granularity::Minute,
            "hour" => Granularity::Hour,
            "day" => Granularity::Day,
            _ => Granularity::Minute,
        }
    }
}

// ============================================================================
// REST API Endpoints
// ============================================================================

/// GET /api/v2/analytics/overview
///
/// Get dashboard overview metrics including:
/// - Total requests, errors, error rate
/// - Latency percentiles (avg, p50, p95, p99)
/// - Active connections, throughput
/// - Top protocols and endpoints
pub async fn get_overview(
    State(state): State<AnalyticsV2State>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<ApiResponse<OverviewMetrics>>, StatusCode> {
    debug!("Fetching analytics overview for duration: {}s", query.duration);

    match state.db.get_overview_metrics(query.duration).await {
        Ok(overview) => Ok(Json(ApiResponse::success(overview))),
        Err(e) => {
            error!("Failed to get overview metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v2/analytics/requests
///
/// Get request count time-series data
#[derive(Debug, Serialize)]
pub struct TimeSeriesResponse {
    pub series: Vec<SeriesData>,
}

#[derive(Debug, Serialize)]
pub struct SeriesData {
    pub label: String,
    pub data: Vec<DataPoint>,
}

#[derive(Debug, Serialize)]
pub struct DataPoint {
    pub timestamp: i64,
    pub value: f64,
}

pub async fn get_requests_timeseries(
    State(state): State<AnalyticsV2State>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<ApiResponse<TimeSeriesResponse>>, StatusCode> {
    debug!("Fetching request time-series");

    let filter = query.to_filter();
    let granularity = query.get_granularity();

    match state.db.get_request_time_series(&filter, granularity).await {
        Ok(time_series) => {
            let series = time_series
                .into_iter()
                .map(|ts| SeriesData {
                    label: ts.label,
                    data: ts
                        .data
                        .into_iter()
                        .map(|point| DataPoint {
                            timestamp: point.timestamp,
                            value: point.value,
                        })
                        .collect(),
                })
                .collect();

            Ok(Json(ApiResponse::success(TimeSeriesResponse { series })))
        }
        Err(e) => {
            error!("Failed to get request time-series: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v2/analytics/latency
///
/// Get latency trends (percentiles over time)
#[derive(Debug, Serialize)]
pub struct LatencyResponse {
    pub trends: Vec<LatencyTrendData>,
}

#[derive(Debug, Serialize)]
pub struct LatencyTrendData {
    pub timestamp: i64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub avg: f64,
    pub min: f64,
    pub max: f64,
}

pub async fn get_latency_trends(
    State(state): State<AnalyticsV2State>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<ApiResponse<LatencyResponse>>, StatusCode> {
    debug!("Fetching latency trends");

    let filter = query.to_filter();

    match state.db.get_latency_trends(&filter).await {
        Ok(trends) => {
            let trend_data = trends
                .into_iter()
                .map(|t| LatencyTrendData {
                    timestamp: t.timestamp,
                    p50: t.p50,
                    p95: t.p95,
                    p99: t.p99,
                    avg: t.avg,
                    min: t.min,
                    max: t.max,
                })
                .collect();

            Ok(Json(ApiResponse::success(LatencyResponse { trends: trend_data })))
        }
        Err(e) => {
            error!("Failed to get latency trends: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v2/analytics/errors
///
/// Get error summary (grouped by type and category)
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub errors: Vec<ErrorSummaryData>,
}

#[derive(Debug, Serialize)]
pub struct ErrorSummaryData {
    pub error_type: String,
    pub error_category: String,
    pub count: i64,
    pub endpoints: Vec<String>,
    pub last_occurrence: String,
}

pub async fn get_error_summary(
    State(state): State<AnalyticsV2State>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<ApiResponse<ErrorResponse>>, StatusCode> {
    debug!("Fetching error summary");

    let filter = query.to_filter();

    match state.db.get_error_summary(&filter, query.limit).await {
        Ok(errors) => {
            let error_data = errors
                .into_iter()
                .map(|e| ErrorSummaryData {
                    error_type: e.error_type,
                    error_category: e.error_category,
                    count: e.count,
                    endpoints: e.endpoints,
                    last_occurrence: e.last_occurrence.to_rfc3339(),
                })
                .collect();

            Ok(Json(ApiResponse::success(ErrorResponse { errors: error_data })))
        }
        Err(e) => {
            error!("Failed to get error summary: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v2/analytics/endpoints
///
/// Get top endpoints by traffic
#[derive(Debug, Serialize)]
pub struct EndpointsResponse {
    pub endpoints: Vec<EndpointData>,
}

#[derive(Debug, Serialize)]
pub struct EndpointData {
    pub endpoint: String,
    pub protocol: String,
    pub method: Option<String>,
    pub total_requests: i64,
    pub total_errors: i64,
    pub error_rate: f64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub bytes_sent: i64,
    pub bytes_received: i64,
}

pub async fn get_top_endpoints(
    State(state): State<AnalyticsV2State>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<ApiResponse<EndpointsResponse>>, StatusCode> {
    debug!("Fetching top {} endpoints", query.limit);

    match state.db.get_top_endpoints(query.limit, query.workspace_id.as_deref()).await {
        Ok(endpoints) => {
            let endpoint_data = endpoints
                .into_iter()
                .map(|e| {
                    let error_rate = if e.total_requests > 0 {
                        (e.total_errors as f64 / e.total_requests as f64) * 100.0
                    } else {
                        0.0
                    };

                    EndpointData {
                        endpoint: e.endpoint,
                        protocol: e.protocol,
                        method: e.method,
                        total_requests: e.total_requests,
                        total_errors: e.total_errors,
                        error_rate,
                        avg_latency_ms: e.avg_latency_ms.unwrap_or(0.0),
                        p95_latency_ms: e.p95_latency_ms.unwrap_or(0.0),
                        bytes_sent: e.total_bytes_sent,
                        bytes_received: e.total_bytes_received,
                    }
                })
                .collect();

            Ok(Json(ApiResponse::success(EndpointsResponse {
                endpoints: endpoint_data,
            })))
        }
        Err(e) => {
            error!("Failed to get top endpoints: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v2/analytics/protocols
///
/// Get traffic breakdown by protocol
#[derive(Debug, Serialize)]
pub struct ProtocolsResponse {
    pub protocols: Vec<ProtocolData>,
}

#[derive(Debug, Serialize)]
pub struct ProtocolData {
    pub protocol: String,
    pub request_count: i64,
    pub error_count: i64,
    pub avg_latency_ms: f64,
}

pub async fn get_protocol_breakdown(
    State(state): State<AnalyticsV2State>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<ApiResponse<ProtocolsResponse>>, StatusCode> {
    debug!("Fetching protocol breakdown");

    match state.db.get_top_protocols(10, query.workspace_id.as_deref()).await {
        Ok(protocols) => {
            let protocol_data = protocols
                .into_iter()
                .map(|p| ProtocolData {
                    protocol: p.protocol,
                    request_count: p.request_count,
                    error_count: p.error_count,
                    avg_latency_ms: p.avg_latency_ms,
                })
                .collect();

            Ok(Json(ApiResponse::success(ProtocolsResponse {
                protocols: protocol_data,
            })))
        }
        Err(e) => {
            error!("Failed to get protocol breakdown: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v2/analytics/traffic-patterns
///
/// Get traffic patterns for heatmap visualization
#[derive(Debug, Serialize)]
pub struct TrafficPatternsResponse {
    pub patterns: Vec<TrafficPatternData>,
}

#[derive(Debug, Serialize)]
pub struct TrafficPatternData {
    pub date: String,
    pub hour: i32,
    pub day_of_week: i32,
    pub request_count: i64,
    pub error_count: i64,
    pub avg_latency_ms: f64,
}

#[derive(Debug, Deserialize)]
pub struct TrafficPatternsQuery {
    #[serde(default = "default_pattern_days")]
    pub days: i64,
    pub workspace_id: Option<String>,
}

fn default_pattern_days() -> i64 {
    30
}

pub async fn get_traffic_patterns(
    State(state): State<AnalyticsV2State>,
    Query(query): Query<TrafficPatternsQuery>,
) -> Result<Json<ApiResponse<TrafficPatternsResponse>>, StatusCode> {
    debug!("Fetching traffic patterns for {} days", query.days);

    match state.db.get_traffic_patterns(query.days, query.workspace_id.as_deref()).await {
        Ok(patterns) => {
            let pattern_data = patterns
                .into_iter()
                .map(|p| TrafficPatternData {
                    date: p.date,
                    hour: p.hour,
                    day_of_week: p.day_of_week,
                    request_count: p.request_count,
                    error_count: p.error_count,
                    avg_latency_ms: p.avg_latency_ms.unwrap_or(0.0),
                })
                .collect();

            Ok(Json(ApiResponse::success(TrafficPatternsResponse {
                patterns: pattern_data,
            })))
        }
        Err(e) => {
            error!("Failed to get traffic patterns: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v2/analytics/export/csv
///
/// Export analytics data to CSV format
pub async fn export_csv(
    State(state): State<AnalyticsV2State>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<(StatusCode, String), StatusCode> {
    debug!("Exporting analytics to CSV");

    let filter = query.to_filter();
    let mut buffer = Vec::new();

    match state.db.export_to_csv(&mut buffer, &filter).await {
        Ok(_) => {
            let csv_data = String::from_utf8(buffer).unwrap_or_default();
            Ok((StatusCode::OK, csv_data))
        }
        Err(e) => {
            error!("Failed to export to CSV: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v2/analytics/export/json
///
/// Export analytics data to JSON format
pub async fn export_json(
    State(state): State<AnalyticsV2State>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<(StatusCode, String), StatusCode> {
    debug!("Exporting analytics to JSON");

    let filter = query.to_filter();

    match state.db.export_to_json(&filter).await {
        Ok(json) => Ok((StatusCode::OK, json)),
        Err(e) => {
            error!("Failed to export to JSON: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_query_to_filter() {
        let query = AnalyticsQuery {
            start_time: Some(100),
            end_time: Some(200),
            duration: 3600,
            protocol: Some("HTTP".to_string()),
            endpoint: Some("/api/test".to_string()),
            method: Some("GET".to_string()),
            status_code: Some(200),
            workspace_id: None,
            environment: Some("prod".to_string()),
            limit: 50,
            granularity: "minute".to_string(),
        };

        let filter = query.to_filter();
        assert_eq!(filter.start_time, Some(100));
        assert_eq!(filter.end_time, Some(200));
        assert_eq!(filter.protocol, Some("HTTP".to_string()));
        assert_eq!(filter.limit, Some(50));
    }

    #[test]
    fn test_granularity_parsing() {
        let query = AnalyticsQuery {
            start_time: None,
            end_time: None,
            duration: 3600,
            protocol: None,
            endpoint: None,
            method: None,
            status_code: None,
            workspace_id: None,
            environment: None,
            limit: 100,
            granularity: "hour".to_string(),
        };

        assert_eq!(query.get_granularity(), Granularity::Hour);
    }
}
