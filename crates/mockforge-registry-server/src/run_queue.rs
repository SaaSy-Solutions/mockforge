//! Helpers for pushing test_run jobs onto the Redis queue that the
//! `mockforge-test-runner` worker consumes.
//!
//! The runner crate owns the wire format — see
//! `mockforge_test_runner::queue::QueuedJobDescriptor` — but we don't
//! depend on it here (registry-server → runner would be a backwards
//! dep). Instead this module produces the same JSON shape inline so a
//! drift between the two would be a serde test failure on the runner
//! side, not a build break.

use redis::AsyncCommands;
use serde::Serialize;
use uuid::Uuid;

use crate::redis::RedisPool;

/// Default queue key. Mirrors the runner's `MOCKFORGE_RUNNER_QUEUE_KEY`
/// default. Override at boot via env var if multiple runner pools share
/// one Redis (e.g., dev + staging).
pub const DEFAULT_QUEUE_KEY: &str = "test_runs:queued";

/// JSON shape the runner expects on the wire. Keep in sync with
/// `mockforge_test_runner::queue::QueuedJobDescriptor`.
#[derive(Debug, Serialize)]
pub struct EnqueuedJob<'a> {
    pub run_id: Uuid,
    pub org_id: Uuid,
    pub source_id: Uuid,
    pub kind: &'a str,
    pub payload: serde_json::Value,
}

/// Push a job descriptor onto the runner's queue. When Redis isn't
/// configured (e.g., local dev without a Redis container) this logs a
/// warning and returns Ok — the test_runs row still exists, it just
/// never gets picked up. That matches the existing pattern in other
/// places that treat Redis as optional.
pub async fn enqueue(redis: Option<&RedisPool>, job: EnqueuedJob<'_>) -> redis::RedisResult<()> {
    let Some(redis) = redis else {
        tracing::warn!(
            run_id = %job.run_id,
            kind = job.kind,
            "Redis not configured — test_run will sit in 'queued' until a runner connects",
        );
        return Ok(());
    };

    let raw = match serde_json::to_string(&job) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "failed to serialize queue job; not enqueueing");
            return Ok(());
        }
    };

    // ConnectionManager is Clone, so deref the Arc + clone for the
    // mutable handle redis async commands need.
    let mut conn = (*redis.get_connection()).clone();
    let _: () = conn.lpush(DEFAULT_QUEUE_KEY, raw).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn enqueued_job_serializes_to_runner_wire_format() {
        // The runner deserializes via:
        //   #[derive(Deserialize)]
        //   struct QueuedJobDescriptor {
        //       run_id: Uuid, org_id: Uuid, source_id: Uuid,
        //       kind: String, payload: serde_json::Value,
        //   }
        // Confirm we produce exactly that.
        let run_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();
        let source_id = Uuid::new_v4();
        let job = EnqueuedJob {
            run_id,
            org_id,
            source_id,
            kind: "unit",
            payload: json!({ "synthetic_steps": 5 }),
        };
        let raw = serde_json::to_string(&job).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed["run_id"], run_id.to_string());
        assert_eq!(parsed["org_id"], org_id.to_string());
        assert_eq!(parsed["source_id"], source_id.to_string());
        assert_eq!(parsed["kind"], "unit");
        assert_eq!(parsed["payload"]["synthetic_steps"], 5);
    }
}
