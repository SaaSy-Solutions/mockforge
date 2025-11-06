//! Background task scheduler
//!
//! This module provides background tasks for data evolution, periodic cleanup jobs,
//! and time-based field updates.

use crate::Result;
use tokio::time::{interval, Duration};

/// Background task scheduler
pub struct Scheduler {
    /// Cleanup interval
    cleanup_interval: Duration,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new(cleanup_interval_secs: u64) -> Self {
        Self {
            cleanup_interval: Duration::from_secs(cleanup_interval_secs),
        }
    }

    /// Start background cleanup tasks
    ///
    /// Runs periodic cleanup tasks in the background. This should be spawned
    /// as a tokio task that runs for the lifetime of the application.
    pub async fn start_cleanup_tasks(
        &self,
        aging_manager: std::sync::Arc<crate::aging::AgingManager>,
        database: std::sync::Arc<dyn crate::database::VirtualDatabase + Send + Sync>,
        registry: std::sync::Arc<tokio::sync::RwLock<crate::entities::EntityRegistry>>,
    ) -> Result<()> {
        let mut interval = interval(self.cleanup_interval);

        loop {
            interval.tick().await;

            // Run cleanup tasks
            let registry_read = registry.read().await;
            if let Err(e) = aging_manager.cleanup_expired(database.as_ref(), &registry_read).await {
                tracing::warn!("Error during data aging cleanup: {}", e);
            }
            drop(registry_read);
        }
    }

    /// Spawn cleanup tasks as a background task
    ///
    /// Returns a handle that can be used to abort the task.
    pub fn spawn_cleanup_tasks(
        self,
        aging_manager: std::sync::Arc<crate::aging::AgingManager>,
        database: std::sync::Arc<dyn crate::database::VirtualDatabase + Send + Sync>,
        registry: std::sync::Arc<tokio::sync::RwLock<crate::entities::EntityRegistry>>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            if let Err(e) = self.start_cleanup_tasks(aging_manager, database, registry).await {
                tracing::error!("Cleanup task error: {}", e);
            }
        })
    }
}
