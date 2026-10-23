//! Contract diff executor (#8 / Phase 2).
//!
//! A contract-diff payload supplies either `openapi_spec_url` or
//! `openapi_spec_inline`. The executor parses the spec, enumerates declared
//! endpoints, and emits one event per endpoint plus summary stats.

use async_trait::async_trait;
use std::time::Instant;

use crate::callbacks::{ContractDriftEndpoint, ContractDriftScoreRequest, RegistryCallbacks};
use crate::error::{Error, Result};
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};

/// Executor for contract-diff runs.
pub struct ContractExecutor;

enum ContractSpecInput {
    Url(String),
    Inline(serde_json::Value),
}

#[async_trait]
impl Executor for ContractExecutor {
    fn kind(&self) -> &'static str {
        "contract_diff"
    }

    async fn execute(&self, job: RunJob, callbacks: &RegistryCallbacks) -> Result<JobOutcome> {
        let service_name = job
            .payload
            .get("service_name")
            .and_then(|v| v.as_str())
            .unwrap_or("(unspecified)")
            .to_string();
        let spec_input = contract_spec_input(&job.payload)?;

        let started = Instant::now();
        callbacks.run_started(job.run_id).await?;
        run_real_contract_diff(job, callbacks, started, &service_name, spec_input).await
    }
}

fn contract_spec_input(payload: &serde_json::Value) -> Result<ContractSpecInput> {
    if let Some(url) = payload
        .get("openapi_spec_url")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(ContractSpecInput::Url(url.to_owned()));
    }

    if let Some(inline) = payload.get("openapi_spec_inline").filter(|value| !value.is_null()) {
        if !inline.as_str().is_some_and(|value| value.trim().is_empty()) {
            return Ok(ContractSpecInput::Inline(inline.clone()));
        }
    }

    Err(Error::Executor(
        "contract_diff requires `openapi_spec_url` or `openapi_spec_inline`".into(),
    ))
}

async fn run_real_contract_diff(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    service_name: &str,
    spec_input: ContractSpecInput,
) -> Result<JobOutcome> {
    let (spec_text, spec_url, spec_source) = match spec_input {
        ContractSpecInput::Url(spec_url) => {
            callbacks
                .run_event(
                    job.run_id,
                    1,
                    "log",
                    serde_json::json!({
                        "level": "info",
                        "message": format!("Fetching OpenAPI spec: {spec_url}"),
                        "tracking_task": 8,
                    }),
                )
                .await?;

            let client = crate::target_client::TargetClient::new(
                std::time::Duration::from_secs(30),
                "mockforge-contract-diff/1.0",
            )?;
            let request = client.get(&spec_url).map_err(|error| {
                Error::Executor(format!("openapi_spec_url rejected by SSRF guard: {error}"))
            })?;
            let response = request.send().await.map_err(|error| {
                Error::Executor(format!(
                    "contract_diff failed to fetch `openapi_spec_url`: {error}"
                ))
            })?;
            let status = response.status();
            if !status.is_success() {
                return Err(Error::Executor(format!(
                    "contract_diff spec fetch returned HTTP {status}"
                )));
            }
            let spec_text = response.text().await.map_err(|error| {
                Error::Executor(format!("contract_diff failed to read the spec body: {error}"))
            })?;
            (spec_text, Some(spec_url), "url")
        }
        ContractSpecInput::Inline(inline) => {
            callbacks
                .run_event(
                    job.run_id,
                    1,
                    "log",
                    serde_json::json!({
                        "level": "info",
                        "message": "Parsing inline OpenAPI spec",
                        "tracking_task": 8,
                    }),
                )
                .await?;
            let spec_text = match inline {
                serde_json::Value::String(text) => text,
                value => serde_json::to_string(&value).map_err(|error| {
                    Error::Executor(format!(
                        "contract_diff could not serialize `openapi_spec_inline`: {error}"
                    ))
                })?,
            };
            (spec_text, None, "inline")
        }
    };

    let spec = parse_contract_spec(&spec_text)?;

    let endpoints = required_endpoints(&spec)?;

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

    // Pull recent traffic so we can compare declared vs actually-hit endpoints.
    let workspace_id = job
        .payload
        .get("workspace_id")
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok());
    let hits = fetch_endpoint_hits(callbacks, workspace_id).await?;
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
            "spec_source": spec_source,
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

fn parse_contract_spec(spec_text: &str) -> Result<serde_json::Value> {
    serde_json::from_str(spec_text)
        .or_else(|_| serde_yaml::from_str(spec_text))
        .map_err(|error| {
            Error::Executor(format!(
                "contract_diff could not parse the spec as JSON or YAML: {error}"
            ))
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
        run_id: job.run_id,
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

fn required_endpoints(spec: &serde_json::Value) -> Result<Vec<(String, String)>> {
    let endpoints = collect_endpoints(spec);
    if endpoints.is_empty() {
        Err(Error::Executor(
            "contract_diff spec contains zero endpoints under `paths`".into(),
        ))
    } else {
        Ok(endpoints)
    }
}

async fn fetch_endpoint_hits(
    callbacks: &RegistryCallbacks,
    workspace_id: Option<uuid::Uuid>,
) -> Result<Vec<crate::callbacks::EndpointHit>> {
    match workspace_id {
        Some(workspace_id) => callbacks
            .fetch_workspace_endpoint_hits(workspace_id)
            .await
            .map_err(|error| {
                Error::Executor(format!(
                    "contract_diff failed to fetch endpoint traffic for workspace {workspace_id}: {error}"
                ))
            }),
        None => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn missing_spec_is_an_executor_error() {
        let callbacks = RegistryCallbacks::for_test();
        let job = RunJob {
            run_id: uuid::Uuid::new_v4(),
            org_id: uuid::Uuid::new_v4(),
            source_id: uuid::Uuid::new_v4(),
            kind: "contract_diff".to_string(),
            payload: serde_json::json!({}),
        };
        let result = ContractExecutor.execute(job, &callbacks).await;
        assert!(
            matches!(result, Err(Error::Executor(ref message)) if message == "contract_diff requires `openapi_spec_url` or `openapi_spec_inline`"),
            "missing spec must error, got {result:?}"
        );
    }

    #[tokio::test]
    async fn inline_spec_executes_without_a_url() {
        let callbacks = RegistryCallbacks::for_test();
        let job = RunJob {
            run_id: uuid::Uuid::new_v4(),
            org_id: uuid::Uuid::new_v4(),
            source_id: uuid::Uuid::new_v4(),
            kind: "contract_diff".to_string(),
            payload: serde_json::json!({
                "service_name": "inline service",
                "openapi_spec_inline": {
                    "openapi": "3.0.0",
                    "paths": {"/health": {"get": {}}}
                }
            }),
        };

        let outcome = ContractExecutor.execute(job, &callbacks).await.unwrap();

        assert_eq!(outcome.status, JobStatus::Passed);
        let summary = outcome.summary.unwrap();
        assert_eq!(summary["endpoint_count"], 1);
        assert_eq!(summary["spec_source"], "inline");
        assert!(summary["spec_url"].is_null());
    }

    #[tokio::test]
    async fn rejected_spec_url_is_an_executor_error() {
        let callbacks = RegistryCallbacks::for_test();
        let job = RunJob {
            run_id: uuid::Uuid::new_v4(),
            org_id: uuid::Uuid::new_v4(),
            source_id: uuid::Uuid::new_v4(),
            kind: "contract_diff".to_string(),
            payload: serde_json::json!({"openapi_spec_url": "http://127.0.0.1/spec.json"}),
        };
        let result = ContractExecutor.execute(job, &callbacks).await;
        assert!(
            matches!(result, Err(Error::Executor(ref message)) if message.contains("openapi_spec_url rejected")),
            "rejected spec URL must error, got {result:?}"
        );
    }

    #[test]
    fn spec_with_zero_endpoints_is_zero_work() {
        let spec = serde_json::json!({"openapi": "3.0.0", "paths": {}});
        assert!(
            matches!(required_endpoints(&spec), Err(Error::Executor(message)) if message.contains("zero endpoints"))
        );
    }

    #[test]
    fn inline_and_fetched_text_use_the_same_json_or_yaml_parser() {
        let json =
            parse_contract_spec(r#"{"openapi":"3.0.0","paths":{"/json":{"get":{}}}}"#).unwrap();
        let yaml = parse_contract_spec("openapi: 3.0.0\npaths:\n  /yaml:\n    post: {}\n").unwrap();

        assert_eq!(required_endpoints(&json).unwrap(), vec![("GET".into(), "/json".into())]);
        assert_eq!(required_endpoints(&yaml).unwrap(), vec![("POST".into(), "/yaml".into())]);
    }

    #[tokio::test]
    async fn endpoint_hits_fetch_failure_is_an_executor_error() {
        let callbacks = RegistryCallbacks::for_test_failing_endpoint_hits_fetch();
        let workspace_id = uuid::Uuid::new_v4();
        let result = fetch_endpoint_hits(&callbacks, Some(workspace_id)).await;
        assert!(
            matches!(result, Err(Error::Executor(ref message)) if message.contains("failed to fetch endpoint traffic") && message.contains(&workspace_id.to_string()))
        );
    }

    #[tokio::test]
    async fn ai_callback_uses_job_identity_instead_of_user_authored_payload() {
        let callbacks = RegistryCallbacks::for_test();
        let job = RunJob {
            run_id: uuid::Uuid::new_v4(),
            org_id: uuid::Uuid::new_v4(),
            source_id: uuid::Uuid::new_v4(),
            kind: "contract_diff".into(),
            payload: serde_json::json!({"run_id": uuid::Uuid::new_v4(), "triggered_by_user_id": uuid::Uuid::new_v4()}),
        };
        let workspace = uuid::Uuid::new_v4();
        let mut seq = 1;
        let mut findings = 0;
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            run_ai_second_pass(
                &job,
                &callbacks,
                &mut seq,
                &mut findings,
                workspace,
                "openapi: 3.0.0",
                Vec::new(),
            ),
        )
        .await
        .unwrap();
        let body = callbacks.contract_score_requests().pop().unwrap();
        assert_eq!(body["run_id"], job.run_id.to_string());
        assert_eq!(body["org_id"], job.org_id.to_string());
        assert_eq!(body["workspace_id"], workspace.to_string());
        assert!(body.get("triggered_by_user_id").is_none());

        let wire_callbacks = RegistryCallbacks::new(
            "https://registry.example.test".into(),
            "internal-test-token".into(),
        );
        let request = wire_callbacks
            .contract_drift_score_request(&ContractDriftScoreRequest {
                run_id: job.run_id,
                org_id: job.org_id,
                workspace_id: workspace,
                spec_excerpt: "openapi: 3.0.0".into(),
                endpoints: Vec::new(),
                max_samples_per_endpoint: None,
            })
            .unwrap();
        assert_eq!(request.method(), reqwest::Method::POST);
        assert_eq!(request.url().path(), "/api/v1/internal/contract-diff/score");
        assert_eq!(request.headers()[reqwest::header::AUTHORIZATION], "Bearer internal-test-token");
        let wire_body: serde_json::Value =
            serde_json::from_slice(request.body().and_then(reqwest::Body::as_bytes).unwrap())
                .unwrap();
        assert_eq!(wire_body, body);
    }

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
