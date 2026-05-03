//! Connection-accept counter for HTTP serve paths.
//!
//! Wraps a Tower service (in `axum::serve` / `axum_server::Server::serve`
//! parlance, the "make-service") so that the per-connection
//! `make_service.call(target)` increments the global `record_accept()`
//! counter. The dashboard sampler reads the counter every 10s to derive
//! a connections-per-second rate.
//!
//! Counting at the make-service layer (not the TCP listener layer) works
//! for **both**:
//! - plain HTTP via `axum::serve(listener, make_svc)`
//! - HTTPS via `axum_server::Server::bind_rustls(...).serve(make_svc)`
//!
//! For HTTPS, the counter only ticks once the TLS handshake completes —
//! failed handshakes aren't counted, which is the correct semantic for
//! "successful connection".

use std::task::{Context, Poll};

/// Tower service that bumps the global accept counter on each `call()`.
///
/// `Service<T>` for any `T` — works as an axum make-service regardless
/// of the connection target type (`SocketAddr`, `ChaosClientAddr`, etc.).
#[derive(Clone)]
pub struct CountingMakeService<M> {
    inner: M,
}

impl<M> CountingMakeService<M> {
    /// Wrap a make-service. Each `call(target)` will bump the global
    /// accept counter before delegating.
    pub fn new(inner: M) -> Self {
        Self { inner }
    }
}

impl<M, T> tower::Service<T> for CountingMakeService<M>
where
    M: tower::Service<T>,
{
    type Response = M::Response;
    type Error = M::Error;
    type Future = M::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, target: T) -> Self::Future {
        mockforge_foundation::rate_counters::record_accept();
        self.inner.call(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_foundation::rate_counters;
    use std::convert::Infallible;
    use std::sync::atomic::Ordering;
    use tower::{service_fn, Service, ServiceExt};

    #[tokio::test]
    async fn counting_make_service_bumps_accept_counter() {
        let before = rate_counters::HTTP_ACCEPTS_TOTAL.load(Ordering::Relaxed);

        let inner = service_fn(|_addr: std::net::SocketAddr| async {
            Ok::<_, Infallible>(service_fn(|_req: ()| async { Ok::<_, Infallible>(()) }))
        });
        let mut counted = CountingMakeService::new(inner);
        let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();

        let _service = counted.ready().await.unwrap().call(addr).await.unwrap();
        let _service = counted.ready().await.unwrap().call(addr).await.unwrap();

        let after = rate_counters::HTTP_ACCEPTS_TOTAL.load(Ordering::Relaxed);
        assert_eq!(after, before + 2, "each make-service call bumps the counter");
    }
}
