//! Replay executor (#6 / Phase 3). Handles `replay`.
//!
//! Synthetic-pass mode: emits "request_replayed" events and reports
//! `passed`. Real impl will load the source capture_session, replay
//! each exchange against the target URL, compare actual vs. recorded
//! responses, write a summary.

use async_trait::async_trait;
use std::time::Instant;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};

/// Executor for capture-session replay.
pub struct ReplayExecutor;

#[async_trait]
impl Executor for ReplayExecutor {
    fn kind(&self) -> &'static str {
        "replay"
    }

    async fn execute(&self, job: RunJob, callbacks: &RegistryCallbacks) -> Result<JobOutcome> {
        let started = Instant::now();
        callbacks.run_started(job.run_id).await?;

        let captures = job
            .payload
            .get("synthetic_captures")
            .and_then(|v| v.as_u64())
            .unwrap_or(5)
            .clamp(1, 200) as u32;

        callbacks
            .run_event(
                job.run_id,
                1,
                "log",
                serde_json::json!({
                    "level": "info",
                    "message": format!("Synthetic replay: {} captures", captures),
                    "synthetic": true,
                    "tracking_task": 6,
                }),
            )
            .await?;

        let mut next_seq: u32 = 2;
        let mut matched = 0u32;
        for i in 1..=captures {
            // Synthetic replay: every capture matches.
            matched += 1;
            callbacks
                .run_event(
                    job.run_id,
                    next_seq,
                    "request_replayed",
                    serde_json::json!({
                        "index": i,
                        "matched": true,
                    }),
                )
                .await?;
            next_seq += 1;
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }

        let elapsed = started.elapsed();
        let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);

        Ok(JobOutcome {
            status: JobStatus::Passed,
            runner_seconds: secs,
            summary: Some(serde_json::json!({
                "executor_phase": "synthetic",
                "tracking_task": 6,
                "captures_replayed": captures,
                "matched": matched,
                "wall_ms": elapsed.as_millis() as u64,
            })),
        })
    }
}
