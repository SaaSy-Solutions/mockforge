//! Chaos campaign executor (#7 / Phase 2). Handles `chaos_campaign`.
//!
//! Two real paths:
//! - `target_kind=hosted_mock`: toggles in-process chaos middleware on
//!   the deployment via `/__mockforge/chaos/toggle`. The deployment
//!   owns the listener so the toggle is enough.
//! - `target_kind=external`: routes probe requests through
//!   `mockforge-chaos-proxy`'s `ChaosClient` so latency / error /
//!   drop dice are applied to real outbound HTTP — see #349.
//!
//! Both paths share the same fault-loop event shape so the UI's
//! chaos timeline doesn't have to know which kind produced an event.

use async_trait::async_trait;
use mockforge_chaos_proxy::{CampaignCounters, ChaosClient, ChaosDirective};
use std::time::Instant;

use crate::callbacks::RegistryCallbacks;
use crate::error::{Error, Result};
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};

/// Executor for chaos campaigns.
pub struct ChaosExecutor;

impl ChaosExecutor {
    /// Pull the user's declared faults out of campaign.config so the
    /// executor can iterate the real list. Recognized shape:
    /// `config.faults = [{ kind, duration_ms?, name? }, ...]`.
    /// Returns (kind, duration_ms) tuples, capped at 50 entries.
    fn extract_faults(payload: &serde_json::Value) -> Vec<(String, u64)> {
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

    /// Foot-gun guard for `target_kind=external`: the suite's
    /// `config.external_target_authorized: true` flag must be set
    /// before we'll point chaos at an arbitrary URL. Full
    /// DNS-TXT / `.well-known/mockforge-chaos-authorized` proof is
    /// out of scope per #349.
    ///
    /// This is NOT a security control. The flag lives in tenant-authored
    /// suite config, so any tenant can set it; it only stops a user from
    /// pointing chaos at a URL by accident. Isolation from internal
    /// addresses comes from `ChaosClient`'s SSRF-guarded client, and
    /// ownership of the external target is not verified at all.
    fn external_target_authorized(payload: &serde_json::Value) -> bool {
        payload
            .get("config")
            .and_then(|c| c.get("external_target_authorized"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// How many probe requests should the external path send per
    /// fault iteration? Defaults to 1 (one probe per declared
    /// fault), capped at 50. Higher values are useful when the
    /// directive has probabilistic fault rates (so the executor
    /// can sample enough probes to actually observe the chaos).
    fn external_probes_per_fault(payload: &serde_json::Value) -> u32 {
        payload
            .get("config")
            .and_then(|c| c.get("external_probes_per_fault"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .clamp(1, 50) as u32
    }

    /// HTTP method used for external probes. Defaults to GET — chaos
    /// campaigns are about exercising fault behaviour, not state
    /// mutation, so we don't want to default to POST/PUT against
    /// arbitrary URLs.
    fn external_probe_method(payload: &serde_json::Value) -> String {
        payload
            .get("config")
            .and_then(|c| c.get("external_probe_method"))
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase()
    }
}

#[async_trait]
impl Executor for ChaosExecutor {
    fn kind(&self) -> &'static str {
        "chaos_campaign"
    }

    async fn execute(&self, job: RunJob, callbacks: &RegistryCallbacks) -> Result<JobOutcome> {
        let faults = Self::extract_faults(&job.payload);
        if faults.is_empty() {
            return Err(Error::Executor(
                "chaos_campaign requires at least one entry in `config.faults`".into(),
            ));
        }
        let target_kind = job
            .payload
            .get("target_kind")
            .and_then(|v| v.as_str())
            .unwrap_or("hosted_mock")
            .to_string();
        let target_ref = job
            .payload
            .get("target_ref")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or_default()
            .to_string();
        let safety_cap = Self::safety_max_duration_ms(&job.payload);
        let hosted_deployment_id = match target_kind.as_str() {
            "hosted_mock" => Some(uuid::Uuid::parse_str(&target_ref).map_err(|_| {
                Error::Executor(
                    "chaos_campaign with `target_kind: hosted_mock` requires a valid deployment UUID in `target_ref`"
                        .into(),
                )
            })?),
            "external" => None,
            other => {
                return Err(Error::Executor(format!(
                    "chaos_campaign has unsupported `target_kind` `{other}`; use `hosted_mock` or `external`"
                )))
            }
        };

        let started = Instant::now();
        callbacks.run_started(job.run_id).await?;

        if target_kind == "external" {
            return run_external_chaos(job, callbacks, started, &target_ref, &faults, safety_cap)
                .await;
        }

        let deployment_id = hosted_deployment_id.ok_or_else(|| {
            Error::Executor("chaos_campaign hosted target validation failed".into())
        })?;
        callbacks
            .toggle_hosted_chaos(job.run_id, deployment_id, true)
            .await
            .map_err(|error| {
                Error::Executor(format!(
                    "chaos_campaign failed to enable chaos on deployment {deployment_id}: {error}"
                ))
            })?;

        callbacks
            .run_event(
                job.run_id,
                1,
                "log",
                serde_json::json!({
                    "level": "info",
                    "message": format!("Chaos enabled on deployment {deployment_id}; faults={}", faults.len()),
                    "tracking_task": 7,
                }),
            )
            .await?;

        let mut next_seq: u32 = 2;
        let mut aborted = false;
        let mut abort_reason: Option<String> = None;
        let mut fault_count = 0u32;
        for (index, (fault_kind, fault_ms)) in faults.iter().enumerate() {
            if let Some(max_ms) = safety_cap {
                let elapsed_ms = started.elapsed().as_millis() as u64;
                if elapsed_ms >= max_ms {
                    aborted = true;
                    abort_reason = Some(format!(
                        "safety_config.max_duration_ms ({}ms) reached after {} faults",
                        max_ms, fault_count
                    ));
                    break;
                }
            }
            let fault_index = index + 1;
            callbacks
                .run_event(
                    job.run_id,
                    next_seq,
                    "fault_injected",
                    serde_json::json!({
                        "fault_index": fault_index,
                        "fault_kind": fault_kind,
                        "duration_ms": fault_ms,
                    }),
                )
                .await?;
            next_seq += 1;
            tokio::time::sleep(std::time::Duration::from_millis(*fault_ms)).await;
            callbacks
                .run_event(
                    job.run_id,
                    next_seq,
                    "fault_recovered",
                    serde_json::json!({ "fault_index": fault_index, "fault_kind": fault_kind }),
                )
                .await?;
            next_seq += 1;
            fault_count += 1;
        }

        callbacks
            .toggle_hosted_chaos(job.run_id, deployment_id, false)
            .await
            .map_err(|error| {
                Error::Executor(format!(
                    "chaos_campaign failed to disable chaos on deployment {deployment_id}: {error}"
                ))
            })?;

        let elapsed = started.elapsed();
        let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
        let status = if aborted || fault_count == 0 {
            JobStatus::Failed
        } else {
            JobStatus::Passed
        };

        Ok(JobOutcome {
            status,
            runner_seconds: secs,
            summary: Some(serde_json::json!({
                "executor_phase": "real_hosted_mock",
                "tracking_task": 7,
                "target_kind": target_kind,
                "target_ref": target_ref,
                "fault_count": fault_count,
                "aborted": aborted,
                "abort_reason": abort_reason,
                "wall_ms": elapsed.as_millis() as u64,
            })),
        })
    }
}

/// Drive an external-target chaos campaign through
/// `mockforge-chaos-proxy::ChaosClient`. One iteration per declared
/// fault, each sending `external_probes_per_fault` real HTTP probes
/// through the chaos client. Probe outcomes are aggregated into a
/// `chaos_summary` metric event at the end.
async fn run_external_chaos(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    target_ref: &str,
    faults: &[(String, u64)],
    safety_cap: Option<u64>,
) -> Result<JobOutcome> {
    // 1. Authorization gate. Refuse to run if the user hasn't
    //    explicitly set external_target_authorized=true.
    if !ChaosExecutor::external_target_authorized(&job.payload) {
        callbacks
            .run_event(
                job.run_id,
                1,
                "log",
                serde_json::json!({
                    "level": "error",
                    "message": "external chaos refused: config.external_target_authorized must be true",
                    "target_kind": "external",
                    "target_ref": target_ref,
                }),
            )
            .await?;
        return Ok(JobOutcome {
            status: JobStatus::Errored,
            runner_seconds: (started.elapsed().as_secs_f64().ceil() as i32).max(1),
            summary: Some(serde_json::json!({
                "executor_phase": "external_chaos_unauthorized",
                "target_kind": "external",
                "target_ref": target_ref,
                "tracking_task": 7,
            })),
        });
    }

    if target_ref.is_empty() || target_ref == "(unspecified)" {
        callbacks
            .run_event(
                job.run_id,
                1,
                "log",
                serde_json::json!({
                    "level": "error",
                    "message": "external chaos refused: target_ref must be a URL",
                    "target_kind": "external",
                }),
            )
            .await?;
        return Ok(JobOutcome {
            status: JobStatus::Errored,
            runner_seconds: (started.elapsed().as_secs_f64().ceil() as i32).max(1),
            summary: Some(serde_json::json!({
                "executor_phase": "external_chaos_missing_target",
                "target_kind": "external",
                "tracking_task": 7,
            })),
        });
    }

    let probes_per_fault = ChaosExecutor::external_probes_per_fault(&job.payload);
    let probe_method = ChaosExecutor::external_probe_method(&job.payload);
    let fault_count = faults.len() as u32;

    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!(
                    "External chaos campaign: target='{target_ref}', faults={fault_count}, probes/fault={probes_per_fault}"
                ),
                "target_kind": "external",
                "target_ref": target_ref,
                "tracking_task": 7,
            }),
        )
        .await?;

    let mut next_seq: u32 = 2;
    let mut counters = CampaignCounters::default();
    let mut aborted = false;
    let mut abort_reason: Option<String> = None;

    for i in 1..=fault_count {
        // Safety cap.
        if let Some(max_ms) = safety_cap {
            let elapsed_ms = started.elapsed().as_millis() as u64;
            if elapsed_ms >= max_ms {
                aborted = true;
                abort_reason = Some(format!(
                    "safety_config.max_duration_ms ({max_ms}ms) reached after {} faults",
                    i - 1
                ));
                break;
            }
        }

        // Map the declared fault entry onto a chaos directive. The
        // current campaign config schema is intentionally simple:
        // `kind` is one of "latency" | "error" | "drop", and
        // `duration_ms` is the latency (when kind=latency) or the
        // intended fault duration window. Probabilistic fault rates
        // would land in a follow-up — for v1 each fault iteration
        // applies its single directive at probability 1.0.
        let (fault_kind, fault_ms) = faults[(i - 1) as usize].clone();

        let directive = build_directive(&fault_kind, fault_ms);

        callbacks
            .run_event(
                job.run_id,
                next_seq,
                "fault_injected",
                serde_json::json!({
                    "fault_index": i,
                    "fault_kind": fault_kind,
                    "duration_ms": fault_ms,
                    "target_kind": "external",
                }),
            )
            .await?;
        next_seq += 1;

        // Build a fresh client per iteration so the directive's
        // egress timeout reflects the current fault's intent (e.g.
        // a long-latency fault gets a longer timeout).
        let client = match ChaosClient::new(directive) {
            Ok(c) => c,
            Err(e) => {
                callbacks
                    .run_event(
                        job.run_id,
                        next_seq,
                        "log",
                        serde_json::json!({
                            "level": "error",
                            "message": format!("chaos client build failed: {e}"),
                            "fault_index": i,
                        }),
                    )
                    .await?;
                aborted = true;
                abort_reason = Some(format!("chaos client build failed: {e}"));
                break;
            }
        };

        // Send the configured number of probes through the client.
        // We aggregate outcomes into the campaign counters; per-probe
        // outcome data lives in the summary, not the SSE stream, so
        // a 1000-probe campaign doesn't drown the UI in events.
        let mut iteration_failures = 0u32;
        for _ in 0..probes_per_fault {
            match client.probe(&probe_method, target_ref, None).await {
                Ok(outcome) => {
                    counters.record(&outcome);
                    if !outcome.succeeded {
                        iteration_failures += 1;
                    }
                }
                Err(e) => {
                    // Setup error (SSRF reject, malformed URL) — abort
                    // the whole campaign rather than continue probing.
                    callbacks
                        .run_event(
                            job.run_id,
                            next_seq,
                            "log",
                            serde_json::json!({
                                "level": "error",
                                "message": format!("chaos probe setup failed: {e}"),
                                "fault_index": i,
                            }),
                        )
                        .await?;
                    aborted = true;
                    abort_reason = Some(format!("probe setup failed: {e}"));
                    break;
                }
            }
        }
        if aborted {
            break;
        }

        callbacks
            .run_event(
                job.run_id,
                next_seq,
                "fault_recovered",
                serde_json::json!({
                    "fault_index": i,
                    "fault_kind": fault_kind,
                    "probes_sent": probes_per_fault,
                    "probe_failures": iteration_failures,
                }),
            )
            .await?;
        next_seq += 1;
    }

    // Final summary event so the UI can render aggregate counters.
    callbacks
        .run_event(
            job.run_id,
            next_seq,
            "metric",
            serde_json::json!({
                "name": "chaos_summary",
                "target_kind": "external",
                "target_ref": target_ref,
                "counters": &counters,
            }),
        )
        .await?;

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
            "executor_phase": "real_external",
            "target_kind": "external",
            "target_ref": target_ref,
            "fault_count": fault_count,
            "probes_per_fault": probes_per_fault,
            "counters": counters,
            "aborted": aborted,
            "abort_reason": abort_reason,
            "wall_ms": elapsed.as_millis() as u64,
            "tracking_task": 7,
        })),
    })
}

/// Translate one entry from the campaign's `config.faults` array into
/// a chaos directive. The campaign schema is intentionally small:
/// `kind="latency"` injects fixed latency at probability 1.0,
/// `kind="error"` synthesises HTTP 5xx, `kind="drop"` drops the
/// request. Unknown kinds default to latency so a misconfigured
/// campaign still produces observable chaos rather than failing
/// silently.
fn build_directive(kind: &str, duration_ms: u64) -> ChaosDirective {
    match kind.to_ascii_lowercase().as_str() {
        "error" | "http_error" | "5xx" => ChaosDirective::default().with_error_rate(1.0),
        "drop" | "connection_drop" => ChaosDirective::default().with_drop_rate(1.0),
        // "latency" plus the unknown-kind fallback.
        _ => ChaosDirective::default().with_latency_ms(duration_ms),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn job(payload: serde_json::Value) -> RunJob {
        RunJob {
            run_id: uuid::Uuid::new_v4(),
            org_id: uuid::Uuid::new_v4(),
            source_id: uuid::Uuid::new_v4(),
            kind: "chaos_campaign".to_string(),
            payload,
        }
    }

    #[tokio::test]
    async fn campaign_without_faults_is_an_executor_error() {
        let callbacks = RegistryCallbacks::for_test();
        let result = ChaosExecutor.execute(job(json!({})), &callbacks).await;
        assert!(
            matches!(result, Err(Error::Executor(ref message)) if message.contains("config.faults")),
            "campaign without real faults must error, got {result:?}"
        );
    }

    #[tokio::test]
    async fn hosted_toggle_failure_is_an_executor_error() {
        let callbacks = RegistryCallbacks::for_test_failing_chaos_toggle();
        let deployment_id = uuid::Uuid::new_v4();
        let result = ChaosExecutor
            .execute(
                job(json!({
                    "target_kind": "hosted_mock",
                    "target_ref": deployment_id,
                    "config": {"faults": [{"kind": "latency", "duration_ms": 0}]},
                })),
                &callbacks,
            )
            .await;
        assert!(
            matches!(result, Err(Error::Executor(ref message)) if message.contains("failed to enable chaos")),
            "toggle failure must error, got {result:?}"
        );
    }

    #[test]
    fn external_authorization_defaults_to_false() {
        assert!(!ChaosExecutor::external_target_authorized(&json!({})));
        assert!(!ChaosExecutor::external_target_authorized(&json!({ "config": {} })));
        assert!(!ChaosExecutor::external_target_authorized(&json!({
            "config": { "external_target_authorized": false }
        })));
    }

    #[test]
    fn external_authorization_only_true_with_explicit_flag() {
        assert!(ChaosExecutor::external_target_authorized(&json!({
            "config": { "external_target_authorized": true }
        })));
    }

    #[test]
    fn probes_per_fault_clamps() {
        assert_eq!(ChaosExecutor::external_probes_per_fault(&json!({})), 1);
        assert_eq!(
            ChaosExecutor::external_probes_per_fault(&json!({
                "config": { "external_probes_per_fault": 999 }
            })),
            50
        );
        assert_eq!(
            ChaosExecutor::external_probes_per_fault(&json!({
                "config": { "external_probes_per_fault": 0 }
            })),
            1
        );
    }

    #[test]
    fn probe_method_uppercases() {
        assert_eq!(ChaosExecutor::external_probe_method(&json!({})), "GET");
        assert_eq!(
            ChaosExecutor::external_probe_method(&json!({
                "config": { "external_probe_method": "post" }
            })),
            "POST"
        );
    }

    #[test]
    fn build_directive_dispatch() {
        // Latency kind → directive carries latency.
        let d = build_directive("latency", 250);
        assert_eq!(d.latency_ms, Some(250));
        assert!(d.error_rate.is_none());
        assert!(d.drop_rate.is_none());

        // Error kinds → directive carries error rate 1.0.
        for kind in ["error", "http_error", "5xx", "ERROR"] {
            let d = build_directive(kind, 100);
            assert_eq!(d.error_rate, Some(1.0), "kind={kind}");
            assert!(d.latency_ms.is_none(), "kind={kind}");
        }

        // Drop kinds.
        for kind in ["drop", "connection_drop", "Drop"] {
            let d = build_directive(kind, 100);
            assert_eq!(d.drop_rate, Some(1.0), "kind={kind}");
        }

        // Unknown kind → falls back to latency.
        let d = build_directive("nonsense", 999);
        assert_eq!(d.latency_ms, Some(999));
    }
}
