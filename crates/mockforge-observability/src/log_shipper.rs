//! Structured request log shipper for hosted-mock deployments.
//!
//! `mockforge-cli serve` runs this in-container when configured by the
//! orchestrator, capturing one event per HTTP request and forwarding the
//! batch to MockForge Cloud's log-ingest endpoint. The Cloud admin UI then
//! reads them back via the per-deployment "Requests" tab.
//!
//! Phase 2 (#224) gave hosted mocks Fly's container stdout/stderr. That's
//! good for "did the app boot" but not for "what URL did the user just hit
//! and what did we return." This module fills that gap.
//!
//! ## Configuration
//!
//! All env vars are optional — when any required one is missing the shipper
//! is a no-op and `enqueue` calls drop their events. The orchestrator sets
//! these on hosted-mock Fly machines:
//!
//! - `MOCKFORGE_LOG_INGEST_URL` — full URL to the ingest endpoint, e.g.
//!   `https://api.mockforge.dev/api/v1/hosted-mocks/<id>/log-ingest`.
//! - `MOCKFORGE_LOG_INGEST_TOKEN` — short-lived JWT scoped to the deployment.
//! - `MOCKFORGE_LOG_INGEST_BATCH_SIZE` — events per POST (default 50).
//! - `MOCKFORGE_LOG_INGEST_FLUSH_MS` — max batch age before flush (default 2000).
//! - `MOCKFORGE_LOG_INGEST_BUFFER` — bounded channel capacity (default 1024).
//!   When full, oldest events are dropped — observability code must never
//!   block the request path.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// One captured request/response pair. Fields kept thin so the shipper has
/// no opinion on storage schema — the cloud side decides what to keep.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLogEvent {
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub latency_ms: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_in: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_out: Option<u64>,
}

#[derive(Debug, Serialize)]
struct IngestPayload<'a> {
    events: &'a [RequestLogEvent],
}

/// Cheap cloneable handle. `enqueue` is non-blocking and never fails — it's
/// safe to call from the request path. When the shipper isn't configured
/// the handle is `None` and calls are zero-cost.
#[derive(Clone)]
pub struct LogShipperHandle {
    inner: Option<Arc<Inner>>,
}

struct Inner {
    sender: mpsc::Sender<RequestLogEvent>,
}

impl LogShipperHandle {
    /// Construct a no-op handle. Used when the shipper isn't configured —
    /// the request middleware can still call `enqueue` without checking.
    pub fn disabled() -> Self {
        Self { inner: None }
    }

    /// Non-blocking enqueue. Drops the event silently when the buffer is
    /// full, which is the right tradeoff for a request-path component:
    /// observability never blocks user traffic.
    pub fn enqueue(&self, event: RequestLogEvent) {
        if let Some(inner) = &self.inner {
            let _ = inner.sender.try_send(event);
        }
    }

    /// True if the shipper is actually running. Useful for skipping work
    /// in middleware (e.g., decoding the user-agent header).
    pub fn is_active(&self) -> bool {
        self.inner.is_some()
    }
}

/// Construct a shipper from environment variables. Returns a no-op handle
/// when required env vars are missing (this is the common case in local
/// dev). Spawns a background task that drains the channel and POSTs to the
/// configured ingest URL; the task runs for the lifetime of the process.
pub fn from_env() -> LogShipperHandle {
    let url = match std::env::var("MOCKFORGE_LOG_INGEST_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => return LogShipperHandle::disabled(),
    };
    let token = match std::env::var("MOCKFORGE_LOG_INGEST_TOKEN") {
        Ok(t) if !t.trim().is_empty() => t,
        _ => return LogShipperHandle::disabled(),
    };
    let batch_size: usize = std::env::var("MOCKFORGE_LOG_INGEST_BATCH_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);
    let flush_ms: u64 = std::env::var("MOCKFORGE_LOG_INGEST_FLUSH_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000);
    let buffer: usize = std::env::var("MOCKFORGE_LOG_INGEST_BUFFER")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1024);

    let client = match reqwest::Client::builder().timeout(Duration::from_secs(5)).build() {
        Ok(c) => c,
        Err(e) => {
            warn!("LogShipper HTTP client init failed: {}", e);
            return LogShipperHandle::disabled();
        }
    };

    let (sender, receiver) = mpsc::channel::<RequestLogEvent>(buffer);
    let inner = Arc::new(Inner { sender });

    tokio::spawn(run(receiver, client, url, token, batch_size, flush_ms));

    LogShipperHandle { inner: Some(inner) }
}

/// Background task: batch by count or by elapsed time, then POST. Errors
/// are warn-logged and dropped — request volume on a healthy mock is far
/// higher than retry capacity is worth, and the cloud-side ingest is the
/// canonical store; we don't need at-least-once.
async fn run(
    mut receiver: mpsc::Receiver<RequestLogEvent>,
    client: reqwest::Client,
    url: String,
    token: String,
    batch_size: usize,
    flush_ms: u64,
) {
    let flush_after = Duration::from_millis(flush_ms);
    let mut batch: Vec<RequestLogEvent> = Vec::with_capacity(batch_size);
    let mut deadline = tokio::time::Instant::now() + flush_after;

    loop {
        let timeout = tokio::time::sleep_until(deadline);
        tokio::pin!(timeout);

        tokio::select! {
            biased;
            evt = receiver.recv() => {
                match evt {
                    Some(e) => {
                        batch.push(e);
                        if batch.len() >= batch_size {
                            send_batch(&client, &url, &token, &batch).await;
                            batch.clear();
                            deadline = tokio::time::Instant::now() + flush_after;
                        }
                    }
                    None => {
                        // Channel closed (process shutting down). Best-effort
                        // final flush.
                        if !batch.is_empty() {
                            send_batch(&client, &url, &token, &batch).await;
                        }
                        return;
                    }
                }
            }
            _ = &mut timeout => {
                if !batch.is_empty() {
                    send_batch(&client, &url, &token, &batch).await;
                    batch.clear();
                }
                deadline = tokio::time::Instant::now() + flush_after;
            }
        }
    }
}

async fn send_batch(client: &reqwest::Client, url: &str, token: &str, events: &[RequestLogEvent]) {
    let payload = IngestPayload { events };
    debug!(count = events.len(), "Shipping log batch");
    match client.post(url).bearer_auth(token).json(&payload).send().await {
        Ok(resp) if resp.status().is_success() => {}
        Ok(resp) => {
            warn!(
                status = %resp.status(),
                count = events.len(),
                "Log ingest non-success; dropping batch"
            );
        }
        Err(e) => {
            warn!(error = %e, count = events.len(), "Log ingest send failed; dropping batch");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_handle_is_inert() {
        let h = LogShipperHandle::disabled();
        assert!(!h.is_active());
        // Should not panic — non-blocking, drops silently.
        h.enqueue(RequestLogEvent {
            timestamp: Utc::now(),
            method: "GET".into(),
            path: "/test".into(),
            status: 200,
            latency_ms: 1,
            matched_route: None,
            client_ip: None,
            user_agent: None,
            request_id: None,
            bytes_in: None,
            bytes_out: None,
        });
    }

    #[test]
    fn from_env_returns_disabled_without_url_or_token() {
        std::env::remove_var("MOCKFORGE_LOG_INGEST_URL");
        std::env::remove_var("MOCKFORGE_LOG_INGEST_TOKEN");
        assert!(!from_env().is_active());
    }
}
