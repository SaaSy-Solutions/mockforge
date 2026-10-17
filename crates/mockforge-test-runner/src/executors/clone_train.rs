//! Behavioral-cloning training executor (#6 / Phase 2). Handles
//! `behavioral_clone`.
//!
//! Not implemented. Training needs the source capture session's
//! exchanges, a model trained via `mockforge-behavioral-cloning`, an
//! artifact upload to blob storage, and a callback that writes
//! `clone_models.artifact_url` + metrics. None of that exists in the
//! runner yet, so every run finishes `errored` with
//! `status_reason = "not_implemented"` and the registry marks the clone
//! model failed (#1264). It previously emitted fake `training_epoch`
//! events and reported `passed`.

use async_trait::async_trait;
use std::time::Instant;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{unsupported, Executor, JobOutcome, RunJob};

/// Executor for behavioral-cloning training.
pub struct CloneTrainExecutor;

#[async_trait]
impl Executor for CloneTrainExecutor {
    fn kind(&self) -> &'static str {
        "behavioral_clone"
    }

    async fn execute(&self, job: RunJob, callbacks: &RegistryCallbacks) -> Result<JobOutcome> {
        let started = Instant::now();
        callbacks.run_started(job.run_id).await?;
        unsupported(&job, callbacks, started, 1, "no model was trained or uploaded").await
    }
}
