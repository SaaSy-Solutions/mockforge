//! TCP-level connection-error injection.
//!
//! `ChaosTcpListener` wraps a `tokio::net::TcpListener` and implements
//! `axum::serve::Listener`. When fault injection is enabled and the
//! `connection_errors` knob fires, the wrapper drops the just-accepted
//! socket *before* axum/hyper sees it — surfacing as a real transport-level
//! failure (RST or unclean EOF) instead of an HTTP 503 on a healthy
//! connection.
//!
//! ```text
//! TCP accept ──▶ chaos check ──▶ drop with linger=0  → RST  (TcpReset)
//!                            └─▶ drop normally       → FIN  (TcpClose)
//!                            └─▶ pass to axum/hyper      (Http503 / no fault)
//! ```
//!
//! The decision is per-connection (not per-request), so a single chaos hit
//! affects every HTTP request that would have been pipelined onto that
//! socket. That's the correct semantics for a connection-level fault.

use crate::config::{ChaosConfig, ConnectionErrorKind};
use rand::Rng;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::RwLock};
use tracing::warn;

/// Listener wrapper that injects TCP-level connection faults.
pub struct ChaosTcpListener {
    inner: TcpListener,
    config: Arc<RwLock<ChaosConfig>>,
}

impl ChaosTcpListener {
    /// Wrap an existing TCP listener with chaos behavior. The wrapper observes
    /// the shared config on each accept, so changes via the chaos hot-reload
    /// API take effect immediately for newly accepted sockets.
    pub fn new(inner: TcpListener, config: Arc<RwLock<ChaosConfig>>) -> Self {
        Self { inner, config }
    }
}

/// Connect-info wrapper for the chaos listener.
///
/// Rust's orphan rules forbid `impl Connected<IncomingStream<ChaosTcpListener>> for SocketAddr`
/// (both `Connected` and `SocketAddr` are foreign), so handlers using
/// `axum::extract::ConnectInfo<SocketAddr>` won't see an address with the
/// chaos listener installed. Use `ConnectInfo<ChaosClientAddr>` instead;
/// it deref'd to `SocketAddr`.
#[derive(Clone, Copy, Debug)]
pub struct ChaosClientAddr(pub SocketAddr);

impl ChaosClientAddr {
    pub fn into_inner(self) -> SocketAddr {
        self.0
    }
}

impl std::ops::Deref for ChaosClientAddr {
    type Target = SocketAddr;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl axum::extract::connect_info::Connected<axum::serve::IncomingStream<'_, ChaosTcpListener>>
    for ChaosClientAddr
{
    fn connect_info(stream: axum::serve::IncomingStream<'_, ChaosTcpListener>) -> Self {
        ChaosClientAddr(*stream.remote_addr())
    }
}

impl axum::serve::Listener for ChaosTcpListener {
    type Io = tokio::net::TcpStream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let (stream, addr) = match self.inner.accept().await {
                Ok(tup) => tup,
                Err(e) => {
                    // Mirror axum's own backoff for transient accept errors
                    // so we don't hot-spin on EMFILE etc.
                    tracing::warn!("[chaos-listener] accept error: {e}");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                }
            };

            let kind = {
                let cfg = self.config.read().await;
                cfg.fault_injection
                    .as_ref()
                    .filter(|f| f.enabled && f.connection_errors)
                    .and_then(|f| {
                        let mut rng = rand::rng();
                        if rng.random::<f64>() < f.connection_error_probability {
                            Some(f.connection_error_kind)
                        } else {
                            None
                        }
                    })
            };

            match kind {
                Some(ConnectionErrorKind::TcpReset) => {
                    // Setting SO_LINGER=0 then dropping the stream causes the
                    // kernel to send a TCP RST instead of a graceful FIN. The
                    // stdlib deprecation warns about blocking-on-drop; in this
                    // chaos use case the brief block is the desired side effect
                    // (it's how we synthesize an RST), so the deprecation is
                    // acceptable here.
                    #[allow(deprecated)]
                    if let Err(e) = stream.set_linger(Some(Duration::ZERO)) {
                        warn!(
                            "[chaos] set_linger(0) failed for {}: {} — falling back to FIN",
                            addr, e
                        );
                    }
                    drop(stream);
                    warn!("[chaos] injected TCP RST on connection from {}", addr);
                    crate::metrics::CHAOS_METRICS.record_fault("tcp_reset", "_listener");
                    continue;
                }
                Some(ConnectionErrorKind::TcpClose) => {
                    drop(stream);
                    warn!("[chaos] injected TCP FIN on connection from {}", addr);
                    crate::metrics::CHAOS_METRICS.record_fault("tcp_close", "_listener");
                    continue;
                }
                // `Http503` (or no fault hit) lets the connection through; the
                // chaos middleware will still apply its HTTP-level 503 logic.
                // The accept counter is bumped by `CountingMakeService` when
                // axum hands the stream to the make-service, so this listener
                // doesn't need to touch it.
                _ => return (stream, addr),
            }
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        self.inner.local_addr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FaultInjectionConfig;

    #[tokio::test]
    async fn pass_through_when_disabled() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let cfg = Arc::new(RwLock::new(ChaosConfig {
            enabled: false,
            ..Default::default()
        }));
        let mut wrapped = ChaosTcpListener::new(listener, cfg);

        let _client = tokio::spawn(async move {
            tokio::net::TcpStream::connect(addr).await.unwrap();
        });

        let (_stream, _peer) = axum::serve::Listener::accept(&mut wrapped).await;
        // If we get here without looping forever, pass-through worked.
    }

    #[tokio::test]
    async fn tcp_reset_drops_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let cfg = Arc::new(RwLock::new(ChaosConfig {
            enabled: true,
            fault_injection: Some(FaultInjectionConfig {
                enabled: true,
                connection_errors: true,
                connection_error_probability: 1.0,
                connection_error_kind: ConnectionErrorKind::TcpReset,
                ..Default::default()
            }),
            ..Default::default()
        }));
        let mut wrapped = ChaosTcpListener::new(listener, cfg);

        // Open one bad connection (gets dropped), then one good one (after we flip prob).
        let cfg_clone = wrapped.config.clone();
        let client = tokio::spawn(async move {
            // First connection: dropped by chaos.
            let _ = tokio::net::TcpStream::connect(addr).await;
            // Flip off chaos so the second connection passes through.
            cfg_clone.write().await.fault_injection.as_mut().unwrap().connection_errors = false;
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _good = tokio::net::TcpStream::connect(addr).await.unwrap();
        });

        let (_stream, _peer) = tokio::time::timeout(
            Duration::from_secs(5),
            axum::serve::Listener::accept(&mut wrapped),
        )
        .await
        .expect("accept timed out — chaos may have looped forever");
        client.await.unwrap();
    }
}
