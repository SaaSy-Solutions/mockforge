//! Replay executor (#6 / Phase 3). Handles `replay`.
//!
//! Real mode: when payload contains a `target_url` and a `session_id`
//! (set by the registry's replay trigger handler), the executor fetches
//! the session's captured exchanges via the internal API, replays each
//! request against `target_url` with reqwest, and reports per-capture
//! match/mismatch events. The match check compares actual response
//! status code against the recorded one.
//!
//! A missing `target_url`/`session_id`, a failed capture lookup, or an
//! empty session finishes the run `errored` with a `status_reason`
//! (#1264). These cases used to emit synthetic `request_replayed`
//! events with `matched=true` and report `passed`.

use async_trait::async_trait;
use std::time::Instant;

use crate::callbacks::{CaptureExchange, RegistryCallbacks};
use crate::error::Result;
use crate::executors::{errored_with_reason, Executor, JobOutcome, JobStatus, RunJob};

/// Executor for capture-session replay.
pub struct ReplayExecutor;

#[async_trait]
impl Executor for ReplayExecutor {
    fn kind(&self) -> &'static str {
        "replay"
    }

    async fn execute(&self, job: RunJob, callbacks: &RegistryCallbacks) -> Result<JobOutcome> {
        let started = Instant::now();
        callbacks.run_started(job.run_id).await?;

        let target_url = job.payload.get("target_url").and_then(|v| v.as_str()).map(String::from);
        let session_id = job
            .payload
            .get("session_id")
            .and_then(|v| v.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok());

        // Replay requires both a target_url and a session_id we can ask
        // the registry about.
        let target_url = target_url.filter(|u| !u.trim().is_empty());
        match (target_url.as_deref(), session_id) {
            (Some(target_url), Some(session_id)) => {
                run_real_replay(job, callbacks, started, target_url, session_id).await
            }
            _ => {
                errored_with_reason(
                    &job,
                    callbacks,
                    started,
                    1,
                    "replay_target_missing",
                    "replay requires `target_url` and `session_id`; nothing was replayed"
                        .to_string(),
                )
                .await
            }
        }
    }
}

async fn run_real_replay(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
    target_url: &str,
    session_id: uuid::Uuid,
) -> Result<JobOutcome> {
    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!(
                    "Real replay against {target_url} (session {session_id})",
                ),
                "synthetic": false,
                "tracking_task": 6,
            }),
        )
        .await?;

    let exchanges = match callbacks.fetch_capture_exchanges(session_id).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, %session_id, "fetch_capture_exchanges failed");
            return errored_with_reason(
                &job,
                callbacks,
                started,
                2,
                "captures_unavailable",
                format!("fetch_capture_exchanges failed: {e}; nothing was replayed"),
            )
            .await;
        }
    };

    if exchanges.is_empty() {
        return errored_with_reason(
            &job,
            callbacks,
            started,
            2,
            "no_captures",
            "session has no captured exchanges; nothing was replayed".to_string(),
        )
        .await;
    }

    let client = crate::target_client::TargetClient::new(
        std::time::Duration::from_secs(30),
        "mockforge-replay/1.0",
    )?;

    let mut next_seq: u32 = 2;
    let mut matched_count = 0u32;
    let mut mismatched_count = 0u32;
    let mut errored_count = 0u32;

    for (i, ex) in exchanges.iter().enumerate() {
        let result = replay_one(&client, target_url, ex).await;
        match result {
            ReplayResult::Match {
                actual_status,
                duration_ms,
            } => {
                matched_count += 1;
                callbacks
                    .run_event(
                        job.run_id,
                        next_seq,
                        "request_replayed",
                        serde_json::json!({
                            "index": i + 1,
                            "capture_id": ex.capture_id,
                            "method": ex.method,
                            "path": ex.path,
                            "matched": true,
                            "expected_status": ex.response_status_code,
                            "actual_status": actual_status,
                            "duration_ms": duration_ms,
                        }),
                    )
                    .await?;
            }
            ReplayResult::Mismatch {
                expected,
                actual,
                duration_ms,
            } => {
                mismatched_count += 1;
                callbacks
                    .run_event(
                        job.run_id,
                        next_seq,
                        "request_replayed",
                        serde_json::json!({
                            "index": i + 1,
                            "capture_id": ex.capture_id,
                            "method": ex.method,
                            "path": ex.path,
                            "matched": false,
                            "expected_status": expected,
                            "actual_status": actual,
                            "duration_ms": duration_ms,
                        }),
                    )
                    .await?;
            }
            ReplayResult::Error { error } => {
                errored_count += 1;
                callbacks
                    .run_event(
                        job.run_id,
                        next_seq,
                        "request_replayed",
                        serde_json::json!({
                            "index": i + 1,
                            "capture_id": ex.capture_id,
                            "method": ex.method,
                            "path": ex.path,
                            "matched": false,
                            "error": error,
                        }),
                    )
                    .await?;
            }
        }
        next_seq += 1;
    }

    let elapsed = started.elapsed();
    let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
    let total = matched_count + mismatched_count + errored_count;
    // Fail when a non-trivial share of replays mismatched/errored, so
    // CI can fail the build. 100% match = passed. Anything else = failed.
    let status = if mismatched_count == 0 && errored_count == 0 {
        JobStatus::Passed
    } else {
        JobStatus::Failed
    };

    Ok(JobOutcome {
        status,
        runner_seconds: secs,
        work_count: u64::from(total),
        summary: Some(serde_json::json!({
            "executor_phase": "real",
            "tracking_task": 6,
            "target_url": target_url,
            "session_id": session_id,
            "captures_replayed": total,
            "matched": matched_count,
            "mismatched": mismatched_count,
            "errored": errored_count,
            "wall_ms": elapsed.as_millis() as u64,
        })),
    })
}

enum ReplayResult {
    Match {
        actual_status: u16,
        duration_ms: u128,
    },
    Mismatch {
        expected: Option<i32>,
        actual: u16,
        duration_ms: u128,
    },
    Error {
        error: String,
    },
}

async fn replay_one(
    client: &crate::target_client::TargetClient,
    target_url: &str,
    ex: &CaptureExchange,
) -> ReplayResult {
    let url = format!("{}{}", target_url.trim_end_matches('/'), ex.path);
    let method = match reqwest::Method::from_bytes(ex.method.as_bytes()) {
        Ok(m) => m,
        Err(e) => {
            return ReplayResult::Error {
                error: format!("invalid HTTP method '{}': {e}", ex.method),
            };
        }
    };

    let mut req = match client.request(method, &url) {
        Ok(r) => r,
        Err(e) => {
            return ReplayResult::Error {
                error: format!("url rejected by SSRF guard: {e}"),
            };
        }
    };

    // Replay request body if present. The recorder stores body as a
    // string + encoding label ('utf8' / 'base64' / 'binary'). For non-
    // utf8 bodies we just pass the raw string through — the recorder's
    // shipper is the one that decided the encoding so trust that on
    // first cut.
    if let Some(body) = ex.request_body.as_deref() {
        if !body.is_empty() {
            req = req.body(body.to_string());
        }
    }

    let started = Instant::now();
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            return ReplayResult::Error {
                error: format!("{e}"),
            };
        }
    };
    let duration_ms = started.elapsed().as_millis();
    let actual = resp.status().as_u16();
    match ex.response_status_code {
        Some(expected) if expected as u16 == actual => ReplayResult::Match {
            actual_status: actual,
            duration_ms,
        },
        Some(expected) => ReplayResult::Mismatch {
            expected: Some(expected),
            actual,
            duration_ms,
        },
        None => {
            // No recorded status — count as match if we got any 2xx;
            // otherwise mismatch so the operator sees something
            // happened.
            if (200..300).contains(&actual) {
                ReplayResult::Match {
                    actual_status: actual,
                    duration_ms,
                }
            } else {
                ReplayResult::Mismatch {
                    expected: None,
                    actual,
                    duration_ms,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executors::test_support::{job, reason, FakeHttp};

    async fn replay_with(status: u16, body: &'static str) -> JobOutcome {
        let registry = FakeHttp::start(move |line| {
            if line.contains("/exchanges") {
                (status, body.to_string())
            } else {
                (200, "{}".to_string())
            }
        })
        .await;
        let callbacks = RegistryCallbacks::new(registry.base_url.clone(), "t".into());
        let payload = serde_json::json!({
            "target_url": "https://example.invalid",
            "session_id": uuid::Uuid::new_v4().to_string(),
        });
        ReplayExecutor
            .execute(job("replay", payload), &callbacks)
            .await
            .expect("outcome")
            .finalize()
    }

    #[tokio::test]
    async fn empty_session_errors_instead_of_passing() {
        let o = replay_with(200, "[]").await;
        assert_eq!(o.status, JobStatus::Errored);
        assert_eq!(reason(&o).as_deref(), Some("no_captures"));
        assert_eq!(o.work_count, 0);
    }

    #[tokio::test]
    async fn capture_lookup_failure_errors_instead_of_passing() {
        let o = replay_with(500, "{}").await;
        assert_eq!(o.status, JobStatus::Errored);
        assert_eq!(reason(&o).as_deref(), Some("captures_unavailable"));
    }
}
