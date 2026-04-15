//! Unified protocol server implementation for the HTTP mock server.

use async_trait::async_trait;
use axum::Router;
use mockforge_core::config::HttpTlsConfig;
use mockforge_core::protocol_server::MockProtocolServer;
use mockforge_foundation::protocol::Protocol;

/// A `MockProtocolServer` wrapper around the HTTP server startup.
///
/// Wraps [`crate::serve_router_with_tls`] with shutdown-signal integration.
/// The caller must supply a pre-built [`Router`] — this struct does not
/// construct one itself, because router construction depends on OpenAPI
/// specs, middleware, and CLI flags that are outside this crate's scope.
pub struct HttpMockServer {
    port: u16,
    router: Router,
    tls_config: Option<HttpTlsConfig>,
}

impl HttpMockServer {
    /// Create a new `HttpMockServer` with the given configuration.
    pub fn new(port: u16, router: Router, tls_config: Option<HttpTlsConfig>) -> Self {
        Self {
            port,
            router,
            tls_config,
        }
    }
}

#[async_trait]
impl MockProtocolServer for HttpMockServer {
    fn protocol(&self) -> Protocol {
        Protocol::Http
    }

    async fn start(
        &self,
        mut shutdown: tokio::sync::watch::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Router is Clone (axum::Router is cheaply cloneable)
        let router = self.router.clone();
        let port = self.port;
        let tls_config = self.tls_config.clone();

        tokio::select! {
            result = crate::serve_router_with_tls(port, router, tls_config) => {
                result
            }
            _ = shutdown.changed() => {
                tracing::info!("Shutting down HTTP server on port {}", port);
                Ok(())
            }
        }
    }

    fn port(&self) -> u16 {
        self.port
    }

    fn description(&self) -> String {
        format!("HTTP server on port {}", self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_mock_server_protocol() {
        let server = HttpMockServer::new(3000, Router::new(), None);
        assert_eq!(server.protocol(), Protocol::Http);
    }

    #[test]
    fn test_http_mock_server_port() {
        let server = HttpMockServer::new(8080, Router::new(), None);
        assert_eq!(server.port(), 8080);
    }

    #[test]
    fn test_http_mock_server_description() {
        let server = HttpMockServer::new(3000, Router::new(), None);
        assert_eq!(server.description(), "HTTP server on port 3000");
    }
}
