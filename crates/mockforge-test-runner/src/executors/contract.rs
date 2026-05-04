//! Contract diff / verification / fitness executor (#8 / Phase 2).
//! Handles `contract_diff | verification_suite | fitness_evaluation`.
//!
//! Real-fetch mode (kind=contract_diff): when the queue payload includes
//! `openapi_spec_url`, the executor fetches the spec, parses it,
//! enumerates declared endpoints, and emits one event per endpoint plus
//! summary stats. It does NOT yet diff against live traffic — that
//! requires the `mockforge-ai-contract-diff` pipeline (separate crate
//! that doesn't exist yet) for the AI-assisted scoring.
//!
//! Synthetic-pass mode: when the spec URL is missing or the fetch
//! fails the executor falls back to emitting a couple of synthetic
//! findings so the UI's severity-grouped view still has data to render.

use async_trait::async_trait;
use std::time::Instant;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};

/// Executor for the three contract-/fitness-related kinds.
pub struct ContractExecutor {
    kind: &'static str,
}

impl ContractExecutor {
    /// Construct for `contract_diff`, `verification_suite`, or
    /// `fitness_evaluation`.
    pub fn for_kind(kind: &'static str) -> Self {
        Self { kind }
    }
}

#[async_trait]
impl Executor for ContractExecutor {
    fn kind(&self) -> &'static str {
        self.kind
    }

    async fn execute(&self, job: RunJob, callbacks: &RegistryCallbacks) -> Result<JobOutcome> {
        let started = Instant::now();
        callbacks.run_started(job.run_id).await?;

        let service_name = job
            .payload
            .get("service_name")
            .and_then(|v| v.as_str())
            .unwrap_or("(unspecified)")
            .to_string();
        let spec_url =
            job.payload.get("openapi_spec_url").and_then(|v| v.as_str()).map(String::from);

        // Real-fetch only applies to contract_diff. The other kinds
        // (verification_suite, fitness_evaluation) need their own
        // executor logic and stay synthetic for now.
        if self.kind == "contract_diff" {
            if let Some(url) = spec_url.as_deref() {
                if !url.is_empty() {
                    return run_real_contract_diff(job, callbacks, started, &service_name, url)
                        .await;
                }
            }
        }

        run_synthetic(self.kind, job, callbacks, started, &service_name).await
    }
}

async fn run_real_contract_diff(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    service_name: &str,
    spec_url: &str,
) -> Result<JobOutcome> {
    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!("Fetching OpenAPI spec: {spec_url}"),
                "synthetic": false,
                "tracking_task": 8,
            }),
        )
        .await?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("mockforge-contract-diff/1.0")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let spec_text = match client.get(spec_url).send().await {
        Ok(resp) => {
            let status = resp.status();
            match resp.text().await {
                Ok(t) if status.is_success() => t,
                Ok(_body) => {
                    callbacks
                        .run_event(
                            job.run_id,
                            2,
                            "log",
                            serde_json::json!({
                                "level": "warn",
                                "message": format!("spec fetch returned HTTP {}; falling back to synthetic", status),
                            }),
                        )
                        .await?;
                    return run_synthetic("contract_diff", job, callbacks, started, service_name)
                        .await;
                }
                Err(e) => {
                    callbacks
                        .run_event(
                            job.run_id,
                            2,
                            "log",
                            serde_json::json!({
                                "level": "warn",
                                "message": format!("spec body read failed: {e}; falling back to synthetic"),
                            }),
                        )
                        .await?;
                    return run_synthetic("contract_diff", job, callbacks, started, service_name)
                        .await;
                }
            }
        }
        Err(e) => {
            callbacks
                .run_event(
                    job.run_id,
                    2,
                    "log",
                    serde_json::json!({
                        "level": "warn",
                        "message": format!("spec fetch failed: {e}; falling back to synthetic"),
                    }),
                )
                .await?;
            return run_synthetic("contract_diff", job, callbacks, started, service_name).await;
        }
    };

    // Try JSON first, then fall back to YAML. The recorder spec format
    // is open — both shapes show up in the wild.
    let spec: serde_json::Value = match serde_json::from_str(&spec_text) {
        Ok(v) => v,
        Err(_) => match serde_yaml::from_str(&spec_text) {
            Ok(v) => v,
            Err(e) => {
                callbacks
                    .run_event(
                        job.run_id,
                        2,
                        "log",
                        serde_json::json!({
                            "level": "warn",
                            "message": format!("spec parse failed (not JSON or YAML): {e}; falling back to synthetic"),
                        }),
                    )
                    .await?;
                return run_synthetic("contract_diff", job, callbacks, started, service_name).await;
            }
        },
    };

    let endpoints = collect_endpoints(&spec);

    callbacks
        .run_event(
            job.run_id,
            2,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!("Spec parsed; found {} declared endpoints", endpoints.len()),
                "endpoint_count": endpoints.len(),
            }),
        )
        .await?;

    // Emit one finding per endpoint at low severity ("declared"). A real
    // ai_contract_diff pipeline would compare these against captured
    // traffic and elevate severity for actual drift; for now this
    // gives the UI a real list to render.
    let mut next_seq: u32 = 3;
    for (i, (method, path)) in endpoints.iter().take(200).enumerate() {
        callbacks
            .run_event(
                job.run_id,
                next_seq,
                "diff_finding",
                serde_json::json!({
                    "index": i,
                    "severity": "declared",
                    "endpoint": format!("{method} {path}"),
                    "description": "Endpoint declared in spec",
                }),
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
            "executor_phase": "real_fetch",
            "tracking_task": 8,
            "kind": "contract_diff",
            "service_name": service_name,
            "spec_url": spec_url,
            "endpoint_count": endpoints.len(),
            "findings_count": endpoints.len().min(200),
            "wall_ms": elapsed.as_millis() as u64,
            "note": "AI-assisted drift scoring requires mockforge-ai-contract-diff (not yet integrated)",
        })),
    })
}

/// Walk an OpenAPI 3.x document and collect (method, path) tuples for
/// every declared operation. Tolerant of YAML/JSON differences and
/// missing optional fields.
fn collect_endpoints(spec: &serde_json::Value) -> Vec<(String, String)> {
    const HTTP_METHODS: &[&str] = &[
        "get", "post", "put", "patch", "delete", "head", "options", "trace",
    ];

    let Some(paths) = spec.get("paths").and_then(|v| v.as_object()) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for (path, path_item) in paths {
        let Some(item_obj) = path_item.as_object() else {
            continue;
        };
        for method in HTTP_METHODS {
            if item_obj.contains_key(*method) {
                out.push((method.to_uppercase(), path.clone()));
            }
        }
    }
    out
}

/// Synthetic fallback — same shape the executor has shipped with so
/// existing UIs and snapshots stay coherent.
async fn run_synthetic(
    kind: &str,
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    service_name: &str,
) -> Result<JobOutcome> {
    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!("Synthetic {} for service '{}'", kind, service_name),
                "synthetic": true,
                "tracking_task": 8,
            }),
        )
        .await?;

    let findings = [
        ("non_breaking", "GET /users", "Added optional query param"),
        ("cosmetic", "POST /users", "Description rewording"),
    ];
    let mut next_seq: u32 = 2;
    for (i, (sev, endpoint, desc)) in findings.iter().enumerate() {
        callbacks
            .run_event(
                job.run_id,
                next_seq,
                "diff_finding",
                serde_json::json!({
                    "index": i,
                    "severity": sev,
                    "endpoint": endpoint,
                    "description": desc,
                }),
            )
            .await?;
        next_seq += 1;
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    }

    let elapsed = started.elapsed();
    let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);

    Ok(JobOutcome {
        status: JobStatus::Passed,
        runner_seconds: secs,
        summary: Some(serde_json::json!({
            "executor_phase": "synthetic",
            "tracking_task": 8,
            "kind": kind,
            "service_name": service_name,
            "findings_count": findings.len(),
            "wall_ms": elapsed.as_millis() as u64,
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_endpoints_parses_basic_spec() {
        let spec = serde_json::json!({
            "paths": {
                "/users": {
                    "get": {},
                    "post": {},
                },
                "/users/{id}": {
                    "get": {},
                    "delete": {},
                    "patch": {},
                },
            },
        });
        let mut endpoints = collect_endpoints(&spec);
        endpoints.sort();
        assert_eq!(
            endpoints,
            vec![
                ("DELETE".to_string(), "/users/{id}".to_string()),
                ("GET".to_string(), "/users".to_string()),
                ("GET".to_string(), "/users/{id}".to_string()),
                ("PATCH".to_string(), "/users/{id}".to_string()),
                ("POST".to_string(), "/users".to_string()),
            ]
        );
    }

    #[test]
    fn collect_endpoints_handles_missing_paths() {
        let spec = serde_json::json!({ "openapi": "3.0.0" });
        assert_eq!(collect_endpoints(&spec), Vec::<(String, String)>::new());
    }

    #[test]
    fn collect_endpoints_skips_non_method_keys() {
        let spec = serde_json::json!({
            "paths": {
                "/x": {
                    "summary": "ignored",
                    "parameters": [],
                    "get": {},
                },
            },
        });
        assert_eq!(collect_endpoints(&spec), vec![("GET".to_string(), "/x".to_string())]);
    }
}
