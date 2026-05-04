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

    /// Pull the user's declared faults out of campaign.config so the
    /// executor can iterate the real list. Recognized shape:
    /// `config.faults = [{ kind, duration_ms?, name? }, ...]`.
    /// Returns (kind, duration_ms) tuples, capped at 50 entries.
    fn extract_real_faults(payload: &serde_json::Value) -> Vec<(String, u64)> {
        let arr = payload.get("config").and_then(|c| c.get("faults")).and_then(|v| v.as_array());
        let Some(arr) = arr else {
            return Vec::new();
        };
        arr.iter()
            .take(50)
            .map(|f| {
                let kind = f.get("kind").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                let dur = f.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(150).min(5000);
                (kind, dur)
            })
            .collect()
    }

    /// Read max_duration_ms from safety_config (caller sets this so a
    /// runaway fault list can't pin the worker forever).
    fn safety_max_duration_ms(payload: &serde_json::Value) -> Option<u64> {
        payload
            .get("safety_config")
            .and_then(|s| s.get("max_duration_ms"))
            .and_then(|v| v.as_u64())
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

        let real_faults = Self::extract_real_faults(&job.payload);
        let using_real_config = !real_faults.is_empty();
        let synth_fault_count = Self::synthetic_fault_count(&job.payload);
        let synth_fault_ms = Self::synthetic_fault_ms(&job.payload);
        let target_kind =
            job.payload.get("target_kind").and_then(|v| v.as_str()).unwrap_or("hosted_mock");
        let target_ref = job
            .payload
            .get("target_ref")
            .and_then(|v| v.as_str())
            .unwrap_or("(unspecified)");
        let safety_cap = Self::safety_max_duration_ms(&job.payload);

        // For target_kind=hosted_mock, target_ref is the deployment_id.
        // Try real chaos: enable on the deployment for the run, disable
        // at the end. If the toggle call fails (deployment doesn't
        // expose admin, network blip, etc.) we still emit synthetic
        // events so the run produces a coherent outcome.
        let real_chaos_target_id = if target_kind == "hosted_mock" {
            uuid::Uuid::parse_str(target_ref).ok()
        } else {
            None
        };
        let real_chaos_enabled = if let Some(deployment_id) = real_chaos_target_id {
            match callbacks.toggle_hosted_chaos(deployment_id, true).await {
                Ok(()) => {
                    callbacks
                        .run_event(
                            job.run_id,
                            1,
                            "log",
                            serde_json::json!({
                                "level": "info",
                                "message": format!(
                                    "Real chaos enabled on deployment {deployment_id}"
                                ),
                                "synthetic": false,
                                "tracking_task": 7,
                            }),
                        )
                        .await?;
                    true
                }
                Err(e) => {
                    tracing::warn!(error = %e, %deployment_id, "real chaos toggle failed; falling back to synthetic events");
                    callbacks
                        .run_event(
                            job.run_id,
                            1,
                            "log",
                            serde_json::json!({
                                "level": "warn",
                                "message": format!(
                                    "real chaos toggle failed: {e}; emitting synthetic events"
                                ),
                                "synthetic": true,
                                "tracking_task": 7,
                            }),
                        )
                        .await?;
                    false
                }
            }
        } else {
            false
        };

        let fault_count = if using_real_config {
            real_faults.len() as u32
        } else {
            synth_fault_count
        };

        callbacks
            .run_event(
                job.run_id,
                2,
                "log",
                serde_json::json!({
                    "level": "info",
                    "message": format!(
                        "{}: target_kind='{}', target_ref='{}', faults={}",
                        if using_real_config { "Chaos campaign (config-driven)" } else { "Synthetic chaos campaign" },
                        target_kind, target_ref, fault_count,
                    ),
                    "synthetic": !using_real_config,
                    "real_chaos_enabled": real_chaos_enabled,
                    "tracking_task": 7,
                }),
            )
            .await?;

        let mut next_seq: u32 = 3;
        let mut aborted = false;
        let mut abort_reason: Option<String> = None;
        for i in 1..=fault_count {
            // Safety: respect max_duration_ms from safety_config.
            if let Some(max_ms) = safety_cap {
                let elapsed_ms = started.elapsed().as_millis() as u64;
                if elapsed_ms >= max_ms {
                    aborted = true;
                    abort_reason = Some(format!(
                        "safety_config.max_duration_ms ({}ms) reached after {} faults",
                        max_ms,
                        i - 1
                    ));
                    break;
                }
            }
            let (fault_kind, fault_ms) = if using_real_config {
                real_faults
                    .get((i - 1) as usize)
                    .cloned()
                    .unwrap_or_else(|| ("unknown".into(), synth_fault_ms))
            } else {
                ("synthetic-latency".into(), synth_fault_ms)
            };
            callbacks
                .run_event(
                    job.run_id,
                    next_seq,
                    "fault_injected",
                    serde_json::json!({
                        "fault_index": i,
                        "fault_kind": fault_kind,
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
                    serde_json::json!({ "fault_index": i, "fault_kind": fault_kind }),
                )
                .await?;
            next_seq += 1;
        }

        // Disable real chaos at the end of the run if we enabled it.
        // Best-effort — failure to disable is logged but doesn't change
        // the run's pass/fail status (the safety_config.max_duration_ms
        // cap still bounds blast radius).
        if real_chaos_enabled {
            if let Some(deployment_id) = real_chaos_target_id {
                if let Err(e) = callbacks.toggle_hosted_chaos(deployment_id, false).await {
                    tracing::error!(
                        error = %e,
                        %deployment_id,
                        "failed to disable real chaos at end of run — investigate immediately",
                    );
                    callbacks
                        .run_event(
                            job.run_id,
                            next_seq,
                            "log",
                            serde_json::json!({
                                "level": "error",
                                "message": format!(
                                    "failed to disable real chaos at end of run: {e} — manual intervention required"
                                ),
                            }),
                        )
                        .await?;
                }
            }
        }

        let elapsed = started.elapsed();
        let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
        let status = if aborted {
            JobStatus::Failed
        } else {
            JobStatus::Passed
        };

        Ok(JobOutcome {
            status,
            runner_seconds: secs,
            summary: Some(serde_json::json!({
                "executor_phase": if real_chaos_enabled {
                    "real_hosted_mock"
                } else if using_real_config {
                    "config_driven"
                } else {
                    "synthetic"
                },
                "tracking_task": 7,
                "target_kind": target_kind,
                "target_ref": target_ref,
                "real_chaos_enabled": real_chaos_enabled,
                "fault_count": fault_count,
                "aborted": aborted,
                "abort_reason": abort_reason,
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
