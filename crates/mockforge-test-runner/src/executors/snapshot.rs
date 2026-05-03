//! Snapshot capture/restore executor (#10 / Phase 2). Handles
//! `snapshot_capture` and `snapshot_restore`.
//!
//! Real impl will: capture-path dumps the workspace (mocks, scenarios,
//! fixtures, world-state) to blob storage and calls
//! `Snapshot::mark_ready` via a callback; restore-path downloads the
//! blob and applies it to the target workspace. Phase 1 stub returns
//! errored.

use async_trait::async_trait;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};

/// Executor for snapshot capture + restore.
pub struct SnapshotExecutor {
    kind: &'static str,
}

impl SnapshotExecutor {
    /// Construct for one of `snapshot_capture` or `snapshot_restore`.
    pub fn for_kind(kind: &'static str) -> Self {
        Self { kind }
    }
}

#[async_trait]
impl Executor for SnapshotExecutor {
    fn kind(&self) -> &'static str {
        self.kind
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
                    "message": format!(
                        "Snapshot executor for kind '{}' is scaffolded but not yet implemented",
                        self.kind
                    ),
                }),
            )
            .await?;
        Ok(JobOutcome {
            status: JobStatus::Errored,
            runner_seconds: 0,
            summary: Some(serde_json::json!({
                "executor_phase": "stub",
                "tracking_task": 10,
            })),
        })
    }
}
