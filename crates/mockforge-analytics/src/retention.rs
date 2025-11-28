//! Data retention and cleanup service

use crate::config::RetentionConfig;
use crate::database::AnalyticsDatabase;
use crate::error::Result;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info};

/// Data retention service
pub struct RetentionService {
    db: AnalyticsDatabase,
    config: RetentionConfig,
}

impl RetentionService {
    /// Create a new retention service
    #[must_use]
    pub const fn new(db: AnalyticsDatabase, config: RetentionConfig) -> Self {
        Self { db, config }
    }

    /// Start the retention service
    pub async fn start(self: Arc<Self>) {
        info!("Starting data retention service");

        let interval_seconds = u64::from(self.config.cleanup_interval_hours) * 3600;
        let mut interval = interval(Duration::from_secs(interval_seconds));

        loop {
            interval.tick().await;

            if let Err(e) = self.run_cleanup().await {
                error!("Error running data cleanup: {}", e);
            }
        }
    }

    /// Run cleanup for all tables
    async fn run_cleanup(&self) -> Result<()> {
        info!("Running analytics data cleanup");

        // Cleanup minute aggregates
        let deleted = self.db.cleanup_minute_aggregates(self.config.minute_aggregates_days).await?;
        info!("Deleted {} old minute aggregates", deleted);

        // Cleanup hour aggregates
        let deleted = self.db.cleanup_hour_aggregates(self.config.hour_aggregates_days).await?;
        info!("Deleted {} old hour aggregates", deleted);

        // Cleanup error events
        let deleted = self.db.cleanup_error_events(self.config.error_events_days).await?;
        info!("Deleted {} old error events", deleted);

        // Run vacuum to reclaim space
        self.db.vacuum().await?;

        info!("Data cleanup completed successfully");
        Ok(())
    }

    /// Manually trigger cleanup (useful for testing or admin commands)
    pub async fn trigger_cleanup(&self) -> Result<()> {
        self.run_cleanup().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[tokio::test]
    async fn test_retention_service_creation() {
        let db = AnalyticsDatabase::new(Path::new(":memory:")).await.unwrap();
        db.run_migrations().await.unwrap();

        let config = RetentionConfig::default();
        let service = RetentionService::new(db, config);

        // Test manual cleanup
        service.trigger_cleanup().await.unwrap();
    }
}
