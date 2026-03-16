//! Deployment cleanup worker
//!
//! Periodically cleans up orphaned, stuck, and soft-deleted deployment resources.

use anyhow::{Context, Result};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use crate::deployment::flyio::FlyioClient;
use crate::models::DeploymentStatus;

/// Background worker that handles deployment lifecycle cleanup
pub struct DeploymentCleanup {
    db: Arc<PgPool>,
    flyio_client: Option<FlyioClient>,
}

impl DeploymentCleanup {
    pub fn new(db: Arc<PgPool>, flyio_client: Option<FlyioClient>) -> Self {
        Self { db, flyio_client }
    }

    /// Start the cleanup worker (runs every hour)
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3600)); // Every 1 hour

            loop {
                interval.tick().await;

                if let Err(e) = self.run_cleanup().await {
                    error!("Error during deployment cleanup: {}", e);
                }
            }
        })
    }

    /// Run all cleanup tasks
    async fn run_cleanup(&self) -> Result<()> {
        self.hard_delete_old_records().await?;
        self.mark_stuck_deployments().await?;
        self.retry_stuck_deletions().await?;
        Ok(())
    }

    /// Hard-delete rows that were soft-deleted more than 30 days ago
    async fn hard_delete_old_records(&self) -> Result<()> {
        let pool = self.db.as_ref();

        let result = sqlx::query(
            r#"
            DELETE FROM hosted_mocks
            WHERE deleted_at IS NOT NULL
            AND deleted_at < NOW() - INTERVAL '30 days'
            "#,
        )
        .execute(pool)
        .await
        .context("Failed to hard-delete old records")?;

        let count = result.rows_affected();
        if count > 0 {
            info!("Hard-deleted {} old soft-deleted deployment records", count);
        }

        Ok(())
    }

    /// Mark deployments stuck in 'deploying' status for >1 hour as failed
    async fn mark_stuck_deployments(&self) -> Result<()> {
        let pool = self.db.as_ref();

        let result = sqlx::query(
            r#"
            UPDATE hosted_mocks
            SET status = $1, error_message = 'Deployment timed out', updated_at = NOW()
            WHERE status = 'deploying'
            AND updated_at < NOW() - INTERVAL '1 hour'
            AND deleted_at IS NULL
            "#,
        )
        .bind(DeploymentStatus::Failed.to_string())
        .execute(pool)
        .await
        .context("Failed to mark stuck deployments")?;

        let count = result.rows_affected();
        if count > 0 {
            warn!("Marked {} stuck deployments as failed", count);
        }

        Ok(())
    }

    /// Retry deletions stuck in 'deleting' status for >1 hour
    async fn retry_stuck_deletions(&self) -> Result<()> {
        let pool = self.db.as_ref();

        let stuck = sqlx::query_as::<_, crate::models::HostedMock>(
            r#"
            SELECT * FROM hosted_mocks
            WHERE status = 'deleting'
            AND updated_at < NOW() - INTERVAL '1 hour'
            AND deleted_at IS NULL
            "#,
        )
        .fetch_all(pool)
        .await
        .context("Failed to fetch stuck deletions")?;

        if stuck.is_empty() {
            return Ok(());
        }

        warn!("Found {} deployments stuck in deleting state", stuck.len());

        for deployment in &stuck {
            // Try to clean up Fly.io resources if possible
            if let Some(ref flyio_client) = self.flyio_client {
                let app_name = format!(
                    "mockforge-{}-{}",
                    deployment
                        .org_id
                        .to_string()
                        .replace('-', "")
                        .chars()
                        .take(8)
                        .collect::<String>(),
                    deployment.slug
                );

                // Try to delete any remaining machines
                if let Ok(machines) = flyio_client.list_machines(&app_name).await {
                    for machine in machines {
                        if let Err(e) = flyio_client.delete_machine(&app_name, &machine.id).await {
                            warn!("Cleanup: failed to delete machine {}: {}", machine.id, e);
                        }
                    }
                }

                // Try to delete the app
                if let Err(e) = flyio_client.delete_app(&app_name).await {
                    warn!("Cleanup: failed to delete app {}: {}", app_name, e);
                }
            }

            // Soft-delete the record regardless
            sqlx::query(
                r#"
                UPDATE hosted_mocks
                SET deleted_at = NOW(), updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(deployment.id)
            .execute(pool)
            .await
            .ok();

            info!("Cleanup: completed stuck deletion for deployment {}", deployment.id);
        }

        Ok(())
    }
}
