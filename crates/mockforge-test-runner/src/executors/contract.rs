//! Contract diff / verification / fitness executor (#8 / Phase 2).
//! Handles `contract_diff | verification_suite | fitness_evaluation`.
//!
//! Real impl will: probe-path fetches the live spec from the
//! monitored_service's openapi_spec_url + sample traffic, runs the
//! `mockforge-core::ai_contract_diff` pipeline, writes findings + raises
//! drift incidents via the IncidentBus from #3. Phase 1 stub returns
//! errored.

use async_trait::async_trait;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};

/// Executor for the three contract-/fitness-related kinds.
pub struct ContractExecutor {
    kind: &'static str,
}

impl ContractExecutor {
    /// Construct for `contract_diff`, `verification_suite`, or
    /// `fitness_evaluation`.
    pub fn for_kind(kind: &'static str) -> Self {
        Self { kind }
    }
}

#[async_trait]
impl Executor for ContractExecutor {
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
                        "Contract executor for kind '{}' is scaffolded but not yet implemented",
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
                "tracking_task": 8,
            })),
        })
    }
}
