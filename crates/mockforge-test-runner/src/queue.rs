//! Redis queue consumer.
//!
//! The registry pushes new jobs onto a Redis list with `LPUSH`; this
//! module BLPOPs them and yields `RunJob`s for the dispatcher.

use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;

/// Wire format of a queued job descriptor. The registry serializes one
/// of these per run; the runner deserializes and routes by `kind`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedJobDescriptor {
    /// Primary key of the test_runs row.
    pub run_id: Uuid,
    /// Org owning the run; used for billing meter callbacks.
    pub org_id: Uuid,
    /// Suite id (or flow id, monitored_service id, capture_session id,
    /// etc. — depends on `kind`).
    pub source_id: Uuid,
    /// Drives executor selection. See `Executor` impls per file in
    /// `executors::*`.
    pub kind: String,
    /// Opaque per-kind payload deserialized by the executor.
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// Wraps a Redis connection and provides a typed BLPOP loop.
pub struct Consumer {
    redis: redis::aio::ConnectionManager,
    queue_key: String,
    poll_timeout_secs: usize,
}

impl Consumer {
    /// Connect to Redis and prepare the consumer.
    pub async fn connect(
        redis_url: &str,
        queue_key: String,
        poll_timeout_secs: usize,
    ) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let redis = redis::aio::ConnectionManager::new(client).await?;
        Ok(Self {
            redis,
            queue_key,
            poll_timeout_secs,
        })
    }

    /// Block waiting for a job. Returns `None` when the BLPOP timeout
    /// fires with no job available — the caller should loop and try
    /// again, with a chance to handle shutdown signals between calls.
    pub async fn pop(&mut self) -> Result<Option<QueuedJobDescriptor>> {
        // BLPOP returns Option<(key, value)>; we only push to one key
        // so the key half is uninteresting.
        let popped: Option<(String, String)> =
            self.redis.blpop(&self.queue_key, self.poll_timeout_secs as f64).await?;
        let Some((_, raw)) = popped else {
            return Ok(None);
        };
        let job: QueuedJobDescriptor = serde_json::from_str(&raw)?;
        Ok(Some(job))
    }

    /// Push a job back onto the queue. Used for transient executor
    /// failures the dispatcher decides to retry. Goes to the *front* of
    /// the queue so retried jobs don't get starved by new arrivals.
    pub async fn requeue_front(&mut self, job: &QueuedJobDescriptor) -> Result<()> {
        let raw = serde_json::to_string(job)?;
        let _: () = self.redis.lpush(&self.queue_key, raw).await?;
        Ok(())
    }
}

impl From<QueuedJobDescriptor> for crate::executors::RunJob {
    fn from(d: QueuedJobDescriptor) -> Self {
        crate::executors::RunJob {
            run_id: d.run_id,
            org_id: d.org_id,
            source_id: d.source_id,
            kind: d.kind,
            payload: d.payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn descriptor_roundtrips() {
        let d = QueuedJobDescriptor {
            run_id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            source_id: Uuid::new_v4(),
            kind: "snapshot_capture".into(),
            payload: json!({ "name": "pre-chaos" }),
        };
        let raw = serde_json::to_string(&d).unwrap();
        let back: QueuedJobDescriptor = serde_json::from_str(&raw).unwrap();
        assert_eq!(back.run_id, d.run_id);
        assert_eq!(back.kind, "snapshot_capture");
        assert_eq!(back.payload, json!({ "name": "pre-chaos" }));
    }

    #[test]
    fn descriptor_to_run_job() {
        let run_id = Uuid::new_v4();
        let d = QueuedJobDescriptor {
            run_id,
            org_id: Uuid::new_v4(),
            source_id: Uuid::new_v4(),
            kind: "chaos_campaign".into(),
            payload: json!({}),
        };
        let job: crate::executors::RunJob = d.into();
        assert_eq!(job.run_id, run_id);
        assert_eq!(job.kind, "chaos_campaign");
    }

    /// Ensures unknown fields are tolerated (forward-compat with
    /// registry-side schema changes).
    #[test]
    fn descriptor_ignores_unknown_fields() {
        let raw = r#"{
            "run_id": "00000000-0000-0000-0000-000000000001",
            "org_id": "00000000-0000-0000-0000-000000000002",
            "source_id": "00000000-0000-0000-0000-000000000003",
            "kind": "unit",
            "payload": null,
            "future_field": "ignore me"
        }"#;
        let _: QueuedJobDescriptor = serde_json::from_str(raw).expect("should parse");
    }
}
