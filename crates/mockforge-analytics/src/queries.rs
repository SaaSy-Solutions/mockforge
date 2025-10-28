//! High-level query API for analytics data

use crate::database::AnalyticsDatabase;
use crate::error::Result;
use crate::models::*;
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
        let p95_latency_ms = if !p95_latencies.is_empty() {
            p95_latencies.iter().sum::<f64>() / p95_latencies.len() as f64
        } else {
            0.0
        };

        let p99_latencies: Vec<f64> = aggregates.iter().filter_map(|a| a.latency_p99).collect();
        let p99_latency_ms = if !p99_latencies.is_empty() {
            p99_latencies.iter().sum::<f64>() / p99_latencies.len() as f64
        } else {
            0.0
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

            let entry = error_map.entry(format!("{}:{}", error_category, error_type)).or_insert((
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
                    error_type: parts.get(1).unwrap_or(&"unknown").to_string(),
                    error_category: parts.first().unwrap_or(&"other").to_string(),
                    count,
                    endpoints: endpoints.into_iter().collect(),
                    last_occurrence: DateTime::from_timestamp(last_ts, 0)
                        .unwrap_or_else(|| Utc::now()),
                }
            })
            .collect();

        summaries.sort_by(|a, b| b.count.cmp(&a.count));
        summaries.truncate(limit as usize);

        Ok(summaries)
    }
}
