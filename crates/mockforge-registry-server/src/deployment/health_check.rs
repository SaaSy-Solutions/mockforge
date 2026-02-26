//! Health check worker for deployed mock services

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, warn};

use crate::models::{DeploymentLog, HealthStatus, HostedMock};

/// Health check worker that periodically checks deployed services
pub struct HealthCheckWorker {
    db: Arc<PgPool>,
    client: reqwest::Client,
}

impl HealthCheckWorker {
    pub fn new(db: Arc<PgPool>) -> Self {
        Self {
            db,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Start the health check worker
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds

            loop {
                interval.tick().await;

                if let Err(e) = self.check_all_deployments().await {
                    error!("Error checking deployment health: {}", e);
                }
            }
        })
    }

    /// Check health of all active deployments
    async fn check_all_deployments(&self) -> Result<()> {
        let pool = self.db.as_ref();

        // Get all active deployments with health check URLs
        let deployments = sqlx::query_as::<_, HostedMock>(
            r#"
            SELECT * FROM hosted_mocks
            WHERE status = 'active'
            AND health_check_url IS NOT NULL
            AND deleted_at IS NULL
            "#,
        )
        .fetch_all(pool)
        .await
        .context("Failed to fetch active deployments")?;

        for deployment in deployments {
            if let Some(ref health_url) = deployment.health_check_url {
                let health_status = self.check_health(health_url).await;

                let status = match health_status {
                    Ok(true) => HealthStatus::Healthy,
                    Ok(false) => HealthStatus::Unhealthy,
                    Err(e) => {
                        warn!("Health check failed for {}: {}", deployment.id, e);
                        HealthStatus::Unhealthy
                    }
                };

                // Update health status
                sqlx::query(
                    r#"
                    UPDATE hosted_mocks
                    SET
                        health_status = $1,
                        last_health_check = NOW(),
                        updated_at = NOW()
                    WHERE id = $2
                    "#,
                )
                .bind(status.to_string())
                .bind(deployment.id)
                .execute(pool)
                .await?;

                // If unhealthy for too long, mark as failed
                if matches!(status, HealthStatus::Unhealthy) {
                    self.handle_unhealthy_deployment(&deployment).await?;
                }
            }
        }

        Ok(())
    }

    /// Check health of a single deployment
    async fn check_health(&self, url: &str) -> Result<bool> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to send health check request")?;

        Ok(response.status().is_success())
    }

    /// Handle unhealthy deployment
    async fn handle_unhealthy_deployment(&self, deployment: &HostedMock) -> Result<()> {
        let pool = self.db.as_ref();

        // Check if it's been unhealthy for more than 5 minutes
        if let Some(last_check) = deployment.last_health_check {
            let unhealthy_duration = Utc::now() - last_check;
            if unhealthy_duration.num_minutes() > 5 {
                warn!(
                    "Deployment {} has been unhealthy for {} minutes",
                    deployment.id,
                    unhealthy_duration.num_minutes()
                );

                // Log warning
                DeploymentLog::create(
                    pool,
                    deployment.id,
                    "warning",
                    &format!(
                        "Service has been unhealthy for {} minutes",
                        unhealthy_duration.num_minutes()
                    ),
                    None,
                )
                .await?;

                // Optionally: mark as failed if unhealthy for too long
                if unhealthy_duration.num_minutes() > 15 {
                    use crate::models::hosted_mock::HostedMock;
                    use crate::models::DeploymentStatus;

                    HostedMock::update_status(
                        pool,
                        deployment.id,
                        DeploymentStatus::Failed,
                        Some("Service unhealthy for more than 15 minutes"),
                    )
                    .await?;

                    DeploymentLog::create(
                        pool,
                        deployment.id,
                        "error",
                        "Service marked as failed due to prolonged unhealthy status",
                        None,
                    )
                    .await?;
                }
            }
        }

        Ok(())
    }
}
