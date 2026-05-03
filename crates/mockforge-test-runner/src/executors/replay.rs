//! Replay executor (#6 / Phase 3). Handles `replay`.
//!
//! Real impl will: load the source capture_session, replay each
//! exchange against the target URL, compare actual vs. recorded
//! responses, write a summary. Phase 1 stub returns errored.

use async_trait::async_trait;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};

/// Executor for capture-session replay.
pub struct ReplayExecutor;

#[async_trait]
impl Executor for ReplayExecutor {
    fn kind(&self) -> &'static str {
        "replay"
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
                    "message": "Replay executor is scaffolded but not yet implemented",
                }),
            )
            .await?;
        Ok(JobOutcome {
            status: JobStatus::Errored,
            runner_seconds: 0,
            summary: Some(serde_json::json!({
                "executor_phase": "stub",
                "tracking_task": 6,
            })),
        })
    }
}
