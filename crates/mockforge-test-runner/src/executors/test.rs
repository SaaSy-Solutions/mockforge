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

        // OWASP kind has a real path when payload.target_url is set —
        // scan response headers for the standard security-header set.
        // No external scanner needed; this is a header-presence check
        // (level-1 OWASP ASVS) that catches the most common
        // misconfigurations (missing CSP, no HSTS, etc.).
        if self.kind == "owasp" {
            let target_url =
                job.payload.get("target_url").and_then(|v| v.as_str()).map(String::from);
            if let Some(target) = target_url {
                if !target.is_empty() {
                    return run_real_owasp_scan(job, callbacks, started, &target).await;
                }
            }
        }

        // Bench kind has a real path when payload.target_url is set —
        // hammers the target with concurrent requests and computes
        // p50 / p95 / p99 latencies + error rate. No external load tool
        // needed; for k6/wrk/-style heavy benchmarks the operator can
        // still wire up a separate executor in a follow-up slice.
        if self.kind == "bench" {
            let target_url =
                job.payload.get("target_url").and_then(|v| v.as_str()).map(String::from);
            if let Some(target) = target_url {
                if !target.is_empty() {
                    return run_real_bench(job, callbacks, started, &target).await;
                }
            }
        }

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

/// Reqwest-based load runner for kind=bench. Hammers the target with
/// `concurrency` parallel workers for `duration_secs` (both clamped to
/// safe defaults) and reports p50/p95/p99 latencies + error rate.
///
/// Knobs from the queue payload:
/// - `bench_concurrency` (u32, default 10, capped at 50)
/// - `bench_duration_secs` (u64, default 10, capped at 60)
async fn run_real_bench(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    target_url: &str,
) -> Result<JobOutcome> {
    let concurrency = job
        .payload
        .get("bench_concurrency")
        .and_then(|v| v.as_u64())
        .unwrap_or(10)
        .clamp(1, 50) as u32;
    let duration_secs = job
        .payload
        .get("bench_duration_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(10)
        .clamp(1, 60);

    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!(
                    "Bench against {target_url} — concurrency={concurrency}, duration={duration_secs}s",
                ),
                "synthetic": false,
                "tracking_task": 4,
            }),
        )
        .await?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("mockforge-bench/1.0")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let target = target_url.to_string();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(duration_secs);
    let mut handles = Vec::with_capacity(concurrency as usize);
    for _ in 0..concurrency {
        let client = client.clone();
        let target = target.clone();
        handles.push(tokio::spawn(async move {
            let mut latencies_ns: Vec<u128> = Vec::new();
            let mut ok = 0u64;
            let mut err = 0u64;
            while std::time::Instant::now() < deadline {
                let started = std::time::Instant::now();
                match client.get(&target).send().await {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            ok += 1;
                        } else {
                            err += 1;
                        }
                    }
                    Err(_) => err += 1,
                }
                latencies_ns.push(started.elapsed().as_nanos());
            }
            (latencies_ns, ok, err)
        }));
    }

    let mut all_latencies: Vec<u128> = Vec::new();
    let mut total_ok = 0u64;
    let mut total_err = 0u64;
    for h in handles {
        if let Ok((lats, ok, err)) = h.await {
            all_latencies.extend(lats);
            total_ok += ok;
            total_err += err;
        }
    }

    all_latencies.sort_unstable();
    let total_requests = (total_ok + total_err) as f64;
    let percentile = |p: f64| -> f64 {
        if all_latencies.is_empty() {
            return 0.0;
        }
        let idx = ((p / 100.0) * (all_latencies.len() as f64 - 1.0)).round() as usize;
        let idx = idx.min(all_latencies.len() - 1);
        all_latencies[idx] as f64 / 1_000_000.0 // → ms
    };
    let p50 = percentile(50.0);
    let p95 = percentile(95.0);
    let p99 = percentile(99.0);
    let error_rate_pct = if total_requests > 0.0 {
        (total_err as f64) / total_requests * 100.0
    } else {
        0.0
    };

    callbacks
        .run_event(
            job.run_id,
            2,
            "metric",
            serde_json::json!({
                "name": "bench_summary",
                "total_requests": total_ok + total_err,
                "ok": total_ok,
                "errors": total_err,
                "error_rate_pct": error_rate_pct,
                "p50_ms": p50,
                "p95_ms": p95,
                "p99_ms": p99,
            }),
        )
        .await?;

    let elapsed = started.elapsed();
    let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
    // Pass when error rate is < 1% AND p95 < 1000ms. Tighter SLOs are
    // a follow-up — the suite config could carry user-defined
    // thresholds.
    let status = if error_rate_pct < 1.0 && p95 < 1000.0 {
        JobStatus::Passed
    } else {
        JobStatus::Failed
    };
    Ok(JobOutcome {
        status,
        runner_seconds: secs,
        summary: Some(serde_json::json!({
            "executor_phase": "real_bench",
            "tracking_task": 4,
            "kind": "bench",
            "target_url": target_url,
            "concurrency": concurrency,
            "duration_secs": duration_secs,
            "total_requests": total_ok + total_err,
            "ok": total_ok,
            "errors": total_err,
            "error_rate_pct": error_rate_pct,
            "p50_ms": p50,
            "p95_ms": p95,
            "p99_ms": p99,
            "wall_ms": elapsed.as_millis() as u64,
        })),
    })
}

/// OWASP ASVS level-1 header-presence scan. Fetches the target URL and
/// reports per-header pass/fail for the standard security headers most
/// services should ship.
async fn run_real_owasp_scan(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    target_url: &str,
) -> Result<JobOutcome> {
    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!("OWASP header scan against {target_url}"),
                "synthetic": false,
                "tracking_task": 4,
            }),
        )
        .await?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("mockforge-owasp-scan/1.0")
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let response = match client.get(target_url).send().await {
        Ok(r) => r,
        Err(e) => {
            callbacks
                .run_event(
                    job.run_id,
                    2,
                    "step_fail",
                    serde_json::json!({
                        "step": 1,
                        "name": "fetch_target",
                        "error": e.to_string(),
                    }),
                )
                .await?;
            let elapsed = started.elapsed();
            return Ok(JobOutcome {
                status: JobStatus::Errored,
                runner_seconds: (elapsed.as_secs_f64().ceil() as i32).max(1),
                summary: Some(serde_json::json!({
                    "executor_phase": "real_owasp_scan",
                    "tracking_task": 4,
                    "kind": "owasp",
                    "target_url": target_url,
                    "passed": 0,
                    "failed": 0,
                    "errored": 1,
                    "error": e.to_string(),
                })),
            });
        }
    };

    let status_code = response.status().as_u16();
    let headers = response.headers().clone();

    callbacks
        .run_event(
            job.run_id,
            2,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!("Target returned HTTP {status_code}"),
                "status_code": status_code,
                "header_count": headers.len(),
            }),
        )
        .await?;

    // Standard security headers — each row is (header name, OWASP
    // recommendation, fail severity if missing).
    let checks: &[(&str, &str, &str)] = &[
        (
            "strict-transport-security",
            "Force HTTPS for at least 6 months (max-age >= 15768000)",
            "high",
        ),
        ("content-security-policy", "Mitigate XSS via CSP", "high"),
        ("x-content-type-options", "nosniff to block MIME sniffing", "medium"),
        ("x-frame-options", "DENY/SAMEORIGIN to block clickjacking", "medium"),
        ("referrer-policy", "Restrict Referer header leakage", "low"),
        ("permissions-policy", "Lock down camera/microphone/geolocation features", "low"),
    ];

    let mut next_seq: u32 = 3;
    let mut passed = 0u32;
    let mut failed = 0u32;
    for (i, (header, advice, severity)) in checks.iter().enumerate() {
        let present = headers.contains_key(*header);
        let value = headers.get(*header).and_then(|v| v.to_str().ok()).unwrap_or("");
        if present {
            passed += 1;
            callbacks
                .run_event(
                    job.run_id,
                    next_seq,
                    "step_pass",
                    serde_json::json!({
                        "step": i + 1,
                        "name": format!("header_{header}"),
                        "header": header,
                        "value": value,
                    }),
                )
                .await?;
        } else {
            failed += 1;
            callbacks
                .run_event(
                    job.run_id,
                    next_seq,
                    "step_fail",
                    serde_json::json!({
                        "step": i + 1,
                        "name": format!("header_{header}"),
                        "header": header,
                        "severity": severity,
                        "advice": advice,
                    }),
                )
                .await?;
        }
        next_seq += 1;
    }

    let elapsed = started.elapsed();
    let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
    let status = if failed == 0 {
        JobStatus::Passed
    } else {
        JobStatus::Failed
    };

    Ok(JobOutcome {
        status,
        runner_seconds: secs,
        summary: Some(serde_json::json!({
            "executor_phase": "real_owasp_scan",
            "tracking_task": 4,
            "kind": "owasp",
            "target_url": target_url,
            "target_status_code": status_code,
            "checks_total": checks.len(),
            "passed": passed,
            "failed": failed,
            "wall_ms": elapsed.as_millis() as u64,
        })),
    })
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
