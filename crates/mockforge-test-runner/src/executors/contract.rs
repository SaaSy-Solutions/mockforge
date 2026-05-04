//! Contract diff / verification / fitness executor (#8 / Phase 2).
//! Handles `contract_diff | verification_suite | fitness_evaluation`.
//!
//! Synthetic-pass mode: emits a few "diff_finding" events with mixed
//! severities then reports `passed`. Real impl will run the
//! `mockforge-core::ai_contract_diff` pipeline against the live spec +
//! sample traffic, raise drift incidents through #3 IncidentBus, write
//! findings into contract_diff_findings.

use async_trait::async_trait;
use std::time::Instant;

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
        let started = Instant::now();
        callbacks.run_started(job.run_id).await?;

        let service_name = job
            .payload
            .get("service_name")
            .and_then(|v| v.as_str())
            .unwrap_or("(unspecified)");

        callbacks
            .run_event(
                job.run_id,
                1,
                "log",
                serde_json::json!({
                    "level": "info",
                    "message": format!(
                        "Synthetic {} for service '{}'",
                        self.kind, service_name
                    ),
                    "synthetic": true,
                    "tracking_task": 8,
                }),
            )
            .await?;

        // Synthetic findings: one per severity level so the UI's
        // severity-grouped view has data to render.
        let findings = [
            ("non_breaking", "GET /users", "Added optional query param"),
            ("cosmetic", "POST /users", "Description rewording"),
        ];
        let mut next_seq: u32 = 2;
        for (i, (sev, endpoint, desc)) in findings.iter().enumerate() {
            callbacks
                .run_event(
                    job.run_id,
                    next_seq,
                    "diff_finding",
                    serde_json::json!({
                        "index": i,
                        "severity": sev,
                        "endpoint": endpoint,
                        "description": desc,
                    }),
                )
                .await?;
            next_seq += 1;
            tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        }

        let elapsed = started.elapsed();
        let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);

        Ok(JobOutcome {
            status: JobStatus::Passed,
            runner_seconds: secs,
            summary: Some(serde_json::json!({
                "executor_phase": "synthetic",
                "tracking_task": 8,
                "kind": self.kind,
                "service_name": service_name,
                "findings_count": findings.len(),
                "wall_ms": elapsed.as_millis() as u64,
            })),
        })
    }
}
