//! Architectural fitness function evaluator (#355 item 2).
//!
//! Replaces the synthetic `fitness_evaluation` arm of `ContractExecutor`
//! with real, data-source-backed evaluation. Each fitness function row
//! has a `kind` + `config` blob; the executor fetches the definition
//! from the registry, dispatches on kind, and runs the corresponding
//! evaluator against live data.
//!
//! Currently implemented:
//!   - `latency_threshold` — queries `runtime_request_logs` aggregates
//!     for a deployment and asserts a percentile latency < threshold.
//!   - `error_rate` — same aggregate endpoint, asserts
//!     `error_count / count <= threshold_rate`.
//!   - `contract_stability` — aggregates `contract_diff_findings`
//!     for a monitored service over a window, asserts
//!     `breaking_count <= max_breaking` (default 0).
//!
//! Stubbed (returns `errored` with a clear "not yet implemented in
//! cloud" message; tracking PR follows this one):
//!   - `custom_query` — likely permanent stub since we don't run
//!     arbitrary user code on cloud workers; will surface in the UI as
//!     "self-hosted only".
//!
//! `run.suite_id` is the fitness function's id (per the `mirror_kind_status`
//! convention). The summary the executor returns includes
//! `measured_value` + `threshold_value` so `mirror_kind_status` can
//! write a row into `fitness_evaluations` and update
//! `fitness_functions.last_status` for the timeline UI.
//!
//! Failure → incident handoff lives in `mirror_kind_status` already; we
//! just need to return `Failed`/`Errored` cleanly here and the
//! existing notification dispatcher fires.

use std::time::Instant;

use async_trait::async_trait;
use uuid::Uuid;

use crate::callbacks::{
    DeploymentLatencyStats, FitnessFunctionDefinition, MonitoredServiceContractStability,
    RegistryCallbacks,
};
use crate::error::Result;
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};

/// Default look-back window for stat queries when the function's
/// config doesn't pin one. 60 minutes balances "recent enough to flag
/// regressions promptly" against "wide enough to have signal" for low-
/// QPS deployments.
const DEFAULT_WINDOW_MINUTES: i64 = 60;

/// Per-call hard cap on the look-back. Mirrors the registry handler's
/// 24-hour clamp so a misshapen config doesn't burn worker time on a
/// week-long aggregate.
const MAX_WINDOW_MINUTES: i64 = 1_440;

/// Executor for `kind = "fitness_evaluation"`. See module docs for
/// payload + dispatch semantics.
pub struct FitnessExecutor;

#[async_trait]
impl Executor for FitnessExecutor {
    fn kind(&self) -> &'static str {
        "fitness_evaluation"
    }

    async fn execute(&self, job: RunJob, callbacks: &RegistryCallbacks) -> Result<JobOutcome> {
        let started = Instant::now();
        callbacks.run_started(job.run_id).await?;

        // Fetch the function definition. `run.suite_id` is the fitness
        // function's id (set when the run was enqueued — see the
        // mirror_kind_status comment chain).
        let function = match callbacks.fetch_fitness_function(job.source_id).await {
            Ok(f) => f,
            Err(e) => {
                return errored_run(
                    callbacks,
                    &job,
                    started,
                    1,
                    format!("failed to fetch fitness function definition: {e}"),
                )
                .await;
            }
        };

        callbacks
            .run_event(
                job.run_id,
                1,
                "log",
                serde_json::json!({
                    "level": "info",
                    "message": format!(
                        "Evaluating fitness function '{}' (kind={})",
                        function.name, function.kind,
                    ),
                    "function_id": function.id,
                    "tracking_task": 8,
                }),
            )
            .await?;

        match function.kind.as_str() {
            "latency_threshold" => {
                evaluate_latency_threshold(&function, callbacks, &job, started).await
            }
            "error_rate" => evaluate_error_rate(&function, callbacks, &job, started).await,
            "contract_stability" => {
                evaluate_contract_stability(&function, callbacks, &job, started).await
            }
            "custom_query" => {
                errored_run(
                    callbacks,
                    &job,
                    started,
                    2,
                    format!(
                        "fitness kind '{}' isn't implemented in the cloud executor yet — \
                         tracking PR follows #355 item 2",
                        function.kind
                    ),
                )
                .await
            }
            other => {
                errored_run(
                    callbacks,
                    &job,
                    started,
                    2,
                    format!(
                        "unknown fitness kind '{}' — must be one of latency_threshold, \
                         error_rate, contract_stability, custom_query",
                        other
                    ),
                )
                .await
            }
        }
    }
}

// ─── latency_threshold evaluator ─────────────────────────────────────

/// Resolved `latency_threshold` config pulled out of the function's
/// JSONB blob. `deployment_id` is required; everything else has a
/// sensible default.
#[derive(Debug)]
struct LatencyThresholdConfig {
    deployment_id: Uuid,
    threshold_ms: f64,
    percentile: LatencyPercentile,
    window_minutes: i64,
    /// Optional path filter — narrows the aggregate to one endpoint.
    path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LatencyPercentile {
    P50,
    P95,
    P99,
    Max,
    Avg,
}

impl LatencyPercentile {
    fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "p50" | "50" | "median" => Some(Self::P50),
            "p95" | "95" => Some(Self::P95),
            "p99" | "99" => Some(Self::P99),
            "max" => Some(Self::Max),
            "avg" | "mean" | "average" => Some(Self::Avg),
            _ => None,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::P50 => "p50",
            Self::P95 => "p95",
            Self::P99 => "p99",
            Self::Max => "max",
            Self::Avg => "avg",
        }
    }

    fn value_from(&self, stats: &DeploymentLatencyStats) -> Option<f64> {
        match self {
            Self::P50 => stats.p50_ms,
            Self::P95 => stats.p95_ms,
            Self::P99 => stats.p99_ms,
            Self::Max => stats.max_ms,
            Self::Avg => stats.avg_ms,
        }
    }
}

fn parse_latency_config(
    config: &serde_json::Value,
) -> std::result::Result<LatencyThresholdConfig, String> {
    let deployment_id = config
        .get("deployment_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "config.deployment_id missing".to_string())
        .and_then(|s| {
            Uuid::parse_str(s).map_err(|e| format!("config.deployment_id invalid: {e}"))
        })?;
    let threshold_ms = config
        .get("threshold_ms")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "config.threshold_ms missing or non-numeric".to_string())?;
    if threshold_ms <= 0.0 || !threshold_ms.is_finite() {
        return Err(format!(
            "config.threshold_ms must be a positive finite number (got {threshold_ms})"
        ));
    }
    let percentile = match config.get("percentile").and_then(|v| v.as_str()) {
        Some(s) => LatencyPercentile::parse(s)
            .ok_or_else(|| format!("config.percentile '{s}' must be p50|p95|p99|max|avg"))?,
        // Default to p95 — the most common SLO percentile and matches
        // the migration's column naming convention.
        None => LatencyPercentile::P95,
    };
    let window_minutes = config
        .get("window_minutes")
        .and_then(|v| v.as_i64())
        .unwrap_or(DEFAULT_WINDOW_MINUTES)
        .clamp(1, MAX_WINDOW_MINUTES);
    let path = config
        .get("path")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    Ok(LatencyThresholdConfig {
        deployment_id,
        threshold_ms,
        percentile,
        window_minutes,
        path,
    })
}

async fn evaluate_latency_threshold(
    function: &FitnessFunctionDefinition,
    callbacks: &RegistryCallbacks,
    job: &RunJob,
    started: Instant,
) -> Result<JobOutcome> {
    let config = match parse_latency_config(&function.config) {
        Ok(c) => c,
        Err(reason) => {
            return errored_run(
                callbacks,
                job,
                started,
                2,
                format!("invalid latency_threshold config: {reason}"),
            )
            .await;
        }
    };

    let stats = match callbacks
        .fetch_deployment_latency_stats(
            config.deployment_id,
            config.window_minutes,
            config.path.as_deref(),
        )
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return errored_run(
                callbacks,
                job,
                started,
                2,
                format!("failed to fetch latency stats: {e}"),
            )
            .await;
        }
    };

    // No traffic in the window: report `unknown` (we don't have a
    // measurement, so passing or failing would both be lying). Returns
    // `Errored` rather than `Passed` so the cloud-side dispatcher
    // surfaces it as something the operator should see.
    if stats.count == 0 {
        callbacks
            .run_event(
                job.run_id,
                2,
                "log",
                serde_json::json!({
                    "level": "warn",
                    "message": format!(
                        "No traffic for deployment {} in the last {} minutes — cannot measure {}",
                        config.deployment_id, config.window_minutes, config.percentile.label(),
                    ),
                    "function_name": function.name,
                }),
            )
            .await?;
        let elapsed = started.elapsed();
        return Ok(JobOutcome {
            status: JobStatus::Errored,
            runner_seconds: (elapsed.as_secs_f64().ceil() as i32).max(1),
            summary: Some(serde_json::json!({
                "executor_phase": "real_latency_threshold",
                "tracking_task": 8,
                "function_id": function.id,
                "function_name": function.name,
                "kind": "latency_threshold",
                "deployment_id": config.deployment_id,
                "window_minutes": config.window_minutes,
                "path": config.path,
                "percentile": config.percentile.label(),
                "threshold_value": config.threshold_ms,
                "measured_value": serde_json::Value::Null,
                "count": 0,
                "reason": "no_traffic",
            })),
        });
    }

    let measured = match config.percentile.value_from(&stats) {
        Some(v) => v,
        None => {
            // count > 0 but the percentile came back NULL — shouldn't
            // happen with `percentile_cont` over a non-empty set, but
            // defend against future schema changes (e.g. `latency_ms`
            // becoming nullable).
            return errored_run(
                callbacks,
                job,
                started,
                2,
                format!(
                    "deployment {} has {} request(s) but {} percentile is NULL",
                    config.deployment_id,
                    stats.count,
                    config.percentile.label(),
                ),
            )
            .await;
        }
    };

    let pass = measured <= config.threshold_ms;
    let event_type = if pass { "fitness_pass" } else { "fitness_fail" };
    let reason = if pass {
        String::new()
    } else {
        format!(
            "{} latency = {:.1}ms > threshold {:.1}ms",
            config.percentile.label(),
            measured,
            config.threshold_ms,
        )
    };

    callbacks
        .run_event(
            job.run_id,
            2,
            event_type,
            serde_json::json!({
                "function_id": function.id,
                "function_name": function.name,
                "kind": "latency_threshold",
                "deployment_id": config.deployment_id,
                "percentile": config.percentile.label(),
                "measured_value": measured,
                "threshold_value": config.threshold_ms,
                "count": stats.count,
                "error_count": stats.error_count,
                "window_minutes": config.window_minutes,
                "path": config.path,
                "reason": reason,
            }),
        )
        .await?;

    let elapsed = started.elapsed();
    let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
    Ok(JobOutcome {
        status: if pass {
            JobStatus::Passed
        } else {
            JobStatus::Failed
        },
        runner_seconds: secs,
        summary: Some(serde_json::json!({
            "executor_phase": "real_latency_threshold",
            "tracking_task": 8,
            "function_id": function.id,
            "function_name": function.name,
            "kind": "latency_threshold",
            "deployment_id": config.deployment_id,
            "window_minutes": config.window_minutes,
            "path": config.path,
            "percentile": config.percentile.label(),
            // These two field names are the contract `mirror_kind_status`
            // reads to populate `fitness_evaluations.measured_value` /
            // `threshold_value`. Don't rename without updating both.
            "measured_value": measured,
            "threshold_value": config.threshold_ms,
            "count": stats.count,
            "error_count": stats.error_count,
            "wall_ms": elapsed.as_millis() as u64,
        })),
    })
}

// ─── error_rate evaluator ────────────────────────────────────────────

/// Resolved `error_rate` config. Same data source as
/// `latency_threshold` (the deployment's runtime_request_logs
/// aggregate), but the assertion is against `error_count / count`
/// rather than a percentile latency. `threshold_rate` is a fraction
/// in `[0.0, 1.0]` — values like `0.05` ("5% error rate ceiling")
/// are typical.
#[derive(Debug)]
struct ErrorRateConfig {
    deployment_id: Uuid,
    threshold_rate: f64,
    window_minutes: i64,
    /// Optional path filter — same semantics as latency_threshold.
    path: Option<String>,
}

fn parse_error_rate_config(
    config: &serde_json::Value,
) -> std::result::Result<ErrorRateConfig, String> {
    let deployment_id = config
        .get("deployment_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "config.deployment_id missing".to_string())
        .and_then(|s| {
            Uuid::parse_str(s).map_err(|e| format!("config.deployment_id invalid: {e}"))
        })?;
    let threshold_rate = config
        .get("threshold_rate")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "config.threshold_rate missing or non-numeric".to_string())?;
    if !threshold_rate.is_finite() || !(0.0..=1.0).contains(&threshold_rate) {
        return Err(format!(
            "config.threshold_rate must be a finite fraction in [0.0, 1.0] (got {threshold_rate})"
        ));
    }
    let window_minutes = config
        .get("window_minutes")
        .and_then(|v| v.as_i64())
        .unwrap_or(DEFAULT_WINDOW_MINUTES)
        .clamp(1, MAX_WINDOW_MINUTES);
    let path = config
        .get("path")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    Ok(ErrorRateConfig {
        deployment_id,
        threshold_rate,
        window_minutes,
        path,
    })
}

async fn evaluate_error_rate(
    function: &FitnessFunctionDefinition,
    callbacks: &RegistryCallbacks,
    job: &RunJob,
    started: Instant,
) -> Result<JobOutcome> {
    let config = match parse_error_rate_config(&function.config) {
        Ok(c) => c,
        Err(reason) => {
            return errored_run(
                callbacks,
                job,
                started,
                2,
                format!("invalid error_rate config: {reason}"),
            )
            .await;
        }
    };

    let stats = match callbacks
        .fetch_deployment_latency_stats(
            config.deployment_id,
            config.window_minutes,
            config.path.as_deref(),
        )
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return errored_run(
                callbacks,
                job,
                started,
                2,
                format!("failed to fetch deployment stats: {e}"),
            )
            .await;
        }
    };

    // Same no-traffic semantics as latency_threshold: we don't have a
    // measurement, so return `Errored` rather than implying the
    // assertion passed by silence.
    if stats.count == 0 {
        callbacks
            .run_event(
                job.run_id,
                2,
                "log",
                serde_json::json!({
                    "level": "warn",
                    "message": format!(
                        "No traffic for deployment {} in the last {} minutes — cannot measure error rate",
                        config.deployment_id, config.window_minutes,
                    ),
                    "function_name": function.name,
                }),
            )
            .await?;
        let elapsed = started.elapsed();
        return Ok(JobOutcome {
            status: JobStatus::Errored,
            runner_seconds: (elapsed.as_secs_f64().ceil() as i32).max(1),
            summary: Some(serde_json::json!({
                "executor_phase": "real_error_rate",
                "tracking_task": 8,
                "function_id": function.id,
                "function_name": function.name,
                "kind": "error_rate",
                "deployment_id": config.deployment_id,
                "window_minutes": config.window_minutes,
                "path": config.path,
                "threshold_value": config.threshold_rate,
                "measured_value": serde_json::Value::Null,
                "count": 0,
                "error_count": 0,
                "reason": "no_traffic",
            })),
        });
    }

    // Casts are safe — `stats.count > 0` (checked above) and i64 → f64
    // loses precision only above 2^53, while a single deployment
    // window holds at most a few hundred thousand requests in
    // practice.
    let measured = stats.error_count as f64 / stats.count as f64;
    let pass = measured <= config.threshold_rate;
    let event_type = if pass { "fitness_pass" } else { "fitness_fail" };
    let reason = if pass {
        String::new()
    } else {
        format!(
            "error rate = {:.2}% ({} / {}) > threshold {:.2}%",
            measured * 100.0,
            stats.error_count,
            stats.count,
            config.threshold_rate * 100.0,
        )
    };

    callbacks
        .run_event(
            job.run_id,
            2,
            event_type,
            serde_json::json!({
                "function_id": function.id,
                "function_name": function.name,
                "kind": "error_rate",
                "deployment_id": config.deployment_id,
                "measured_value": measured,
                "threshold_value": config.threshold_rate,
                "count": stats.count,
                "error_count": stats.error_count,
                "window_minutes": config.window_minutes,
                "path": config.path,
                "reason": reason,
            }),
        )
        .await?;

    let elapsed = started.elapsed();
    let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
    Ok(JobOutcome {
        status: if pass {
            JobStatus::Passed
        } else {
            JobStatus::Failed
        },
        runner_seconds: secs,
        summary: Some(serde_json::json!({
            "executor_phase": "real_error_rate",
            "tracking_task": 8,
            "function_id": function.id,
            "function_name": function.name,
            "kind": "error_rate",
            "deployment_id": config.deployment_id,
            "window_minutes": config.window_minutes,
            "path": config.path,
            // Same `measured_value` / `threshold_value` contract that
            // latency_threshold uses — `mirror_kind_status` reads these
            // names to populate `fitness_evaluations`.
            "measured_value": measured,
            "threshold_value": config.threshold_rate,
            "count": stats.count,
            "error_count": stats.error_count,
            "wall_ms": elapsed.as_millis() as u64,
        })),
    })
}

// ─── contract_stability evaluator ────────────────────────────────────

/// Resolved `contract_stability` config. Different shape from the
/// latency-based evaluators because it queries a different data
/// source (contract_diff_findings) keyed off `monitored_service_id`,
/// not `deployment_id`.
///
/// `max_breaking` defaults to 0 — the strictest reading of "stable".
/// `max_non_breaking` is opt-in: when omitted, non-breaking findings
/// don't fail the assertion, matching the typical "I only care about
/// breaking drift" check users want by default.
#[derive(Debug)]
struct ContractStabilityConfig {
    monitored_service_id: Uuid,
    max_breaking: i64,
    max_non_breaking: Option<i64>,
    window_minutes: i64,
}

/// 24h is the default window — contract diffs run on cron schedules
/// (typically per-deploy or daily), so a 1h window often misses signal.
const CONTRACT_STABILITY_DEFAULT_WINDOW: i64 = 1_440;
/// 1 week ceiling. Matches the registry handler's clamp.
const CONTRACT_STABILITY_MAX_WINDOW: i64 = 10_080;

fn parse_contract_stability_config(
    config: &serde_json::Value,
) -> std::result::Result<ContractStabilityConfig, String> {
    let monitored_service_id = config
        .get("monitored_service_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "config.monitored_service_id missing".to_string())
        .and_then(|s| {
            Uuid::parse_str(s).map_err(|e| format!("config.monitored_service_id invalid: {e}"))
        })?;
    let max_breaking = config.get("max_breaking").and_then(|v| v.as_i64()).unwrap_or(0);
    if max_breaking < 0 {
        return Err(format!(
            "config.max_breaking must be a non-negative integer (got {max_breaking})"
        ));
    }
    let max_non_breaking = match config.get("max_non_breaking") {
        Some(v) if v.is_null() => None,
        Some(v) => {
            let n = v
                .as_i64()
                .ok_or_else(|| "config.max_non_breaking must be an integer or null".to_string())?;
            if n < 0 {
                return Err(format!(
                    "config.max_non_breaking must be a non-negative integer (got {n})"
                ));
            }
            Some(n)
        }
        None => None,
    };
    let window_minutes = config
        .get("window_minutes")
        .and_then(|v| v.as_i64())
        .unwrap_or(CONTRACT_STABILITY_DEFAULT_WINDOW)
        .clamp(1, CONTRACT_STABILITY_MAX_WINDOW);
    Ok(ContractStabilityConfig {
        monitored_service_id,
        max_breaking,
        max_non_breaking,
        window_minutes,
    })
}

async fn evaluate_contract_stability(
    function: &FitnessFunctionDefinition,
    callbacks: &RegistryCallbacks,
    job: &RunJob,
    started: Instant,
) -> Result<JobOutcome> {
    let config = match parse_contract_stability_config(&function.config) {
        Ok(c) => c,
        Err(reason) => {
            return errored_run(
                callbacks,
                job,
                started,
                2,
                format!("invalid contract_stability config: {reason}"),
            )
            .await;
        }
    };

    let stats: MonitoredServiceContractStability = match callbacks
        .fetch_monitored_service_contract_stability(
            config.monitored_service_id,
            config.window_minutes,
        )
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return errored_run(
                callbacks,
                job,
                started,
                2,
                format!("failed to fetch contract stability stats: {e}"),
            )
            .await;
        }
    };

    // No diff runs in the window: report `unknown` rather than passing
    // by silence. A monitored service with no recent diffs probably
    // means the schedule is misconfigured — operator should know.
    if stats.run_count == 0 {
        callbacks
            .run_event(
                job.run_id,
                2,
                "log",
                serde_json::json!({
                    "level": "warn",
                    "message": format!(
                        "No contract diff runs for monitored service {} in the last {} minutes — \
                         cannot evaluate stability",
                        config.monitored_service_id, config.window_minutes,
                    ),
                    "function_name": function.name,
                }),
            )
            .await?;
        let elapsed = started.elapsed();
        return Ok(JobOutcome {
            status: JobStatus::Errored,
            runner_seconds: (elapsed.as_secs_f64().ceil() as i32).max(1),
            summary: Some(serde_json::json!({
                "executor_phase": "real_contract_stability",
                "tracking_task": 8,
                "function_id": function.id,
                "function_name": function.name,
                "kind": "contract_stability",
                "monitored_service_id": config.monitored_service_id,
                "window_minutes": config.window_minutes,
                "max_breaking": config.max_breaking,
                "measured_value": serde_json::Value::Null,
                "threshold_value": config.max_breaking,
                "run_count": 0,
                "reason": "no_diff_runs",
            })),
        });
    }

    let breaking_pass = stats.breaking_count <= config.max_breaking;
    let non_breaking_pass = match config.max_non_breaking {
        Some(cap) => stats.non_breaking_count <= cap,
        None => true,
    };
    let pass = breaking_pass && non_breaking_pass;
    let event_type = if pass { "fitness_pass" } else { "fitness_fail" };
    let reason = if pass {
        String::new()
    } else if !breaking_pass {
        format!(
            "{} breaking finding(s) > max_breaking {}",
            stats.breaking_count, config.max_breaking,
        )
    } else {
        // Only non-breaking ceiling exceeded.
        format!(
            "{} non-breaking finding(s) > max_non_breaking {}",
            stats.non_breaking_count,
            config.max_non_breaking.unwrap_or(0),
        )
    };

    callbacks
        .run_event(
            job.run_id,
            2,
            event_type,
            serde_json::json!({
                "function_id": function.id,
                "function_name": function.name,
                "kind": "contract_stability",
                "monitored_service_id": config.monitored_service_id,
                // Primary axis the assertion fires on. The non-breaking
                // count is reported alongside but the threshold for it
                // is opt-in.
                "measured_value": stats.breaking_count,
                "threshold_value": config.max_breaking,
                "breaking_count": stats.breaking_count,
                "non_breaking_count": stats.non_breaking_count,
                "cosmetic_count": stats.cosmetic_count,
                "max_non_breaking": config.max_non_breaking,
                "run_count": stats.run_count,
                "latest_run_at": stats.latest_run_at,
                "window_minutes": config.window_minutes,
                "reason": reason,
            }),
        )
        .await?;

    let elapsed = started.elapsed();
    let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
    Ok(JobOutcome {
        status: if pass {
            JobStatus::Passed
        } else {
            JobStatus::Failed
        },
        runner_seconds: secs,
        summary: Some(serde_json::json!({
            "executor_phase": "real_contract_stability",
            "tracking_task": 8,
            "function_id": function.id,
            "function_name": function.name,
            "kind": "contract_stability",
            "monitored_service_id": config.monitored_service_id,
            "window_minutes": config.window_minutes,
            // Same `measured_value` / `threshold_value` contract used
            // by the other evaluators so mirror_kind_status can
            // populate `fitness_evaluations` without per-kind logic.
            // Cast to f64 because the column is DOUBLE PRECISION.
            "measured_value": stats.breaking_count as f64,
            "threshold_value": config.max_breaking as f64,
            "breaking_count": stats.breaking_count,
            "non_breaking_count": stats.non_breaking_count,
            "cosmetic_count": stats.cosmetic_count,
            "max_breaking": config.max_breaking,
            "max_non_breaking": config.max_non_breaking,
            "run_count": stats.run_count,
            "latest_run_at": stats.latest_run_at,
            "wall_ms": elapsed.as_millis() as u64,
        })),
    })
}

// ─── Helpers ─────────────────────────────────────────────────────────

/// Emit a single error log + return an `Errored` outcome. Used for
/// pre-flight failures (bad config, fetch errors) that prevent any
/// real evaluation. Includes `function_name` in the summary when
/// available so the incidents UI surfaces it.
async fn errored_run(
    callbacks: &RegistryCallbacks,
    job: &RunJob,
    started: Instant,
    seq: u32,
    message: String,
) -> Result<JobOutcome> {
    callbacks
        .run_event(
            job.run_id,
            seq,
            "log",
            serde_json::json!({
                "level": "error",
                "message": message,
            }),
        )
        .await?;

    let elapsed = started.elapsed();
    let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
    Ok(JobOutcome {
        status: JobStatus::Errored,
        runner_seconds: secs,
        summary: Some(serde_json::json!({
            "executor_phase": "errored_pre_flight",
            "tracking_task": 8,
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(json: serde_json::Value) -> std::result::Result<LatencyThresholdConfig, String> {
        parse_latency_config(&json)
    }

    #[test]
    fn parse_latency_config_minimal() {
        let dep = Uuid::new_v4();
        let c = cfg(serde_json::json!({
            "deployment_id": dep.to_string(),
            "threshold_ms": 500,
        }))
        .unwrap();
        assert_eq!(c.deployment_id, dep);
        assert_eq!(c.threshold_ms, 500.0);
        // Default percentile is p95, default window 60m.
        assert_eq!(c.percentile, LatencyPercentile::P95);
        assert_eq!(c.window_minutes, DEFAULT_WINDOW_MINUTES);
        assert_eq!(c.path, None);
    }

    #[test]
    fn parse_latency_config_full() {
        let dep = Uuid::new_v4();
        let c = cfg(serde_json::json!({
            "deployment_id": dep.to_string(),
            "threshold_ms": 250.5,
            "percentile": "p99",
            "window_minutes": 120,
            "path": "/api/checkout",
        }))
        .unwrap();
        assert_eq!(c.threshold_ms, 250.5);
        assert_eq!(c.percentile, LatencyPercentile::P99);
        assert_eq!(c.window_minutes, 120);
        assert_eq!(c.path.as_deref(), Some("/api/checkout"));
    }

    #[test]
    fn parse_latency_config_clamps_window() {
        let dep = Uuid::new_v4();
        // window above the cap clamps to MAX_WINDOW_MINUTES.
        let c = cfg(serde_json::json!({
            "deployment_id": dep.to_string(),
            "threshold_ms": 1,
            "window_minutes": 99_999,
        }))
        .unwrap();
        assert_eq!(c.window_minutes, MAX_WINDOW_MINUTES);
        // window below 1 clamps to 1.
        let c = cfg(serde_json::json!({
            "deployment_id": dep.to_string(),
            "threshold_ms": 1,
            "window_minutes": 0,
        }))
        .unwrap();
        assert_eq!(c.window_minutes, 1);
    }

    #[test]
    fn parse_latency_config_drops_empty_path() {
        let dep = Uuid::new_v4();
        let c = cfg(serde_json::json!({
            "deployment_id": dep.to_string(),
            "threshold_ms": 1,
            "path": "",
        }))
        .unwrap();
        assert_eq!(c.path, None);
    }

    #[test]
    fn parse_latency_config_rejects_missing_deployment() {
        let err = cfg(serde_json::json!({
            "threshold_ms": 100,
        }))
        .unwrap_err();
        assert!(err.contains("deployment_id"), "got: {err}");
    }

    #[test]
    fn parse_latency_config_rejects_invalid_deployment_uuid() {
        let err = cfg(serde_json::json!({
            "deployment_id": "not-a-uuid",
            "threshold_ms": 100,
        }))
        .unwrap_err();
        assert!(err.contains("deployment_id"), "got: {err}");
    }

    #[test]
    fn parse_latency_config_rejects_zero_threshold() {
        let dep = Uuid::new_v4();
        let err = cfg(serde_json::json!({
            "deployment_id": dep.to_string(),
            "threshold_ms": 0,
        }))
        .unwrap_err();
        assert!(err.contains("positive"), "got: {err}");
    }

    #[test]
    fn parse_latency_config_rejects_negative_threshold() {
        let dep = Uuid::new_v4();
        let err = cfg(serde_json::json!({
            "deployment_id": dep.to_string(),
            "threshold_ms": -10,
        }))
        .unwrap_err();
        assert!(err.contains("positive"), "got: {err}");
    }

    #[test]
    fn parse_latency_config_rejects_unknown_percentile() {
        let dep = Uuid::new_v4();
        let err = cfg(serde_json::json!({
            "deployment_id": dep.to_string(),
            "threshold_ms": 1,
            "percentile": "p42",
        }))
        .unwrap_err();
        assert!(err.contains("p42"), "got: {err}");
    }

    #[test]
    fn percentile_parse_accepts_synonyms() {
        assert_eq!(LatencyPercentile::parse("p50"), Some(LatencyPercentile::P50));
        assert_eq!(LatencyPercentile::parse("median"), Some(LatencyPercentile::P50));
        assert_eq!(LatencyPercentile::parse("MEDIAN"), Some(LatencyPercentile::P50));
        assert_eq!(LatencyPercentile::parse("avg"), Some(LatencyPercentile::Avg));
        assert_eq!(LatencyPercentile::parse("MEAN"), Some(LatencyPercentile::Avg));
        assert_eq!(LatencyPercentile::parse("Average"), Some(LatencyPercentile::Avg));
        assert_eq!(LatencyPercentile::parse("Max"), Some(LatencyPercentile::Max));
        assert_eq!(LatencyPercentile::parse("99"), Some(LatencyPercentile::P99));
        assert_eq!(LatencyPercentile::parse("garbage"), None);
    }

    #[test]
    fn percentile_value_from_picks_right_field() {
        let stats = DeploymentLatencyStats {
            count: 100,
            error_count: 1,
            p50_ms: Some(10.0),
            p95_ms: Some(50.0),
            p99_ms: Some(80.0),
            max_ms: Some(200.0),
            avg_ms: Some(15.0),
        };
        assert_eq!(LatencyPercentile::P50.value_from(&stats), Some(10.0));
        assert_eq!(LatencyPercentile::P95.value_from(&stats), Some(50.0));
        assert_eq!(LatencyPercentile::P99.value_from(&stats), Some(80.0));
        assert_eq!(LatencyPercentile::Max.value_from(&stats), Some(200.0));
        assert_eq!(LatencyPercentile::Avg.value_from(&stats), Some(15.0));
    }

    fn err_cfg(json: serde_json::Value) -> std::result::Result<ErrorRateConfig, String> {
        parse_error_rate_config(&json)
    }

    #[test]
    fn parse_error_rate_config_minimal() {
        let dep = Uuid::new_v4();
        let c = err_cfg(serde_json::json!({
            "deployment_id": dep.to_string(),
            "threshold_rate": 0.05,
        }))
        .unwrap();
        assert_eq!(c.deployment_id, dep);
        assert_eq!(c.threshold_rate, 0.05);
        assert_eq!(c.window_minutes, DEFAULT_WINDOW_MINUTES);
        assert_eq!(c.path, None);
    }

    #[test]
    fn parse_error_rate_config_full() {
        let dep = Uuid::new_v4();
        let c = err_cfg(serde_json::json!({
            "deployment_id": dep.to_string(),
            "threshold_rate": 0.01,
            "window_minutes": 30,
            "path": "/checkout",
        }))
        .unwrap();
        assert_eq!(c.threshold_rate, 0.01);
        assert_eq!(c.window_minutes, 30);
        assert_eq!(c.path.as_deref(), Some("/checkout"));
    }

    #[test]
    fn parse_error_rate_config_accepts_zero_and_one() {
        // 0.0 means "no errors allowed", 1.0 means "no ceiling at all".
        // Both edge cases should parse cleanly — the assertion logic
        // handles them naturally.
        let dep = Uuid::new_v4();
        assert!(err_cfg(serde_json::json!({
            "deployment_id": dep.to_string(),
            "threshold_rate": 0.0,
        }))
        .is_ok());
        assert!(err_cfg(serde_json::json!({
            "deployment_id": dep.to_string(),
            "threshold_rate": 1.0,
        }))
        .is_ok());
    }

    #[test]
    fn parse_error_rate_config_rejects_out_of_range() {
        let dep = Uuid::new_v4();
        let err = err_cfg(serde_json::json!({
            "deployment_id": dep.to_string(),
            "threshold_rate": 1.5,
        }))
        .unwrap_err();
        assert!(err.contains("[0.0, 1.0]"), "got: {err}");

        let err = err_cfg(serde_json::json!({
            "deployment_id": dep.to_string(),
            "threshold_rate": -0.1,
        }))
        .unwrap_err();
        assert!(err.contains("[0.0, 1.0]"), "got: {err}");
    }

    #[test]
    fn parse_error_rate_config_rejects_missing_threshold() {
        let dep = Uuid::new_v4();
        let err = err_cfg(serde_json::json!({
            "deployment_id": dep.to_string(),
        }))
        .unwrap_err();
        assert!(err.contains("threshold_rate"), "got: {err}");
    }

    #[test]
    fn parse_error_rate_config_clamps_window() {
        let dep = Uuid::new_v4();
        let c = err_cfg(serde_json::json!({
            "deployment_id": dep.to_string(),
            "threshold_rate": 0.05,
            "window_minutes": 99_999,
        }))
        .unwrap();
        assert_eq!(c.window_minutes, MAX_WINDOW_MINUTES);
    }

    #[test]
    fn parse_error_rate_config_rejects_missing_deployment() {
        let err = err_cfg(serde_json::json!({
            "threshold_rate": 0.05,
        }))
        .unwrap_err();
        assert!(err.contains("deployment_id"), "got: {err}");
    }

    fn cs_cfg(json: serde_json::Value) -> std::result::Result<ContractStabilityConfig, String> {
        parse_contract_stability_config(&json)
    }

    #[test]
    fn parse_contract_stability_config_minimal() {
        let svc = Uuid::new_v4();
        let c = cs_cfg(serde_json::json!({
            "monitored_service_id": svc.to_string(),
        }))
        .unwrap();
        assert_eq!(c.monitored_service_id, svc);
        assert_eq!(c.max_breaking, 0);
        assert_eq!(c.max_non_breaking, None);
        assert_eq!(c.window_minutes, CONTRACT_STABILITY_DEFAULT_WINDOW);
    }

    #[test]
    fn parse_contract_stability_config_full() {
        let svc = Uuid::new_v4();
        let c = cs_cfg(serde_json::json!({
            "monitored_service_id": svc.to_string(),
            "max_breaking": 2,
            "max_non_breaking": 10,
            "window_minutes": 60,
        }))
        .unwrap();
        assert_eq!(c.max_breaking, 2);
        assert_eq!(c.max_non_breaking, Some(10));
        assert_eq!(c.window_minutes, 60);
    }

    #[test]
    fn parse_contract_stability_config_explicit_null_non_breaking() {
        let svc = Uuid::new_v4();
        // Explicit null is the same as omitting the key — no ceiling
        // on non-breaking findings.
        let c = cs_cfg(serde_json::json!({
            "monitored_service_id": svc.to_string(),
            "max_non_breaking": serde_json::Value::Null,
        }))
        .unwrap();
        assert_eq!(c.max_non_breaking, None);
    }

    #[test]
    fn parse_contract_stability_config_clamps_window() {
        let svc = Uuid::new_v4();
        let c = cs_cfg(serde_json::json!({
            "monitored_service_id": svc.to_string(),
            "window_minutes": 99_999,
        }))
        .unwrap();
        assert_eq!(c.window_minutes, CONTRACT_STABILITY_MAX_WINDOW);
    }

    #[test]
    fn parse_contract_stability_config_rejects_missing_service() {
        let err = cs_cfg(serde_json::json!({})).unwrap_err();
        assert!(err.contains("monitored_service_id"), "got: {err}");
    }

    #[test]
    fn parse_contract_stability_config_rejects_invalid_uuid() {
        let err = cs_cfg(serde_json::json!({
            "monitored_service_id": "not-a-uuid",
        }))
        .unwrap_err();
        assert!(err.contains("monitored_service_id"), "got: {err}");
    }

    #[test]
    fn parse_contract_stability_config_rejects_negative_max_breaking() {
        let svc = Uuid::new_v4();
        let err = cs_cfg(serde_json::json!({
            "monitored_service_id": svc.to_string(),
            "max_breaking": -1,
        }))
        .unwrap_err();
        assert!(err.contains("max_breaking"), "got: {err}");
    }

    #[test]
    fn parse_contract_stability_config_rejects_negative_max_non_breaking() {
        let svc = Uuid::new_v4();
        let err = cs_cfg(serde_json::json!({
            "monitored_service_id": svc.to_string(),
            "max_non_breaking": -5,
        }))
        .unwrap_err();
        assert!(err.contains("max_non_breaking"), "got: {err}");
    }

    #[test]
    fn parse_contract_stability_config_rejects_non_integer_max_non_breaking() {
        let svc = Uuid::new_v4();
        let err = cs_cfg(serde_json::json!({
            "monitored_service_id": svc.to_string(),
            "max_non_breaking": "many",
        }))
        .unwrap_err();
        assert!(err.contains("max_non_breaking"), "got: {err}");
    }
}
