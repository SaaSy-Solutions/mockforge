//! Contract diff executor (#8 / Phase 2). Handles `contract_diff` and
//! `verification_suite`.
//!
//! `contract_diff` fetches the OpenAPI spec at `openapi_spec_url`, parses
//! it, and compares its declared endpoints with the workspace's recent
//! traffic. A missing URL, a fetch error, a non-2xx response, or a body
//! that is not an OpenAPI/Swagger document finishes the run `errored`
//! with a `status_reason` (#1264). It used to fall back to synthetic
//! findings and report `passed`, so a broken spec URL produced a green
//! contract check.
//!
//! `verification_suite` has no executor yet and finishes `errored` with
//! `status_reason = "not_implemented"`.

use async_trait::async_trait;
use std::time::Instant;

use crate::callbacks::{ContractDriftEndpoint, ContractDriftScoreRequest, RegistryCallbacks};
use crate::error::Result;
use crate::executors::{errored_with_reason, unsupported, Executor, JobOutcome, JobStatus, RunJob};
use crate::target_client::TargetClient;

/// `status_reason` when the payload has no `openapi_spec_url`.
pub const REASON_SPEC_MISSING: &str = "spec_missing";
/// `status_reason` when the SSRF guard refuses `openapi_spec_url`.
pub const REASON_SPEC_URL_REJECTED: &str = "spec_url_rejected";
/// `status_reason` when the spec request fails or its body can't be read.
pub const REASON_SPEC_FETCH_FAILED: &str = "spec_fetch_failed";
/// `status_reason` when the spec URL answers with a non-2xx status.
pub const REASON_SPEC_HTTP_ERROR: &str = "spec_http_error";
/// `status_reason` when the body is not an OpenAPI/Swagger document.
pub const REASON_SPEC_INVALID: &str = "spec_invalid";
/// `status_reason` when the workspace traffic lookup fails.
pub const REASON_TRAFFIC_UNAVAILABLE: &str = "traffic_unavailable";

/// Executor for the contract kinds.
pub struct ContractExecutor {
    kind: &'static str,
}

impl ContractExecutor {
    /// Construct for `contract_diff` or `verification_suite`.
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

        if self.kind != "contract_diff" {
            return unsupported(&job, callbacks, started, 1, "no verification was performed").await;
        }

        let service_name = job
            .payload
            .get("service_name")
            .and_then(|v| v.as_str())
            .unwrap_or("(unspecified)")
            .to_string();
        let spec_url = job
            .payload
            .get("openapi_spec_url")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from);
        let Some(spec_url) = spec_url else {
            return errored_with_reason(
                &job,
                callbacks,
                started,
                1,
                REASON_SPEC_MISSING,
                "contract_diff requires `openapi_spec_url`; no spec was compared".to_string(),
            )
            .await;
        };

        let client =
            TargetClient::new(std::time::Duration::from_secs(30), "mockforge-contract-diff/1.0")?;
        run_real_contract_diff(job, callbacks, started, &service_name, &spec_url, &client).await
    }
}

/// Why a spec could not be loaded: a `status_reason` plus a message.
#[derive(Debug)]
pub(crate) struct SpecError {
    reason: &'static str,
    message: String,
}

/// Fetch and parse the OpenAPI document at `spec_url`. Returns the raw
/// text (for the AI pass) and the parsed document. Fails on transport
/// errors, non-2xx responses, and bodies that are not an OpenAPI 3.x or
/// Swagger 2.0 document with a `paths` object.
pub(crate) async fn fetch_spec(
    client: &TargetClient,
    spec_url: &str,
) -> std::result::Result<(String, serde_json::Value), SpecError> {
    let request = client.get(spec_url).map_err(|e| SpecError {
        reason: REASON_SPEC_URL_REJECTED,
        message: format!("openapi_spec_url rejected by SSRF guard: {e}"),
    })?;
    let resp = request.send().await.map_err(|e| SpecError {
        reason: REASON_SPEC_FETCH_FAILED,
        message: format!("spec fetch failed: {e}"),
    })?;
    let status = resp.status();
    if !status.is_success() {
        return Err(SpecError {
            reason: REASON_SPEC_HTTP_ERROR,
            message: format!("spec fetch returned HTTP {status}"),
        });
    }
    let text = resp.text().await.map_err(|e| SpecError {
        reason: REASON_SPEC_FETCH_FAILED,
        message: format!("spec body read failed: {e}"),
    })?;
    let spec = parse_spec(&text).map_err(|message| SpecError {
        reason: REASON_SPEC_INVALID,
        message,
    })?;
    Ok((text, spec))
}

/// Parse JSON or YAML and require an OpenAPI/Swagger document. YAML
/// accepts almost any text as a scalar, so the shape check is what stops
/// an HTML error page from parsing as an empty spec.
fn parse_spec(text: &str) -> std::result::Result<serde_json::Value, String> {
    let spec: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => serde_yaml::from_str(text)
            .map_err(|e| format!("spec parse failed (not JSON or YAML): {e}"))?,
    };
    let Some(obj) = spec.as_object() else {
        return Err("spec is not a JSON/YAML object".to_string());
    };
    if !obj.contains_key("openapi") && !obj.contains_key("swagger") {
        return Err("spec has no `openapi` or `swagger` version field".to_string());
    }
    if !obj.get("paths").is_some_and(|p| p.is_object()) {
        return Err("spec has no `paths` object".to_string());
    }
    Ok(spec)
}

pub(crate) async fn run_real_contract_diff(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    service_name: &str,
    spec_url: &str,
    client: &TargetClient,
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

    let (spec_text, spec) = match fetch_spec(client, spec_url).await {
        Ok(loaded) => loaded,
        Err(e) => {
            return errored_with_reason(&job, callbacks, started, 2, e.reason, e.message).await;
        }
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
    // endpoints. No traffic → the declared findings carry no drift
    // markers. A failed lookup errors the run: comparing against an
    // empty list we could not fetch would report "no drift" falsely.
    let workspace_id = job
        .payload
        .get("workspace_id")
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok());
    let hits = match workspace_id {
        Some(wid) => match callbacks.fetch_workspace_endpoint_hits(wid).await {
            Ok(hits) => hits,
            Err(e) => {
                return errored_with_reason(
                    &job,
                    callbacks,
                    started,
                    3,
                    REASON_TRAFFIC_UNAVAILABLE,
                    format!("workspace traffic lookup failed: {e}"),
                )
                .await;
            }
        },
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
        // One unit per declared endpoint checked plus one per undeclared
        // endpoint found in traffic.
        work_count: endpoints.len() as u64 + u64::from(undeclared_count),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ai_callback_uses_job_identity_instead_of_user_authored_payload() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let callbacks = RegistryCallbacks::new(
            format!("http://{}", listener.local_addr().unwrap()),
            "synthetic-internal-fixture".into(),
        );
        let server = tokio::spawn(async move {
            let mut scoring = None;
            for index in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                let (start, length) = loop {
                    let mut chunk = [0; 1024];
                    let count = socket.read(&mut chunk).await.unwrap();
                    assert!(count > 0 && bytes.len() + count <= 16384);
                    bytes.extend_from_slice(&chunk[..count]);
                    if let Some(end) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                        let headers = std::str::from_utf8(&bytes[..end]).unwrap();
                        let length = headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().unwrap())
                            })
                            .unwrap();
                        break (end + 4, length);
                    }
                };
                while bytes.len() < start + length {
                    let mut chunk = [0; 1024];
                    let count = socket.read(&mut chunk).await.unwrap();
                    assert!(count > 0 && bytes.len() + count <= 16384);
                    bytes.extend_from_slice(&chunk[..count]);
                }
                let body = if index == 0 {
                    assert!(bytes.starts_with(b"POST /api/v1/internal/contract-diff/score "));
                    scoring = Some(
                        serde_json::from_slice::<serde_json::Value>(&bytes[start..start + length])
                            .unwrap(),
                    );
                    r#"{"findings":[],"tokens_used":17,"provider":"platform","no_traffic":false}"#
                } else {
                    "{}"
                };
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                socket.write_all(response.as_bytes()).await.unwrap();
            }
            scoring.unwrap()
        });
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
        let body = tokio::time::timeout(std::time::Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(body["run_id"], job.run_id.to_string());
        assert_eq!(body["org_id"], job.org_id.to_string());
        assert_eq!(body["workspace_id"], workspace.to_string());
        assert!(body.get("triggered_by_user_id").is_none());
    }

    use crate::executors::test_support::{job, reason, FakeHttp};
    use crate::executors::{JobStatus, REASON_NO_WORK};

    const VALID_SPEC: &str = r#"{"openapi":"3.0.0","info":{"title":"t","version":"1"},"paths":{"/users":{"get":{},"post":{}}}}"#;

    fn loopback_client() -> TargetClient {
        TargetClient::with_policy(
            mockforge_bench::ssrf::Policy::for_test(),
            std::time::Duration::from_secs(5),
            "test",
            0,
        )
        .expect("client builds")
    }

    /// Serves the spec routes used by the contract_diff tests.
    async fn spec_server() -> FakeHttp {
        FakeHttp::start(|line| {
            if line.starts_with("GET /valid ") {
                (200, VALID_SPEC.to_string())
            } else if line.starts_with("GET /valid.yaml ") {
                (200, "openapi: 3.0.0\npaths:\n  /a:\n    get: {}\n".to_string())
            } else if line.starts_with("GET /empty-paths ") {
                (200, r#"{"openapi":"3.0.0","paths":{}}"#.to_string())
            } else if line.starts_with("GET /html ") {
                (200, "<html><body>Sign in</body></html>".to_string())
            } else if line.starts_with("GET /not-openapi ") {
                (200, r#"{"hello":"world"}"#.to_string())
            } else {
                (404, r#"{"error":"not found"}"#.to_string())
            }
        })
        .await
    }

    /// Run `contract_diff` against `spec_url` with a fake registry and
    /// return the finalized outcome.
    async fn diff(spec_url: &str, payload: serde_json::Value) -> JobOutcome {
        let registry = FakeHttp::registry().await;
        let callbacks = RegistryCallbacks::new(registry.base_url.clone(), "t".into());
        let outcome = run_real_contract_diff(
            job("contract_diff", payload),
            &callbacks,
            Instant::now(),
            "svc",
            spec_url,
            &loopback_client(),
        )
        .await
        .expect("executor returns an outcome");
        outcome.finalize()
    }

    #[tokio::test]
    async fn contract_diff_errors_when_spec_fetch_fails() {
        // Bind then drop a listener so the port refuses connections.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        drop(listener);
        let o = diff(&format!("http://{addr}/spec.json"), serde_json::json!({})).await;
        assert_eq!(o.status, JobStatus::Errored);
        assert_eq!(reason(&o).as_deref(), Some(REASON_SPEC_FETCH_FAILED));
        assert_eq!(o.work_count, 0);
    }

    #[tokio::test]
    async fn contract_diff_errors_on_spec_404() {
        let spec = spec_server().await;
        let o = diff(&format!("{}/missing", spec.base_url), serde_json::json!({})).await;
        assert_eq!(o.status, JobStatus::Errored);
        assert_eq!(reason(&o).as_deref(), Some(REASON_SPEC_HTTP_ERROR));
        let error = o.summary.as_ref().and_then(|s| s["error"].as_str()).unwrap_or_default();
        assert!(error.contains("404"), "error names the HTTP status: {error}");
    }

    #[tokio::test]
    async fn contract_diff_errors_on_invalid_spec() {
        let spec = spec_server().await;
        for path in ["/html", "/not-openapi"] {
            let o = diff(&format!("{}{path}", spec.base_url), serde_json::json!({})).await;
            assert_eq!(o.status, JobStatus::Errored, "{path}");
            assert_eq!(reason(&o).as_deref(), Some(REASON_SPEC_INVALID), "{path}");
        }
    }

    #[tokio::test]
    async fn contract_diff_rejected_url_errors() {
        let registry = FakeHttp::registry().await;
        let callbacks = RegistryCallbacks::new(registry.base_url.clone(), "t".into());
        let strict = TargetClient::with_policy(
            mockforge_bench::ssrf::Policy::strict(),
            std::time::Duration::from_secs(5),
            "test",
            0,
        )
        .expect("client builds");
        let o = run_real_contract_diff(
            job("contract_diff", serde_json::json!({})),
            &callbacks,
            Instant::now(),
            "svc",
            "http://127.0.0.1:9/spec.json",
            &strict,
        )
        .await
        .expect("outcome")
        .finalize();
        assert_eq!(o.status, JobStatus::Errored);
        assert_eq!(reason(&o).as_deref(), Some(REASON_SPEC_URL_REJECTED));
    }

    #[tokio::test]
    async fn contract_diff_passes_with_real_spec_and_counts_endpoints() {
        let spec = spec_server().await;
        let o = diff(&format!("{}/valid", spec.base_url), serde_json::json!({})).await;
        assert_eq!(o.status, JobStatus::Passed);
        assert_eq!(o.work_count, 2);
        assert_eq!(o.summary.as_ref().map(|s| &s["work_count"]), Some(&2.into()));

        let o = diff(&format!("{}/valid.yaml", spec.base_url), serde_json::json!({})).await;
        assert_eq!(o.status, JobStatus::Passed);
        assert_eq!(o.work_count, 1);
    }

    #[tokio::test]
    async fn contract_diff_with_no_endpoints_is_not_a_pass() {
        let spec = spec_server().await;
        let o = diff(&format!("{}/empty-paths", spec.base_url), serde_json::json!({})).await;
        assert_eq!(o.status, JobStatus::Failed);
        assert_eq!(reason(&o).as_deref(), Some(REASON_NO_WORK));
    }

    #[tokio::test]
    async fn contract_diff_errors_when_traffic_lookup_fails() {
        let spec = spec_server().await;
        // Registry: callbacks succeed except the endpoint-hits lookup.
        let registry = FakeHttp::start(|line| {
            if line.contains("/endpoint-hits") {
                (500, "{}".to_string())
            } else {
                (200, "{}".to_string())
            }
        })
        .await;
        let callbacks = RegistryCallbacks::new(registry.base_url.clone(), "t".into());
        let payload = serde_json::json!({ "workspace_id": uuid::Uuid::new_v4().to_string() });
        let o = run_real_contract_diff(
            job("contract_diff", payload),
            &callbacks,
            Instant::now(),
            "svc",
            &format!("{}/valid", spec.base_url),
            &loopback_client(),
        )
        .await
        .expect("outcome")
        .finalize();
        assert_eq!(o.status, JobStatus::Errored);
        assert_eq!(reason(&o).as_deref(), Some(REASON_TRAFFIC_UNAVAILABLE));
    }

    #[test]
    fn parse_spec_requires_openapi_document() {
        assert!(parse_spec(VALID_SPEC).is_ok());
        assert!(parse_spec("swagger: '2.0'\npaths: {}\n").is_ok());
        assert!(parse_spec("just some text").is_err(), "YAML scalar is not a spec");
        assert!(parse_spec("[1, 2]").is_err());
        assert!(parse_spec(r#"{"openapi":"3.0.0"}"#).is_err(), "paths required");
        assert!(parse_spec(r#"{"paths":{}}"#).is_err(), "version field required");
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
