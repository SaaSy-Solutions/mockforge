//! Snapshot capture/restore executor (#10 / Phase 2). Handles
//! `snapshot_capture` and `snapshot_restore`.
//!
//! Not implemented. A real implementation must dump/load workspace state,
//! write it to blob storage, and call `Snapshot::mark_ready` via a
//! callback. Until then every run finishes `errored` with
//! `status_reason = "not_implemented"`, so the registry marks the
//! snapshot failed instead of telling the user their state was captured
//! or restored (#1264).

use async_trait::async_trait;
use std::time::Instant;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{unsupported, Executor, JobOutcome, RunJob};

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
        let started = Instant::now();
        callbacks.run_started(job.run_id).await?;
        let detail = if self.kind == "snapshot_restore" {
            "no workspace state was restored"
        } else {
            "no workspace state was captured"
        };
        unsupported(&job, callbacks, started, 1, detail).await
    }
}
