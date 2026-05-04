//! Chaos campaign executor (#7 / Phase 2). Handles `chaos_campaign`.
//!
//! Synthetic-pass mode (same shape as TestExecutor): emits a few
//! "fault_injected" events then reports `passed`. Real impl will load
//! campaign config + safety_config from the registry, inject faults
//! via `mockforge-chaos`, monitor target health, abort if kill-switch
//! trips.

use async_trait::async_trait;
use std::time::Instant;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};

/// Executor for chaos campaigns.
pub struct ChaosExecutor;

impl ChaosExecutor {
    /// How many synthetic fault events to emit. Capped to keep a
    /// runaway campaign config from spamming the event log.
    fn synthetic_fault_count(payload: &serde_json::Value) -> u32 {
        let raw = payload.get("synthetic_faults").and_then(|v| v.as_u64()).unwrap_or(2);
        raw.clamp(1, 50) as u32
    }

    fn synthetic_fault_ms(payload: &serde_json::Value) -> u64 {
        let raw = payload.get("synthetic_fault_ms").and_then(|v| v.as_u64()).unwrap_or(150);
        raw.min(5000)
    }
}

#[async_trait]
impl Executor for ChaosExecutor {
    fn kind(&self) -> &'static str {
        "chaos_campaign"
    }

    async fn execute(&self, job: RunJob, callbacks: &RegistryCallbacks) -> Result<JobOutcome> {
        let started = Instant::now();
        callbacks.run_started(job.run_id).await?;

        let faults = Self::synthetic_fault_count(&job.payload);
        let fault_ms = Self::synthetic_fault_ms(&job.payload);
        let target_kind =
            job.payload.get("target_kind").and_then(|v| v.as_str()).unwrap_or("hosted_mock");
        let target_ref = job
            .payload
            .get("target_ref")
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
                        "Synthetic chaos campaign: target_kind='{}', target_ref='{}', faults={}, fault_ms={}",
                        target_kind, target_ref, faults, fault_ms,
                    ),
                    "synthetic": true,
                    "tracking_task": 7,
                }),
            )
            .await?;

        let mut next_seq: u32 = 2;
        for i in 1..=faults {
            callbacks
                .run_event(
                    job.run_id,
                    next_seq,
                    "fault_injected",
                    serde_json::json!({
                        "fault_index": i,
                        "fault_kind": "synthetic-latency",
                        "duration_ms": fault_ms,
                    }),
                )
                .await?;
            next_seq += 1;
            tokio::time::sleep(std::time::Duration::from_millis(fault_ms)).await;
            callbacks
                .run_event(
                    job.run_id,
                    next_seq,
                    "fault_recovered",
                    serde_json::json!({ "fault_index": i }),
                )
                .await?;
            next_seq += 1;
        }

        let elapsed = started.elapsed();
        let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);

        Ok(JobOutcome {
            status: JobStatus::Passed,
            runner_seconds: secs,
            summary: Some(serde_json::json!({
                "executor_phase": "synthetic",
                "tracking_task": 7,
                "target_kind": target_kind,
                "target_ref": target_ref,
                "faults_injected": faults,
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
    fn fault_count_default() {
        assert_eq!(ChaosExecutor::synthetic_fault_count(&json!({})), 2);
    }

    #[test]
    fn fault_count_clamps() {
        assert_eq!(ChaosExecutor::synthetic_fault_count(&json!({ "synthetic_faults": 999 })), 50);
        assert_eq!(ChaosExecutor::synthetic_fault_count(&json!({ "synthetic_faults": 0 })), 1);
    }

    #[test]
    fn fault_ms_caps() {
        assert_eq!(
            ChaosExecutor::synthetic_fault_ms(&json!({ "synthetic_fault_ms": 60_000 })),
            5000
        );
    }
}
