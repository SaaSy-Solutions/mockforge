//! Capture cloud-sync shipper for hosted-mock deployments (#234 part 2).
//!
//! The recorder writes captures to local SQLite, which is wiped when the
//! Fly machine restarts. This module ships each completed exchange to
//! MockForge Cloud's capture-ingest endpoint so the captures survive
//! container restart.
//!
//! Architecturally identical to `mockforge-observability::log_shipper`:
//! a cloneable handle with a non-blocking `enqueue` is held by the
//! recorder; a background task drains the channel, batches, and POSTs.
//! When env vars aren't set, the handle is a no-op — local recorder
//! usage outside hosted mocks is unaffected.
//!
//! ## Configuration
//!
//! - `MOCKFORGE_CAPTURE_INGEST_URL` — full URL of the ingest endpoint.
//! - `MOCKFORGE_CAPTURE_INGEST_TOKEN` — short-lived deployment-scoped JWT.
//! - `MOCKFORGE_CAPTURE_INGEST_BATCH_SIZE` — events per POST (default 25).
//!   Smaller than log-ingest because capture rows are larger (full bodies).
//! - `MOCKFORGE_CAPTURE_INGEST_FLUSH_MS` — max batch age (default 2000).
//! - `MOCKFORGE_CAPTURE_INGEST_BUFFER` — channel capacity (default 256).

use crate::models::RecordedExchange;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, warn};

#[derive(Debug, Serialize)]
struct IngestPayload<'a> {
    exchanges: &'a [RecordedExchange],
}

/// Cheap cloneable handle to the cloud-sync shipper. `enqueue` is
/// non-blocking and never fails — safe to call from the recorder's
/// hot path. When the shipper isn't configured the handle is `None`
/// and calls are zero-cost.
#[derive(Clone)]
pub struct CaptureCloudSyncHandle {
    inner: Option<Arc<Inner>>,
}

struct Inner {
    sender: mpsc::Sender<RecordedExchange>,
}

impl Default for CaptureCloudSyncHandle {
    fn default() -> Self {
        Self::disabled()
    }
}

impl CaptureCloudSyncHandle {
    /// No-op handle. Used when the shipper isn't configured.
    pub fn disabled() -> Self {
        Self { inner: None }
    }

    /// Non-blocking enqueue. Drops silently when the buffer is full
    /// (under sustained back-pressure we'd rather drop the newest than
    /// stall the recorder; the local SQLite is the canonical store).
    pub fn enqueue(&self, exchange: RecordedExchange) {
        if let Some(inner) = &self.inner {
            let _ = inner.sender.try_send(exchange);
        }
    }

    /// True when the shipper is actually running.
    pub fn is_active(&self) -> bool {
        self.inner.is_some()
    }
}

/// Construct from environment variables. Returns a disabled handle when
/// required vars are missing (the common case in dev / self-hosted).
pub fn from_env() -> CaptureCloudSyncHandle {
    let url = match std::env::var("MOCKFORGE_CAPTURE_INGEST_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => return CaptureCloudSyncHandle::disabled(),
    };
    let token = match std::env::var("MOCKFORGE_CAPTURE_INGEST_TOKEN") {
        Ok(t) if !t.trim().is_empty() => t,
        _ => return CaptureCloudSyncHandle::disabled(),
    };
    let batch_size: usize = std::env::var("MOCKFORGE_CAPTURE_INGEST_BATCH_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(25);
    let flush_ms: u64 = std::env::var("MOCKFORGE_CAPTURE_INGEST_FLUSH_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000);
    let buffer: usize = std::env::var("MOCKFORGE_CAPTURE_INGEST_BUFFER")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(256);

    let client = match reqwest::Client::builder().timeout(Duration::from_secs(10)).build() {
        Ok(c) => c,
        Err(e) => {
            warn!("CaptureCloudSync HTTP client init failed: {}", e);
            return CaptureCloudSyncHandle::disabled();
        }
    };

    let (sender, receiver) = mpsc::channel::<RecordedExchange>(buffer);
    let inner = Arc::new(Inner { sender });

    tokio::spawn(run(receiver, client, url, token, batch_size, flush_ms));

    CaptureCloudSyncHandle { inner: Some(inner) }
}

async fn run(
    mut receiver: mpsc::Receiver<RecordedExchange>,
    client: reqwest::Client,
    url: String,
    token: String,
    batch_size: usize,
    flush_ms: u64,
) {
    let flush_after = Duration::from_millis(flush_ms);
    let mut batch: Vec<RecordedExchange> = Vec::with_capacity(batch_size);
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

async fn send_batch(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    exchanges: &[RecordedExchange],
) {
    let payload = IngestPayload { exchanges };
    debug!(count = exchanges.len(), "Shipping capture batch");
    match client.post(url).bearer_auth(token).json(&payload).send().await {
        Ok(resp) if resp.status().is_success() => {}
        Ok(resp) => {
            warn!(
                status = %resp.status(),
                count = exchanges.len(),
                "Capture ingest non-success; dropping batch"
            );
        }
        Err(e) => {
            warn!(
                error = %e,
                count = exchanges.len(),
                "Capture ingest send failed; dropping batch"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Protocol, RecordedRequest};
    use chrono::Utc;

    fn dummy_exchange() -> RecordedExchange {
        RecordedExchange {
            request: RecordedRequest {
                id: "test".into(),
                protocol: Protocol::Http,
                timestamp: Utc::now(),
                method: "GET".into(),
                path: "/".into(),
                query_params: None,
                headers: "{}".into(),
                body: None,
                body_encoding: "utf8".into(),
                client_ip: None,
                trace_id: None,
                span_id: None,
                duration_ms: None,
                status_code: None,
                tags: None,
            },
            response: None,
        }
    }

    #[test]
    fn disabled_handle_is_inert() {
        let h = CaptureCloudSyncHandle::disabled();
        assert!(!h.is_active());
        h.enqueue(dummy_exchange());
    }

    #[test]
    fn from_env_disabled_without_url_or_token() {
        std::env::remove_var("MOCKFORGE_CAPTURE_INGEST_URL");
        std::env::remove_var("MOCKFORGE_CAPTURE_INGEST_TOKEN");
        assert!(!from_env().is_active());
    }
}
