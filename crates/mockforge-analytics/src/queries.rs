//! High-level query API for analytics data

use crate::database::AnalyticsDatabase;
use crate::error::Result;
use crate::models::{
    AnalyticsFilter, EndpointStat, ErrorSummary, Granularity, LatencyTrend, MetricsAggregate,
    OverviewMetrics, ProtocolStat, TimeSeries, TimeSeriesPoint,
};
use chrono::{DateTime, Utc};
use sqlx::Row;

impl AnalyticsDatabase {
    /// Get overview metrics for the dashboard
    pub async fn get_overview_metrics(&self, duration_seconds: i64) -> Result<OverviewMetrics> {
        let end_time = Utc::now().timestamp();
        let start_time = end_time - duration_seconds;

        let filter = AnalyticsFilter {
            start_time: Some(start_time),
            end_time: Some(end_time),
            ..Default::default()
        };

        let aggregates = self.get_minute_aggregates(&filter).await?;

        let total_requests: i64 = aggregates.iter().map(|a| a.request_count).sum();
        let total_errors: i64 = aggregates.iter().map(|a| a.error_count).sum();
        let error_rate = if total_requests > 0 {
            (total_errors as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let total_latency: f64 = aggregates.iter().map(|a| a.latency_sum).sum();
        let latency_count: i64 = aggregates.iter().filter(|a| a.latency_sum > 0.0).count() as i64;
        let avg_latency_ms = if latency_count > 0 {
            total_latency / latency_count as f64
        } else {
            0.0
        };

        let p95_latencies: Vec<f64> = aggregates.iter().filter_map(|a| a.latency_p95).collect();
        let p95_latency_ms = if p95_latencies.is_empty() {
            0.0
        } else {
            p95_latencies.iter().sum::<f64>() / p95_latencies.len() as f64
        };

        let p99_latencies: Vec<f64> = aggregates.iter().filter_map(|a| a.latency_p99).collect();
        let p99_latency_ms = if p99_latencies.is_empty() {
            0.0
        } else {
            p99_latencies.iter().sum::<f64>() / p99_latencies.len() as f64
        };

        let total_bytes_sent: i64 = aggregates.iter().map(|a| a.bytes_sent).sum();
        let total_bytes_received: i64 = aggregates.iter().map(|a| a.bytes_received).sum();

        let active_connections =
            aggregates.iter().filter_map(|a| a.active_connections).max().unwrap_or(0);

        let requests_per_second = total_requests as f64 / duration_seconds as f64;

        // Get top protocols
        let top_protocols = self.get_top_protocols(5, None).await?;

        // Get top endpoints
        let top_endpoints_data = self.get_top_endpoints(10, None).await?;
        let top_endpoints: Vec<EndpointStat> = top_endpoints_data
            .iter()
            .map(|e| {
                let error_rate = if e.total_requests > 0 {
                    (e.total_errors as f64 / e.total_requests as f64) * 100.0
                } else {
                    0.0
                };
                EndpointStat {
                    endpoint: e.endpoint.clone(),
                    protocol: e.protocol.clone(),
                    method: e.method.clone(),
                    request_count: e.total_requests,
                    error_count: e.total_errors,
                    error_rate,
                    avg_latency_ms: e.avg_latency_ms.unwrap_or(0.0),
                    p95_latency_ms: e.p95_latency_ms.unwrap_or(0.0),
                }
            })
            .collect();

        Ok(OverviewMetrics {
            total_requests,
            total_errors,
            error_rate,
            avg_latency_ms,
            p95_latency_ms,
            p99_latency_ms,
            active_connections,
            total_bytes_sent,
            total_bytes_received,
            requests_per_second,
            top_protocols,
            top_endpoints,
        })
    }

    /// Get top protocols by request count
    pub async fn get_top_protocols(
        &self,
        limit: i64,
        workspace_id: Option<&str>,
    ) -> Result<Vec<ProtocolStat>> {
        let mut query = String::from(
            r"
            SELECT
                protocol,
                SUM(request_count) as total_requests,
                SUM(error_count) as total_errors,
                AVG(latency_sum / NULLIF(request_count, 0)) as avg_latency_ms
            FROM metrics_aggregates_minute
            WHERE 1=1
            ",
        );

        if workspace_id.is_some() {
            query.push_str(" AND workspace_id = ?");
        }

        query.push_str(
            "
            GROUP BY protocol
            ORDER BY total_requests DESC
            LIMIT ?
            ",
        );

        let mut sql_query = sqlx::query(&query);

        if let Some(workspace) = workspace_id {
            sql_query = sql_query.bind(workspace);
        }

        sql_query = sql_query.bind(limit);

        let rows = sql_query.fetch_all(self.pool()).await?;

        let mut protocols = Vec::new();
        for row in rows {
            protocols.push(ProtocolStat {
                protocol: row.get("protocol"),
                request_count: row.get("total_requests"),
                error_count: row.get("total_errors"),
                avg_latency_ms: row.try_get("avg_latency_ms").unwrap_or(0.0),
            });
        }

        Ok(protocols)
    }

    /// Get request count time series
    pub async fn get_request_time_series(
        &self,
        filter: &AnalyticsFilter,
        granularity: Granularity,
    ) -> Result<Vec<TimeSeries>> {
        let aggregates = self.get_minute_aggregates(filter).await?;

        let bucket_size = match granularity {
            Granularity::Minute => 60,
            Granularity::Hour => 3600,
            Granularity::Day => 86400,
        };

        // Group by protocol and time bucket
        let mut series_map: std::collections::HashMap<String, Vec<TimeSeriesPoint>> =
            std::collections::HashMap::new();

        for agg in aggregates {
            let bucket = (agg.timestamp / bucket_size) * bucket_size;
            let point = TimeSeriesPoint {
                timestamp: bucket,
                value: agg.request_count as f64,
            };

            series_map.entry(agg.protocol.clone()).or_default().push(point);
        }

        // Convert to TimeSeries objects
        let mut result: Vec<TimeSeries> = series_map
            .into_iter()
            .map(|(protocol, mut points)| {
                points.sort_by_key(|p| p.timestamp);

                // Aggregate points in the same bucket
                let mut aggregated = Vec::new();
                let mut current_bucket = None;
                let mut current_sum = 0.0;

                for point in points {
                    match current_bucket {
                        Some(bucket) if bucket == point.timestamp => {
                            current_sum += point.value;
                        }
                        _ => {
                            if let Some(bucket) = current_bucket {
                                aggregated.push(TimeSeriesPoint {
                                    timestamp: bucket,
                                    value: current_sum,
                                });
                            }
                            current_bucket = Some(point.timestamp);
                            current_sum = point.value;
                        }
                    }
                }

                if let Some(bucket) = current_bucket {
                    aggregated.push(TimeSeriesPoint {
                        timestamp: bucket,
                        value: current_sum,
                    });
                }

                TimeSeries {
                    label: protocol,
                    data: aggregated,
                }
            })
            .collect();

        result.sort_by(|a, b| b.data.len().cmp(&a.data.len()));
        Ok(result)
    }

    /// Get latency trends
    pub async fn get_latency_trends(&self, filter: &AnalyticsFilter) -> Result<Vec<LatencyTrend>> {
        let aggregates = self.get_minute_aggregates(filter).await?;

        let mut trends = Vec::new();

        // Group by timestamp and aggregate
        let mut bucket_map: std::collections::HashMap<i64, Vec<&MetricsAggregate>> =
            std::collections::HashMap::new();

        for agg in &aggregates {
            bucket_map.entry(agg.timestamp).or_default().push(agg);
        }

        for (timestamp, group) in bucket_map {
            let avg = group
                .iter()
                .filter_map(|a| {
                    if a.request_count > 0 {
                        Some(a.latency_sum / a.request_count as f64)
                    } else {
                        None
                    }
                })
                .sum::<f64>()
                / group.len() as f64;

            let min = group.iter().filter_map(|a| a.latency_min).fold(f64::INFINITY, f64::min);
            let max = group.iter().filter_map(|a| a.latency_max).fold(f64::NEG_INFINITY, f64::max);
            let p50 = group.iter().filter_map(|a| a.latency_p50).sum::<f64>() / group.len() as f64;
            let p95 = group.iter().filter_map(|a| a.latency_p95).sum::<f64>() / group.len() as f64;
            let p99 = group.iter().filter_map(|a| a.latency_p99).sum::<f64>() / group.len() as f64;

            trends.push(LatencyTrend {
                timestamp,
                p50,
                p95,
                p99,
                avg,
                min: if min.is_finite() { min } else { 0.0 },
                max: if max.is_finite() { max } else { 0.0 },
            });
        }

        trends.sort_by_key(|t| t.timestamp);
        Ok(trends)
    }

    /// Get error summary
    pub async fn get_error_summary(
        &self,
        filter: &AnalyticsFilter,
        limit: i64,
    ) -> Result<Vec<ErrorSummary>> {
        let errors = self.get_recent_errors(1000, filter).await?;

        // Group by error type
        let mut error_map: std::collections::HashMap<
            String,
            (i64, std::collections::HashSet<String>, i64),
        > = std::collections::HashMap::new();

        for error in errors {
            let error_type = error.error_type.clone().unwrap_or_else(|| "unknown".to_string());
            let error_category =
                error.error_category.clone().unwrap_or_else(|| "other".to_string());
            let endpoint = error.endpoint.clone().unwrap_or_default();

            let entry = error_map.entry(format!("{error_category}:{error_type}")).or_insert((
                0,
                std::collections::HashSet::new(),
                0,
            ));

            entry.0 += 1;
            entry.1.insert(endpoint);
            entry.2 = entry.2.max(error.timestamp);
        }

        let mut summaries: Vec<ErrorSummary> = error_map
            .into_iter()
            .map(|(key, (count, endpoints, last_ts))| {
                let parts: Vec<&str> = key.split(':').collect();
                ErrorSummary {
                    error_type: (*parts.get(1).unwrap_or(&"unknown")).to_string(),
                    error_category: (*parts.first().unwrap_or(&"other")).to_string(),
                    count,
                    endpoints: endpoints.into_iter().collect(),
                    last_occurrence: DateTime::from_timestamp(last_ts, 0).unwrap_or_else(Utc::now),
                }
            })
            .collect();

        summaries.sort_by(|a, b| b.count.cmp(&a.count));
        summaries.truncate(limit as usize);

        Ok(summaries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::AnalyticsDatabase;
    use crate::models::{ErrorEvent, MetricsAggregate};
    use std::path::Path;

    async fn setup_test_db() -> AnalyticsDatabase {
        let db = AnalyticsDatabase::new(Path::new(":memory:")).await.unwrap();
        db.run_migrations().await.unwrap();
        db
    }

    fn create_test_aggregate(
        timestamp: i64,
        protocol: &str,
        request_count: i64,
        error_count: i64,
        latency_sum: f64,
    ) -> MetricsAggregate {
        MetricsAggregate {
            id: None,
            timestamp,
            protocol: protocol.to_string(),
            method: Some("GET".to_string()),
            endpoint: Some("/api/test".to_string()),
            status_code: Some(200),
            workspace_id: None,
            environment: None,
            request_count,
            error_count,
            latency_sum,
            latency_min: Some(10.0),
            latency_max: Some(100.0),
            latency_p50: Some(50.0),
            latency_p95: Some(95.0),
            latency_p99: Some(99.0),
            bytes_sent: 1000,
            bytes_received: 500,
            active_connections: Some(5),
            created_at: None,
        }
    }

    fn create_test_error(
        timestamp: i64,
        error_type: &str,
        error_category: &str,
        endpoint: &str,
    ) -> ErrorEvent {
        ErrorEvent {
            id: None,
            timestamp,
            protocol: "http".to_string(),
            method: Some("GET".to_string()),
            endpoint: Some(endpoint.to_string()),
            status_code: Some(500),
            error_type: Some(error_type.to_string()),
            error_message: Some("Test error".to_string()),
            error_category: Some(error_category.to_string()),
            request_id: Some("req-123".to_string()),
            trace_id: None,
            span_id: None,
            client_ip: Some("127.0.0.1".to_string()),
            user_agent: None,
            workspace_id: None,
            environment: None,
            metadata: None,
            created_at: None,
        }
    }

    // ==================== get_overview_metrics Tests ====================

    #[tokio::test]
    async fn test_get_overview_metrics_empty_db() {
        let db = setup_test_db().await;
        let metrics = db.get_overview_metrics(3600).await.unwrap();

        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.total_errors, 0);
        assert_eq!(metrics.error_rate, 0.0);
        assert_eq!(metrics.avg_latency_ms, 0.0);
        assert_eq!(metrics.requests_per_second, 0.0);
    }

    #[tokio::test]
    async fn test_get_overview_metrics_with_data() {
        let db = setup_test_db().await;

        // Insert test data
        let now = Utc::now().timestamp();
        let agg1 = create_test_aggregate(now - 60, "http", 100, 5, 5000.0);
        let agg2 = create_test_aggregate(now - 120, "http", 200, 10, 10000.0);

        db.insert_minute_aggregate(&agg1).await.unwrap();
        db.insert_minute_aggregate(&agg2).await.unwrap();

        let metrics = db.get_overview_metrics(3600).await.unwrap();

        assert_eq!(metrics.total_requests, 300);
        assert_eq!(metrics.total_errors, 15);
        assert!((metrics.error_rate - 5.0).abs() < 0.01); // 15/300 * 100 = 5%
    }

    #[tokio::test]
    async fn test_get_overview_metrics_calculates_rps() {
        let db = setup_test_db().await;

        let now = Utc::now().timestamp();
        let agg = create_test_aggregate(now - 30, "http", 100, 0, 1000.0);
        db.insert_minute_aggregate(&agg).await.unwrap();

        let metrics = db.get_overview_metrics(100).await.unwrap();

        // 100 requests over 100 seconds = 1.0 rps
        assert!((metrics.requests_per_second - 1.0).abs() < 0.01);
    }

    // ==================== get_top_protocols Tests ====================

    #[tokio::test]
    async fn test_get_top_protocols_empty() {
        let db = setup_test_db().await;
        let protocols = db.get_top_protocols(5, None).await.unwrap();
        assert!(protocols.is_empty());
    }

    #[tokio::test]
    async fn test_get_top_protocols_multiple_protocols() {
        let db = setup_test_db().await;

        let now = Utc::now().timestamp();
        // HTTP has more requests
        let http_agg = create_test_aggregate(now - 60, "http", 1000, 10, 50000.0);
        // gRPC has fewer requests
        let grpc_agg = create_test_aggregate(now - 60, "grpc", 500, 5, 25000.0);
        // WebSocket has the fewest
        let ws_agg = create_test_aggregate(now - 60, "websocket", 100, 1, 5000.0);

        db.insert_minute_aggregate(&http_agg).await.unwrap();
        db.insert_minute_aggregate(&grpc_agg).await.unwrap();
        db.insert_minute_aggregate(&ws_agg).await.unwrap();

        let protocols = db.get_top_protocols(10, None).await.unwrap();

        assert_eq!(protocols.len(), 3);
        // Should be ordered by request count descending
        assert_eq!(protocols[0].protocol, "http");
        assert_eq!(protocols[0].request_count, 1000);
        assert_eq!(protocols[1].protocol, "grpc");
        assert_eq!(protocols[1].request_count, 500);
        assert_eq!(protocols[2].protocol, "websocket");
        assert_eq!(protocols[2].request_count, 100);
    }

    #[tokio::test]
    async fn test_get_top_protocols_respects_limit() {
        let db = setup_test_db().await;

        let now = Utc::now().timestamp();
        db.insert_minute_aggregate(&create_test_aggregate(now, "http", 100, 0, 1000.0))
            .await
            .unwrap();
        db.insert_minute_aggregate(&create_test_aggregate(now, "grpc", 80, 0, 800.0))
            .await
            .unwrap();
        db.insert_minute_aggregate(&create_test_aggregate(now, "websocket", 60, 0, 600.0))
            .await
            .unwrap();

        let protocols = db.get_top_protocols(2, None).await.unwrap();
        assert_eq!(protocols.len(), 2);
    }

    // ==================== get_request_time_series Tests ====================

    #[tokio::test]
    async fn test_get_request_time_series_empty() {
        let db = setup_test_db().await;

        let filter = AnalyticsFilter::default();
        let series = db.get_request_time_series(&filter, Granularity::Minute).await.unwrap();

        assert!(series.is_empty());
    }

    #[tokio::test]
    async fn test_get_request_time_series_minute_granularity() {
        let db = setup_test_db().await;

        // Insert data at different minute timestamps
        let base_time = 1700000000i64; // Fixed timestamp for reproducibility
        db.insert_minute_aggregate(&create_test_aggregate(base_time, "http", 100, 0, 1000.0))
            .await
            .unwrap();
        db.insert_minute_aggregate(&create_test_aggregate(base_time + 60, "http", 150, 0, 1500.0))
            .await
            .unwrap();

        let filter = AnalyticsFilter {
            start_time: Some(base_time - 60),
            end_time: Some(base_time + 120),
            ..Default::default()
        };

        let series = db.get_request_time_series(&filter, Granularity::Minute).await.unwrap();

        assert!(!series.is_empty());
        // Should have HTTP series
        let http_series = series.iter().find(|s| s.label == "http").unwrap();
        assert!(!http_series.data.is_empty());
    }

    #[tokio::test]
    async fn test_get_request_time_series_hour_granularity() {
        let db = setup_test_db().await;

        // Insert data in the same hour
        let base_time = 1700000000i64;
        db.insert_minute_aggregate(&create_test_aggregate(base_time, "http", 100, 0, 1000.0))
            .await
            .unwrap();
        db.insert_minute_aggregate(&create_test_aggregate(base_time + 60, "http", 100, 0, 1000.0))
            .await
            .unwrap();

        let filter = AnalyticsFilter {
            start_time: Some(base_time - 60),
            end_time: Some(base_time + 3700),
            ..Default::default()
        };

        let series = db.get_request_time_series(&filter, Granularity::Hour).await.unwrap();

        assert!(!series.is_empty());
        let http_series = series.iter().find(|s| s.label == "http").unwrap();
        // Data points in the same hour bucket should be aggregated
        // Both 100 request counts should aggregate to 200
        let total: f64 = http_series.data.iter().map(|p| p.value).sum();
        assert_eq!(total, 200.0);
    }

    // ==================== get_latency_trends Tests ====================

    #[tokio::test]
    async fn test_get_latency_trends_empty() {
        let db = setup_test_db().await;

        let filter = AnalyticsFilter::default();
        let trends = db.get_latency_trends(&filter).await.unwrap();

        assert!(trends.is_empty());
    }

    #[tokio::test]
    async fn test_get_latency_trends_with_data() {
        let db = setup_test_db().await;

        let base_time = 1700000000i64;
        let mut agg = create_test_aggregate(base_time, "http", 100, 0, 5000.0);
        agg.latency_p50 = Some(50.0);
        agg.latency_p95 = Some(95.0);
        agg.latency_p99 = Some(99.0);
        agg.latency_min = Some(10.0);
        agg.latency_max = Some(150.0);

        db.insert_minute_aggregate(&agg).await.unwrap();

        let filter = AnalyticsFilter {
            start_time: Some(base_time - 60),
            end_time: Some(base_time + 60),
            ..Default::default()
        };

        let trends = db.get_latency_trends(&filter).await.unwrap();

        assert_eq!(trends.len(), 1);
        let trend = &trends[0];
        assert_eq!(trend.timestamp, base_time);
        assert_eq!(trend.p50, 50.0);
        assert_eq!(trend.p95, 95.0);
        assert_eq!(trend.p99, 99.0);
        assert_eq!(trend.min, 10.0);
        assert_eq!(trend.max, 150.0);
    }

    #[tokio::test]
    async fn test_get_latency_trends_sorted_by_timestamp() {
        let db = setup_test_db().await;

        let base_time = 1700000000i64;
        db.insert_minute_aggregate(&create_test_aggregate(base_time + 120, "http", 100, 0, 1000.0))
            .await
            .unwrap();
        db.insert_minute_aggregate(&create_test_aggregate(base_time, "http", 100, 0, 1000.0))
            .await
            .unwrap();
        db.insert_minute_aggregate(&create_test_aggregate(base_time + 60, "http", 100, 0, 1000.0))
            .await
            .unwrap();

        let filter = AnalyticsFilter {
            start_time: Some(base_time - 60),
            end_time: Some(base_time + 180),
            ..Default::default()
        };

        let trends = db.get_latency_trends(&filter).await.unwrap();

        // Should be sorted by timestamp ascending
        assert!(trends.windows(2).all(|w| w[0].timestamp <= w[1].timestamp));
    }

    // ==================== get_error_summary Tests ====================

    #[tokio::test]
    async fn test_get_error_summary_empty() {
        let db = setup_test_db().await;

        let filter = AnalyticsFilter::default();
        let summary = db.get_error_summary(&filter, 10).await.unwrap();

        assert!(summary.is_empty());
    }

    #[tokio::test]
    async fn test_get_error_summary_groups_by_type() {
        let db = setup_test_db().await;

        let base_time = Utc::now().timestamp();
        // Insert multiple errors of the same type
        for i in 0..5 {
            db.insert_error_event(&create_test_error(
                base_time + i,
                "ConnectionError",
                "network_error",
                "/api/users",
            ))
            .await
            .unwrap();
        }

        // Insert errors of a different type
        for i in 0..3 {
            db.insert_error_event(&create_test_error(
                base_time + i,
                "ValidationError",
                "client_error",
                "/api/orders",
            ))
            .await
            .unwrap();
        }

        let filter = AnalyticsFilter {
            start_time: Some(base_time - 60),
            end_time: Some(base_time + 60),
            ..Default::default()
        };

        let summary = db.get_error_summary(&filter, 10).await.unwrap();

        assert_eq!(summary.len(), 2);
        // Should be sorted by count descending
        assert_eq!(summary[0].count, 5);
        assert_eq!(summary[0].error_type, "ConnectionError");
        assert_eq!(summary[1].count, 3);
        assert_eq!(summary[1].error_type, "ValidationError");
    }

    #[tokio::test]
    async fn test_get_error_summary_collects_endpoints() {
        let db = setup_test_db().await;

        let base_time = Utc::now().timestamp();
        // Same error type from different endpoints
        db.insert_error_event(&create_test_error(
            base_time,
            "Timeout",
            "timeout_error",
            "/api/users",
        ))
        .await
        .unwrap();
        db.insert_error_event(&create_test_error(
            base_time + 1,
            "Timeout",
            "timeout_error",
            "/api/orders",
        ))
        .await
        .unwrap();
        db.insert_error_event(&create_test_error(
            base_time + 2,
            "Timeout",
            "timeout_error",
            "/api/products",
        ))
        .await
        .unwrap();

        let filter = AnalyticsFilter {
            start_time: Some(base_time - 60),
            end_time: Some(base_time + 60),
            ..Default::default()
        };

        let summary = db.get_error_summary(&filter, 10).await.unwrap();

        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].count, 3);
        assert_eq!(summary[0].endpoints.len(), 3);
        assert!(summary[0].endpoints.contains(&"/api/users".to_string()));
        assert!(summary[0].endpoints.contains(&"/api/orders".to_string()));
        assert!(summary[0].endpoints.contains(&"/api/products".to_string()));
    }

    #[tokio::test]
    async fn test_get_error_summary_respects_limit() {
        let db = setup_test_db().await;

        let base_time = Utc::now().timestamp();
        // Create 5 different error types
        for i in 0..5 {
            db.insert_error_event(&create_test_error(
                base_time + i,
                &format!("Error{}", i),
                "server_error",
                "/api/test",
            ))
            .await
            .unwrap();
        }

        let filter = AnalyticsFilter {
            start_time: Some(base_time - 60),
            end_time: Some(base_time + 60),
            ..Default::default()
        };

        let summary = db.get_error_summary(&filter, 3).await.unwrap();

        assert_eq!(summary.len(), 3);
    }

    #[tokio::test]
    async fn test_get_error_summary_tracks_last_occurrence() {
        let db = setup_test_db().await;

        let base_time = 1700000000i64;
        db.insert_error_event(&create_test_error(
            base_time,
            "TestError",
            "server_error",
            "/api/test",
        ))
        .await
        .unwrap();
        db.insert_error_event(&create_test_error(
            base_time + 100,
            "TestError",
            "server_error",
            "/api/test",
        ))
        .await
        .unwrap();
        db.insert_error_event(&create_test_error(
            base_time + 50,
            "TestError",
            "server_error",
            "/api/test",
        ))
        .await
        .unwrap();

        let filter = AnalyticsFilter {
            start_time: Some(base_time - 60),
            end_time: Some(base_time + 200),
            ..Default::default()
        };

        let summary = db.get_error_summary(&filter, 10).await.unwrap();

        assert_eq!(summary.len(), 1);
        // Last occurrence should be the max timestamp
        assert_eq!(summary[0].last_occurrence.timestamp(), base_time + 100);
    }
}
