//! Cron scheduler for simulated recurring events
//!
//! This module provides a cron-like scheduler that integrates with the virtual clock
//! to support time-based recurring events. It works alongside the ResponseScheduler
//! to handle complex recurring schedules while ResponseScheduler handles one-time
//! and simple interval responses.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use mockforge_core::time_travel::{CronScheduler, CronJob, CronJobAction};
//! use std::sync::Arc;
//!
//! let scheduler = CronScheduler::new(clock.clone());
//!
//! // Schedule a job that runs every day at 3am
//! let job = CronJob {
//!     id: "daily-cleanup".to_string(),
//!     name: "Daily Cleanup".to_string(),
//!     schedule: "0 3 * * *".to_string(), // 3am every day
//!     action: CronJobAction::Callback(Box::new(|_| {
//!         println!("Running daily cleanup");
//!         Ok(())
//!     })),
//!     enabled: true,
//! };
//!
//! scheduler.add_job(job).await?;
//! ```

use chrono::{DateTime, Utc};
use cron::Schedule as CronSchedule;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::{get_global_clock, VirtualClock};

/// Action to execute when a cron job triggers
pub enum CronJobAction {
    /// Execute a callback function
    Callback(Box<dyn Fn(DateTime<Utc>) -> Result<(), String> + Send + Sync>),
    /// Send a scheduled response (integrated with ResponseScheduler)
    ScheduledResponse {
        /// Response body
        body: serde_json::Value,
        /// HTTP status code
        status: u16,
        /// Response headers
        headers: HashMap<String, String>,
    },
    /// Trigger a data mutation (for VBR integration)
    DataMutation {
        /// Entity name
        entity: String,
        /// Mutation operation
        operation: String,
    },
}

// Note: CronJobAction cannot be Serialized/Deserialized due to the callback.
// For persistence, we'll need to store job metadata separately.

/// A cron job definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    /// Unique identifier for this job
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Cron expression (e.g., "0 3 * * *" for 3am daily)
    pub schedule: String,
    /// Whether this job is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    /// Last execution time
    #[serde(default)]
    pub last_execution: Option<DateTime<Utc>>,
    /// Next scheduled execution time
    #[serde(default)]
    pub next_execution: Option<DateTime<Utc>>,
    /// Number of times this job has executed
    #[serde(default)]
    pub execution_count: usize,
    /// Action type (for serialization - actual action stored separately)
    #[serde(default)]
    pub action_type: String,
    /// Action metadata (for serialization)
    #[serde(default)]
    pub action_metadata: serde_json::Value,
}

fn default_true() -> bool {
    true
}

impl CronJob {
    /// Create a new cron job
    pub fn new(id: String, name: String, schedule: String) -> Self {
        Self {
            id,
            name,
            schedule,
            enabled: true,
            description: None,
            last_execution: None,
            next_execution: None,
            execution_count: 0,
            action_type: String::new(),
            action_metadata: serde_json::Value::Null,
        }
    }

    /// Calculate the next execution time based on the cron schedule
    pub fn calculate_next_execution(&self, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
        if !self.enabled {
            return None;
        }

        match CronSchedule::from_str(&self.schedule) {
            Ok(schedule) => {
                // Get the next occurrence after the given time
                schedule.after(&from).next()
            }
            Err(e) => {
                warn!("Invalid cron schedule '{}' for job '{}': {}", self.schedule, self.id, e);
                None
            }
        }
    }
}

/// Cron scheduler that integrates with the virtual clock
pub struct CronScheduler {
    /// Virtual clock reference
    clock: Arc<VirtualClock>,
    /// Registered cron jobs
    jobs: Arc<RwLock<HashMap<String, CronJob>>>,
    /// Job actions (stored separately since they can't be serialized)
    actions: Arc<RwLock<HashMap<String, Arc<CronJobAction>>>>,
}

impl CronScheduler {
    /// Create a new cron scheduler
    pub fn new(clock: Arc<VirtualClock>) -> Self {
        Self {
            clock,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            actions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new cron scheduler using the global clock
    ///
    /// This will use the global virtual clock registry if available,
    /// or create a new clock if none is registered.
    pub fn new_with_global_clock() -> Self {
        let clock = get_global_clock().unwrap_or_else(|| Arc::new(VirtualClock::new()));
        Self::new(clock)
    }

    /// Add a cron job
    pub async fn add_job(&self, job: CronJob, action: CronJobAction) -> Result<(), String> {
        // Validate cron expression
        CronSchedule::from_str(&job.schedule)
            .map_err(|e| format!("Invalid cron expression '{}': {}", job.schedule, e))?;

        // Calculate next execution time
        let now = self.clock.now();
        let next_execution = job.calculate_next_execution(now);

        let mut job_with_next = job;
        job_with_next.next_execution = next_execution;

        let job_id = job_with_next.id.clone();

        // Store job and action
        let mut jobs = self.jobs.write().await;
        jobs.insert(job_id.clone(), job_with_next);

        let mut actions = self.actions.write().await;
        actions.insert(job_id.clone(), Arc::new(action));

        info!("Added cron job '{}' with schedule '{}'", job_id, jobs[&job_id].schedule);
        Ok(())
    }

    /// Remove a cron job
    pub async fn remove_job(&self, job_id: &str) -> bool {
        let mut jobs = self.jobs.write().await;
        let mut actions = self.actions.write().await;

        let removed = jobs.remove(job_id).is_some();
        actions.remove(job_id);

        if removed {
            info!("Removed cron job '{}'", job_id);
        }

        removed
    }

    /// Get a cron job by ID
    pub async fn get_job(&self, job_id: &str) -> Option<CronJob> {
        let jobs = self.jobs.read().await;
        jobs.get(job_id).cloned()
    }

    /// List all cron jobs
    pub async fn list_jobs(&self) -> Vec<CronJob> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }

    /// Enable or disable a cron job
    pub async fn set_job_enabled(&self, job_id: &str, enabled: bool) -> Result<(), String> {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get_mut(job_id) {
            job.enabled = enabled;

            // Recalculate next execution if enabling
            if enabled {
                let now = self.clock.now();
                job.next_execution = job.calculate_next_execution(now);
            } else {
                job.next_execution = None;
            }

            info!("Cron job '{}' {}", job_id, if enabled { "enabled" } else { "disabled" });
            Ok(())
        } else {
            Err(format!("Cron job '{}' not found", job_id))
        }
    }

    /// Check for jobs that should execute now and execute them
    ///
    /// This should be called periodically (e.g., every second) to check
    /// if any jobs are due for execution.
    pub async fn check_and_execute(&self) -> Result<usize, String> {
        let now = self.clock.now();
        let mut executed = 0;

        // Get jobs that need to execute
        let mut jobs_to_execute = Vec::new();

        {
            let jobs = self.jobs.read().await;
            for job in jobs.values() {
                if !job.enabled {
                    continue;
                }

                if let Some(next) = job.next_execution {
                    if now >= next {
                        jobs_to_execute.push(job.id.clone());
                    }
                }
            }
        }

        // Execute jobs
        for job_id in jobs_to_execute {
            if let Err(e) = self.execute_job(&job_id).await {
                warn!("Error executing cron job '{}': {}", job_id, e);
            } else {
                executed += 1;
            }
        }

        Ok(executed)
    }

    /// Execute a specific cron job
    async fn execute_job(&self, job_id: &str) -> Result<(), String> {
        let now = self.clock.now();

        // Get job and action
        let (job, action) = {
            let jobs = self.jobs.read().await;
            let actions = self.actions.read().await;

            let job = jobs.get(job_id).ok_or_else(|| format!("Job '{}' not found", job_id))?;
            let action = actions
                .get(job_id)
                .ok_or_else(|| format!("Action for job '{}' not found", job_id))?;

            (job.clone(), Arc::clone(action))
        };

        // Execute the action
        match action.as_ref() {
            CronJobAction::Callback(callback) => {
                debug!("Executing callback for cron job '{}'", job_id);
                callback(now)?;
            }
            CronJobAction::ScheduledResponse {
                body,
                status,
                headers,
            } => {
                debug!("Scheduled response for cron job '{}'", job_id);
                // TODO: Integrate with ResponseScheduler
                // For now, just log
                info!("Cron job '{}' triggered scheduled response: {}", job_id, status);
            }
            CronJobAction::DataMutation { entity, operation } => {
                debug!("Data mutation for cron job '{}': {} on {}", job_id, operation, entity);
                // TODO: Integrate with VBR mutation rules
                // For now, just log
                info!("Cron job '{}' triggered data mutation: {} on {}", job_id, operation, entity);
            }
        }

        // Update job state
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(job_id) {
                job.last_execution = Some(now);
                job.execution_count += 1;

                // Calculate next execution
                job.next_execution = job.calculate_next_execution(now);
            }
        }

        info!("Executed cron job '{}'", job_id);
        Ok(())
    }

    /// Get the virtual clock
    pub fn clock(&self) -> Arc<VirtualClock> {
        self.clock.clone()
    }
}

// Helper function to parse cron schedule string
fn parse_cron_schedule(schedule: &str) -> Result<CronSchedule, String> {
    CronSchedule::from_str(schedule).map_err(|e| format!("Invalid cron expression: {}", e))
}

// Re-export Schedule for convenience
pub use cron::Schedule;

// Import Schedule::from_str
use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_job_creation() {
        let job =
            CronJob::new("test-1".to_string(), "Test Job".to_string(), "0 3 * * *".to_string());

        assert_eq!(job.id, "test-1");
        assert_eq!(job.name, "Test Job");
        assert_eq!(job.schedule, "0 3 * * *");
        assert!(job.enabled);
    }

    #[test]
    fn test_cron_schedule_parsing() {
        let schedule = CronSchedule::from_str("0 3 * * *").unwrap();
        let now = Utc::now();
        let next = schedule.after(&now).next();
        assert!(next.is_some());
    }

    #[tokio::test]
    async fn test_cron_scheduler_add_job() {
        let clock = Arc::new(VirtualClock::new());
        clock.enable_and_set(Utc::now());
        let scheduler = CronScheduler::new(clock);

        let job =
            CronJob::new("test-1".to_string(), "Test Job".to_string(), "0 3 * * *".to_string());

        let action = CronJobAction::Callback(Box::new(|_| {
            println!("Test callback");
            Ok(())
        }));

        scheduler.add_job(job, action).await.unwrap();

        let jobs = scheduler.list_jobs().await;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "test-1");
    }
}
