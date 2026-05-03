//! Test execution suite executor (#4 / Phases 3-4).
//!
//! Handles `unit | integration | conformance | bench | owasp` kinds. The
//! existing `mockforge-bench` and `mockforge-test` crates own the actual
//! per-kind logic; this executor is the cloud worker that picks up the
//! queued run and dispatches to those.
//!
//! Phase 1 stub returns `errored` so the test_runs row transitions out
//! of `queued`. Real per-kind dispatch lands when the worker pool is
//! actually deployed.

use async_trait::async_trait;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};

/// Executor for all five test_suites.kind values.
pub struct TestExecutor {
    kind: &'static str,
}

impl TestExecutor {
    /// Construct the executor for one of unit/integration/conformance/
    /// bench/owasp.
    pub fn for_kind(kind: &'static str) -> Self {
        Self { kind }
    }
}

#[async_trait]
impl Executor for TestExecutor {
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
                        "Test execution kind '{}' is scaffolded but not yet implemented in the cloud worker",
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
                "tracking_task": 4,
            })),
        })
    }
}
