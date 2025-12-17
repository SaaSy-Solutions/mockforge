//! Metrics collection for deployed mock services

use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate, Utc};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info};
use uuid::Uuid;

use crate::models::{DeploymentMetrics, HostedMock};

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

        let response = self.client.get(&metrics_url).send().await;

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

        if let Some(mut metrics) = metrics {
            // Update existing metrics
            // In a real implementation, we'd parse the metrics response
            // For now, we'll just increment counters based on health checks

            // This is a placeholder - in production, you'd parse Prometheus metrics
            // or use a metrics API endpoint
            sqlx::query(
                r#"
                UPDATE deployment_metrics
                SET
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(metrics.id)
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
                    response_time_avg_ms,
                    status_2xx_count,
                    status_4xx_count,
                    status_5xx_count
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
