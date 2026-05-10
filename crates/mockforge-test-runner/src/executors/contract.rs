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

use crate::callbacks::{ContractDriftEndpoint, ContractDriftScoreRequest, RegistryCallbacks};
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

    // Pull recent traffic so we can compare declared vs actually-hit
    // endpoints. Empty/error → executor still emits the declared
    // findings without drift markers.
    let workspace_id = job
        .payload
        .get("workspace_id")
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok());
    let hits = match workspace_id {
        Some(wid) => callbacks.fetch_workspace_endpoint_hits(wid).await.unwrap_or_default(),
        None => Vec::new(),
    };
    let hits_by_endpoint: std::collections::HashMap<(String, String), i64> = hits
        .iter()
        .map(|h| ((h.method.to_uppercase(), h.path.clone()), h.hits))
        .collect();
    let declared_set: std::collections::HashSet<(String, String)> =
        endpoints.iter().cloned().collect();

    callbacks
        .run_event(
            job.run_id,
            3,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!(
                    "Traffic: {} unique endpoints hit in last 24h",
                    hits_by_endpoint.len()
                ),
                "traffic_endpoint_count": hits_by_endpoint.len(),
            }),
        )
        .await?;

    let mut next_seq: u32 = 4;
    let mut declared_count = 0u32;
    let mut undeclared_count = 0u32;
    let mut unused_count = 0u32;

    // Walk the spec endpoints and emit drift severity based on traffic.
    for (method, path) in &endpoints {
        let hit_count = hits_by_endpoint.get(&(method.clone(), path.clone())).copied();
        let (severity, description) = match hit_count {
            Some(n) => ("declared", format!("Endpoint declared in spec, {n} hits in last 24h")),
            None if hits_by_endpoint.is_empty() => {
                ("declared", "Endpoint declared in spec; no traffic data available".to_string())
            }
            None => (
                "non_breaking",
                "Endpoint declared in spec but never hit in last 24h (potentially unused)"
                    .to_string(),
            ),
        };
        if hit_count.is_some() || hits_by_endpoint.is_empty() {
            declared_count += 1;
        } else {
            unused_count += 1;
        }
        callbacks
            .run_event(
                job.run_id,
                next_seq,
                "diff_finding",
                serde_json::json!({
                    "severity": severity,
                    "endpoint": format!("{method} {path}"),
                    "description": description,
                    "hits_24h": hit_count.unwrap_or(0),
                }),
            )
            .await?;
        next_seq += 1;
    }

    // Walk the traffic for endpoints NOT in the spec — these are
    // breaking-style drift findings (clients hit something the spec
    // doesn't declare).
    for ((method, path), hits) in &hits_by_endpoint {
        if !declared_set.contains(&(method.clone(), path.clone())) {
            undeclared_count += 1;
            callbacks
                .run_event(
                    job.run_id,
                    next_seq,
                    "diff_finding",
                    serde_json::json!({
                        "severity": "breaking",
                        "endpoint": format!("{method} {path}"),
                        "description": format!(
                            "Endpoint hit {hits} times in last 24h but not declared in spec — drift"
                        ),
                        "hits_24h": hits,
                    }),
                )
                .await?;
            next_seq += 1;
        }
    }

    // Optional AI-assisted second pass (#348). Opt-in via the suite
    // config's `ai_drift_enabled` flag. Scores parameter / schema /
    // response-shape drift on declared endpoints that actually have
    // traffic — orphaned and undeclared ones already have clear
    // severity from the structural pass. Failures from the AI call
    // are non-fatal: a missing BYOK / exhausted quota / timeout
    // degrades to "no AI findings" rather than failing the run.
    let mut ai_findings_count = 0u32;
    let ai_breaking_count = match (uses_ai_drift_scoring(&job.payload), workspace_id) {
        (true, Some(wid)) => {
            let candidates: Vec<ContractDriftEndpoint> = endpoints
                .iter()
                .filter(|(method, path)| {
                    hits_by_endpoint.contains_key(&(method.clone(), path.clone()))
                })
                .map(|(method, path)| ContractDriftEndpoint {
                    method: method.clone(),
                    path: path.clone(),
                })
                .collect();

            if candidates.is_empty() {
                callbacks
                    .run_event(
                        job.run_id,
                        next_seq,
                        "log",
                        serde_json::json!({
                            "level": "info",
                            "message": "AI drift scoring skipped: no declared endpoints with recent traffic",
                            "ai_phase": true,
                        }),
                    )
                    .await?;
                // Function returns shortly after; we don't bump
                // next_seq because nothing else in this scope reads
                // it (clippy::unused_assignments).
                0u32
            } else {
                run_ai_second_pass(
                    &job,
                    callbacks,
                    &mut next_seq,
                    &mut ai_findings_count,
                    wid,
                    &spec_text,
                    candidates,
                )
                .await
            }
        }
        _ => 0u32,
    };

    let elapsed = started.elapsed();
    let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
    // Pass when the spec covers everything the workspace has been
    // serving AND the AI pass didn't surface any new breaking
    // findings. AI cosmetic / non_breaking findings don't fail the
    // run — they're informational.
    let status = if undeclared_count == 0 && ai_breaking_count == 0 {
        JobStatus::Passed
    } else {
        JobStatus::Failed
    };

    Ok(JobOutcome {
        status,
        runner_seconds: secs,
        summary: Some(serde_json::json!({
            "executor_phase": "real_diff",
            "tracking_task": 8,
            "kind": "contract_diff",
            "service_name": service_name,
            "spec_url": spec_url,
            "endpoint_count": endpoints.len(),
            "traffic_endpoint_count": hits_by_endpoint.len(),
            "declared_count": declared_count,
            "unused_count": unused_count,
            "undeclared_count": undeclared_count,
            "ai_findings_count": ai_findings_count,
            "ai_breaking_count": ai_breaking_count,
            "wall_ms": elapsed.as_millis() as u64,
        })),
    })
}

/// Did the suite config opt into AI drift scoring? Stored on the
/// suite's `config` JSON as `ai_drift_enabled: bool` (default false).
fn uses_ai_drift_scoring(payload: &serde_json::Value) -> bool {
    payload.get("ai_drift_enabled").and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Run the AI scoring callback and emit one `diff_finding` event per
/// finding. Returns the count of `breaking`-severity findings so the
/// caller can decide whether to fail the run.
async fn run_ai_second_pass(
    job: &RunJob,
    callbacks: &RegistryCallbacks,
    next_seq: &mut u32,
    ai_findings_count: &mut u32,
    workspace_id: uuid::Uuid,
    spec_text: &str,
    endpoints: Vec<ContractDriftEndpoint>,
) -> u32 {
    let body = ContractDriftScoreRequest {
        org_id: job.org_id,
        workspace_id,
        spec_excerpt: spec_text.to_string(),
        endpoints,
        max_samples_per_endpoint: None,
    };

    let response = match callbacks.score_contract_drift(&body).await {
        Ok(r) => r,
        Err(e) => {
            // Non-fatal — log and continue. Common causes: free plan
            // without BYOK (registry returns 403), platform LLM key
            // unset on the registry, transient HTTP error.
            let _ = callbacks
                .run_event(
                    job.run_id,
                    *next_seq,
                    "log",
                    serde_json::json!({
                        "level": "warn",
                        "message": format!("AI drift scoring failed: {e}"),
                        "ai_phase": true,
                    }),
                )
                .await;
            *next_seq += 1;
            return 0;
        }
    };

    if response.no_traffic {
        let _ = callbacks
            .run_event(
                job.run_id,
                *next_seq,
                "log",
                serde_json::json!({
                    "level": "info",
                    "message": "AI drift scoring skipped: no captured exchanges to sample",
                    "ai_phase": true,
                }),
            )
            .await;
        *next_seq += 1;
        return 0;
    }

    let _ = callbacks
        .run_event(
            job.run_id,
            *next_seq,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!(
                    "AI drift scoring returned {} findings ({} tokens, provider={})",
                    response.findings.len(),
                    response.tokens_used,
                    response.provider,
                ),
                "ai_phase": true,
                "tokens_used": response.tokens_used,
                "provider": response.provider,
            }),
        )
        .await;
    *next_seq += 1;

    let mut breaking_count = 0u32;
    for finding in response.findings {
        if finding.severity == "breaking" {
            breaking_count += 1;
        }
        *ai_findings_count += 1;
        let _ = callbacks
            .run_event(
                job.run_id,
                *next_seq,
                "diff_finding",
                serde_json::json!({
                    "severity": finding.severity,
                    "endpoint": finding.endpoint,
                    "description": finding.description,
                    "confidence": finding.confidence,
                    "rationale": finding.rationale,
                    "ai": true,
                }),
            )
            .await;
        *next_seq += 1;
    }
    breaking_count
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
