//! Metrics aggregation service
//!
//! This module provides background services that:
//! - Query Prometheus metrics at regular intervals
//! - Aggregate and store metrics in the analytics database
//! - Roll up minute data to hour/day granularity

use crate::config::AnalyticsConfig;
use crate::database::AnalyticsDatabase;
use crate::error::Result;
use crate::models::{
    AnalyticsFilter, DayMetricsAggregate, EndpointStats, HourMetricsAggregate, MetricsAggregate,
};
use chrono::{Timelike, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

/// Prometheus query client
#[derive(Clone)]
pub struct PrometheusClient {
    base_url: String,
    client: Client,
}

impl PrometheusClient {
    /// Create a new Prometheus client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
        }
    }

    /// Execute a Prometheus instant query
    pub async fn query(&self, query: &str, time: Option<i64>) -> Result<PrometheusResponse> {
        let mut url = format!("{}/api/v1/query", self.base_url);
        url.push_str(&format!("?query={}", urlencoding::encode(query)));

        if let Some(t) = time {
            url.push_str(&format!("&time={t}"));
        }

        let response = self.client.get(&url).send().await?.json::<PrometheusResponse>().await?;

        Ok(response)
    }

    /// Execute a Prometheus range query
    pub async fn query_range(
        &self,
        query: &str,
        start: i64,
        end: i64,
        step: &str,
    ) -> Result<PrometheusResponse> {
        let url = format!(
            "{}/api/v1/query_range?query={}&start={}&end={}&step={}",
            self.base_url,
            urlencoding::encode(query),
            start,
            end,
            step
        );

        let response = self.client.get(&url).send().await?.json::<PrometheusResponse>().await?;

        Ok(response)
    }
}

/// Prometheus API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusResponse {
    pub status: String,
    pub data: PrometheusData,
}

/// Prometheus data payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrometheusData {
    pub result_type: String,
    pub result: Vec<PrometheusResult>,
}

/// Prometheus query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusResult {
    pub metric: HashMap<String, String>,
    pub value: Option<PrometheusValue>,
    pub values: Option<Vec<PrometheusValue>>,
}

/// Prometheus metric value (timestamp, value)
pub type PrometheusValue = (f64, String);

/// Metrics aggregation service
pub struct MetricsAggregator {
    db: AnalyticsDatabase,
    prom_client: PrometheusClient,
    config: AnalyticsConfig,
}

impl MetricsAggregator {
    /// Create a new metrics aggregator
    pub fn new(
        db: AnalyticsDatabase,
        prometheus_url: impl Into<String>,
        config: AnalyticsConfig,
    ) -> Self {
        Self {
            db,
            prom_client: PrometheusClient::new(prometheus_url),
            config,
        }
    }

    /// Start the aggregation service
    pub async fn start(self: Arc<Self>) {
        info!("Starting metrics aggregation service");

        // Spawn minute aggregation task
        let self_clone = Arc::clone(&self);
        tokio::spawn(async move {
            self_clone.run_minute_aggregation().await;
        });

        // Spawn hourly rollup task
        let self_clone = Arc::clone(&self);
        tokio::spawn(async move {
            self_clone.run_hourly_rollup().await;
        });

        // Spawn daily rollup task
        let self_clone = Arc::clone(&self);
        tokio::spawn(async move {
            self_clone.run_daily_rollup().await;
        });
    }

    /// Run minute-level aggregation loop
    async fn run_minute_aggregation(&self) {
        let mut interval = interval(Duration::from_secs(self.config.aggregation_interval_seconds));

        loop {
            interval.tick().await;

            if let Err(e) = self.aggregate_minute_metrics().await {
                error!("Error aggregating minute metrics: {}", e);
            }
        }
    }

    /// Aggregate metrics for the last minute
    async fn aggregate_minute_metrics(&self) -> Result<()> {
        let now = Utc::now();
        let minute_start =
            now.with_second(0).unwrap().with_nanosecond(0).unwrap() - chrono::Duration::minutes(1);
        let timestamp = minute_start.timestamp();

        debug!("Aggregating metrics for minute: {}", minute_start);

        // Query request counts by protocol, method, path
        let query = r"sum by (protocol, method, path, status) (
                increase(mockforge_requests_by_path_total{}[1m]) > 0
            )"
        .to_string();

        let response = self.prom_client.query(&query, Some(timestamp)).await?;

        let mut aggregates = Vec::new();

        for result in response.data.result {
            let protocol = result
                .metric
                .get("protocol")
                .map_or_else(|| "unknown".to_string(), ToString::to_string);
            let method = result.metric.get("method").cloned();
            let endpoint = result.metric.get("path").cloned();
            let status_code = result.metric.get("status").and_then(|s| s.parse::<i32>().ok());

            let request_count = if let Some((_, value)) = result.value {
                value.parse::<f64>().unwrap_or(0.0) as i64
            } else {
                0
            };

            // Query latency metrics for this combination
            let latency_query = if let (Some(ref p), Some(ref m), Some(ref e)) =
                (&Some(protocol.clone()), &method, &endpoint)
            {
                format!(
                    r#"histogram_quantile(0.95, sum(rate(mockforge_request_duration_by_path_seconds_bucket{{protocol="{p}",method="{m}",path="{e}"}}[1m])) by (le)) * 1000"#
                )
            } else {
                continue;
            };

            let latency_p95 = match self.prom_client.query(&latency_query, Some(timestamp)).await {
                Ok(resp) => resp
                    .data
                    .result
                    .first()
                    .and_then(|r| r.value.as_ref().and_then(|(_, v)| v.parse::<f64>().ok())),
                Err(e) => {
                    warn!("Failed to query latency: {}", e);
                    None
                }
            };

            let agg = MetricsAggregate {
                id: None,
                timestamp,
                protocol: protocol.clone(),
                method: method.clone(),
                endpoint: endpoint.clone(),
                status_code,
                workspace_id: None,
                environment: None,
                request_count,
                error_count: if let Some(sc) = status_code {
                    if sc >= 400 {
                        request_count
                    } else {
                        0
                    }
                } else {
                    0
                },
                latency_sum: 0.0,
                latency_min: None,
                latency_max: None,
                latency_p50: None,
                latency_p95,
                latency_p99: None,
                bytes_sent: 0,
                bytes_received: 0,
                active_connections: None,
                created_at: None,
            };

            aggregates.push(agg);
        }

        if !aggregates.is_empty() {
            self.db.insert_minute_aggregates_batch(&aggregates).await?;
            info!("Stored {} minute aggregates", aggregates.len());

            // Also update endpoint stats
            for agg in &aggregates {
                let stats = EndpointStats {
                    id: None,
                    endpoint: agg.endpoint.clone().unwrap_or_default(),
                    protocol: agg.protocol.clone(),
                    method: agg.method.clone(),
                    workspace_id: agg.workspace_id.clone(),
                    environment: agg.environment.clone(),
                    total_requests: agg.request_count,
                    total_errors: agg.error_count,
                    avg_latency_ms: agg.latency_p95,
                    min_latency_ms: agg.latency_min,
                    max_latency_ms: agg.latency_max,
                    p95_latency_ms: agg.latency_p95,
                    status_codes: None,
                    total_bytes_sent: agg.bytes_sent,
                    total_bytes_received: agg.bytes_received,
                    first_seen: timestamp,
                    last_seen: timestamp,
                    updated_at: None,
                };

                if let Err(e) = self.db.upsert_endpoint_stats(&stats).await {
                    warn!("Failed to update endpoint stats: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Run hourly rollup loop
    async fn run_hourly_rollup(&self) {
        let mut interval = interval(Duration::from_secs(self.config.rollup_interval_hours * 3600));

        loop {
            interval.tick().await;

            if let Err(e) = self.rollup_to_hour().await {
                error!("Error rolling up to hourly metrics: {}", e);
            }
        }
    }

    /// Roll up minute data to hour-level aggregates
    async fn rollup_to_hour(&self) -> Result<()> {
        let now = Utc::now();
        let hour_start =
            now.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap()
                - chrono::Duration::hours(1);
        let hour_end = hour_start + chrono::Duration::hours(1);

        info!("Rolling up metrics to hour: {}", hour_start);

        let filter = AnalyticsFilter {
            start_time: Some(hour_start.timestamp()),
            end_time: Some(hour_end.timestamp()),
            ..Default::default()
        };

        let minute_data = self.db.get_minute_aggregates(&filter).await?;

        if minute_data.is_empty() {
            debug!("No minute data to roll up");
            return Ok(());
        }

        // Group by protocol, method, endpoint, status_code
        let mut groups: HashMap<
            (String, Option<String>, Option<String>, Option<i32>),
            Vec<&MetricsAggregate>,
        > = HashMap::new();

        for agg in &minute_data {
            let key =
                (agg.protocol.clone(), agg.method.clone(), agg.endpoint.clone(), agg.status_code);
            groups.entry(key).or_default().push(agg);
        }

        for ((protocol, method, endpoint, status_code), group) in groups {
            let request_count: i64 = group.iter().map(|a| a.request_count).sum();
            let error_count: i64 = group.iter().map(|a| a.error_count).sum();
            let latency_sum: f64 = group.iter().map(|a| a.latency_sum).sum();
            let latency_min =
                group.iter().filter_map(|a| a.latency_min).fold(f64::INFINITY, f64::min);
            let latency_max =
                group.iter().filter_map(|a| a.latency_max).fold(f64::NEG_INFINITY, f64::max);

            let hour_agg = HourMetricsAggregate {
                id: None,
                timestamp: hour_start.timestamp(),
                protocol,
                method,
                endpoint,
                status_code,
                workspace_id: None,
                environment: None,
                request_count,
                error_count,
                latency_sum,
                latency_min: if latency_min.is_finite() {
                    Some(latency_min)
                } else {
                    None
                },
                latency_max: if latency_max.is_finite() {
                    Some(latency_max)
                } else {
                    None
                },
                latency_p50: None,
                latency_p95: None,
                latency_p99: None,
                bytes_sent: group.iter().map(|a| a.bytes_sent).sum(),
                bytes_received: group.iter().map(|a| a.bytes_received).sum(),
                active_connections_avg: None,
                active_connections_max: group.iter().filter_map(|a| a.active_connections).max(),
                created_at: None,
            };

            self.db.insert_hour_aggregate(&hour_agg).await?;
        }

        info!("Rolled up {} minute aggregates into hour aggregates", minute_data.len());
        Ok(())
    }

    /// Run daily rollup loop
    async fn run_daily_rollup(&self) {
        let mut interval = interval(Duration::from_secs(86400)); // Daily

        loop {
            interval.tick().await;

            if let Err(e) = self.rollup_to_day().await {
                error!("Error rolling up to daily metrics: {}", e);
            }
        }
    }

    /// Roll up hour data to day-level aggregates
    async fn rollup_to_day(&self) -> Result<()> {
        let now = Utc::now();
        let day_start = now
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap()
            - chrono::Duration::days(1);
        let day_end = day_start + chrono::Duration::days(1);

        info!("Rolling up metrics to day: {}", day_start.format("%Y-%m-%d"));

        let filter = AnalyticsFilter {
            start_time: Some(day_start.timestamp()),
            end_time: Some(day_end.timestamp()),
            ..Default::default()
        };

        let hour_data = self.db.get_hour_aggregates(&filter).await?;

        if hour_data.is_empty() {
            debug!("No hour data to roll up");
            return Ok(());
        }

        // Group by protocol, method, endpoint, status_code
        let mut groups: HashMap<
            (String, Option<String>, Option<String>, Option<i32>),
            Vec<&HourMetricsAggregate>,
        > = HashMap::new();

        for agg in &hour_data {
            let key =
                (agg.protocol.clone(), agg.method.clone(), agg.endpoint.clone(), agg.status_code);
            groups.entry(key).or_default().push(agg);
        }

        // Find peak hour (hour with max request count)
        let mut peak_hour: Option<i32> = None;
        let mut max_requests = 0i64;
        for agg in &hour_data {
            if agg.request_count > max_requests {
                max_requests = agg.request_count;
                // Extract hour from timestamp
                if let Some(dt) = chrono::DateTime::from_timestamp(agg.timestamp, 0) {
                    peak_hour = Some(dt.hour() as i32);
                }
            }
        }

        for ((protocol, method, endpoint, status_code), group) in groups {
            let request_count: i64 = group.iter().map(|a| a.request_count).sum();
            let error_count: i64 = group.iter().map(|a| a.error_count).sum();
            let latency_sum: f64 = group.iter().map(|a| a.latency_sum).sum();
            let latency_min =
                group.iter().filter_map(|a| a.latency_min).fold(f64::INFINITY, f64::min);
            let latency_max =
                group.iter().filter_map(|a| a.latency_max).fold(f64::NEG_INFINITY, f64::max);

            // Calculate percentiles from hour aggregates (average of hour percentiles)
            let latency_p50_avg: Option<f64> = {
                let p50_values: Vec<f64> = group.iter().filter_map(|a| a.latency_p50).collect();
                if !p50_values.is_empty() {
                    Some(p50_values.iter().sum::<f64>() / p50_values.len() as f64)
                } else {
                    None
                }
            };
            let latency_p95_avg: Option<f64> = {
                let p95_values: Vec<f64> = group.iter().filter_map(|a| a.latency_p95).collect();
                if !p95_values.is_empty() {
                    Some(p95_values.iter().sum::<f64>() / p95_values.len() as f64)
                } else {
                    None
                }
            };
            let latency_p99_avg: Option<f64> = {
                let p99_values: Vec<f64> = group.iter().filter_map(|a| a.latency_p99).collect();
                if !p99_values.is_empty() {
                    Some(p99_values.iter().sum::<f64>() / p99_values.len() as f64)
                } else {
                    None
                }
            };

            // Average active connections
            let active_connections_avg: Option<f64> = {
                let avg_values: Vec<f64> =
                    group.iter().filter_map(|a| a.active_connections_avg).collect();
                if !avg_values.is_empty() {
                    Some(avg_values.iter().sum::<f64>() / avg_values.len() as f64)
                } else {
                    None
                }
            };

            // Max active connections
            let active_connections_max =
                group.iter().filter_map(|a| a.active_connections_max).max();

            let day_agg = DayMetricsAggregate {
                id: None,
                date: day_start.format("%Y-%m-%d").to_string(),
                timestamp: day_start.timestamp(),
                protocol,
                method,
                endpoint,
                status_code,
                workspace_id: group.first().and_then(|a| a.workspace_id.clone()),
                environment: group.first().and_then(|a| a.environment.clone()),
                request_count,
                error_count,
                latency_sum,
                latency_min: if latency_min.is_finite() {
                    Some(latency_min)
                } else {
                    None
                },
                latency_max: if latency_max.is_finite() {
                    Some(latency_max)
                } else {
                    None
                },
                latency_p50: latency_p50_avg,
                latency_p95: latency_p95_avg,
                latency_p99: latency_p99_avg,
                bytes_sent: group.iter().map(|a| a.bytes_sent).sum(),
                bytes_received: group.iter().map(|a| a.bytes_received).sum(),
                active_connections_avg,
                active_connections_max,
                unique_clients: None, // Would need to track unique clients separately
                peak_hour,
                created_at: None,
            };

            self.db.insert_day_aggregate(&day_agg).await?;
        }

        info!("Rolled up {} hour aggregates into day aggregates", hour_data.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prometheus_client_creation() {
        let client = PrometheusClient::new("http://localhost:9090");
        assert_eq!(client.base_url, "http://localhost:9090");
    }
}
