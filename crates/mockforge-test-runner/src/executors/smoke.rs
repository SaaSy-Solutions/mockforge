//! Smoke-test executor for hosted-mock deployments (Issue #392).
//!
//! Given a hosted-mock deployment id and its OpenAPI spec, walk the
//! declared routes and probe each one with a basic 2xx-class assertion
//! plus a latency budget. Emits `route_pass` / `route_fail` /
//! `route_skipped` events live so the UI can stream the result row by
//! row.
//!
//! Dispatched on `kind = "smoke"`. The registry handler that creates
//! the test_run is responsible for populating the payload with at
//! minimum a `base_url` and either an inline `spec` or an
//! `openapi_spec_url`.
//!
//! Payload schema:
//!
//! ```json
//! {
//!   "deployment_id":      "uuid",         // optional — context only
//!   "base_url":           "https://...",  // required
//!   "spec":               "{...}",        // OR openapi_spec_url
//!   "openapi_spec_url":   "https://...",  // OR spec
//!   "latency_budget_ms":  5000,           // default 5000
//!   "methods":            ["GET"],        // default ["GET"]
//!   "allow_loopback":     false           // default false; true for local dev
//! }
//! ```
//!
//! Path templates with `{param}` placeholders are skipped in v1 (we
//! don't substitute synthetic values — that risks 404s that aren't
//! actually failures). The follow-up issue can land a "synthetic param
//! values" mode if needed.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use mockforge_bench::{validate_target_url, SsrfPolicy};
use uuid::Uuid;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};

/// Default per-route latency budget. A 5-second ceiling on a hosted
/// mock is generous; anything slower probably indicates a stuck
/// dependency rather than a normal probe.
const DEFAULT_LATENCY_BUDGET_MS: u64 = 5_000;

/// Hard cap on routes probed in a single smoke run, to keep cloud
/// worker time bounded for misshapen specs that declare hundreds of
/// endpoints. Routes beyond this are dropped with a `log` event.
const MAX_ROUTES_PER_RUN: usize = 200;

/// Per-request timeout when fetching the spec from a URL. Should be
/// well under the run's overall budget so a stuck spec server doesn't
/// burn the whole runner-second allowance.
const SPEC_FETCH_TIMEOUT_SECS: u64 = 30;

/// Executor for `kind = "smoke"`. See module docs for the payload
/// schema and event semantics.
pub struct SmokeTestExecutor;

#[async_trait]
impl Executor for SmokeTestExecutor {
    fn kind(&self) -> &'static str {
        "smoke"
    }

    async fn execute(&self, job: RunJob, callbacks: &RegistryCallbacks) -> Result<JobOutcome> {
        let started = Instant::now();
        callbacks.run_started(job.run_id).await?;

        let config = SmokeConfig::from_payload(&job.payload);
        let mut next_seq: u32 = 1;

        // ─── Validate target URL up front ───────────────────────────
        // Strict policy by default — reject loopback/RFC1918 to prevent
        // a misconfigured payload from probing the cloud worker's
        // internal network. Local dev opts in via `allow_loopback: true`.
        let policy = if config.allow_loopback {
            SsrfPolicy::for_test()
        } else {
            SsrfPolicy::strict()
        };
        if let Err(e) = validate_target_url(&config.base_url, policy).await {
            return errored_run(
                callbacks,
                &job,
                started,
                next_seq,
                format!("base_url rejected by SSRF guard: {e}"),
            )
            .await;
        }

        // ─── Fetch + parse the spec ─────────────────────────────────
        let spec = match load_spec(&job.payload).await {
            Ok(s) => s,
            Err(reason) => {
                return errored_run(callbacks, &job, started, next_seq, reason).await;
            }
        };
        let endpoints = collect_probeable_endpoints(&spec, &config.methods);
        let total_endpoints = endpoints.len();

        callbacks
            .run_event(
                job.run_id,
                next_seq,
                "log",
                serde_json::json!({
                    "level": "info",
                    "message": format!(
                        "Smoke probe starting: {} endpoint(s) against {}",
                        total_endpoints, config.base_url
                    ),
                    "deployment_id": config.deployment_id,
                    "latency_budget_ms": config.latency_budget_ms,
                    "tracking_task": 4,
                }),
            )
            .await?;
        next_seq += 1;

        // ─── HTTP client used for all probes. Per-request timeout
        // matches the latency budget plus a small grace so the
        // assertion is what reports the budget breach, not the timeout
        // itself eating the response.
        let request_timeout = Duration::from_millis(config.latency_budget_ms + 1_000);
        let client = match reqwest::Client::builder()
            .timeout(request_timeout)
            .user_agent("mockforge-smoke/1.0")
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return errored_run(
                    callbacks,
                    &job,
                    started,
                    next_seq,
                    format!("failed to build HTTP client: {e}"),
                )
                .await;
            }
        };

        // ─── Probe every endpoint serially ──────────────────────────
        let mut probed = 0usize;
        let mut passed = 0usize;
        let mut failed = 0usize;
        let mut skipped = 0usize;
        let mut dropped = 0usize;

        for (idx, ep) in endpoints.into_iter().enumerate() {
            if idx >= MAX_ROUTES_PER_RUN {
                dropped = total_endpoints - idx;
                break;
            }

            if has_path_parameter(&ep.path) {
                callbacks
                    .run_event(
                        job.run_id,
                        next_seq,
                        "route_skipped",
                        serde_json::json!({
                            "path": ep.path,
                            "method": ep.method,
                            "reason": "path contains template parameter",
                        }),
                    )
                    .await?;
                next_seq += 1;
                skipped += 1;
                continue;
            }

            let url = join_url(&config.base_url, &ep.path);
            let probe_start = Instant::now();
            let probe_result = client.request(method_from_str(&ep.method), &url).send().await;
            let latency_ms = probe_start.elapsed().as_millis() as u64;

            let (event_type, payload, is_pass) = match probe_result {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let status_pass = (200..300).contains(&status);
                    let latency_pass = latency_ms <= config.latency_budget_ms;
                    let pass = status_pass && latency_pass;
                    let mut reasons: Vec<&str> = Vec::new();
                    if !status_pass {
                        reasons.push("status not 2xx");
                    }
                    if !latency_pass {
                        reasons.push("latency over budget");
                    }
                    let event = if pass { "route_pass" } else { "route_fail" };
                    let body = serde_json::json!({
                        "path": ep.path,
                        "method": ep.method,
                        "status": status,
                        "latency_ms": latency_ms,
                        "reason": if pass { String::new() } else { reasons.join(", ") },
                    });
                    (event, body, pass)
                }
                Err(e) => {
                    let body = serde_json::json!({
                        "path": ep.path,
                        "method": ep.method,
                        "status": serde_json::Value::Null,
                        "latency_ms": latency_ms,
                        "reason": format!("request error: {}", truncate_error(&e)),
                    });
                    ("route_fail", body, false)
                }
            };

            callbacks.run_event(job.run_id, next_seq, event_type, payload).await?;
            next_seq += 1;
            probed += 1;
            if is_pass {
                passed += 1;
            } else {
                failed += 1;
            }
        }

        if dropped > 0 {
            callbacks
                .run_event(
                    job.run_id,
                    next_seq,
                    "log",
                    serde_json::json!({
                        "level": "warn",
                        "message": format!(
                            "Capped at {} endpoints; {} skipped (spec declared {})",
                            MAX_ROUTES_PER_RUN, dropped, total_endpoints
                        ),
                    }),
                )
                .await?;
            next_seq += 1;
        }

        let elapsed = started.elapsed();
        let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
        let status = if failed == 0 {
            JobStatus::Passed
        } else {
            JobStatus::Failed
        };

        // Final summary log so the run timeline reads cleanly when scrolling.
        callbacks
            .run_event(
                job.run_id,
                next_seq,
                "log",
                serde_json::json!({
                    "level": "info",
                    "message": format!(
                        "Smoke probe complete: {} passed / {} failed / {} skipped (of {})",
                        passed, failed, skipped, total_endpoints
                    ),
                }),
            )
            .await?;

        Ok(JobOutcome {
            status,
            runner_seconds: secs,
            summary: Some(serde_json::json!({
                "executor_phase": "real",
                "tracking_task": 4,
                "deployment_id": config.deployment_id,
                "base_url": config.base_url,
                "latency_budget_ms": config.latency_budget_ms,
                "total_routes": total_endpoints,
                "probed": probed,
                "passed": passed,
                "failed": failed,
                "skipped": skipped,
                "dropped": dropped,
            })),
        })
    }
}

// ─── Config + endpoint extraction ────────────────────────────────────

/// Resolved smoke-run configuration extracted from the test_runs.config
/// payload. Defaults applied here so the executor proper can assume a
/// fully populated struct.
struct SmokeConfig {
    deployment_id: Option<Uuid>,
    base_url: String,
    latency_budget_ms: u64,
    methods: Vec<String>,
    allow_loopback: bool,
}

impl SmokeConfig {
    fn from_payload(payload: &serde_json::Value) -> Self {
        let deployment_id = payload
            .get("deployment_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok());
        let base_url = payload
            .get("base_url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let latency_budget_ms = payload
            .get("latency_budget_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_LATENCY_BUDGET_MS);
        let methods = payload
            .get("methods")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.as_str())
                    .map(|s| s.to_uppercase())
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| vec!["GET".to_string()]);
        let allow_loopback =
            payload.get("allow_loopback").and_then(|v| v.as_bool()).unwrap_or(false);

        Self {
            deployment_id,
            base_url,
            latency_budget_ms,
            methods,
            allow_loopback,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Endpoint {
    method: String,
    path: String,
}

/// Walk an OpenAPI 3.x spec and collect endpoints whose method appears
/// in `wanted`. Tolerant of YAML/JSON differences and missing optional
/// fields. Mirrors `contract::collect_endpoints` but filters by method
/// up front since smoke only probes a subset.
fn collect_probeable_endpoints(spec: &serde_json::Value, wanted: &[String]) -> Vec<Endpoint> {
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
            if !item_obj.contains_key(*method) {
                continue;
            }
            let upper = method.to_uppercase();
            if !wanted.iter().any(|w| w == &upper) {
                continue;
            }
            out.push(Endpoint {
                method: upper,
                path: path.clone(),
            });
        }
    }
    out
}

/// True if the path contains an OpenAPI template parameter like
/// `/users/{id}`. Smoke skips these in v1.
fn has_path_parameter(path: &str) -> bool {
    path.contains('{') && path.contains('}')
}

/// Concatenate the base URL and the spec path. Both sides are
/// normalised so neither double-slashes nor missing slashes break the
/// resulting URL.
fn join_url(base: &str, path: &str) -> String {
    let base_trim = base.trim_end_matches('/');
    let path_trim = path.trim_start_matches('/');
    format!("{base_trim}/{path_trim}")
}

fn method_from_str(s: &str) -> reqwest::Method {
    match s {
        "GET" => reqwest::Method::GET,
        "HEAD" => reqwest::Method::HEAD,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "PATCH" => reqwest::Method::PATCH,
        "DELETE" => reqwest::Method::DELETE,
        "OPTIONS" => reqwest::Method::OPTIONS,
        "TRACE" => reqwest::Method::TRACE,
        // Unknown method strings collapse to GET — the only way to hit
        // this is a misconfigured payload, and probing GET is the
        // safest fallback (matches the "v1 only probes GET" spec).
        _ => reqwest::Method::GET,
    }
}

/// Truncate a reqwest error's display string. The full chain can be
/// long (DNS error → connection error → kernel error), and the SSE
/// payload is best kept terse.
fn truncate_error(e: &reqwest::Error) -> String {
    let s = e.to_string();
    if s.len() > 200 {
        format!("{}…", &s[..200])
    } else {
        s
    }
}

// ─── Spec loading ────────────────────────────────────────────────────

/// Load the spec either from an inline `spec` string or by fetching
/// from `openapi_spec_url`. Tolerant of both JSON and YAML — same
/// lenience the contract executor applies. Returns a human-readable
/// error string on failure (the executor surfaces this as an `errored`
/// run rather than panicking).
async fn load_spec(payload: &serde_json::Value) -> std::result::Result<serde_json::Value, String> {
    // Inline takes precedence — it's cheaper than a fetch and the
    // registry handler can populate it directly from `hosted_mocks.spec`.
    if let Some(raw) = payload.get("spec").and_then(|v| v.as_str()) {
        if !raw.trim().is_empty() {
            return parse_spec_text(raw);
        }
    }

    let url = payload.get("openapi_spec_url").and_then(|v| v.as_str()).unwrap_or("").trim();
    if url.is_empty() {
        return Err("payload must include either 'spec' or 'openapi_spec_url'".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(SPEC_FETCH_TIMEOUT_SECS))
        .user_agent("mockforge-smoke/1.0")
        .build()
        .map_err(|e| format!("failed to build spec-fetch client: {e}"))?;

    let resp = client.get(url).send().await.map_err(|e| format!("spec fetch failed: {e}"))?;
    let status = resp.status();
    let body = resp.text().await.map_err(|e| format!("spec body read failed: {e}"))?;
    if !status.is_success() {
        return Err(format!("spec fetch returned HTTP {status}"));
    }
    parse_spec_text(&body)
}

fn parse_spec_text(text: &str) -> std::result::Result<serde_json::Value, String> {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        return Ok(v);
    }
    serde_yaml::from_str::<serde_json::Value>(text)
        .map_err(|e| format!("spec is neither valid JSON nor YAML: {e}"))
}

// ─── Failure helpers ─────────────────────────────────────────────────

/// Emit a single failure log + return an `Errored` outcome. Used for
/// pre-flight failures (bad URL, missing spec) that prevent any actual
/// probing — a clean negative result would be misleading.
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
            "tracking_task": 4,
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_config_defaults_when_payload_empty() {
        let cfg = SmokeConfig::from_payload(&serde_json::json!({}));
        assert_eq!(cfg.deployment_id, None);
        assert_eq!(cfg.base_url, "");
        assert_eq!(cfg.latency_budget_ms, DEFAULT_LATENCY_BUDGET_MS);
        assert_eq!(cfg.methods, vec!["GET".to_string()]);
        assert!(!cfg.allow_loopback);
    }

    #[test]
    fn smoke_config_reads_explicit_values() {
        let cfg = SmokeConfig::from_payload(&serde_json::json!({
            "deployment_id": "11111111-2222-3333-4444-555555555555",
            "base_url": "https://example.com/",
            "latency_budget_ms": 1000,
            "methods": ["get", "head"],
            "allow_loopback": true,
        }));
        assert!(cfg.deployment_id.is_some());
        assert_eq!(cfg.base_url, "https://example.com/");
        assert_eq!(cfg.latency_budget_ms, 1000);
        assert_eq!(cfg.methods, vec!["GET".to_string(), "HEAD".to_string()]);
        assert!(cfg.allow_loopback);
    }

    #[test]
    fn smoke_config_falls_back_to_get_on_empty_methods_array() {
        let cfg = SmokeConfig::from_payload(&serde_json::json!({ "methods": [] }));
        assert_eq!(cfg.methods, vec!["GET".to_string()]);
    }

    #[test]
    fn collect_probeable_endpoints_filters_by_method() {
        let spec = serde_json::json!({
            "paths": {
                "/users":      { "get": {}, "post": {} },
                "/users/{id}": { "get": {}, "delete": {} },
                "/health":     { "get": {} },
            }
        });
        let wanted = vec!["GET".to_string()];
        let mut endpoints = collect_probeable_endpoints(&spec, &wanted);
        endpoints.sort_by(|a, b| a.path.cmp(&b.path));
        assert_eq!(endpoints.len(), 3);
        assert!(endpoints.iter().all(|e| e.method == "GET"));
        let paths: Vec<&str> = endpoints.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, vec!["/health", "/users", "/users/{id}"]);
    }

    #[test]
    fn collect_probeable_endpoints_skips_non_method_keys() {
        let spec = serde_json::json!({
            "paths": {
                "/x": { "get": {}, "summary": "ignored", "parameters": [] }
            }
        });
        let endpoints = collect_probeable_endpoints(&spec, &["GET".to_string()]);
        assert_eq!(endpoints.len(), 1);
    }

    #[test]
    fn collect_probeable_endpoints_handles_missing_paths() {
        let spec = serde_json::json!({});
        let endpoints = collect_probeable_endpoints(&spec, &["GET".to_string()]);
        assert!(endpoints.is_empty());
    }

    #[test]
    fn has_path_parameter_detects_templates() {
        assert!(has_path_parameter("/users/{id}"));
        assert!(has_path_parameter("/orgs/{slug}/members"));
        assert!(!has_path_parameter("/health"));
        assert!(!has_path_parameter("/api/v1/status"));
    }

    #[test]
    fn join_url_normalises_slashes() {
        assert_eq!(join_url("https://x.com", "/users"), "https://x.com/users");
        assert_eq!(join_url("https://x.com/", "/users"), "https://x.com/users");
        assert_eq!(join_url("https://x.com/", "users"), "https://x.com/users");
        assert_eq!(join_url("https://x.com", "users"), "https://x.com/users");
    }

    #[test]
    fn method_from_str_handles_known_and_unknown() {
        assert_eq!(method_from_str("GET"), reqwest::Method::GET);
        assert_eq!(method_from_str("DELETE"), reqwest::Method::DELETE);
        // Unknown collapses to GET (safest fallback).
        assert_eq!(method_from_str("WEIRD"), reqwest::Method::GET);
    }

    #[test]
    fn parse_spec_text_accepts_json() {
        let v = parse_spec_text(r#"{"paths":{"/x":{"get":{}}}}"#).unwrap();
        assert!(v.get("paths").is_some());
    }

    #[test]
    fn parse_spec_text_accepts_yaml() {
        let v = parse_spec_text("paths:\n  /x:\n    get: {}\n").unwrap();
        assert!(v.get("paths").is_some());
    }

    #[test]
    fn parse_spec_text_rejects_garbage() {
        // Pure binary nonsense — not parseable as either JSON or YAML.
        let res = parse_spec_text("\x00\x01\x02not-json-or-yaml: : :");
        assert!(res.is_err(), "expected error, got {res:?}");
    }
}
