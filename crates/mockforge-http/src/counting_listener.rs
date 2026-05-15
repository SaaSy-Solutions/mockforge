//! Connection lifecycle counter for HTTP serve paths.
//!
//! Wraps a Tower service (in `axum::serve` / `axum_server::Server::serve`
//! parlance, the "make-service") so that:
//!
//! * Each `make_service.call(target)` bumps the global `record_accept()`
//!   counter (which increments both `HTTP_ACCEPTS_TOTAL` and the
//!   currently-open gauge).
//! * The returned per-connection service is wrapped in a [`TrackedService`]
//!   whose `Drop` impl calls `record_close()` (decrementing the open gauge
//!   and bumping `HTTP_CONNECTIONS_CLOSED_TOTAL`).
//!
//! The dashboard sampler reads the counters every 10s to derive a
//! connections-per-second rate, and the TUI / admin UI reads the
//! `connections_open` gauge for the live "Active Connections" widget
//! (issue #79 round 6, Srikanth's "open/closed/total" ask).
//!
//! Tracking at the make-service layer (not the TCP listener layer) works
//! for **both**:
//! - plain HTTP via `axum::serve(listener, make_svc)`
//! - HTTPS via `axum_server::Server::bind_rustls(...).serve(make_svc)`
//!
//! For HTTPS, the counter only ticks once the TLS handshake completes —
//! failed handshakes aren't counted, which is the correct semantic for
//! "successful connection". Likewise, `record_close` fires when hyper
//! drops the per-connection service — i.e. *after* the connection is
//! torn down from MockForge's side, regardless of which end initiated
//! the FIN. The `MOCKFORGE_HTTP_LOG_CONN` env var (see
//! `middleware::conn_diagnostics`) turns on a per-close INFO log line
//! that includes the connection's duration and the number of requests
//! it served — useful for distinguishing "MockForge closed after one
//! request" from "the peer closed after one request" in
//! single-request-per-connection PCAPs (issue #79 round 5/6).
//!
//! ### Why `Arc<ConnectionGuard>`?
//!
//! axum's `serve` bound requires the per-connection service to be
//! `Clone`. If hyper clones the wrapped service (e.g. for an upgrade
//! handshake), naïvely owning the guard inline would record-close
//! once per clone instead of once per connection. Sharing a single
//! guard behind an `Arc` makes the close fire exactly once: when the
//! last clone is dropped.

use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

/// Tower service that bumps the global accept counter on each `call()`,
/// then wraps the inner service in a [`TrackedService`] so the open-gauge
/// decrement and the optional close-log fire when the connection ends.
///
/// `Service<T>` for any `T` — works as an axum make-service regardless
/// of the connection target type (`SocketAddr`, `ChaosClientAddr`, etc.).
#[derive(Clone)]
pub struct CountingMakeService<M> {
    inner: M,
}

impl<M> CountingMakeService<M> {
    /// Wrap a make-service. Each `call(target)` bumps the global accept
    /// counter (and the currently-open gauge) before delegating, and the
    /// returned per-connection service is wrapped so the open-gauge
    /// decrement fires when the connection terminates.
    pub fn new(inner: M) -> Self {
        Self { inner }
    }
}

impl<M, T> tower::Service<T> for CountingMakeService<M>
where
    M: tower::Service<T>,
    M::Future: Send + 'static,
    M::Response: 'static,
{
    type Response = TrackedService<M::Response>;
    type Error = M::Error;
    type Future =
        Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, target: T) -> Self::Future {
        // Bump accept + open-gauge BEFORE the make-service runs so even an
        // immediate make-service failure (no Service produced) still keeps
        // the gauge balanced via the error branch below.
        mockforge_foundation::rate_counters::record_accept();
        let fut = self.inner.call(target);
        Box::pin(async move {
            match fut.await {
                Ok(inner) => Ok(TrackedService::new(inner)),
                Err(err) => {
                    // No TrackedService exists to close on drop — balance
                    // the gauge here.
                    mockforge_foundation::rate_counters::record_close();
                    Err(err)
                }
            }
        })
    }
}

/// Per-connection service wrapper. The connection's lifetime is owned by
/// the inner `Arc<ConnectionGuard>` — when every clone of this service is
/// dropped, the guard's refcount hits zero and `record_close()` fires
/// exactly once (with optional INFO log of duration + requests served).
pub struct TrackedService<S> {
    inner: S,
    guard: Arc<ConnectionGuard>,
}

impl<S> TrackedService<S> {
    fn new(inner: S) -> Self {
        Self {
            inner,
            guard: Arc::new(ConnectionGuard::new()),
        }
    }
}

impl<S: Clone> Clone for TrackedService<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            // Share the guard — the close fires when the LAST clone drops.
            guard: self.guard.clone(),
        }
    }
}

impl<S, R> tower::Service<R> for TrackedService<S>
where
    S: tower::Service<R>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: R) -> Self::Future {
        self.guard.bump_request();
        self.inner.call(req)
    }
}

/// RAII guard for a single accepted connection. Drop calls `record_close`
/// and (when `MOCKFORGE_HTTP_LOG_CONN=1`) logs the connection's duration
/// and request count.
struct ConnectionGuard {
    opened_at: Instant,
    requests: AtomicU64,
}

impl ConnectionGuard {
    fn new() -> Self {
        Self {
            opened_at: Instant::now(),
            requests: AtomicU64::new(0),
        }
    }

    fn bump_request(&self) {
        self.requests.fetch_add(1, Ordering::Relaxed);
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let duration = self.opened_at.elapsed();
        let requests = self.requests.load(Ordering::Relaxed);
        mockforge_foundation::rate_counters::record_close();
        if is_conn_log_enabled() {
            tracing::info!(
                target: "mockforge_http::conn_diag",
                duration_ms = duration.as_millis() as u64,
                requests = requests,
                "http_conn_closed",
            );
        }
    }
}

/// Inline copy of the env-flag check used by the diagnostics middleware,
/// kept here to avoid a cyclic dependency between this module and
/// `middleware::conn_diagnostics` (which lives at a higher layer).
fn is_conn_log_enabled() -> bool {
    std::env::var("MOCKFORGE_HTTP_LOG_CONN")
        .ok()
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

#[cfg(test)]
// Tests hold the serializing `TEST_LOCK` (a `std::sync::Mutex`) across
// `await` points by design — the lock has no purpose during the test
// body other than excluding other tests from touching the global
// counters at the same time. Clippy's `await_holding_lock` is a real
// concern in production async code, but here we run on
// `current_thread` runtimes so the guard never crosses threads and
// there's no deadlock surface; suppress at the module level.
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::*;
    use mockforge_foundation::rate_counters;
    use std::convert::Infallible;
    use std::sync::Mutex;
    use tower::{service_fn, Service, ServiceExt};

    // Global counters are shared across all tests in this process. Serialize
    // every test that asserts on the gauge so parallel runs (the default
    // `cargo test` mode) don't see each other's increments.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn reset_counters() {
        rate_counters::SUCCESSFUL_RESPONSES_TOTAL.store(0, Ordering::Relaxed);
        rate_counters::OK_RESPONSES_TOTAL.store(0, Ordering::Relaxed);
        rate_counters::HTTP_ACCEPTS_TOTAL.store(0, Ordering::Relaxed);
        rate_counters::HTTP_CONNECTIONS_OPEN.store(0, Ordering::Relaxed);
        rate_counters::HTTP_CONNECTIONS_CLOSED_TOTAL.store(0, Ordering::Relaxed);
    }

    #[tokio::test]
    async fn counting_make_service_bumps_accept_counter() {
        let _g = TEST_LOCK.lock().unwrap();
        reset_counters();

        let inner = service_fn(|_addr: std::net::SocketAddr| async {
            Ok::<_, Infallible>(service_fn(|_req: ()| async { Ok::<_, Infallible>(()) }))
        });
        let mut counted = CountingMakeService::new(inner);
        let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();

        let _service_a = counted.ready().await.unwrap().call(addr).await.unwrap();
        let _service_b = counted.ready().await.unwrap().call(addr).await.unwrap();

        let snap = rate_counters::snapshot();
        assert_eq!(snap.accepts, 2, "each make-service call bumps the accept counter");
        assert_eq!(snap.connections_open, 2, "both services held => 2 open");
    }

    #[tokio::test]
    async fn dropping_tracked_service_records_close() {
        let _g = TEST_LOCK.lock().unwrap();
        reset_counters();

        let inner = service_fn(|_addr: std::net::SocketAddr| async {
            Ok::<_, Infallible>(service_fn(|_req: ()| async { Ok::<_, Infallible>(()) }))
        });
        let mut counted = CountingMakeService::new(inner);
        let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();

        let service = counted.ready().await.unwrap().call(addr).await.unwrap();
        assert_eq!(rate_counters::snapshot().connections_open, 1);

        drop(service);
        let snap = rate_counters::snapshot();
        assert_eq!(snap.connections_open, 0, "drop decrements open gauge");
        assert_eq!(snap.connections_closed, 1, "drop bumps closed counter");
        assert_eq!(snap.accepts, 1, "accepts unchanged on drop");
    }

    #[tokio::test]
    async fn cloned_service_only_records_close_once_when_all_clones_drop() {
        let _g = TEST_LOCK.lock().unwrap();
        reset_counters();

        // Wrap a Cloneable inner service so TrackedService can be cloned.
        #[derive(Clone)]
        struct EchoSvc;
        impl tower::Service<u32> for EchoSvc {
            type Response = u32;
            type Error = Infallible;
            type Future = std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<u32, Infallible>> + Send>,
            >;
            fn poll_ready(
                &mut self,
                _cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<(), Infallible>> {
                std::task::Poll::Ready(Ok(()))
            }
            fn call(&mut self, req: u32) -> Self::Future {
                Box::pin(async move { Ok(req) })
            }
        }

        let inner =
            service_fn(|_addr: std::net::SocketAddr| async { Ok::<_, Infallible>(EchoSvc) });
        let mut counted = CountingMakeService::new(inner);
        let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
        let service = counted.ready().await.unwrap().call(addr).await.unwrap();

        let clone_a = service.clone();
        let clone_b = service.clone();
        // Three live references to the same guard — gauge stayed at 1.
        assert_eq!(rate_counters::snapshot().connections_open, 1);

        drop(service);
        drop(clone_a);
        // Still one reference alive (clone_b) — close has not fired.
        assert_eq!(rate_counters::snapshot().connections_open, 1);
        assert_eq!(rate_counters::snapshot().connections_closed, 0);

        drop(clone_b);
        let snap = rate_counters::snapshot();
        assert_eq!(snap.connections_open, 0);
        assert_eq!(snap.connections_closed, 1, "close fires exactly once");
    }

    #[tokio::test]
    async fn make_service_error_balances_open_gauge() {
        let _g = TEST_LOCK.lock().unwrap();
        reset_counters();

        // Concrete dummy Service that satisfies CountingMakeService's
        // bounds — never actually invoked because the make-service
        // returns Err first.
        #[derive(Clone)]
        struct NeverSvc;
        impl tower::Service<()> for NeverSvc {
            type Response = ();
            type Error = Infallible;
            type Future = std::future::Ready<Result<(), Infallible>>;
            fn poll_ready(
                &mut self,
                _cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<(), Infallible>> {
                std::task::Poll::Ready(Ok(()))
            }
            fn call(&mut self, _req: ()) -> Self::Future {
                std::future::ready(Ok(()))
            }
        }

        let inner = service_fn(|_addr: std::net::SocketAddr| async {
            Err::<NeverSvc, _>(std::io::Error::other("boom"))
        });
        let mut counted = CountingMakeService::new(inner);
        let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
        let result = counted.ready().await.unwrap().call(addr).await;
        assert!(result.is_err(), "inner make-service errored");

        let snap = rate_counters::snapshot();
        assert_eq!(snap.accepts, 1, "accept still counted");
        assert_eq!(snap.connections_open, 0, "open gauge balanced after error");
        assert_eq!(snap.connections_closed, 1, "close recorded for the failed make-service");
    }
}
