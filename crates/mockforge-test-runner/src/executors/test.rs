//! Test execution suite executor (#4 / Phases 3-4).
//!
//! Handles `unit | integration | conformance | bench | owasp` kinds. The
//! existing `mockforge-bench` and `mockforge-test` crates own the actual
//! per-kind logic; this executor is the cloud worker that picks up the
//! queued run and dispatches to those.
//!
//! ## Phase 2: synthetic mode
//!
//! Real per-kind dispatch (k6 / mockforge-bench / OWASP suite, etc.)
//! lands incrementally. For now this executor runs in a *synthetic*
//! mode: it streams a configurable number of fake "step_pass" events
//! and returns `passed` with non-zero `runner_seconds`. That validates
//! the full success-path lifecycle (queued → running → passed → meter
//! incremented) end-to-end, which the previous stub did not — it
//! reported `errored` and skipped the metering path.
//!
//! Synthetic mode is driven by the job's `payload`:
//! - `payload.synthetic_steps` (u32, optional, default 3) — number of
//!   step events to emit.
//! - `payload.synthetic_step_ms` (u64, optional, default 100) — sleep
//!   between steps. Mostly to make `runner_seconds` reflect realistic
//!   walltime in tests.
//!
//! When a real test_suite config arrives (config.kind set, config.target
//! pointing at a hosted-mock URL, etc.), the executor falls back to
//! synthetic mode and logs that the real path isn't wired yet.

use async_trait::async_trait;
use std::time::Instant;

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

    /// How many synthetic steps does this job want? Capped to keep a
    /// runaway test_suites.config from spamming the event log.
    fn synthetic_step_count(payload: &serde_json::Value) -> u32 {
        let raw = payload.get("synthetic_steps").and_then(|v| v.as_u64()).unwrap_or(3);
        raw.clamp(1, 100) as u32
    }

    /// Inter-step delay. Capped so a payload can't pin a worker on a
    /// synthetic job for hours.
    fn synthetic_step_ms(payload: &serde_json::Value) -> u64 {
        let raw = payload.get("synthetic_step_ms").and_then(|v| v.as_u64()).unwrap_or(100);
        raw.min(2000)
    }
}

#[async_trait]
impl Executor for TestExecutor {
    fn kind(&self) -> &'static str {
        self.kind
    }

    async fn execute(&self, job: RunJob, callbacks: &RegistryCallbacks) -> Result<JobOutcome> {
        let started = Instant::now();
        callbacks.run_started(job.run_id).await?;

        let steps = Self::synthetic_step_count(&job.payload);
        let step_ms = Self::synthetic_step_ms(&job.payload);

        callbacks
            .run_event(
                job.run_id,
                1,
                "log",
                serde_json::json!({
                    "level": "info",
                    "message": format!(
                        "Synthetic test execution: kind='{}', steps={}, step_ms={}",
                        self.kind, steps, step_ms,
                    ),
                    "synthetic": true,
                    "tracking_task": 4,
                }),
            )
            .await?;

        // Stream step_start + step_pass for each synthetic step. Seq
        // numbers continue from the log event above so the run_events
        // table's UNIQUE(run_id, seq) is satisfied.
        let mut next_seq: u32 = 2;
        for i in 1..=steps {
            callbacks
                .run_event(
                    job.run_id,
                    next_seq,
                    "step_start",
                    serde_json::json!({ "step": i, "name": format!("synthetic-step-{i}") }),
                )
                .await?;
            next_seq += 1;
            tokio::time::sleep(std::time::Duration::from_millis(step_ms)).await;
            callbacks
                .run_event(
                    job.run_id,
                    next_seq,
                    "step_pass",
                    serde_json::json!({ "step": i, "duration_ms": step_ms }),
                )
                .await?;
            next_seq += 1;
        }

        // Real wall-clock for the runner_seconds meter; round up so a
        // short synthetic run still bills 1s and exercises the
        // increment_runner_seconds code path.
        let elapsed = started.elapsed();
        let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);

        Ok(JobOutcome {
            status: JobStatus::Passed,
            runner_seconds: secs,
            summary: Some(serde_json::json!({
                "executor_phase": "synthetic",
                "tracking_task": 4,
                "kind": self.kind,
                "steps_passed": steps,
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
    fn synthetic_step_count_default() {
        assert_eq!(TestExecutor::synthetic_step_count(&json!({})), 3);
        assert_eq!(TestExecutor::synthetic_step_count(&json!(null)), 3);
    }

    #[test]
    fn synthetic_step_count_clamps_to_100() {
        assert_eq!(TestExecutor::synthetic_step_count(&json!({ "synthetic_steps": 999 })), 100);
    }

    #[test]
    fn synthetic_step_count_clamps_to_minimum_1() {
        assert_eq!(TestExecutor::synthetic_step_count(&json!({ "synthetic_steps": 0 })), 1);
    }

    #[test]
    fn synthetic_step_count_honors_intermediate() {
        assert_eq!(TestExecutor::synthetic_step_count(&json!({ "synthetic_steps": 7 })), 7);
    }

    #[test]
    fn synthetic_step_ms_caps_at_2000() {
        assert_eq!(TestExecutor::synthetic_step_ms(&json!({ "synthetic_step_ms": 60_000 })), 2000);
    }

    #[test]
    fn synthetic_step_ms_default() {
        assert_eq!(TestExecutor::synthetic_step_ms(&json!({})), 100);
    }
}
