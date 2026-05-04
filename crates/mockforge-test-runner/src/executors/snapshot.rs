//! Snapshot capture/restore executor (#10 / Phase 2). Handles
//! `snapshot_capture` and `snapshot_restore`.
//!
//! Synthetic-pass mode: emits "component_dumped" events (capture) or
//! "component_restored" events (restore) and returns `passed`. Real
//! impl will dump/load workspace state + write to blob storage and
//! call Snapshot::mark_ready via a callback.

use async_trait::async_trait;
use std::time::Instant;

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

    fn synthetic_components(_payload: &serde_json::Value) -> &'static [&'static str] {
        &["mocks", "scenarios", "fixtures", "world_state"]
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

        let event_type = if self.kind == "snapshot_restore" {
            "component_restored"
        } else {
            "component_dumped"
        };
        let components = Self::synthetic_components(&job.payload);

        callbacks
            .run_event(
                job.run_id,
                1,
                "log",
                serde_json::json!({
                    "level": "info",
                    "message": format!(
                        "Synthetic {} on {} components",
                        self.kind,
                        components.len()
                    ),
                    "synthetic": true,
                    "tracking_task": 10,
                }),
            )
            .await?;

        let mut next_seq: u32 = 2;
        for (i, comp) in components.iter().enumerate() {
            callbacks
                .run_event(
                    job.run_id,
                    next_seq,
                    event_type,
                    serde_json::json!({
                        "component": comp,
                        "size_bytes": 1024 * (i as u64 + 1),
                    }),
                )
                .await?;
            next_seq += 1;
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        let elapsed = started.elapsed();
        let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);

        Ok(JobOutcome {
            status: JobStatus::Passed,
            runner_seconds: secs,
            summary: Some(serde_json::json!({
                "executor_phase": "synthetic",
                "tracking_task": 10,
                "kind": self.kind,
                "components": components,
                "wall_ms": elapsed.as_millis() as u64,
            })),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn components_list_is_stable() {
        let comps = SnapshotExecutor::synthetic_components(&json!({}));
        assert_eq!(comps.len(), 4);
        assert!(comps.contains(&"mocks"));
    }
}
