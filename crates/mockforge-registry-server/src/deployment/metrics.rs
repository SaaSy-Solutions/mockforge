//! Metrics collection for deployed mock services

use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate, Utc};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info};
use uuid::Uuid;

use crate::models::{DeploymentMetrics, HostedMock};

#[derive(Debug, Default, Clone, Copy)]
struct ParsedMetrics {
    requests: Option<i64>,
    requests_2xx: Option<i64>,
    requests_4xx: Option<i64>,
    requests_5xx: Option<i64>,
    egress_bytes: Option<i64>,
    avg_response_time_ms: Option<i64>,
}

fn parse_metric_value(metrics: &str, candidates: &[&str]) -> Option<i64> {
    for line in metrics.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        for candidate in candidates {
            if let Some(rest) = line.strip_prefix(candidate) {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if let Some(raw_value) = parts.last() {
                    if let Ok(value) = raw_value.parse::<f64>() {
                        return Some(value.round() as i64);
                    }
                }
            }
        }
    }
    None
}

fn parse_prometheus_metrics(metrics_text: &str) -> ParsedMetrics {
    ParsedMetrics {
        requests: parse_metric_value(
            metrics_text,
            &[
                "http_requests_total",
                "requests_total",
                "mockforge_requests_total",
            ],
        ),
        requests_2xx: parse_metric_value(
            metrics_text,
            &[
                "http_requests_2xx_total",
                "requests_2xx_total",
                "mockforge_requests_2xx_total",
            ],
        ),
        requests_4xx: parse_metric_value(
            metrics_text,
            &[
                "http_requests_4xx_total",
                "requests_4xx_total",
                "mockforge_requests_4xx_total",
            ],
        ),
        requests_5xx: parse_metric_value(
            metrics_text,
            &[
                "http_requests_5xx_total",
                "requests_5xx_total",
                "mockforge_requests_5xx_total",
            ],
        ),
        egress_bytes: parse_metric_value(
            metrics_text,
            &[
                "http_response_size_bytes_total",
                "egress_bytes_total",
                "mockforge_egress_bytes_total",
            ],
        ),
        avg_response_time_ms: parse_metric_value(
            metrics_text,
            &[
                "http_response_time_avg_ms",
                "response_time_avg_ms",
                "mockforge_response_time_avg_ms",
            ],
        ),
    }
}

/// Metrics collector that gathers usage metrics from deployed services
pub struct MetricsCollector {
    db: Arc<PgPool>,
    client: reqwest::Client,
}

impl MetricsCollector {
    pub fn new(db: Arc<PgPool>) -> Self {
        Self {
            db,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Start the metrics collector
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Collect every minute

            loop {
                interval.tick().await;

                if let Err(e) = self.collect_all_metrics().await {
                    error!("Error collecting metrics: {}", e);
                }
            }
        })
    }

    /// Collect metrics from all active deployments
    async fn collect_all_metrics(&self) -> Result<()> {
        let pool = self.db.as_ref();

        // Get all active deployments
        let deployments = sqlx::query_as::<_, HostedMock>(
            r#"
            SELECT * FROM hosted_mocks
            WHERE status = 'active'
            AND deployment_url IS NOT NULL
            AND deleted_at IS NULL
            "#,
        )
        .fetch_all(pool)
        .await
        .context("Failed to fetch active deployments")?;

        for deployment in deployments {
            if let Some(ref deployment_url) = deployment.deployment_url {
                if let Err(e) = self.collect_metrics(&deployment, deployment_url).await {
                    error!("Failed to collect metrics for {}: {}", deployment.id, e);
                }
            }
        }

        Ok(())
    }

    /// Collect metrics from a single deployment
    async fn collect_metrics(&self, deployment: &HostedMock, base_url: &str) -> Result<()> {
        let pool = self.db.as_ref();

        // Try to fetch metrics from /metrics endpoint
        let metrics_url = format!("{}/metrics", base_url);

        let parsed_metrics = match self.client.get(&metrics_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let body = resp.text().await.context("Failed to read metrics response body")?;
                parse_prometheus_metrics(&body)
            }
            Ok(resp) => {
                info!(
                    "Metrics endpoint returned non-success status for {}: {}",
                    deployment.id,
                    resp.status()
                );
                ParsedMetrics::default()
            }
            Err(err) => {
                info!(
                    "Metrics endpoint unavailable for {} ({}), keeping previous values",
                    deployment.id, err
                );
                ParsedMetrics::default()
            }
        };

        // If metrics endpoint doesn't exist, we'll estimate from logs
        // For now, we'll just update basic counters
        let period_start = NaiveDate::from_ymd_opt(Utc::now().year(), Utc::now().month(), 1)
            .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;

        // Get or create metrics record for this period
        let metrics = sqlx::query_as::<_, DeploymentMetrics>(
            r#"
            SELECT * FROM deployment_metrics
            WHERE hosted_mock_id = $1 AND period_start = $2
            "#,
        )
        .bind(deployment.id)
        .bind(period_start)
        .fetch_optional(pool)
        .await?;

        if let Some(metrics) = metrics {
            // Update existing metrics from parsed values when available.
            sqlx::query(
                r#"
                UPDATE deployment_metrics
                SET
                    requests = $2,
                    requests_2xx = $3,
                    requests_4xx = $4,
                    requests_5xx = $5,
                    egress_bytes = $6,
                    avg_response_time_ms = $7,
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(metrics.id)
            .bind(parsed_metrics.requests.unwrap_or(metrics.requests))
            .bind(parsed_metrics.requests_2xx.unwrap_or(metrics.requests_2xx))
            .bind(parsed_metrics.requests_4xx.unwrap_or(metrics.requests_4xx))
            .bind(parsed_metrics.requests_5xx.unwrap_or(metrics.requests_5xx))
            .bind(parsed_metrics.egress_bytes.unwrap_or(metrics.egress_bytes))
            .bind(parsed_metrics.avg_response_time_ms.unwrap_or(metrics.avg_response_time_ms))
            .execute(pool)
            .await?;
        } else {
            // Create new metrics record
            sqlx::query(
                r#"
                INSERT INTO deployment_metrics (
                    hosted_mock_id,
                    period_start,
                    requests,
                    egress_bytes,
                    avg_response_time_ms,
                    requests_2xx,
                    requests_4xx,
                    requests_5xx
                ) VALUES ($1, $2, 0, 0, 0, 0, 0, 0)
                "#,
            )
            .bind(deployment.id)
            .bind(period_start)
            .execute(pool)
            .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_prometheus_metrics_values() {
        let body = r#"
# HELP http_requests_total Total HTTP requests
http_requests_total 123
requests_2xx_total 100
requests_4xx_total 20
requests_5xx_total 3
egress_bytes_total 4096
response_time_avg_ms 42
"#;

        let parsed = parse_prometheus_metrics(body);
        assert_eq!(parsed.requests, Some(123));
        assert_eq!(parsed.requests_2xx, Some(100));
        assert_eq!(parsed.requests_4xx, Some(20));
        assert_eq!(parsed.requests_5xx, Some(3));
        assert_eq!(parsed.egress_bytes, Some(4096));
        assert_eq!(parsed.avg_response_time_ms, Some(42));
    }
}
