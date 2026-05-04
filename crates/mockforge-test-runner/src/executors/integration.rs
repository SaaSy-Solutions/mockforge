//! Real integration-test execution for `kind = "integration"` test
//! suites (#356).
//!
//! An integration test is a sequence of HTTP steps with per-step
//! assertions and variable extracts. Mirrors the Integration Test
//! Builder UI shape (`steps[]` + `setup`); the same JSON works for
//! cloud and local. Triggered through `TestExecutor` when the suite's
//! config carries `setup.base_url` and a non-empty `steps` array.

use std::collections::HashMap;
use std::time::Instant;

use serde::Deserialize;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{JobOutcome, JobStatus, RunJob};

/// One step in an integration workflow. Matches the UI's
/// `WorkflowStep` shape so cloud and self-hosted persist the same JSON.
#[derive(Debug, Deserialize)]
struct StepConfig {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    request: RequestConfig,
    #[serde(default)]
    validation: ValidationConfig,
    #[serde(default)]
    extract: Vec<ExtractConfig>,
    #[serde(default)]
    delay_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct RequestConfig {
    method: String,
    path: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    query_params: HashMap<String, String>,
    #[serde(default)]
    body: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Default)]
struct ValidationConfig {
    #[serde(default)]
    status_code: Option<u16>,
    #[serde(default)]
    body_assertions: Vec<BodyAssertion>,
    #[serde(default)]
    header_assertions: Vec<HeaderAssertion>,
    #[serde(default)]
    max_response_time_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct BodyAssertion {
    /// Dotted path into the JSON body.
    path: String,
    /// `equals | contains | exists`. Anything else is treated as
    /// `equals` to be lenient with older configs.
    #[serde(default)]
    operator: Option<String>,
    #[serde(default)]
    value: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct HeaderAssertion {
    name: String,
    /// `equals | contains | exists`.
    #[serde(default)]
    operator: Option<String>,
    #[serde(default)]
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExtractConfig {
    name: String,
    /// `Body | Header | StatusCode`.
    source: String,
    /// JSON path (Body), header name (Header), ignored (StatusCode).
    pattern: String,
    #[serde(default)]
    default: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct SetupConfig {
    #[serde(default)]
    variables: HashMap<String, String>,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct WorkflowConfig {
    #[serde(default)]
    setup: SetupConfig,
    #[serde(default)]
    steps: Vec<StepConfig>,
}

/// Execute an integration workflow end-to-end.
pub async fn run_integration(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    config: &serde_json::Value,
) -> Result<JobOutcome> {
    let workflow: WorkflowConfig = match serde_json::from_value(config.clone()) {
        Ok(w) => w,
        Err(e) => {
            callbacks
                .run_event(
                    job.run_id,
                    1,
                    "log",
                    serde_json::json!({
                        "level": "error",
                        "message": format!("Integration workflow config invalid: {e}"),
                    }),
                )
                .await?;
            return Ok(JobOutcome {
                status: JobStatus::Errored,
                runner_seconds: 1,
                summary: Some(serde_json::json!({
                    "executor_phase": "real",
                    "tracking_task": 356,
                    "kind": "integration",
                    "error": format!("invalid workflow config: {e}"),
                })),
            });
        }
    };
    let base_url = workflow
        .setup
        .base_url
        .clone()
        .unwrap_or_else(|| "http://localhost".to_string());
    let timeout_ms = workflow.setup.timeout_ms.unwrap_or(30_000).clamp(1_000, 600_000);

    callbacks.run_started(job.run_id).await?;
    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!(
                    "Integration workflow: {} step(s), base_url={}, timeout_ms={}",
                    workflow.steps.len(),
                    base_url,
                    timeout_ms,
                ),
                "tracking_task": 356,
            }),
        )
        .await?;

    let mut variables: HashMap<String, serde_json::Value> = workflow
        .setup
        .variables
        .iter()
        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
        .collect();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .user_agent("mockforge-integration/1.0")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut next_seq: u32 = 2;
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut errored = 0u32;

    for (i, step) in workflow.steps.iter().enumerate() {
        if let Some(delay) = step.delay_ms {
            tokio::time::sleep(std::time::Duration::from_millis(delay.min(60_000))).await;
        }
        let outcome =
            execute_step(&client, &base_url, step, &workflow.setup.headers, &variables).await;
        let step_label = step
            .name
            .clone()
            .or_else(|| step.id.clone())
            .unwrap_or_else(|| format!("step-{}", i + 1));
        match outcome {
            StepOutcome::Passed {
                actual_status,
                duration_ms,
                extracted,
                assertion_count,
            } => {
                for (k, v) in extracted {
                    variables.insert(k, v);
                }
                passed += 1;
                callbacks
                    .run_event(
                        job.run_id,
                        next_seq,
                        "step_passed",
                        serde_json::json!({
                            "index": i + 1,
                            "step_id": step.id,
                            "step_name": step_label,
                            "method": step.request.method,
                            "path": step.request.path,
                            "status": actual_status,
                            "duration_ms": duration_ms,
                            "assertion_count": assertion_count,
                        }),
                    )
                    .await?;
            }
            StepOutcome::Failed {
                actual_status,
                duration_ms,
                failures,
            } => {
                failed += 1;
                callbacks
                    .run_event(
                        job.run_id,
                        next_seq,
                        "step_failed",
                        serde_json::json!({
                            "index": i + 1,
                            "step_id": step.id,
                            "step_name": step_label,
                            "method": step.request.method,
                            "path": step.request.path,
                            "status": actual_status,
                            "duration_ms": duration_ms,
                            "failures": failures,
                        }),
                    )
                    .await?;
                break;
            }
            StepOutcome::Error { message } => {
                errored += 1;
                callbacks
                    .run_event(
                        job.run_id,
                        next_seq,
                        "step_error",
                        serde_json::json!({
                            "index": i + 1,
                            "step_id": step.id,
                            "step_name": step_label,
                            "error": message,
                        }),
                    )
                    .await?;
                break;
            }
        }
        next_seq += 1;
    }

    let elapsed = started.elapsed();
    let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
    let total = passed + failed + errored;
    let status = if failed == 0 && errored == 0 && total == workflow.steps.len() as u32 {
        JobStatus::Passed
    } else {
        JobStatus::Failed
    };

    Ok(JobOutcome {
        status,
        runner_seconds: secs,
        summary: Some(serde_json::json!({
            "executor_phase": "real",
            "tracking_task": 356,
            "kind": "integration",
            "steps_total": workflow.steps.len(),
            "steps_passed": passed,
            "steps_failed": failed,
            "steps_errored": errored,
            "wall_ms": elapsed.as_millis() as u64,
        })),
    })
}

enum StepOutcome {
    Passed {
        actual_status: u16,
        duration_ms: u128,
        extracted: HashMap<String, serde_json::Value>,
        assertion_count: usize,
    },
    Failed {
        actual_status: u16,
        duration_ms: u128,
        failures: Vec<String>,
    },
    Error {
        message: String,
    },
}

async fn execute_step(
    client: &reqwest::Client,
    base_url: &str,
    step: &StepConfig,
    setup_headers: &HashMap<String, String>,
    variables: &HashMap<String, serde_json::Value>,
) -> StepOutcome {
    let path = substitute(&step.request.path, variables);
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let method = match reqwest::Method::from_bytes(step.request.method.as_bytes()) {
        Ok(m) => m,
        Err(e) => {
            return StepOutcome::Error {
                message: format!("invalid HTTP method '{}': {e}", step.request.method),
            };
        }
    };

    let mut req = client.request(method, &url);
    // Setup headers first, step headers can override.
    for (k, v) in setup_headers {
        req = req.header(k, substitute(v, variables));
    }
    for (k, v) in &step.request.headers {
        req = req.header(k, substitute(v, variables));
    }
    if !step.request.query_params.is_empty() {
        let qp: Vec<(String, String)> = step
            .request
            .query_params
            .iter()
            .map(|(k, v)| (k.clone(), substitute(v, variables)))
            .collect();
        req = req.query(&qp);
    }
    if let Some(body) = &step.request.body {
        let body_str = match serde_json::to_string(body) {
            Ok(s) => substitute(&s, variables),
            Err(e) => {
                return StepOutcome::Error {
                    message: format!("invalid request body JSON: {e}"),
                };
            }
        };
        req = req.header(reqwest::header::CONTENT_TYPE, "application/json");
        req = req.body(body_str);
    }

    let started = Instant::now();
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            return StepOutcome::Error {
                message: format!("request failed: {e}"),
            };
        }
    };
    let actual_status = resp.status().as_u16();
    let response_headers: HashMap<String, String> = resp
        .headers()
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.as_str().to_string(), s.to_string())))
        .collect();
    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            return StepOutcome::Error {
                message: format!("response body read failed: {e}"),
            };
        }
    };
    let duration_ms = started.elapsed().as_millis();
    let json_body: Option<serde_json::Value> = serde_json::from_slice(&bytes).ok();
    let text_body: String = String::from_utf8_lossy(&bytes).to_string();

    // Run assertions; collect every failure so the user sees the full
    // picture, not just the first mismatch.
    let mut failures: Vec<String> = Vec::new();

    if let Some(expected) = step.validation.status_code {
        if actual_status != expected {
            failures.push(format!("status_code: expected {expected}, got {actual_status}",));
        }
    }
    if let Some(max_ms) = step.validation.max_response_time_ms {
        if (duration_ms as u64) > max_ms {
            failures
                .push(format!("max_response_time_ms: expected <= {max_ms}, got {duration_ms}",));
        }
    }
    let body_count = step.validation.body_assertions.len();
    let header_count = step.validation.header_assertions.len();
    for assertion in &step.validation.body_assertions {
        if let Some(reason) = check_body_assertion(json_body.as_ref(), &text_body, assertion) {
            failures.push(reason);
        }
    }
    for assertion in &step.validation.header_assertions {
        if let Some(reason) = check_header_assertion(&response_headers, assertion) {
            failures.push(reason);
        }
    }

    if !failures.is_empty() {
        return StepOutcome::Failed {
            actual_status,
            duration_ms,
            failures,
        };
    }

    let mut extracted = HashMap::new();
    for ex in &step.extract {
        let value = match ex.source.as_str() {
            "Body" => json_body
                .as_ref()
                .and_then(|j| jsonpath_lookup(j, &ex.pattern))
                .or_else(|| ex.default.as_ref().map(|d| serde_json::Value::String(d.clone()))),
            "Header" => response_headers
                .get(&ex.pattern)
                .map(|s| serde_json::Value::String(s.clone()))
                .or_else(|| {
                    response_headers
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case(&ex.pattern))
                        .map(|(_, v)| serde_json::Value::String(v.clone()))
                })
                .or_else(|| ex.default.as_ref().map(|d| serde_json::Value::String(d.clone()))),
            "StatusCode" => Some(serde_json::Value::Number(actual_status.into())),
            _ => None,
        };
        if let Some(v) = value {
            extracted.insert(ex.name.clone(), v);
        }
    }

    StepOutcome::Passed {
        actual_status,
        duration_ms,
        extracted,
        assertion_count: body_count
            + header_count
            + (step.validation.status_code.is_some() as usize)
            + (step.validation.max_response_time_ms.is_some() as usize),
    }
}

fn check_body_assertion(
    json_body: Option<&serde_json::Value>,
    text_body: &str,
    assertion: &BodyAssertion,
) -> Option<String> {
    let op = assertion.operator.as_deref().unwrap_or("equals");
    if op == "exists" {
        let exists = json_body.and_then(|j| jsonpath_lookup(j, &assertion.path)).is_some();
        return if exists {
            None
        } else {
            Some(format!("body.{}: expected to exist", assertion.path))
        };
    }
    let actual = json_body.and_then(|j| jsonpath_lookup(j, &assertion.path));
    match (op, actual, assertion.value.as_ref()) {
        ("equals", Some(actual), Some(expected)) => {
            if &actual == expected {
                None
            } else {
                Some(format!("body.{}: expected {expected}, got {actual}", assertion.path))
            }
        }
        ("contains", Some(actual), Some(expected)) => {
            let actual_str = actual.as_str().unwrap_or(text_body);
            let expected_str = expected.as_str().unwrap_or("");
            if actual_str.contains(expected_str) {
                None
            } else {
                Some(format!(
                    "body.{}: expected to contain '{expected_str}', got '{actual_str}'",
                    assertion.path
                ))
            }
        }
        (op, None, _) => {
            Some(format!("body.{}: cannot evaluate '{op}' on missing value", assertion.path,))
        }
        (op, _, _) => Some(format!(
            "body.{}: unsupported operator '{op}' or missing expected value",
            assertion.path,
        )),
    }
}

fn check_header_assertion(
    headers: &HashMap<String, String>,
    assertion: &HeaderAssertion,
) -> Option<String> {
    let actual = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(&assertion.name))
        .map(|(_, v)| v.clone());
    let op = assertion.operator.as_deref().unwrap_or("equals");
    match (op, actual.as_deref(), assertion.value.as_deref()) {
        ("exists", Some(_), _) => None,
        ("exists", None, _) => Some(format!("header.{}: expected to be present", assertion.name)),
        ("equals", Some(a), Some(e)) if a == e => None,
        ("equals", actual, expected) => Some(format!(
            "header.{}: expected '{}', got '{}'",
            assertion.name,
            expected.unwrap_or("<missing>"),
            actual.unwrap_or("<missing>"),
        )),
        ("contains", Some(a), Some(e)) if a.contains(e) => None,
        ("contains", actual, expected) => Some(format!(
            "header.{}: expected to contain '{}', got '{}'",
            assertion.name,
            expected.unwrap_or("<missing>"),
            actual.unwrap_or("<missing>"),
        )),
        (op, _, _) => Some(format!("header.{}: unsupported operator '{op}'", assertion.name)),
    }
}

/// Replace `${var}` occurrences using the variable bag. Missing
/// variables are left as-is so they fail loudly downstream.
fn substitute(template: &str, variables: &HashMap<String, serde_json::Value>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = match after.find('}') {
            Some(e) => e,
            None => {
                out.push_str(&rest[start..]);
                rest = "";
                break;
            }
        };
        let name = &after[..end];
        match variables.get(name) {
            Some(serde_json::Value::String(s)) => out.push_str(s),
            Some(other) => out.push_str(&other.to_string()),
            None => {
                out.push_str("${");
                out.push_str(name);
                out.push('}');
            }
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

fn jsonpath_lookup(value: &serde_json::Value, path: &str) -> Option<serde_json::Value> {
    let mut current = value;
    let trimmed = path.trim_start_matches('$').trim_start_matches('.');
    for segment in trimmed.split('.').filter(|s| !s.is_empty()) {
        if let Some(bracket_idx) = segment.find('[') {
            let key = &segment[..bracket_idx];
            if !key.is_empty() {
                current = current.get(key)?;
            }
            let rest = &segment[bracket_idx..];
            current = follow_indexes(current, rest)?;
        } else {
            current = current.get(segment)?;
        }
    }
    Some(current.clone())
}

fn follow_indexes<'a>(
    mut current: &'a serde_json::Value,
    rest: &str,
) -> Option<&'a serde_json::Value> {
    let mut chunk = rest;
    while let Some(open) = chunk.find('[') {
        let close = chunk.find(']')?;
        if open >= close {
            return None;
        }
        let idx_str = &chunk[open + 1..close];
        let idx: usize = idx_str.parse().ok()?;
        current = current.get(idx)?;
        chunk = &chunk[close + 1..];
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn substitute_replaces_known() {
        let mut vars = HashMap::new();
        vars.insert("id".into(), json!("abc"));
        assert_eq!(substitute("/users/${id}", &vars), "/users/abc");
    }

    #[test]
    fn body_assertion_equals_passes() {
        let body = json!({ "user": { "id": 7 } });
        let a = BodyAssertion {
            path: "user.id".into(),
            operator: Some("equals".into()),
            value: Some(json!(7)),
        };
        assert!(check_body_assertion(Some(&body), "", &a).is_none());
    }

    #[test]
    fn body_assertion_equals_fails() {
        let body = json!({ "user": { "id": 7 } });
        let a = BodyAssertion {
            path: "user.id".into(),
            operator: Some("equals".into()),
            value: Some(json!(8)),
        };
        assert!(check_body_assertion(Some(&body), "", &a).is_some());
    }

    #[test]
    fn body_assertion_exists_missing_field() {
        let body = json!({ "a": 1 });
        let a = BodyAssertion {
            path: "b.c".into(),
            operator: Some("exists".into()),
            value: None,
        };
        assert!(check_body_assertion(Some(&body), "", &a).is_some());
    }

    #[test]
    fn header_assertion_case_insensitive() {
        let mut h = HashMap::new();
        h.insert("Content-Type".into(), "application/json".into());
        let a = HeaderAssertion {
            name: "content-type".into(),
            operator: Some("contains".into()),
            value: Some("json".into()),
        };
        assert!(check_header_assertion(&h, &a).is_none());
    }
}
