//! Capture forwarder for hosted-mock deployments (#553, supersedes #234 part 2).
//!
//! On hosted-mock machines the local SQLite recorder DB is ephemeral —
//! every machine cycle / reschedule wipes whatever hadn't yet been
//! mirrored to cloud Postgres. #553 migrates us from the batched mirror
//! pattern to a **synchronous-per-capture forwarder**: the recorder
//! POSTs each completed exchange to the registry's existing
//! `runtime_captures` ingest endpoint as soon as it's recorded.
//!
//! The cloud Postgres `runtime_captures` table is now treated as the
//! durable source-of-truth (per #240 + #242); the local SQLite is just
//! a debugging buffer.
//!
//! ## Design
//!
//! - A cloneable [`CaptureCloudSyncHandle`] (kept name for back-compat
//!   with the call sites in `recorder.rs` and `serve.rs`) is held by the
//!   recorder. `enqueue` is non-blocking and never blocks request
//!   serving.
//! - Behind the handle is a **bounded** [`mpsc::Sender`]. If the
//!   channel fills (registry slow / down) we drop the newest capture,
//!   emit a `tracing::warn!`, and bump
//!   `mockforge_capture_forwarder_drops_total{reason="buffer_full"}`.
//!   Dropping captures is strictly preferable to applying back-pressure
//!   onto user requests.
//! - A background task drains the channel and POSTs each capture
//!   individually (wrapped in a 1-element batch so we stay on the
//!   existing `/captures/ingest` wire format). On 5xx or transport
//!   error it retries up to 3 times with exponential backoff
//!   (100ms / 400ms / 1600ms). After exhaustion it bumps
//!   `mockforge_capture_forwarder_drops_total{reason="exhausted"}`
//!   and moves on. 4xx is treated as a permanent error and dropped
//!   immediately (no retry).
//!
//! ## Configuration
//!
//! - `MOCKFORGE_CLOUD_CAPTURES_FORWARDER_URL` *(preferred, #553)* or
//!   `MOCKFORGE_CAPTURE_INGEST_URL` *(deprecated alias, #234 part 2)*
//!   — full URL of the ingest endpoint.
//! - `MOCKFORGE_CAPTURE_INGEST_TOKEN` — short-lived
//!   deployment-scoped JWT. Sent as `Authorization: Bearer <token>`.
//! - `MOCKFORGE_CAPTURE_FORWARDER_BUFFER` — channel capacity
//!   (default 1024). Wider than the old batched shipper's 256 because
//!   we now POST one capture per request and want headroom for a
//!   transient registry stall.
//! - `MOCKFORGE_CAPTURE_FORWARDER_TIMEOUT_MS` — per-request HTTP
//!   timeout (default 5000).

use crate::models::RecordedExchange;
use once_cell::sync::Lazy;
use prometheus::{IntCounterVec, Opts};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tracing::{debug, warn};

/// `mockforge_capture_forwarder_sent_total` — captures successfully
/// accepted by the registry (HTTP 2xx).
static SENT_TOTAL: Lazy<prometheus::IntCounter> = Lazy::new(|| {
    let c = prometheus::IntCounter::new(
        "mockforge_capture_forwarder_sent_total",
        "Captures successfully forwarded to the cloud captures ingest endpoint",
    )
    .expect("Failed to create capture_forwarder_sent_total counter");
    // Best-effort register; double-register on test re-init is harmless.
    let _ = prometheus::default_registry().register(Box::new(c.clone()));
    c
});

/// `mockforge_capture_forwarder_drops_total{reason}` — captures
/// dropped before reaching the registry. Reasons:
///
/// * `buffer_full` — bounded mpsc was at capacity at enqueue time.
/// * `exhausted` — all 3 retries failed (5xx / transport error).
/// * `permanent` — registry returned 4xx (token expired, malformed
///   payload, etc.) and we won't retry.
static DROPS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let c = IntCounterVec::new(
        Opts::new(
            "mockforge_capture_forwarder_drops_total",
            "Captures dropped by the cloud forwarder, by reason",
        ),
        &["reason"],
    )
    .expect("Failed to create capture_forwarder_drops_total counter");
    let _ = prometheus::default_registry().register(Box::new(c.clone()));
    c
});

#[derive(Debug, Serialize)]
struct IngestPayload<'a> {
    exchanges: &'a [&'a RecordedExchange],
}

/// Cheap cloneable handle to the capture forwarder. `enqueue` is
/// non-blocking. When the forwarder isn't configured the handle is
/// `None` and calls are zero-cost.
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
    /// No-op handle. Used when the forwarder isn't configured.
    pub fn disabled() -> Self {
        Self { inner: None }
    }

    /// Non-blocking enqueue. If the bounded channel is full we log a
    /// warning and bump the `buffer_full` drop counter — we never
    /// block the recorder hot path waiting for the registry.
    pub fn enqueue(&self, exchange: RecordedExchange) {
        let Some(inner) = &self.inner else { return };
        match inner.sender.try_send(exchange) {
            Ok(()) => {}
            Err(TrySendError::Full(dropped)) => {
                DROPS_TOTAL.with_label_values(&["buffer_full"]).inc();
                warn!(
                    capture_id = %dropped.request.id,
                    "Capture forwarder buffer full; dropping capture (registry slow or unreachable)"
                );
            }
            Err(TrySendError::Closed(_)) => {
                // Background task is gone — registry is permanently
                // unreachable for this process. Bump and stay quiet
                // so we don't flood logs.
                DROPS_TOTAL.with_label_values(&["closed"]).inc();
            }
        }
    }

    /// True when the forwarder is actually running.
    pub fn is_active(&self) -> bool {
        self.inner.is_some()
    }
}

/// Construct from environment variables. Returns a disabled handle
/// when required vars are missing (the common case in dev /
/// self-hosted). Accepts both the new `MOCKFORGE_CLOUD_CAPTURES_FORWARDER_URL`
/// env var (preferred) and the legacy `MOCKFORGE_CAPTURE_INGEST_URL`
/// alias for back-compat with #234-era deployments.
pub fn from_env() -> CaptureCloudSyncHandle {
    let url = std::env::var("MOCKFORGE_CLOUD_CAPTURES_FORWARDER_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::env::var("MOCKFORGE_CAPTURE_INGEST_URL")
                .ok()
                .filter(|s| !s.trim().is_empty())
        });
    let url = match url {
        Some(u) => u,
        None => return CaptureCloudSyncHandle::disabled(),
    };
    let token = match std::env::var("MOCKFORGE_CAPTURE_INGEST_TOKEN") {
        Ok(t) if !t.trim().is_empty() => t,
        _ => return CaptureCloudSyncHandle::disabled(),
    };
    let buffer: usize = std::env::var("MOCKFORGE_CAPTURE_FORWARDER_BUFFER")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1024);
    let timeout_ms: u64 = std::env::var("MOCKFORGE_CAPTURE_FORWARDER_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5000);

    let client = match reqwest::Client::builder().timeout(Duration::from_millis(timeout_ms)).build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("Capture forwarder HTTP client init failed: {}", e);
            return CaptureCloudSyncHandle::disabled();
        }
    };

    let (sender, receiver) = mpsc::channel::<RecordedExchange>(buffer);
    let inner = Arc::new(Inner { sender });

    tokio::spawn(run(receiver, client, url, token));

    CaptureCloudSyncHandle { inner: Some(inner) }
}

/// Background drain loop. One POST per capture; bounded retries.
async fn run(
    mut receiver: mpsc::Receiver<RecordedExchange>,
    client: reqwest::Client,
    url: String,
    token: String,
) {
    while let Some(exchange) = receiver.recv().await {
        forward_one(&client, &url, &token, &exchange).await;
    }
    debug!("Capture forwarder drain loop exited");
}

/// Send a single capture with up to 3 retries. Backoff: 100ms, 400ms,
/// 1600ms. 4xx is treated as permanent. Counter increments live here
/// so the hot path stays free of metric work.
async fn forward_one(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    exchange: &RecordedExchange,
) {
    const MAX_ATTEMPTS: u32 = 3;
    const BASE_BACKOFF_MS: u64 = 100;

    let payload = IngestPayload {
        exchanges: &[exchange],
    };

    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        let result = client.post(url).bearer_auth(token).json(&payload).send().await;
        match result {
            Ok(resp) if resp.status().is_success() => {
                SENT_TOTAL.inc();
                return;
            }
            Ok(resp) if resp.status().is_client_error() => {
                warn!(
                    status = %resp.status(),
                    capture_id = %exchange.request.id,
                    "Capture forwarder got 4xx; dropping (will not retry)"
                );
                DROPS_TOTAL.with_label_values(&["permanent"]).inc();
                return;
            }
            Ok(resp) => {
                debug!(
                    status = %resp.status(),
                    attempt,
                    capture_id = %exchange.request.id,
                    "Capture forwarder got retriable status"
                );
            }
            Err(e) => {
                debug!(
                    error = %e,
                    attempt,
                    capture_id = %exchange.request.id,
                    "Capture forwarder transport error"
                );
            }
        }

        if attempt >= MAX_ATTEMPTS {
            warn!(
                capture_id = %exchange.request.id,
                attempts = attempt,
                "Capture forwarder exhausted retries; dropping capture"
            );
            DROPS_TOTAL.with_label_values(&["exhausted"]).inc();
            return;
        }

        // Exponential backoff: 100ms, 400ms, (1600ms — never reached
        // because MAX_ATTEMPTS=3 returns before the 4th sleep).
        let backoff = BASE_BACKOFF_MS * 4u64.pow(attempt - 1);
        tokio::time::sleep(Duration::from_millis(backoff)).await;
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
        // Snapshot + clear all three relevant vars so the test stays
        // hermetic regardless of host env.
        let url_old = std::env::var("MOCKFORGE_CAPTURE_INGEST_URL").ok();
        let url_new = std::env::var("MOCKFORGE_CLOUD_CAPTURES_FORWARDER_URL").ok();
        let token = std::env::var("MOCKFORGE_CAPTURE_INGEST_TOKEN").ok();
        std::env::remove_var("MOCKFORGE_CAPTURE_INGEST_URL");
        std::env::remove_var("MOCKFORGE_CLOUD_CAPTURES_FORWARDER_URL");
        std::env::remove_var("MOCKFORGE_CAPTURE_INGEST_TOKEN");

        assert!(!from_env().is_active());

        if let Some(v) = url_old {
            std::env::set_var("MOCKFORGE_CAPTURE_INGEST_URL", v);
        }
        if let Some(v) = url_new {
            std::env::set_var("MOCKFORGE_CLOUD_CAPTURES_FORWARDER_URL", v);
        }
        if let Some(v) = token {
            std::env::set_var("MOCKFORGE_CAPTURE_INGEST_TOKEN", v);
        }
    }

    /// Bounded-channel drop behavior: enqueueing past capacity should
    /// not panic, should not block, and should bump the drop counter.
    /// We construct an Inner directly with a tiny buffer and a
    /// non-draining receiver so the second send fills the queue.
    #[tokio::test]
    async fn enqueue_drops_when_buffer_full() {
        let (sender, _receiver) = mpsc::channel::<RecordedExchange>(1);
        let handle = CaptureCloudSyncHandle {
            inner: Some(Arc::new(Inner { sender })),
        };

        let before = DROPS_TOTAL.with_label_values(&["buffer_full"]).get();
        handle.enqueue(dummy_exchange()); // fits
        handle.enqueue(dummy_exchange()); // dropped
        let after = DROPS_TOTAL.with_label_values(&["buffer_full"]).get();
        assert_eq!(after, before + 1, "buffer_full counter should bump exactly once");
    }

    /// End-to-end forwarder smoke: spin up a tiny axum server that
    /// records every POST it receives, point the forwarder at it,
    /// enqueue a capture, wait briefly, and verify it arrived.
    #[tokio::test]
    async fn forwarder_posts_capture_to_endpoint() {
        use axum::{routing::post, Json, Router};
        use std::sync::Mutex;
        use tokio::net::TcpListener;

        #[derive(serde::Deserialize)]
        struct WirePayload {
            exchanges: Vec<serde_json::Value>,
        }

        let received: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();

        let app = Router::new().route(
            "/captures/ingest",
            post(move |Json(p): Json<WirePayload>| {
                let received = received_clone.clone();
                async move {
                    for ex in p.exchanges {
                        if let Some(id) =
                            ex.get("request").and_then(|r| r.get("id")).and_then(|i| i.as_str())
                        {
                            received.lock().unwrap().push(id.to_string());
                        }
                    }
                    axum::Json(serde_json::json!({ "accepted": 1 }))
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Build the forwarder directly (skip env-var plumbing).
        let client = reqwest::Client::builder().timeout(Duration::from_secs(2)).build().unwrap();
        let (sender, receiver) = mpsc::channel::<RecordedExchange>(8);
        let url = format!("http://{}/captures/ingest", addr);
        tokio::spawn(run(receiver, client, url, "fake-token".into()));

        let handle = CaptureCloudSyncHandle {
            inner: Some(Arc::new(Inner { sender })),
        };
        let mut ex = dummy_exchange();
        ex.request.id = "smoke-1".into();
        handle.enqueue(ex);

        // Poll briefly for the capture to arrive.
        for _ in 0..50 {
            if !received.lock().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        let got = received.lock().unwrap().clone();
        assert_eq!(got, vec!["smoke-1".to_string()]);
    }
}
