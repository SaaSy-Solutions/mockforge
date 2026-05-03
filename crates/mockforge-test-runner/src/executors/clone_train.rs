//! Behavioral-cloning training executor (#6 / Phase 2). Handles
//! `behavioral_clone`.
//!
//! Real impl will: download the source capture_session's exchanges,
//! train a model via `mockforge-behavioral-cloning`, upload to blob
//! storage, write `clone_models.artifact_url` + metrics through a
//! callback. Phase 1 stub returns errored.

use async_trait::async_trait;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};

/// Executor for behavioral-cloning training.
pub struct CloneTrainExecutor;

#[async_trait]
impl Executor for CloneTrainExecutor {
    fn kind(&self) -> &'static str {
        "behavioral_clone"
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
                    "message": "Behavioral-cloning training executor is scaffolded but not yet implemented",
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
