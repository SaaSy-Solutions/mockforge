//! Real chain execution for `kind = "chain"` flows (#354).
//!
//! A chain is a sequence of HTTP requests where later requests can
//! reference values extracted from earlier responses (`${var}`
//! substitution). The local self-hosted Chains page has the same
//! semantics; this module executes the same `ChainDefinition` config
//! against whatever target URL the chain points at, in cloud mode.
//!
//! Triggered through `FlowExecutor` when:
//!   * `kind == "chain"`
//!   * `config.links` is a non-empty array
//!
//! Otherwise FlowExecutor falls back to its synthetic-pass mode so the
//! UI still shows progress events for chains authored with no links yet.

use std::collections::HashMap;
use std::time::Instant;

use serde::Deserialize;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{JobOutcome, JobStatus, RunJob};

/// One step in a chain. Mirrors the UI's `ChainLink` shape so cloud and
/// self-hosted persist the same JSON.
#[derive(Debug, Deserialize)]
struct ChainLinkConfig {
    request: ChainRequestConfig,
    /// Optional extraction map: variable_name -> dotted JSON path. The
    /// path is evaluated against the parsed response body and the
    /// resulting scalar is stashed in the chain's variable bag for
    /// later `${name}` substitutions.
    #[serde(default)]
    extract: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct ChainRequestConfig {
    #[serde(default)]
    id: Option<String>,
    method: String,
    url: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    body: Option<serde_json::Value>,
    #[serde(default, rename = "timeoutSecs", alias = "timeout_secs")]
    timeout_secs: Option<u64>,
    #[serde(default, rename = "expectedStatus", alias = "expected_status")]
    expected_status: Option<Vec<u16>>,
}

/// Execute a chain end-to-end. Streams events back through
/// `callbacks` and returns a final outcome.
pub async fn run_chain(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    config: &serde_json::Value,
) -> Result<JobOutcome> {
    let links: Vec<ChainLinkConfig> = match config.get("links").and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| serde_json::from_value::<ChainLinkConfig>(v.clone()).ok())
            .collect(),
        None => Vec::new(),
    };

    let global_timeout = config
        .get("config")
        .and_then(|c| c.get("globalTimeoutSecs"))
        .and_then(|v| v.as_u64())
        .unwrap_or(60)
        .clamp(1, 600);

    callbacks.run_started(job.run_id).await?;
    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!(
                    "Chain execution: {} link(s), global_timeout={}s",
                    links.len(),
                    global_timeout,
                ),
                "tracking_task": 354,
            }),
        )
        .await?;

    let initial_variables: HashMap<String, serde_json::Value> = config
        .get("variables")
        .and_then(|v| v.as_object())
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(global_timeout))
        .user_agent("mockforge-chain/1.0")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut variables = initial_variables;
    let mut next_seq: u32 = 2;
    let mut succeeded = 0u32;
    let mut failed = 0u32;
    let mut errored = 0u32;

    for (i, link) in links.iter().enumerate() {
        let outcome = execute_link(&client, link, &variables).await;
        match outcome {
            LinkOutcome::Ok {
                actual_status,
                duration_ms,
                extracted,
            } => {
                for (k, v) in extracted {
                    variables.insert(k, v);
                }
                succeeded += 1;
                callbacks
                    .run_event(
                        job.run_id,
                        next_seq,
                        "chain_step",
                        serde_json::json!({
                            "index": i + 1,
                            "step_id": link.request.id,
                            "method": link.request.method,
                            "url": link.request.url,
                            "status": actual_status,
                            "matched_expected_status": true,
                            "duration_ms": duration_ms,
                        }),
                    )
                    .await?;
            }
            LinkOutcome::StatusMismatch {
                expected,
                actual,
                duration_ms,
            } => {
                failed += 1;
                callbacks
                    .run_event(
                        job.run_id,
                        next_seq,
                        "chain_step",
                        serde_json::json!({
                            "index": i + 1,
                            "step_id": link.request.id,
                            "method": link.request.method,
                            "url": link.request.url,
                            "status": actual,
                            "expected_status": expected,
                            "matched_expected_status": false,
                            "duration_ms": duration_ms,
                        }),
                    )
                    .await?;
                break;
            }
            LinkOutcome::Error { message } => {
                errored += 1;
                callbacks
                    .run_event(
                        job.run_id,
                        next_seq,
                        "chain_step",
                        serde_json::json!({
                            "index": i + 1,
                            "step_id": link.request.id,
                            "method": link.request.method,
                            "url": link.request.url,
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
    let total = succeeded + failed + errored;
    let executed_count = total;
    let status = if failed == 0 && errored == 0 && total == links.len() as u32 {
        JobStatus::Passed
    } else {
        JobStatus::Failed
    };

    Ok(JobOutcome {
        status,
        runner_seconds: secs,
        summary: Some(serde_json::json!({
            "executor_phase": "real",
            "tracking_task": 354,
            "kind": "chain",
            "links_total": links.len(),
            "links_executed": executed_count,
            "links_succeeded": succeeded,
            "links_failed": failed,
            "links_errored": errored,
            "wall_ms": elapsed.as_millis() as u64,
        })),
    })
}

enum LinkOutcome {
    Ok {
        actual_status: u16,
        duration_ms: u128,
        extracted: HashMap<String, serde_json::Value>,
    },
    StatusMismatch {
        expected: Vec<u16>,
        actual: u16,
        duration_ms: u128,
    },
    Error {
        message: String,
    },
}

async fn execute_link(
    client: &reqwest::Client,
    link: &ChainLinkConfig,
    variables: &HashMap<String, serde_json::Value>,
) -> LinkOutcome {
    let url = substitute(&link.request.url, variables);
    let method = match reqwest::Method::from_bytes(link.request.method.as_bytes()) {
        Ok(m) => m,
        Err(e) => {
            return LinkOutcome::Error {
                message: format!("invalid HTTP method '{}': {e}", link.request.method),
            };
        }
    };

    let mut req = client.request(method, &url);
    for (k, v) in &link.request.headers {
        req = req.header(k, substitute(v, variables));
    }
    if let Some(body) = &link.request.body {
        // Substitution into structured bodies happens on the serialized
        // form so users can write \"${user_id}\" inline anywhere.
        let body_str = match serde_json::to_string(body) {
            Ok(s) => substitute(&s, variables),
            Err(e) => {
                return LinkOutcome::Error {
                    message: format!("invalid request body JSON: {e}"),
                };
            }
        };
        req = req.header(reqwest::header::CONTENT_TYPE, "application/json");
        req = req.body(body_str);
    }
    if let Some(timeout) = link.request.timeout_secs {
        req = req.timeout(std::time::Duration::from_secs(timeout.clamp(1, 300)));
    }

    let started = Instant::now();
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            return LinkOutcome::Error {
                message: format!("request failed: {e}"),
            };
        }
    };
    let actual = resp.status().as_u16();
    let duration_ms = started.elapsed().as_millis();

    if let Some(expected) = &link.request.expected_status {
        if !expected.is_empty() && !expected.contains(&actual) {
            return LinkOutcome::StatusMismatch {
                expected: expected.clone(),
                actual,
                duration_ms,
            };
        }
    }

    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            return LinkOutcome::Error {
                message: format!("response body read failed: {e}"),
            };
        }
    };
    let json_body: Option<serde_json::Value> = serde_json::from_slice(&bytes).ok();

    let mut extracted = HashMap::new();
    if let Some(json) = &json_body {
        for (var_name, path) in &link.extract {
            if let Some(value) = jsonpath_lookup(json, path) {
                extracted.insert(var_name.clone(), value);
            }
        }
    }

    LinkOutcome::Ok {
        actual_status: actual,
        duration_ms,
        extracted,
    }
}

/// Replace `${var_name}` occurrences with the variable's stringified
/// value. Missing variables are left as-is so they fail loudly in the
/// downstream HTTP request rather than silently turning into the empty
/// string.
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
            Some(v) => match v {
                serde_json::Value::String(s) => out.push_str(s),
                other => out.push_str(&other.to_string()),
            },
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

/// Look up a dotted path against a JSON value. Supports `a.b.c`,
/// `a[0]`, and combinations. Returns `None` if any segment is missing
/// or the indexing kind doesn't match the target.
fn jsonpath_lookup(value: &serde_json::Value, path: &str) -> Option<serde_json::Value> {
    let mut current = value;
    let trimmed = path.trim_start_matches('$').trim_start_matches('.');
    for segment in trimmed.split('.').filter(|s| !s.is_empty()) {
        // A segment can carry `[N]` indexes after the key name.
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
    fn substitute_replaces_known_vars() {
        let mut vars = HashMap::new();
        vars.insert("id".into(), json!("abc"));
        assert_eq!(substitute("/users/${id}/orders", &vars), "/users/abc/orders");
    }

    #[test]
    fn substitute_leaves_unknown_vars_intact() {
        let vars = HashMap::new();
        assert_eq!(substitute("/users/${id}", &vars), "/users/${id}",);
    }

    #[test]
    fn jsonpath_dotted() {
        let v = json!({ "data": { "user": { "id": 42 } } });
        assert_eq!(jsonpath_lookup(&v, "data.user.id"), Some(json!(42)));
    }

    #[test]
    fn jsonpath_indexed() {
        let v = json!({ "items": [{ "name": "a" }, { "name": "b" }] });
        assert_eq!(jsonpath_lookup(&v, "items[1].name"), Some(json!("b")));
    }

    #[test]
    fn jsonpath_missing_segment_returns_none() {
        let v = json!({ "a": 1 });
        assert_eq!(jsonpath_lookup(&v, "b.c"), None);
    }
}
