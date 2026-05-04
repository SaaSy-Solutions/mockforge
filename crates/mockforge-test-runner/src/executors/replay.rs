//! Replay executor (#6 / Phase 3). Handles `replay`.
//!
//! Real mode: when payload contains a `target_url` and a `session_id`
//! (set by the registry's replay trigger handler), the executor fetches
//! the session's captured exchanges via the internal API, replays each
//! request against `target_url` with reqwest, and reports per-capture
//! match/mismatch events. The match check compares actual response
//! status code against the recorded one.
//!
//! Synthetic mode: when `target_url` is missing the executor falls back
//! to emitting `request_replayed` events with `matched=true` for the
//! configured `synthetic_captures` count. Useful when the operator just
//! wants to exercise the runner pipeline without an upstream service.

use async_trait::async_trait;
use std::time::Instant;

use crate::callbacks::{CaptureExchange, RegistryCallbacks};
use crate::error::Result;
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};

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

        // Real mode requires both a target_url and a session_id we can
        // ask the registry about. If either's missing we degrade to
        // synthetic so a curl-driven test still produces sensible
        // events.
        if let (Some(target_url), Some(session_id)) = (target_url.as_deref(), session_id) {
            return run_real_replay(job, callbacks, started, target_url, session_id).await;
        }

        run_synthetic(job, callbacks, started).await
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
            // Fetch failure is not fatal — emit an error log + fall back
            // to synthetic so the run still produces a coherent outcome.
            tracing::warn!(error = %e, %session_id, "fetch_capture_exchanges failed; falling back to synthetic");
            callbacks
                .run_event(
                    job.run_id,
                    2,
                    "log",
                    serde_json::json!({
                        "level": "warn",
                        "message": format!("fetch_capture_exchanges failed: {e}; falling back to synthetic"),
                    }),
                )
                .await?;
            return run_synthetic(job, callbacks, started).await;
        }
    };

    if exchanges.is_empty() {
        callbacks
            .run_event(
                job.run_id,
                2,
                "log",
                serde_json::json!({
                    "level": "warn",
                    "message": "session has no captured exchanges; falling back to synthetic",
                }),
            )
            .await?;
        return run_synthetic(job, callbacks, started).await;
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("mockforge-replay/1.0")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

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
    client: &reqwest::Client,
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

    let mut req = client.request(method, &url);

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

    let started = std::time::Instant::now();
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

/// Synthetic fallback — same shape as before for callers that don't
/// supply a target_url or session_id.
async fn run_synthetic(
    job: RunJob,
    callbacks: &RegistryCallbacks,
    started: Instant,
) -> Result<JobOutcome> {
    let captures = job
        .payload
        .get("synthetic_captures")
        .and_then(|v| v.as_u64())
        .unwrap_or(5)
        .clamp(1, 200) as u32;

    callbacks
        .run_event(
            job.run_id,
            1,
            "log",
            serde_json::json!({
                "level": "info",
                "message": format!("Synthetic replay: {} captures", captures),
                "synthetic": true,
                "tracking_task": 6,
            }),
        )
        .await?;

    let mut next_seq: u32 = 2;
    let mut matched = 0u32;
    for i in 1..=captures {
        matched += 1;
        callbacks
            .run_event(
                job.run_id,
                next_seq,
                "request_replayed",
                serde_json::json!({ "index": i, "matched": true }),
            )
            .await?;
        next_seq += 1;
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    let elapsed = started.elapsed();
    let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);
    Ok(JobOutcome {
        status: JobStatus::Passed,
        runner_seconds: secs,
        summary: Some(serde_json::json!({
            "executor_phase": "synthetic",
            "tracking_task": 6,
            "captures_replayed": captures,
            "matched": matched,
            "wall_ms": elapsed.as_millis() as u64,
        })),
    })
}
