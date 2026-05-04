//! Real fitness-function evaluation for `kind = "fitness_evaluation"`
//! test runs (#355).
//!
//! Fitness functions are declarative checks against the workspace's
//! observability data. Today we evaluate two kinds end-to-end against
//! real `runtime_captures`:
//!
//!   - `latency_threshold` — reads p95 (or configured percentile) over
//!     a window and fails when it exceeds the configured limit.
//!   - `error_rate` — reads the 4xx/5xx fraction of total requests
//!     over the window and fails when it exceeds the limit.
//!
//! `contract_stability` and `custom_query` stay in synthetic mode
//! until follow-up slices land — the executor logs that explicitly so
//! UIs can surface the gap. Failures (real or synthetic) raise a
//! workspace-scoped incident with `source = "fitness"` so the
//! incident dispatcher can route them through the same channels as
//! the rest of #3.

use std::time::Instant;

use serde::Deserialize;
use uuid::Uuid;

use crate::callbacks::{RaiseIncidentBody, RegistryCallbacks};
use crate::error::Result;
use crate::executors::{JobOutcome, JobStatus, RunJob};

#[derive(Debug, Default, Deserialize)]
struct FitnessConfig {
    /// Window in minutes the metric is evaluated over. Default 60.
    #[serde(default)]
    window_minutes: Option<i64>,
    /// Optional path prefix filter (e.g. `/api/v1/users`). Default: any.
    #[serde(default)]
    path_prefix: Option<String>,
    /// `latency_threshold`: which percentile to evaluate (50 / 95 / 99).
    /// Default 95.
    #[serde(default)]
    percentile: Option<u8>,
    /// `latency_threshold`: max allowed value for the chosen percentile.
    #[serde(default, alias = "max_latency_ms")]
    threshold_ms: Option<f64>,
    /// `error_rate`: max allowed fraction (0.0..=1.0). 0.05 = 5%.
    #[serde(default, alias = "max_error_rate")]
    error_rate: Option<f64>,
    /// `error_rate`: which class counts as an error — `5xx` (default)
    /// or `4xx_5xx`.
    #[serde(default)]
    counts: Option<String>,
    /// Severity for the raised incident if the function fails.
    #[serde(default)]
    severity: Option<String>,
}

/// Run a fitness evaluation and report passed/failed back through
/// `callbacks`. On failure also raises a deduped fitness-source
/// incident in the workspace.
pub async fn run_real_fitness(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    workspace_id: Uuid,
    kind: &str,
    config: &serde_json::Value,
) -> Result<JobOutcome> {
    callbacks.run_started(job.run_id).await?;

    let cfg: FitnessConfig = serde_json::from_value(config.clone()).unwrap_or_default();
    let window = cfg.window_minutes.unwrap_or(60).clamp(1, 60 * 24 * 7);
    let path_prefix = cfg.path_prefix.clone().unwrap_or_default();
    let fitness_name = job
        .payload
        .get("fitness_name")
        .and_then(|v| v.as_str())
        .unwrap_or("(unnamed)")
        .to_string();
    let fitness_id = job
        .payload
        .get("fitness_function_id")
        .and_then(|v| v.as_str())
        .map(String::from);

    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!(
                    "Fitness eval: kind='{kind}', name='{fitness_name}', window={}m, path_prefix='{}'",
                    window, path_prefix,
                ),
                "tracking_task": 355,
            }),
        )
        .await?;

    // Pull the same stats payload regardless of kind — it's one
    // network call per evaluation, and the `fetch_workspace_runtime_stats`
    // endpoint already aggregates lazily.
    let stats = match callbacks
        .fetch_workspace_runtime_stats(
            workspace_id,
            window,
            if path_prefix.is_empty() {
                None
            } else {
                Some(&path_prefix)
            },
        )
        .await
    {
        Ok(s) => s,
        Err(e) => {
            callbacks
                .run_event(
                    job.run_id,
                    2,
                    "log",
                    serde_json::json!({
                        "level": "error",
                        "message": format!("runtime-stats fetch failed: {e}"),
                    }),
                )
                .await?;
            return Ok(JobOutcome {
                status: JobStatus::Errored,
                runner_seconds: started.elapsed().as_secs_f64().ceil() as i32,
                summary: Some(serde_json::json!({
                    "executor_phase": "real",
                    "tracking_task": 355,
                    "kind": "fitness_evaluation",
                    "error": format!("runtime-stats fetch failed: {e}"),
                })),
            });
        }
    };

    let evaluation = match kind {
        "latency_threshold" => evaluate_latency(&cfg, &stats),
        "error_rate" => evaluate_error_rate(&cfg, &stats),
        "contract_stability" | "custom_query" => Evaluation::Synthetic {
            reason: format!(
                "kind='{kind}' has no real evaluator yet — passing synthetically. \
                 Follow-up issue tracks the real impl."
            ),
        },
        other => Evaluation::Errored {
            reason: format!("Unknown fitness kind: {other}"),
        },
    };

    callbacks
        .run_event(
            job.run_id,
            2,
            "log",
            serde_json::json!({
                "level": "info",
                "message": "runtime stats fetched",
                "stats": {
                    "total_requests": stats.total_requests,
                    "p50_ms": stats.p50_ms,
                    "p95_ms": stats.p95_ms,
                    "p99_ms": stats.p99_ms,
                    "server_errors": stats.server_errors,
                    "client_errors": stats.client_errors,
                },
            }),
        )
        .await?;

    let elapsed = started.elapsed();
    let runner_seconds = (elapsed.as_secs_f64().ceil() as i32).max(1);

    let (status, summary_extras, failure_msg) = match &evaluation {
        Evaluation::Passed {
            observed,
            threshold,
        } => {
            callbacks
                .run_event(
                    job.run_id,
                    3,
                    "fitness_pass",
                    serde_json::json!({
                        "fitness_name": fitness_name,
                        "kind": kind,
                        "observed": observed,
                        "threshold": threshold,
                    }),
                )
                .await?;
            (
                JobStatus::Passed,
                serde_json::json!({
                    "result": "passed",
                    "observed": observed,
                    "threshold": threshold,
                }),
                None,
            )
        }
        Evaluation::Failed {
            observed,
            threshold,
            description,
        } => {
            callbacks
                .run_event(
                    job.run_id,
                    3,
                    "fitness_fail",
                    serde_json::json!({
                        "fitness_name": fitness_name,
                        "kind": kind,
                        "observed": observed,
                        "threshold": threshold,
                        "description": description,
                    }),
                )
                .await?;
            (
                JobStatus::Failed,
                serde_json::json!({
                    "result": "failed",
                    "observed": observed,
                    "threshold": threshold,
                    "description": description,
                }),
                Some(description.clone()),
            )
        }
        Evaluation::Synthetic { reason } => {
            callbacks
                .run_event(
                    job.run_id,
                    3,
                    "log",
                    serde_json::json!({
                        "level": "warn",
                        "message": reason,
                        "synthetic": true,
                    }),
                )
                .await?;
            (
                JobStatus::Passed,
                serde_json::json!({
                    "result": "synthetic_pass",
                    "reason": reason,
                }),
                None,
            )
        }
        Evaluation::Errored { reason } => {
            callbacks
                .run_event(
                    job.run_id,
                    3,
                    "log",
                    serde_json::json!({
                        "level": "error",
                        "message": reason,
                    }),
                )
                .await?;
            (
                JobStatus::Errored,
                serde_json::json!({
                    "result": "errored",
                    "reason": reason,
                }),
                None,
            )
        }
    };

    if let (Some(msg), Some(id)) = (failure_msg.as_deref(), fitness_id.as_deref()) {
        let dedupe_key = format!("fitness:{}", id);
        let severity = cfg.severity.as_deref().unwrap_or("major");
        let title = format!("Fitness function '{fitness_name}' failed");
        if let Err(e) = callbacks
            .raise_incident(RaiseIncidentBody {
                workspace_id,
                source: "fitness",
                source_ref: Some(id),
                dedupe_key: &dedupe_key,
                severity,
                title: &title,
                description: Some(msg),
            })
            .await
        {
            // Don't fail the run because incident dispatch failed — log
            // it and let the user see the run-level fitness_fail event.
            callbacks
                .run_event(
                    job.run_id,
                    4,
                    "log",
                    serde_json::json!({
                        "level": "warn",
                        "message": format!("incident raise failed: {e}"),
                    }),
                )
                .await?;
        }
    }

    Ok(JobOutcome {
        status,
        runner_seconds,
        summary: Some(serde_json::json!({
            "executor_phase": match &evaluation {
                Evaluation::Synthetic { .. } => "synthetic",
                _ => "real",
            },
            "tracking_task": 355,
            "kind": "fitness_evaluation",
            "fitness_kind": kind,
            "fitness_name": fitness_name,
            "wall_ms": elapsed.as_millis() as u64,
            "evaluation": summary_extras,
        })),
    })
}

enum Evaluation {
    Passed {
        observed: serde_json::Value,
        threshold: serde_json::Value,
    },
    Failed {
        observed: serde_json::Value,
        threshold: serde_json::Value,
        description: String,
    },
    Synthetic {
        reason: String,
    },
    Errored {
        reason: String,
    },
}

fn evaluate_latency(
    cfg: &FitnessConfig,
    stats: &crate::callbacks::WorkspaceRuntimeStats,
) -> Evaluation {
    let percentile = cfg.percentile.unwrap_or(95);
    let observed_ms = match percentile {
        50 => stats.p50_ms,
        99 => stats.p99_ms,
        _ => stats.p95_ms,
    };
    let threshold_ms = match cfg.threshold_ms {
        Some(t) => t,
        None => {
            return Evaluation::Errored {
                reason: "latency_threshold config missing 'threshold_ms'".into(),
            };
        }
    };
    if stats.total_requests == 0 {
        return Evaluation::Synthetic {
            reason: format!(
                "no traffic in the last {} minutes — nothing to evaluate",
                cfg.window_minutes.unwrap_or(60),
            ),
        };
    }
    if observed_ms > threshold_ms {
        Evaluation::Failed {
            observed: serde_json::json!({ "p": percentile, "ms": observed_ms }),
            threshold: serde_json::json!({ "ms": threshold_ms }),
            description: format!(
                "p{percentile} latency {observed_ms:.1}ms > threshold {threshold_ms}ms"
            ),
        }
    } else {
        Evaluation::Passed {
            observed: serde_json::json!({ "p": percentile, "ms": observed_ms }),
            threshold: serde_json::json!({ "ms": threshold_ms }),
        }
    }
}

fn evaluate_error_rate(
    cfg: &FitnessConfig,
    stats: &crate::callbacks::WorkspaceRuntimeStats,
) -> Evaluation {
    let max_rate = match cfg.error_rate {
        Some(r) => r.clamp(0.0, 1.0),
        None => {
            return Evaluation::Errored {
                reason: "error_rate config missing 'error_rate'".into(),
            };
        }
    };
    if stats.total_requests == 0 {
        return Evaluation::Synthetic {
            reason: format!(
                "no traffic in the last {} minutes — nothing to evaluate",
                cfg.window_minutes.unwrap_or(60),
            ),
        };
    }
    let counts = cfg.counts.as_deref().unwrap_or("5xx");
    let error_count = match counts {
        "4xx_5xx" => stats.server_errors + stats.client_errors,
        _ => stats.server_errors,
    };
    let observed_rate = (error_count as f64) / (stats.total_requests as f64);
    if observed_rate > max_rate {
        Evaluation::Failed {
            observed: serde_json::json!({
                "rate": observed_rate,
                "errors": error_count,
                "total": stats.total_requests,
                "counts": counts,
            }),
            threshold: serde_json::json!({ "rate": max_rate }),
            description: format!(
                "{counts} error rate {:.4} ({error_count}/{}) > threshold {max_rate}",
                observed_rate, stats.total_requests,
            ),
        }
    } else {
        Evaluation::Passed {
            observed: serde_json::json!({
                "rate": observed_rate,
                "errors": error_count,
                "total": stats.total_requests,
                "counts": counts,
            }),
            threshold: serde_json::json!({ "rate": max_rate }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::callbacks::WorkspaceRuntimeStats;

    fn stats(
        total: i64,
        p95: f64,
        server_errors: i64,
        client_errors: i64,
    ) -> WorkspaceRuntimeStats {
        WorkspaceRuntimeStats {
            window_minutes: 60,
            path_prefix: String::new(),
            total_requests: total,
            p50_ms: p95 / 2.0,
            p95_ms: p95,
            p99_ms: p95 * 1.2,
            server_errors,
            client_errors,
        }
    }

    #[test]
    fn latency_passes_when_under_threshold() {
        let cfg = FitnessConfig {
            threshold_ms: Some(500.0),
            ..Default::default()
        };
        let s = stats(100, 200.0, 0, 0);
        match evaluate_latency(&cfg, &s) {
            Evaluation::Passed { .. } => {}
            other => panic!(
                "expected Passed, got {other:?}",
                other = match other {
                    Evaluation::Failed { description, .. } => description,
                    Evaluation::Synthetic { reason } => reason,
                    Evaluation::Errored { reason } => reason,
                    _ => "unreachable".into(),
                }
            ),
        }
    }

    #[test]
    fn latency_fails_when_over_threshold() {
        let cfg = FitnessConfig {
            threshold_ms: Some(100.0),
            ..Default::default()
        };
        let s = stats(100, 250.0, 0, 0);
        assert!(matches!(evaluate_latency(&cfg, &s), Evaluation::Failed { .. }));
    }

    #[test]
    fn latency_with_no_traffic_is_synthetic() {
        let cfg = FitnessConfig {
            threshold_ms: Some(100.0),
            ..Default::default()
        };
        let s = stats(0, 0.0, 0, 0);
        assert!(matches!(evaluate_latency(&cfg, &s), Evaluation::Synthetic { .. }));
    }

    #[test]
    fn error_rate_5xx_only_passes() {
        let cfg = FitnessConfig {
            error_rate: Some(0.05),
            ..Default::default()
        };
        // 2/100 5xx = 2% < 5%
        let s = stats(100, 50.0, 2, 0);
        assert!(matches!(evaluate_error_rate(&cfg, &s), Evaluation::Passed { .. }));
    }

    #[test]
    fn error_rate_5xx_only_fails() {
        let cfg = FitnessConfig {
            error_rate: Some(0.05),
            ..Default::default()
        };
        let s = stats(100, 50.0, 10, 0);
        assert!(matches!(evaluate_error_rate(&cfg, &s), Evaluation::Failed { .. }));
    }

    #[test]
    fn error_rate_includes_4xx_when_configured() {
        let cfg = FitnessConfig {
            error_rate: Some(0.05),
            counts: Some("4xx_5xx".into()),
            ..Default::default()
        };
        // 6/100 (4xx+5xx) = 6% > 5%
        let s = stats(100, 50.0, 1, 5);
        assert!(matches!(evaluate_error_rate(&cfg, &s), Evaluation::Failed { .. }));
    }
}
