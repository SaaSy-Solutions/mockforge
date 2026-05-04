//! Behavioral-cloning training executor (#6 / Phase 2). Handles
//! `behavioral_clone`.
//!
//! Synthetic-pass mode: emits "training_epoch" events and reports
//! `passed`. Real impl will load the source capture_session's
//! exchanges, train a model via `mockforge-behavioral-cloning`,
//! upload the artifact to blob storage, write
//! clone_models.artifact_url + metrics through a callback.

use async_trait::async_trait;
use std::time::Instant;

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
        let started = Instant::now();
        callbacks.run_started(job.run_id).await?;

        let epochs = job
            .payload
            .get("synthetic_epochs")
            .and_then(|v| v.as_u64())
            .unwrap_or(3)
            .clamp(1, 20) as u32;

        callbacks
            .run_event(
                job.run_id,
                1,
                "log",
                serde_json::json!({
                    "level": "info",
                    "message": format!("Synthetic clone training: {} epochs", epochs),
                    "synthetic": true,
                    "tracking_task": 6,
                }),
            )
            .await?;

        let mut next_seq: u32 = 2;
        for epoch in 1..=epochs {
            // Loss decreases monotonically across synthetic epochs so the
            // UI's training-curve view has plausible-looking data.
            let synthetic_loss = 1.0 / (epoch as f64);
            callbacks
                .run_event(
                    job.run_id,
                    next_seq,
                    "training_epoch",
                    serde_json::json!({
                        "epoch": epoch,
                        "loss": synthetic_loss,
                    }),
                )
                .await?;
            next_seq += 1;
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        }

        let elapsed = started.elapsed();
        let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);

        Ok(JobOutcome {
            status: JobStatus::Passed,
            runner_seconds: secs,
            summary: Some(serde_json::json!({
                "executor_phase": "synthetic",
                "tracking_task": 6,
                "epochs": epochs,
                "wall_ms": elapsed.as_millis() as u64,
            })),
        })
    }
}
