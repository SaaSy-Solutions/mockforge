//! Chaos campaign executor (#7 / Phase 2). Handles `chaos_campaign`.
//!
//! Real impl will: load campaign config + safety_config from the
//! registry, inject faults via `mockforge-chaos`, monitor target
//! health, abort if kill-switch trips. Phase 1 stub returns errored.

use async_trait::async_trait;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};

/// Executor for chaos campaigns.
pub struct ChaosExecutor;

#[async_trait]
impl Executor for ChaosExecutor {
    fn kind(&self) -> &'static str {
        "chaos_campaign"
    }

    async fn execute(&self, job: RunJob, callbacks: &RegistryCallbacks) -> Result<JobOutcome> {
        callbacks.run_started(job.run_id).await?;
        callbacks
            .run_event(
                job.run_id,
                1,
                "log",
                serde_json::json!({
                    "level": "warn",
                    "message": "Chaos campaign executor is scaffolded but not yet implemented",
                }),
            )
            .await?;
        Ok(JobOutcome {
            status: JobStatus::Errored,
            runner_seconds: 0,
            summary: Some(serde_json::json!({
                "executor_phase": "stub",
                "tracking_task": 7,
            })),
        })
    }
}
