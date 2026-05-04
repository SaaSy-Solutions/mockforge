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

use mockforge_bench::cloud_api::{
    self, CloudBenchInputs, CloudConformanceInputs, CloudCrudFlowInputs, CloudDataDrivenInputs,
    CloudOwaspInputs, CloudRunArtifacts, CloudSecurityInputs, CloudWafBenchInputs, DataFormat,
    SpecFormat,
};

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

        // Cloud-API path: when the payload opts in via `use_cloud_api: true`
        // and supplies an OpenAPI spec, dispatch to the
        // mockforge_bench::cloud_api wrappers — same logic as the local
        // `mockforge-cli bench` command, just driven from the cloud worker
        // against an external target. Requires the k6 binary on $PATH (the
        // runner Dockerfile installs it). When the payload doesn't opt in,
        // bench/owasp fall through to the lighter-weight reqwest paths
        // below and conformance falls through to synthetic mode.
        if uses_cloud_api(&job.payload) {
            match self.kind {
                "conformance" => {
                    return run_cloud_conformance(job, callbacks, started).await;
                }
                "bench" => {
                    if let Some(spec) = extract_spec_bytes(&job.payload) {
                        return run_cloud_bench(job, callbacks, started, spec).await;
                    }
                }
                "owasp" => {
                    if let Some(spec) = extract_spec_bytes(&job.payload) {
                        return run_cloud_owasp(job, callbacks, started, spec).await;
                    }
                }
                "security" => {
                    if let Some(spec) = extract_spec_bytes(&job.payload) {
                        return run_cloud_security(job, callbacks, started, spec).await;
                    }
                }
                "wafbench" => {
                    if let Some(spec) = extract_spec_bytes(&job.payload) {
                        return run_cloud_wafbench(job, callbacks, started, spec).await;
                    }
                }
                "crud_flow" => {
                    if let Some(spec) = extract_spec_bytes(&job.payload) {
                        return run_cloud_crud_flow(job, callbacks, started, spec).await;
                    }
                }
                "data_driven" => {
                    if let Some(spec) = extract_spec_bytes(&job.payload) {
                        return run_cloud_data_driven(job, callbacks, started, spec).await;
                    }
                }
                _ => {}
            }
        }

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

/// Whether the queued payload opts into the [`mockforge_bench::cloud_api`]
/// path. Defaults to `false` so callers that haven't migrated keep the
/// existing reqwest-based behavior.
fn uses_cloud_api(payload: &serde_json::Value) -> bool {
    payload.get("use_cloud_api").and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Pull the OpenAPI spec out of the payload as bytes. Looks at
/// `payload.spec` first (a UTF-8 JSON or YAML document inline) — most
/// callers will use this. Returns `None` when no usable spec is supplied
/// so the caller can decide whether to error or fall back.
fn extract_spec_bytes(payload: &serde_json::Value) -> Option<Vec<u8>> {
    let raw = payload.get("spec")?.as_str()?;
    if raw.trim().is_empty() {
        return None;
    }
    Some(raw.as_bytes().to_vec())
}

fn extract_spec_format(payload: &serde_json::Value) -> SpecFormat {
    match payload.get("spec_format").and_then(|v| v.as_str()).unwrap_or("auto") {
        "json" => SpecFormat::Json,
        "yaml" | "yml" => SpecFormat::Yaml,
        _ => SpecFormat::Auto,
    }
}

fn extract_target_url(payload: &serde_json::Value) -> Option<String> {
    payload
        .get("target_url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

fn extract_string(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

fn extract_string_vec(payload: &serde_json::Value, key: &str) -> Vec<String> {
    payload
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Map a [`CloudRunArtifacts`] result to a `JobOutcome` and emit summary
/// events. Status is `Passed` when no `summary.json` was produced (e.g.
/// conformance native executor) or when k6 reported an error rate below 1%.
async fn finish_cloud_run(
    job_run_id: uuid::Uuid,
    callbacks: &RegistryCallbacks,
    started: Instant,
    kind: &'static str,
    target_url: &str,
    artifacts: CloudRunArtifacts,
    next_seq: u32,
) -> Result<JobOutcome> {
    // Emit a metric event with the structured summary so the SSE stream
    // shows the run results live without consumers having to fetch
    // artifacts.
    let summary_json = artifacts
        .k6_results
        .as_ref()
        .map(|r| {
            serde_json::json!({
                "name": format!("{kind}_summary"),
                "total_requests": r.total_requests,
                "failed_requests": r.failed_requests,
                "error_rate_pct": r.error_rate(),
                "rps": r.rps,
                "p95_ms": r.p95_duration_ms,
                "p99_ms": r.p99_duration_ms,
                "vus_max": r.vus_max,
            })
        })
        .unwrap_or_else(|| {
            serde_json::json!({
                "name": format!("{kind}_summary"),
                "artifacts": artifacts.files.keys().collect::<Vec<_>>(),
            })
        });

    callbacks.run_event(job_run_id, next_seq, "metric", summary_json).await?;

    let elapsed = started.elapsed();
    let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
    let status = match artifacts.k6_results.as_ref() {
        Some(r) if r.error_rate() >= 1.0 => JobStatus::Failed,
        _ => JobStatus::Passed,
    };
    Ok(JobOutcome {
        status,
        runner_seconds: secs,
        summary: Some(serde_json::json!({
            "executor_phase": "cloud_api",
            "kind": kind,
            "target_url": target_url,
            "artifact_files": artifacts.files.keys().collect::<Vec<_>>(),
            "k6_results": artifacts.k6_results,
            "wall_ms": elapsed.as_millis() as u64,
        })),
    })
}

/// Map a `cloud_api` error to an `Errored` outcome with the failure
/// message captured as a `step_fail` event.
async fn finish_cloud_error(
    job_run_id: uuid::Uuid,
    callbacks: &RegistryCallbacks,
    started: Instant,
    kind: &'static str,
    target_url: &str,
    next_seq: u32,
    err: mockforge_bench::error::BenchError,
) -> Result<JobOutcome> {
    callbacks
        .run_event(
            job_run_id,
            next_seq,
            "step_fail",
            serde_json::json!({
                "step": 1,
                "name": format!("cloud_api_{kind}"),
                "error": err.to_string(),
            }),
        )
        .await?;
    let elapsed = started.elapsed();
    Ok(JobOutcome {
        status: JobStatus::Errored,
        runner_seconds: (elapsed.as_secs_f64().ceil() as i32).max(1),
        summary: Some(serde_json::json!({
            "executor_phase": "cloud_api",
            "kind": kind,
            "target_url": target_url,
            "error": err.to_string(),
            "wall_ms": elapsed.as_millis() as u64,
        })),
    })
}

/// Drive a k6 load test via `cloud_api::run_bench`.
async fn run_cloud_bench(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    spec_bytes: Vec<u8>,
) -> Result<JobOutcome> {
    let Some(target_url) = extract_target_url(&job.payload) else {
        return finish_cloud_error(
            job.run_id,
            callbacks,
            started,
            "bench",
            "",
            1,
            mockforge_bench::error::BenchError::Other(
                "use_cloud_api=true bench job missing target_url".to_string(),
            ),
        )
        .await;
    };

    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!("Cloud bench against {target_url}"),
                "executor_phase": "cloud_api",
            }),
        )
        .await?;

    let inputs = CloudBenchInputs {
        spec_bytes,
        spec_format: extract_spec_format(&job.payload),
        target_url: target_url.clone(),
        base_path: extract_string(&job.payload, "base_path"),
        duration: extract_string(&job.payload, "duration").unwrap_or_else(|| "30s".to_string()),
        vus: job.payload.get("vus").and_then(|v| v.as_u64()).unwrap_or(10).clamp(1, 1000) as u32,
        scenario: extract_string(&job.payload, "scenario")
            .unwrap_or_else(|| "constant".to_string()),
        operations: extract_string(&job.payload, "operations"),
        exclude_operations: extract_string(&job.payload, "exclude_operations"),
        auth: extract_string(&job.payload, "auth"),
        headers: extract_string(&job.payload, "headers"),
        threshold_percentile: extract_string(&job.payload, "threshold_percentile")
            .unwrap_or_else(|| "p(95)".to_string()),
        threshold_ms: job.payload.get("threshold_ms").and_then(|v| v.as_u64()).unwrap_or(1000),
        max_error_rate: job.payload.get("max_error_rate").and_then(|v| v.as_f64()).unwrap_or(0.01),
        skip_tls_verify: job
            .payload
            .get("skip_tls_verify")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        chunked_request_bodies: false,
    };

    match cloud_api::run_bench(inputs).await {
        Ok(artifacts) => {
            finish_cloud_run(job.run_id, callbacks, started, "bench", &target_url, artifacts, 2)
                .await
        }
        Err(e) => {
            finish_cloud_error(job.run_id, callbacks, started, "bench", &target_url, 2, e).await
        }
    }
}

/// Drive an OWASP API Top 10 run via `cloud_api::run_owasp`.
async fn run_cloud_owasp(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    spec_bytes: Vec<u8>,
) -> Result<JobOutcome> {
    let Some(target_url) = extract_target_url(&job.payload) else {
        return finish_cloud_error(
            job.run_id,
            callbacks,
            started,
            "owasp",
            "",
            1,
            mockforge_bench::error::BenchError::Other(
                "use_cloud_api=true owasp job missing target_url".to_string(),
            ),
        )
        .await;
    };

    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!("Cloud OWASP scan against {target_url}"),
                "executor_phase": "cloud_api",
            }),
        )
        .await?;

    let inputs = CloudOwaspInputs {
        spec_bytes,
        spec_format: extract_spec_format(&job.payload),
        target_url: target_url.clone(),
        base_path: extract_string(&job.payload, "base_path"),
        categories: extract_string(&job.payload, "owasp_categories"),
        auth_header: extract_string(&job.payload, "owasp_auth_header")
            .unwrap_or_else(|| "Authorization".to_string()),
        auth_token: extract_string(&job.payload, "owasp_auth_token"),
        admin_paths: extract_string_vec(&job.payload, "owasp_admin_paths"),
        id_fields: extract_string(&job.payload, "owasp_id_fields"),
        report_format: extract_string(&job.payload, "owasp_report_format")
            .unwrap_or_else(|| "json".to_string()),
        iterations: job
            .payload
            .get("owasp_iterations")
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .clamp(1, 100) as u32,
        vus: job.payload.get("vus").and_then(|v| v.as_u64()).unwrap_or(10).clamp(1, 1000) as u32,
        skip_tls_verify: job
            .payload
            .get("skip_tls_verify")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        headers: extract_string(&job.payload, "headers"),
    };

    match cloud_api::run_owasp(inputs).await {
        Ok(artifacts) => {
            finish_cloud_run(job.run_id, callbacks, started, "owasp", &target_url, artifacts, 2)
                .await
        }
        Err(e) => {
            finish_cloud_error(job.run_id, callbacks, started, "owasp", &target_url, 2, e).await
        }
    }
}

/// Drive an OpenAPI 3.0.0 conformance run via `cloud_api::run_conformance`.
///
/// Unlike bench/owasp, the spec is optional — the native conformance
/// executor's reference-check mode runs without one. So this path opts
/// in purely on `kind == "conformance" && use_cloud_api == true`.
async fn run_cloud_conformance(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
) -> Result<JobOutcome> {
    let Some(target_url) = extract_target_url(&job.payload) else {
        return finish_cloud_error(
            job.run_id,
            callbacks,
            started,
            "conformance",
            "",
            1,
            mockforge_bench::error::BenchError::Other(
                "use_cloud_api=true conformance job missing target_url".to_string(),
            ),
        )
        .await;
    };

    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!("Cloud conformance against {target_url}"),
                "executor_phase": "cloud_api",
            }),
        )
        .await?;

    let inputs = CloudConformanceInputs {
        spec_bytes: extract_spec_bytes(&job.payload),
        spec_format: extract_spec_format(&job.payload),
        target_url: target_url.clone(),
        base_path: extract_string(&job.payload, "base_path"),
        api_key: extract_string(&job.payload, "conformance_api_key"),
        basic_auth: extract_string(&job.payload, "conformance_basic_auth"),
        categories: extract_string(&job.payload, "conformance_categories"),
        headers: extract_string_vec(&job.payload, "conformance_headers"),
        all_operations: job
            .payload
            .get("conformance_all_operations")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        request_delay_ms: job
            .payload
            .get("conformance_delay_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        use_k6: job.payload.get("use_k6").and_then(|v| v.as_bool()).unwrap_or(false),
        skip_tls_verify: job
            .payload
            .get("skip_tls_verify")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        report_format: extract_string(&job.payload, "conformance_report_format")
            .unwrap_or_else(|| "json".to_string()),
        export_requests: job
            .payload
            .get("export_requests")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        validate_requests: job
            .payload
            .get("validate_requests")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };

    match cloud_api::run_conformance(inputs).await {
        Ok(artifacts) => {
            finish_cloud_run(
                job.run_id,
                callbacks,
                started,
                "conformance",
                &target_url,
                artifacts,
                2,
            )
            .await
        }
        Err(e) => {
            finish_cloud_error(job.run_id, callbacks, started, "conformance", &target_url, 2, e)
                .await
        }
    }
}

/// Drive a payload-injection security run via `cloud_api::run_security`.
async fn run_cloud_security(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    spec_bytes: Vec<u8>,
) -> Result<JobOutcome> {
    let Some(target_url) = extract_target_url(&job.payload) else {
        return finish_cloud_error(
            job.run_id,
            callbacks,
            started,
            "security",
            "",
            1,
            mockforge_bench::error::BenchError::Other(
                "use_cloud_api=true security job missing target_url".to_string(),
            ),
        )
        .await;
    };

    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!("Cloud security scan against {target_url}"),
                "executor_phase": "cloud_api",
            }),
        )
        .await?;

    let inputs = CloudSecurityInputs {
        spec_bytes,
        spec_format: extract_spec_format(&job.payload),
        target_url: target_url.clone(),
        base_path: extract_string(&job.payload, "base_path"),
        duration: extract_string(&job.payload, "duration").unwrap_or_else(|| "30s".to_string()),
        vus: job.payload.get("vus").and_then(|v| v.as_u64()).unwrap_or(10).clamp(1, 1000) as u32,
        scenario: extract_string(&job.payload, "scenario")
            .unwrap_or_else(|| "constant".to_string()),
        categories: extract_string(&job.payload, "security_categories"),
        target_fields: extract_string(&job.payload, "security_target_fields"),
        auth: extract_string(&job.payload, "auth"),
        headers: extract_string(&job.payload, "headers"),
        skip_tls_verify: job
            .payload
            .get("skip_tls_verify")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };

    match cloud_api::run_security(inputs).await {
        Ok(artifacts) => {
            finish_cloud_run(job.run_id, callbacks, started, "security", &target_url, artifacts, 2)
                .await
        }
        Err(e) => {
            finish_cloud_error(job.run_id, callbacks, started, "security", &target_url, 2, e).await
        }
    }
}

/// Drive a WAFBench coverage run via `cloud_api::run_wafbench`.
///
/// `payload.wafbench_rules_dir` must point to a path or glob accessible
/// to the runner. In production this is the bundled CRS install at
/// `/usr/share/mockforge/wafbench/` (see the runner Dockerfile).
async fn run_cloud_wafbench(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    spec_bytes: Vec<u8>,
) -> Result<JobOutcome> {
    let Some(target_url) = extract_target_url(&job.payload) else {
        return finish_cloud_error(
            job.run_id,
            callbacks,
            started,
            "wafbench",
            "",
            1,
            mockforge_bench::error::BenchError::Other(
                "use_cloud_api=true wafbench job missing target_url".to_string(),
            ),
        )
        .await;
    };

    let rules_dir = extract_string(&job.payload, "wafbench_rules_dir")
        .unwrap_or_else(|| "/usr/share/mockforge/wafbench/*.yaml".to_string());

    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!("Cloud WAFBench against {target_url} using {rules_dir}"),
                "executor_phase": "cloud_api",
            }),
        )
        .await?;

    let inputs = CloudWafBenchInputs {
        spec_bytes,
        spec_format: extract_spec_format(&job.payload),
        target_url: target_url.clone(),
        base_path: extract_string(&job.payload, "base_path"),
        duration: extract_string(&job.payload, "duration").unwrap_or_else(|| "30s".to_string()),
        vus: job.payload.get("vus").and_then(|v| v.as_u64()).unwrap_or(10).clamp(1, 1000) as u32,
        scenario: extract_string(&job.payload, "scenario")
            .unwrap_or_else(|| "constant".to_string()),
        rules_dir,
        cycle_all: job.payload.get("wafbench_cycle_all").and_then(|v| v.as_bool()).unwrap_or(false),
        auth: extract_string(&job.payload, "auth"),
        headers: extract_string(&job.payload, "headers"),
        skip_tls_verify: job
            .payload
            .get("skip_tls_verify")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };

    match cloud_api::run_wafbench(inputs).await {
        Ok(artifacts) => {
            finish_cloud_run(job.run_id, callbacks, started, "wafbench", &target_url, artifacts, 2)
                .await
        }
        Err(e) => {
            finish_cloud_error(job.run_id, callbacks, started, "wafbench", &target_url, 2, e).await
        }
    }
}

/// Drive a CRUD-flow chain run via `cloud_api::run_crud_flow`.
async fn run_cloud_crud_flow(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    spec_bytes: Vec<u8>,
) -> Result<JobOutcome> {
    let Some(target_url) = extract_target_url(&job.payload) else {
        return finish_cloud_error(
            job.run_id,
            callbacks,
            started,
            "crud_flow",
            "",
            1,
            mockforge_bench::error::BenchError::Other(
                "use_cloud_api=true crud_flow job missing target_url".to_string(),
            ),
        )
        .await;
    };

    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!("Cloud CRUD flow against {target_url}"),
                "executor_phase": "cloud_api",
            }),
        )
        .await?;

    let inputs = CloudCrudFlowInputs {
        spec_bytes,
        spec_format: extract_spec_format(&job.payload),
        target_url: target_url.clone(),
        base_path: extract_string(&job.payload, "base_path"),
        duration: extract_string(&job.payload, "duration").unwrap_or_else(|| "30s".to_string()),
        vus: job.payload.get("vus").and_then(|v| v.as_u64()).unwrap_or(10).clamp(1, 1000) as u32,
        scenario: extract_string(&job.payload, "scenario")
            .unwrap_or_else(|| "constant".to_string()),
        flow_config_yaml: extract_string(&job.payload, "flow_config_yaml"),
        extract_fields: extract_string(&job.payload, "extract_fields"),
        auth: extract_string(&job.payload, "auth"),
        headers: extract_string(&job.payload, "headers"),
        skip_tls_verify: job
            .payload
            .get("skip_tls_verify")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };

    match cloud_api::run_crud_flow(inputs).await {
        Ok(artifacts) => {
            finish_cloud_run(job.run_id, callbacks, started, "crud_flow", &target_url, artifacts, 2)
                .await
        }
        Err(e) => {
            finish_cloud_error(job.run_id, callbacks, started, "crud_flow", &target_url, 2, e).await
        }
    }
}

/// Maximum size of a data-driven payload the runner will pull from
/// object storage. 64 MB is generous for CSV/JSON test vectors and
/// well below the runner's memory budget; bigger files should be
/// split or pre-processed.
const DATA_DRIVEN_MAX_BYTES: u64 = 64 * 1024 * 1024;

/// Fetch a CSV/JSON test-data file via HTTP. Used by the data-driven
/// kind to pull the test-vector payload from Tigris (or any other
/// presigned-URL-capable storage) before invoking
/// [`cloud_api::run_data_driven`].
async fn fetch_data_bytes(
    url: &str,
) -> std::result::Result<Vec<u8>, mockforge_bench::error::BenchError> {
    use mockforge_bench::error::BenchError;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("mockforge-test-runner/1.0")
        .build()
        .map_err(|e| BenchError::Other(format!("Failed to build HTTP client: {}", e)))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| BenchError::Other(format!("Failed to GET data_url: {}", e)))?;
    if !response.status().is_success() {
        return Err(BenchError::Other(format!("data_url returned HTTP {}", response.status())));
    }
    if let Some(len) = response.content_length() {
        if len > DATA_DRIVEN_MAX_BYTES {
            return Err(BenchError::Other(format!(
                "data_url too large ({} bytes; max {})",
                len, DATA_DRIVEN_MAX_BYTES
            )));
        }
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| BenchError::Other(format!("Failed to read data_url body: {}", e)))?;
    if bytes.len() as u64 > DATA_DRIVEN_MAX_BYTES {
        return Err(BenchError::Other(format!(
            "data_url body too large ({} bytes; max {})",
            bytes.len(),
            DATA_DRIVEN_MAX_BYTES
        )));
    }
    Ok(bytes.to_vec())
}

fn extract_data_format(payload: &serde_json::Value) -> DataFormat {
    match payload.get("data_format").and_then(|v| v.as_str()).unwrap_or("auto") {
        "csv" => DataFormat::Csv,
        "json" => DataFormat::Json,
        _ => DataFormat::Auto,
    }
}

/// Drive a data-driven k6 run via `cloud_api::run_data_driven`.
///
/// Reads `data_url` from the payload (typically a Tigris presigned GET
/// URL), fetches the CSV/JSON body, and dispatches. Inline-bytes mode
/// is intentionally not supported here — large test-vector files belong
/// in object storage, not Postgres JSONB.
async fn run_cloud_data_driven(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    spec_bytes: Vec<u8>,
) -> Result<JobOutcome> {
    let Some(target_url) = extract_target_url(&job.payload) else {
        return finish_cloud_error(
            job.run_id,
            callbacks,
            started,
            "data_driven",
            "",
            1,
            mockforge_bench::error::BenchError::Other(
                "use_cloud_api=true data_driven job missing target_url".to_string(),
            ),
        )
        .await;
    };

    let Some(data_url) = extract_string(&job.payload, "data_url") else {
        return finish_cloud_error(
            job.run_id,
            callbacks,
            started,
            "data_driven",
            &target_url,
            1,
            mockforge_bench::error::BenchError::Other(
                "data_driven job missing data_url (Tigris presigned GET)".to_string(),
            ),
        )
        .await;
    };

    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!("Cloud data-driven against {target_url} (fetching test vectors from {data_url})"),
                "executor_phase": "cloud_api",
            }),
        )
        .await?;

    let data_bytes = match fetch_data_bytes(&data_url).await {
        Ok(b) => b,
        Err(e) => {
            return finish_cloud_error(
                job.run_id,
                callbacks,
                started,
                "data_driven",
                &target_url,
                2,
                e,
            )
            .await;
        }
    };

    let inputs = CloudDataDrivenInputs {
        spec_bytes,
        spec_format: extract_spec_format(&job.payload),
        data_bytes,
        data_format: extract_data_format(&job.payload),
        target_url: target_url.clone(),
        base_path: extract_string(&job.payload, "base_path"),
        duration: extract_string(&job.payload, "duration").unwrap_or_else(|| "30s".to_string()),
        vus: job.payload.get("vus").and_then(|v| v.as_u64()).unwrap_or(10).clamp(1, 1000) as u32,
        scenario: extract_string(&job.payload, "scenario")
            .unwrap_or_else(|| "constant".to_string()),
        distribution: extract_string(&job.payload, "data_distribution")
            .unwrap_or_else(|| "unique-per-vu".to_string()),
        mappings: extract_string(&job.payload, "data_mappings"),
        per_uri_control: job
            .payload
            .get("per_uri_control")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        auth: extract_string(&job.payload, "auth"),
        headers: extract_string(&job.payload, "headers"),
        skip_tls_verify: job
            .payload
            .get("skip_tls_verify")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };

    match cloud_api::run_data_driven(inputs).await {
        Ok(artifacts) => {
            finish_cloud_run(
                job.run_id,
                callbacks,
                started,
                "data_driven",
                &target_url,
                artifacts,
                3,
            )
            .await
        }
        Err(e) => {
            finish_cloud_error(job.run_id, callbacks, started, "data_driven", &target_url, 3, e)
                .await
        }
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

    #[test]
    fn uses_cloud_api_default_false() {
        assert!(!uses_cloud_api(&json!({})));
        assert!(!uses_cloud_api(&json!(null)));
        assert!(!uses_cloud_api(&json!({"use_cloud_api": false})));
    }

    #[test]
    fn uses_cloud_api_explicit_true() {
        assert!(uses_cloud_api(&json!({"use_cloud_api": true})));
    }

    #[test]
    fn extract_spec_bytes_from_string() {
        let p = json!({"spec": "openapi: 3.0.0\n"});
        assert_eq!(extract_spec_bytes(&p).unwrap(), b"openapi: 3.0.0\n".to_vec());
    }

    #[test]
    fn extract_spec_bytes_returns_none_when_missing_or_blank() {
        assert!(extract_spec_bytes(&json!({})).is_none());
        assert!(extract_spec_bytes(&json!({"spec": ""})).is_none());
        assert!(extract_spec_bytes(&json!({"spec": "   "})).is_none());
        assert!(extract_spec_bytes(&json!({"spec": 42})).is_none());
    }

    #[test]
    fn extract_spec_format_maps_known_strings() {
        assert!(matches!(extract_spec_format(&json!({"spec_format": "json"})), SpecFormat::Json));
        assert!(matches!(extract_spec_format(&json!({"spec_format": "yaml"})), SpecFormat::Yaml));
        assert!(matches!(extract_spec_format(&json!({"spec_format": "yml"})), SpecFormat::Yaml));
        assert!(matches!(extract_spec_format(&json!({})), SpecFormat::Auto));
        assert!(matches!(extract_spec_format(&json!({"spec_format": "junk"})), SpecFormat::Auto));
    }

    #[test]
    fn extract_target_url_trims_and_skips_blank() {
        assert_eq!(
            extract_target_url(&json!({"target_url": "  https://x.com  "})),
            Some("https://x.com".to_string())
        );
        assert!(extract_target_url(&json!({"target_url": ""})).is_none());
        assert!(extract_target_url(&json!({"target_url": "   "})).is_none());
        assert!(extract_target_url(&json!({})).is_none());
    }

    #[test]
    fn extract_string_vec_picks_strings() {
        let p = json!({"owasp_admin_paths": ["/admin", "  /root  ", "", 42]});
        assert_eq!(extract_string_vec(&p, "owasp_admin_paths"), vec!["/admin", "/root"]);
    }

    #[test]
    fn extract_string_vec_empty_for_missing() {
        let v: Vec<String> = extract_string_vec(&json!({}), "anything");
        assert!(v.is_empty());
    }
}
