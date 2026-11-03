//! Test execution suite executor (#4 / Phases 3-4).
//!
//! Handles `integration | conformance | bench | owasp | security | wafbench |
//! crud_flow` kinds. The existing `mockforge-bench` and `mockforge-test` crates own the actual
//! per-kind logic; this executor is the cloud worker that picks up the
//! queued run and dispatches to those.

use async_trait::async_trait;
use std::time::Instant;

use mockforge_bench::cloud_api::{
    self, CloudBenchInputs, CloudCrudFlowInputs, CloudOwaspInputs, CloudRunArtifacts,
    CloudSecurityInputs, CloudWafBenchInputs, SpecFormat,
};
use mockforge_bench::conformance::custom::CustomConformanceConfig;
use mockforge_bench::conformance::executor::{ConformanceProgress, NativeConformanceExecutor};
use mockforge_bench::conformance::generator::ConformanceConfig;
use mockforge_bench::executor::K6Results;
use mockforge_bench::ssrf::validate_target_url;
use tokio::sync::mpsc;

use crate::callbacks::RegistryCallbacks;
use crate::error::{Error, Result};
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};
use crate::target_client::{ssrf_policy, TargetClient};

/// Executor for the runnable test-suite kinds.
pub struct TestExecutor {
    kind: &'static str,
}

impl TestExecutor {
    /// Construct the executor for a supported test-suite kind.
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
        validate_test_config(self.kind, &job.payload)?;
        let started = Instant::now();
        callbacks.run_started(job.run_id).await?;

        // Cloud-API path: when the payload opts in via `use_cloud_api: true`
        // and supplies an OpenAPI spec, dispatch to the
        // mockforge_bench::cloud_api wrappers — same logic as the local
        // `mockforge-cli bench` command, just driven from the cloud worker
        // against an external target. Requires the k6 binary on $PATH (the
        // runner Dockerfile installs it). Bench and OWASP can instead use
        // their native HTTP paths when a target URL is supplied.
        let refusal = crate::target_client::base_path_refusal(
            extract_string(&job.payload, "base_path").as_deref(),
        )
        .or_else(|| {
            uses_cloud_api(&job.payload)
                .then(|| {
                    crate::target_client::k6_refusal(
                        self.kind,
                        mockforge_bench::executor::k6_egress_proxy(),
                    )
                })
                .flatten()
        });
        if let Some(reason) = refusal {
            return refuse_k6_run(job, callbacks, started, self.kind, reason).await;
        }
        if uses_cloud_api(&job.payload) {
            return run_cloud_test(self.kind, job, callbacks, started).await;
        }

        run_native_test(self.kind, job, callbacks, started).await
    }
}

/// Select a cloud executor once, sharing inline-spec extraction across kinds.
/// Bench and OWASP retain their native fallback when no inline spec is supplied.
async fn run_cloud_test(
    kind: &str,
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
) -> Result<JobOutcome> {
    match (kind, extract_spec_bytes(&job.payload)) {
        ("conformance", _) => run_cloud_conformance(job, callbacks, started).await,
        ("bench", Some(spec)) => run_cloud_bench(job, callbacks, started, spec).await,
        ("owasp", Some(spec)) => run_cloud_owasp(job, callbacks, started, spec).await,
        ("security", Some(spec)) => run_cloud_security(job, callbacks, started, spec).await,
        ("wafbench", Some(spec)) => run_cloud_wafbench(job, callbacks, started, spec).await,
        ("crud_flow", Some(spec)) => run_cloud_crud_flow(job, callbacks, started, spec).await,
        _ => run_native_test(kind, job, callbacks, started).await,
    }
}

/// Run only an executor that can perform real work against its configured target.
async fn run_native_test(
    kind: &str,
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
) -> Result<JobOutcome> {
    match kind {
        "owasp" => {
            if let Some(target) = extract_target_url(&job.payload) {
                return run_real_owasp_scan(job, callbacks, started, &target).await;
            }
        }
        "integration" => {
            if let Some(config) = integration_workflow(&job.payload).cloned() {
                return crate::executors::integration::run_integration(
                    job, callbacks, started, &config,
                )
                .await;
            }
        }
        "bench" => {
            if let Some(target) = extract_target_url(&job.payload) {
                return run_real_bench(job, callbacks, started, &target).await;
            }
        }
        _ => {}
    }
    Err(Error::Executor(format!(
        "{kind} configuration did not select an executable path"
    )))
}

fn validate_test_config(kind: &str, payload: &serde_json::Value) -> Result<()> {
    let cloud_spec = uses_cloud_api(payload) && extract_spec_bytes(payload).is_some();
    let target_url = extract_target_url(payload).is_some();
    let missing = match kind {
        "integration" => integration_workflow(payload).is_none().then_some("non-empty `steps`"),
        "conformance" => (!uses_cloud_api(payload)).then_some("`use_cloud_api: true`"),
        "bench" | "owasp" => (!target_url && !cloud_spec)
            .then_some("`target_url`, or `use_cloud_api: true` with `spec`"),
        "security" | "wafbench" | "crud_flow" => {
            (!cloud_spec).then_some("`use_cloud_api: true` with `spec`")
        }
        _ => None,
    };
    match missing {
        Some(required) => Err(Error::Executor(format!("{kind} requires {required}"))),
        None => Ok(()),
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
/// events. A k6 run is only valid when `summary.json` was parsed and at
/// least one request completed.
async fn finish_cloud_run(
    job_run_id: uuid::Uuid,
    callbacks: &RegistryCallbacks,
    started: Instant,
    kind: &'static str,
    target_url: &str,
    artifacts: CloudRunArtifacts,
    next_seq: u32,
) -> Result<JobOutcome> {
    let results = artifacts.k6_results.as_ref().ok_or_else(|| {
        Error::Executor(format!(
            "{kind} did not produce a parseable summary.json; no checks can be verified"
        ))
    })?;
    if results.total_requests == 0 {
        return Err(Error::Executor(format!(
            "{kind} completed zero requests; no checks were executed"
        )));
    }
    let reported_findings = (kind == "owasp").then_some(results.owasp_findings);

    // Emit a metric event with the structured summary so the SSE stream
    // shows the run results live without consumers having to fetch
    // artifacts.
    let summary_json = serde_json::json!({
        "name": format!("{kind}_summary"),
        "total_requests": results.total_requests,
        "failed_requests": results.failed_requests,
        "error_rate_pct": results.error_rate(),
        "rps": results.rps,
        "p95_ms": results.p95_duration_ms,
        "p99_ms": results.p99_duration_ms,
        "vus_max": results.vus_max,
        "reported_findings": reported_findings,
    });

    callbacks.run_event(job_run_id, next_seq, "metric", summary_json).await?;

    let elapsed = started.elapsed();
    let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
    let status = cloud_status(kind, results)?;
    let artifact_files = artifacts.files.keys().cloned().collect::<Vec<_>>();
    Ok(JobOutcome {
        status,
        runner_seconds: secs,
        summary: Some(serde_json::json!({
            "executor_phase": "cloud_api",
            "kind": kind,
            "target_url": target_url,
            "artifact_files": artifact_files,
            "k6_results": artifacts.k6_results,
            "reported_findings": reported_findings,
            "wall_ms": elapsed.as_millis() as u64,
        })),
    })
}

fn cloud_status(kind: &str, results: &K6Results) -> Result<JobStatus> {
    if kind == "owasp" {
        if results.failed_requests >= results.total_requests {
            return Err(Error::Executor(
                "owasp got no HTTP response below 500 from the target; no probe could be evaluated"
                    .to_string(),
            ));
        }
        if results.owasp_findings > 0 {
            return Ok(JobStatus::Failed);
        }
    }
    Ok(if results.error_rate() >= 1.0 {
        JobStatus::Failed
    } else {
        JobStatus::Passed
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

/// Finish a k6-backed run that this runner will not start (no egress
/// proxy configured). Errored, not failed: the target was never contacted.
/// The integration workflow carried by a job payload, if it has steps.
///
/// The registry enqueues a suite's config as the payload itself
/// (`{"setup": .., "steps": [..]}`); older callers nested it under
/// `config`. Accept both, so a suite-triggered run executes its real steps
/// instead of entering an executor without enough information to run.
fn integration_workflow(payload: &serde_json::Value) -> Option<&serde_json::Value> {
    let has_steps = |c: &serde_json::Value| {
        c.get("steps").and_then(|v| v.as_array()).is_some_and(|a| !a.is_empty())
    };
    payload
        .get("config")
        .filter(|c| has_steps(c))
        .or_else(|| Some(payload).filter(|p| has_steps(p)))
}

async fn refuse_k6_run(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    kind: &str,
    reason: String,
) -> Result<JobOutcome> {
    tracing::warn!(run_id = %job.run_id, kind, "{reason}");
    callbacks
        .run_event(
            job.run_id,
            1,
            "step_fail",
            serde_json::json!({ "step": 1, "name": format!("cloud_api_{kind}"), "error": reason }),
        )
        .await?;
    let elapsed = started.elapsed();
    Ok(JobOutcome {
        status: JobStatus::Errored,
        runner_seconds: (elapsed.as_secs_f64().ceil() as i32).max(1),
        summary: Some(serde_json::json!({
            "executor_phase": "k6_refused",
            "kind": kind,
            "error": reason,
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
        headers: extract_string_vec(&job.payload, "headers"),
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
        headers: extract_string_vec(&job.payload, "headers"),
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

/// Build conformance settings while preserving category and header parsing.
fn cloud_conformance_config(payload: &serde_json::Value, target_url: &str) -> ConformanceConfig {
    let categories = extract_string(payload, "conformance_categories")
        .map(|s| s.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect());

    let custom_headers: Vec<(String, String)> = extract_string_vec(payload, "conformance_headers")
        .into_iter()
        .filter_map(|raw| {
            let (k, v) = raw.split_once(':')?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect();

    ConformanceConfig {
        target_url: target_url.to_owned(),
        api_key: extract_string(payload, "conformance_api_key"),
        basic_auth: extract_string(payload, "conformance_basic_auth"),
        skip_tls_verify: payload.get("skip_tls_verify").and_then(|v| v.as_bool()).unwrap_or(false),
        categories,
        base_path: extract_string(payload, "base_path"),
        custom_headers,
        output_dir: None,
        all_operations: payload
            .get("conformance_all_operations")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        custom_checks_file: None,
        request_delay_ms: payload.get("conformance_delay_ms").and_then(|v| v.as_u64()).unwrap_or(0),
        custom_filter: None,
        export_requests: false,
        validate_requests: false,
    }
}

fn cloud_conformance_executor(
    payload: &serde_json::Value,
    target_url: &str,
) -> mockforge_bench::Result<NativeConformanceExecutor> {
    let config = cloud_conformance_config(payload, target_url);
    let custom = extract_string(payload, "custom_checks_yaml")
        .map(|yaml| {
            serde_yaml::from_str::<CustomConformanceConfig>(&yaml).map_err(|error| {
                mockforge_bench::error::BenchError::Other(format!(
                    "custom_checks_yaml parse failed: {error}"
                ))
            })
        })
        .transpose()?;
    let executor = NativeConformanceExecutor::with_ssrf_guard(config, Some(ssrf_policy()))?
        .with_reference_checks();
    match custom {
        Some(config) => executor.with_custom_checks_from_config(config),
        None => Ok(executor),
    }
}

fn conformance_progress_event(
    progress: ConformanceProgress,
) -> Option<(&'static str, serde_json::Value)> {
    let event = match progress {
        ConformanceProgress::Started { total_checks } => (
            "started",
            serde_json::json!({ "type": "started", "total_checks": total_checks }),
        ),
        ConformanceProgress::CheckCompleted {
            name,
            passed,
            checks_done,
        } => (
            "check_completed",
            serde_json::json!({
                "type": "check_completed",
                "name": name,
                "passed": passed,
                "checks_done": checks_done,
            }),
        ),
        ConformanceProgress::Finished => return None, // emitted with the final report
        ConformanceProgress::Error { message } => {
            ("error", serde_json::json!({ "type": "error", "message": message }))
        }
    };
    Some(event)
}

/// Drive an OpenAPI 3.0.0 conformance run via the native conformance
/// executor in `mockforge_bench::conformance::executor`, streaming
/// per-check progress through the callbacks channel so the cloud UI can
/// show live status (`started` / `check_completed` / `finished`).
///
/// We bypass `cloud_api::run_conformance` here because that path
/// shells out to the `bench` CLI, runs k6, and returns artifacts in a
/// single shot — no per-check progress. The native executor exposes
/// `execute_with_progress`, which is what the local Conformance page
/// already streams.
///
/// SSRF guard is invoked explicitly before construction; without
/// `cloud_api` in the loop it's the only thing standing between an
/// attacker-supplied `target_url` and the runner's network.
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

    // SSRF guard. Defense-in-depth — the registry already checks at
    // trigger time, but a poisoned suite config could still reach a
    // worker, so we re-validate on this side too. Same env-var
    // contract (`MOCKFORGE_SSRF_ALLOW_LOOPBACK=1` for tests) as
    // mockforge_bench::cloud_api uses for the other kinds.
    if let Err(e) = validate_target_url(&target_url, ssrf_policy()).await {
        return finish_cloud_error(
            job.run_id,
            callbacks,
            started,
            "conformance",
            &target_url,
            1,
            mockforge_bench::error::BenchError::Other(format!(
                "target_url rejected by SSRF guard: {e}"
            )),
        )
        .await;
    }

    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!("Cloud conformance against {target_url}"),
                "executor_phase": "native_conformance",
            }),
        )
        .await?;

    let executor = match cloud_conformance_executor(&job.payload, &target_url) {
        Ok(executor) => executor,
        Err(error) => {
            return finish_cloud_error(
                job.run_id,
                callbacks,
                started,
                "conformance",
                &target_url,
                2,
                error,
            )
            .await;
        }
    };

    let (tx, mut rx) = mpsc::channel::<ConformanceProgress>(32);
    let exec_task = tokio::spawn(async move { executor.execute_with_progress(tx).await });

    let mut seq: u32 = 2;
    while let Some(progress) = rx.recv().await {
        let Some((event_type, payload)) = conformance_progress_event(progress) else {
            continue;
        };
        callbacks.run_event(job.run_id, seq, event_type, payload).await?;
        seq += 1;
    }

    let report_result = exec_task
        .await
        .map_err(|e| Error::Executor(format!("conformance task panicked: {e}")))?;

    match report_result {
        Ok(report) => {
            // ConformanceReport doesn't derive Serialize directly — it
            // ships a to_json() that flattens internal maps for stable
            // wire shape, so use that.
            let report_json = report.to_json();

            // Emit the full report on the terminal "finished" event so
            // the UI can render it without an extra fetch. The cloud
            // SSE stream does not auto-attach test_runs.summary into
            // events.
            callbacks
                .run_event(
                    job.run_id,
                    seq,
                    "finished",
                    serde_json::json!({ "type": "finished", "report": report_json.clone() }),
                )
                .await?;

            let elapsed = started.elapsed();
            let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
            let failed_count = report_json
                .get("summary")
                .and_then(|s| s.get("failed"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let total_count = report_json
                .get("summary")
                .and_then(|s| s.get("total_checks"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let status = conformance_status(total_count, failed_count)?;
            Ok(JobOutcome {
                status,
                runner_seconds: secs,
                summary: Some(serde_json::json!({
                    "executor_phase": "native_conformance",
                    "kind": "conformance",
                    "target_url": target_url,
                    "report": report_json,
                    "total_checks": total_count,
                    "failed_checks": failed_count,
                    "wall_ms": elapsed.as_millis() as u64,
                })),
            })
        }
        Err(e) => {
            finish_cloud_error(job.run_id, callbacks, started, "conformance", &target_url, seq, e)
                .await
        }
    }
}

fn conformance_status(total_checks: u64, failed_checks: u64) -> Result<JobStatus> {
    if total_checks == 0 {
        return Err(Error::Executor(
            "conformance completed with zero checks; configure at least one applicable check"
                .to_string(),
        ));
    }
    if failed_checks == 0 {
        Ok(JobStatus::Passed)
    } else {
        Ok(JobStatus::Failed)
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
        headers: extract_string_vec(&job.payload, "headers"),
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
        headers: extract_string_vec(&job.payload, "headers"),
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
        headers: extract_string_vec(&job.payload, "headers"),
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

type BenchWorkerResult = (Vec<u128>, u64, u64);

/// Fail the run on a lost worker; dropping JoinSet also aborts remaining workers.
async fn collect_bench_results(
    mut workers: tokio::task::JoinSet<BenchWorkerResult>,
) -> Result<BenchWorkerResult> {
    let mut latencies = Vec::new();
    let mut total_ok = 0;
    let mut total_err = 0;
    while let Some(result) = workers.join_next().await {
        let (samples, ok, err) =
            result.map_err(|error| Error::Executor(format!("bench worker failed: {error}")))?;
        latencies.extend(samples);
        total_ok += ok;
        total_err += err;
    }
    Ok((latencies, total_ok, total_err))
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
                "tracking_task": 4,
            }),
        )
        .await?;

    let client = TargetClient::new(std::time::Duration::from_secs(10), "mockforge-bench/1.0")?;
    client
        .check(target_url)
        .map_err(|e| Error::Executor(format!("target_url rejected by SSRF guard: {e}")))?;

    let target = target_url.to_string();
    let deadline = Instant::now() + std::time::Duration::from_secs(duration_secs);
    let mut workers = tokio::task::JoinSet::new();
    for _ in 0..concurrency {
        let client = client.clone();
        let target = target.clone();
        workers.spawn(async move {
            let mut latencies_ns: Vec<u128> = Vec::new();
            let mut ok = 0u64;
            let mut err = 0u64;
            while Instant::now() < deadline {
                let started = Instant::now();
                let sent = match client.get(&target) {
                    Ok(req) => req.send().await,
                    Err(_) => break,
                };
                match sent {
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
        });
    }

    let (mut all_latencies, total_ok, total_err) = collect_bench_results(workers).await?;

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

    let elapsed = started.elapsed();
    let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
    let status = bench_status(total_ok + total_err, error_rate_pct, p95)?;

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

fn bench_status(total_requests: u64, error_rate_pct: f64, p95_ms: f64) -> Result<JobStatus> {
    if total_requests == 0 {
        return Err(Error::Executor(
            "bench completed zero requests; no checks were executed".to_string(),
        ));
    }
    if error_rate_pct < 1.0 && p95_ms < 1000.0 {
        Ok(JobStatus::Passed)
    } else {
        Ok(JobStatus::Failed)
    }
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
                "tracking_task": 4,
            }),
        )
        .await?;

    let client = TargetClient::with_policy(
        ssrf_policy(),
        std::time::Duration::from_secs(15),
        "mockforge-owasp-scan/1.0",
        5,
    )?;

    let sent = match client.get(target_url) {
        Ok(req) => req.send().await.map_err(|e| e.to_string()),
        Err(e) => Err(format!("target_url rejected by SSRF guard: {e}")),
    };
    let response = match sent {
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
    let status = owasp_header_status(failed);

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

fn owasp_header_status(failed: u32) -> JobStatus {
    if failed == 0 {
        JobStatus::Passed
    } else {
        JobStatus::Failed
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn integration_workflow_accepts_suite_config_as_payload() {
        let wf = serde_json::json!({"setup": {"base_url": "https://x"}, "steps": [{"request": {"method": "GET", "path": "/"}}]});
        assert_eq!(integration_workflow(&wf), Some(&wf), "registry sends the suite config itself");
        let nested = serde_json::json!({"config": wf.clone()});
        assert_eq!(integration_workflow(&nested), Some(&wf), "legacy nested form still works");
        assert_eq!(integration_workflow(&serde_json::json!({"steps": []})), None);
        assert_eq!(integration_workflow(&serde_json::json!({"target_url": "https://x"})), None);
    }
    use super::*;
    use serde_json::json;

    fn job(kind: &str, payload: serde_json::Value) -> RunJob {
        RunJob {
            run_id: uuid::Uuid::new_v4(),
            org_id: uuid::Uuid::new_v4(),
            source_id: uuid::Uuid::new_v4(),
            kind: kind.to_string(),
            payload,
        }
    }

    #[tokio::test]
    async fn bench_worker_results_preserve_all_samples_and_counts() {
        let mut workers = tokio::task::JoinSet::new();
        workers.spawn(async { (vec![10], 1, 0) });
        workers.spawn(async { (vec![20, 30], 1, 1) });
        let (mut latencies, ok, errors) = collect_bench_results(workers).await.unwrap();
        latencies.sort_unstable();
        assert_eq!(latencies, vec![10, 20, 30]);
        assert_eq!((ok, errors), (2, 1));
    }

    #[tokio::test]
    async fn lost_bench_worker_errors_and_aborts_remaining_work() {
        let mut workers = tokio::task::JoinSet::new();
        let (alive, dropped) = tokio::sync::oneshot::channel::<()>();
        workers.spawn(async move {
            let _guard = alive;
            std::future::pending::<BenchWorkerResult>().await
        });
        let aborted = workers.spawn(std::future::pending::<BenchWorkerResult>());
        aborted.abort();
        let error = collect_bench_results(workers).await.expect_err("lost worker must fail");
        assert!(error.to_string().contains("bench worker failed:"));
        let remaining = tokio::time::timeout(std::time::Duration::from_secs(1), dropped)
            .await
            .expect("remaining worker must be stopped");
        assert!(remaining.is_err(), "remaining worker guard must be dropped");
    }

    #[test]
    fn cloud_conformance_configuration_preserves_filters_and_headers() {
        let config = cloud_conformance_config(
            &json!({
                "conformance_categories": " Parameters, , Security ",
                "conformance_headers": [" X-Fixture : one:two ", "invalid"],
                "conformance_delay_ms": 17,
                "conformance_all_operations": true,
                "base_path": "/fixture",
            }),
            "https://example.com",
        );
        assert_eq!(config.target_url, "https://example.com");
        assert_eq!(config.categories, Some(vec!["Parameters".to_string(), "Security".to_string()]));
        assert_eq!(config.custom_headers, vec![("X-Fixture".to_string(), "one:two".to_string())]);
        assert_eq!(config.request_delay_ms, 17);
        assert!(config.all_operations);
        assert_eq!(config.base_path.as_deref(), Some("/fixture"));
    }

    #[test]
    fn malformed_custom_conformance_checks_fail_before_executor_construction() {
        let result = cloud_conformance_executor(
            &json!({"custom_checks_yaml": "[invalid"}),
            "https://example.com",
        );
        match result {
            Err(error) => assert!(error.to_string().contains("custom_checks_yaml parse failed:")),
            Ok(_) => panic!("malformed custom checks must be rejected"),
        }
    }

    #[tokio::test]
    async fn missing_real_execution_config_is_an_executor_error() {
        let callbacks = RegistryCallbacks::for_test();
        let cases = [
            ("integration", json!({}), "integration requires non-empty `steps`"),
            ("conformance", json!({}), "conformance requires `use_cloud_api: true`"),
            (
                "bench",
                json!({}),
                "bench requires `target_url`, or `use_cloud_api: true` with `spec`",
            ),
            (
                "owasp",
                json!({}),
                "owasp requires `target_url`, or `use_cloud_api: true` with `spec`",
            ),
            (
                "security",
                json!({"target_url": "https://example.com"}),
                "security requires `use_cloud_api: true` with `spec`",
            ),
            (
                "wafbench",
                json!({"target_url": "https://example.com"}),
                "wafbench requires `use_cloud_api: true` with `spec`",
            ),
            (
                "crud_flow",
                json!({"target_url": "https://example.com"}),
                "crud_flow requires `use_cloud_api: true` with `spec`",
            ),
        ];

        for (kind, payload, expected) in cases {
            let result = TestExecutor::for_kind(kind).execute(job(kind, payload), &callbacks).await;
            match result {
                Err(Error::Executor(message)) => assert_eq!(message, expected, "kind={kind}"),
                Err(other) => panic!("kind={kind}: expected executor error, got {other:?}"),
                Ok(outcome) => {
                    panic!("kind={kind}: missing config must not return {:?}", outcome.status)
                }
            }
        }
    }

    /// Valid cloud k6 configs are refused (errored, never passed) when the
    /// runner has no k6 egress proxy, while a cloud config that does no work
    /// is still rejected as a configuration error before any refusal.
    #[tokio::test]
    async fn k6_refusal_and_no_work_rejection_combine() {
        assert!(
            mockforge_bench::executor::k6_egress_proxy().is_none(),
            "tests assume no k6 egress proxy is configured"
        );
        let callbacks = RegistryCallbacks::for_test();
        let spec = r#"{"openapi":"3.0.0","info":{"title":"t","version":"1"},"paths":{}}"#;
        for kind in ["bench", "owasp", "security", "wafbench", "crud_flow"] {
            let payload = json!({
                "use_cloud_api": true,
                "spec": spec,
                "target_url": "https://example.com",
            });
            let outcome = TestExecutor::for_kind(kind)
                .execute(job(kind, payload), &callbacks)
                .await
                .unwrap_or_else(|e| panic!("kind={kind}: refusal must not error: {e:?}"));
            assert!(matches!(outcome.status, JobStatus::Errored), "kind={kind}");
            let summary = outcome.summary.expect("refusal summary");
            assert_eq!(summary["executor_phase"], "k6_refused", "kind={kind}");
            assert_eq!(summary["kind"], kind);
        }

        for kind in ["security", "wafbench", "crud_flow"] {
            let payload = json!({"use_cloud_api": true, "target_url": "https://example.com"});
            let result = TestExecutor::for_kind(kind).execute(job(kind, payload), &callbacks).await;
            assert_eq!(
                executor_message(result),
                format!("{kind} requires `use_cloud_api: true` with `spec`"),
                "kind={kind}: cloud config without spec must be a config error"
            );
        }
    }

    /// An unsafe base_path is refused before any native or cloud dispatch,
    /// and the refusal is errored rather than passed.
    #[tokio::test]
    async fn unsafe_base_path_is_refused_not_passed() {
        let callbacks = RegistryCallbacks::for_test();
        let payload = json!({"target_url": "https://example.com", "base_path": "@localhost:6565"});
        let outcome = TestExecutor::for_kind("bench")
            .execute(job("bench", payload), &callbacks)
            .await
            .expect("refusal is an outcome");
        assert!(matches!(outcome.status, JobStatus::Errored));
        assert_eq!(outcome.summary.expect("summary")["executor_phase"], "k6_refused");
    }

    #[tokio::test]
    async fn cloud_run_without_k6_results_is_an_executor_error() {
        let callbacks = RegistryCallbacks::for_test();
        let result = finish_cloud_run(
            uuid::Uuid::new_v4(),
            &callbacks,
            Instant::now(),
            "bench",
            "https://example.com",
            CloudRunArtifacts::default(),
            1,
        )
        .await;
        let message = executor_message(result);
        assert!(message.contains("summary.json"), "missing k6 results must error, got {message}");
    }

    #[tokio::test]
    async fn cloud_owasp_findings_fail_the_run() {
        let callbacks = RegistryCallbacks::for_test();
        let artifacts = CloudRunArtifacts::from_files(std::collections::HashMap::from([(
            "summary.json".to_string(),
            br#"{"metrics":{"http_reqs":{"values":{"count":1,"rate":1.0}},"http_req_failed":{"values":{"passes":0}},"owasp_findings":{"values":{"count":2}}}}"#.to_vec(),
        )]));

        let outcome = finish_cloud_run(
            uuid::Uuid::new_v4(),
            &callbacks,
            Instant::now(),
            "owasp",
            "https://example.com",
            artifacts,
            1,
        )
        .await
        .unwrap();

        assert_eq!(outcome.status, JobStatus::Failed);
        assert_eq!(outcome.summary.unwrap()["reported_findings"], 2);
    }

    /// Unwrap the message of an executor error, panicking with the actual
    /// result otherwise so a regression shows what was returned instead.
    fn executor_message<T: std::fmt::Debug>(result: Result<T>) -> String {
        match result {
            Err(Error::Executor(message)) => message,
            other => panic!("expected an executor error, got {other:?}"),
        }
    }

    fn k6(total: u64, failed: u64, findings: u64) -> K6Results {
        K6Results {
            total_requests: total,
            failed_requests: failed,
            owasp_findings: findings,
            ..K6Results::default()
        }
    }

    #[test]
    fn owasp_status_follows_findings_not_rejected_probes() {
        assert_eq!(cloud_status("owasp", &k6(20, 0, 0)).unwrap(), JobStatus::Passed);
        assert_eq!(cloud_status("owasp", &k6(20, 0, 3)).unwrap(), JobStatus::Failed);
        assert_eq!(cloud_status("owasp", &k6(20, 5, 0)).unwrap(), JobStatus::Failed);
        let message = executor_message(cloud_status("owasp", &k6(20, 20, 0)));
        assert!(message.contains("no HTTP response below 500"), "{message}");
    }

    #[test]
    fn non_owasp_cloud_status_uses_error_rate() {
        assert_eq!(cloud_status("bench", &k6(200, 1, 0)).unwrap(), JobStatus::Passed);
        assert_eq!(cloud_status("bench", &k6(100, 1, 0)).unwrap(), JobStatus::Failed);
    }

    #[test]
    fn zero_request_bench_is_an_executor_error() {
        let message = executor_message(bench_status(0, 0.0, 0.0));
        assert!(message.contains("zero requests"), "{message}");
    }

    #[test]
    fn zero_check_conformance_is_an_executor_error() {
        let message = executor_message(conformance_status(0, 0));
        assert!(message.contains("zero checks"), "{message}");
    }

    #[test]
    fn native_owasp_findings_fail_the_run() {
        assert_eq!(owasp_header_status(0), JobStatus::Passed);
        assert_eq!(owasp_header_status(1), JobStatus::Failed);
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
