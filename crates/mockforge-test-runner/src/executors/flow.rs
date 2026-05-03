//! Flow executor (#9 / Phase 2). Handles `scenario | orchestration |
//! state_machine | chain`.
//!
//! Real impl will: load the flow's current_version_id config and
//! dispatch to the kind-specific runtime in `mockforge-scenarios` /
//! `mockforge-pipelines`. Phase 1 stub returns errored.

use async_trait::async_trait;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};

/// Executor for the four flow kinds.
pub struct FlowExecutor {
    kind: &'static str,
}

impl FlowExecutor {
    /// Construct for one of scenario/orchestration/state_machine/chain.
    pub fn for_kind(kind: &'static str) -> Self {
        Self { kind }
    }
}

#[async_trait]
impl Executor for FlowExecutor {
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
                        "Flow executor for kind '{}' is scaffolded but not yet implemented",
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
                "tracking_task": 9,
            })),
        })
    }
}
