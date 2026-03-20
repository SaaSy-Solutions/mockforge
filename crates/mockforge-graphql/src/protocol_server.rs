//! Unified protocol server implementation for the GraphQL mock server.

use async_trait::async_trait;
use mockforge_core::protocol_abstraction::Protocol;
use mockforge_core::protocol_server::MockProtocolServer;

use crate::LatencyProfile;

/// A `MockProtocolServer` wrapper around the GraphQL server startup.
///
/// Wraps [`crate::start_with_latency`] with shutdown-signal integration.
pub struct GraphqlMockServer {
    port: u16,
    latency_profile: Option<LatencyProfile>,
}

impl GraphqlMockServer {
    /// Create a new `GraphqlMockServer` with the given configuration.
    pub fn new(port: u16, latency_profile: Option<LatencyProfile>) -> Self {
        Self {
            port,
            latency_profile,
        }
    }
}

#[async_trait]
impl MockProtocolServer for GraphqlMockServer {
    fn protocol(&self) -> Protocol {
        Protocol::GraphQL
    }

    async fn start(
        &self,
        mut shutdown: tokio::sync::watch::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let port = self.port;
        let latency = self.latency_profile.clone();

        tokio::select! {
            result = crate::start_with_latency(port, latency) => {
                result
            }
            _ = shutdown.changed() => {
                tracing::info!("Shutting down GraphQL server on port {}", port);
                Ok(())
            }
        }
    }

    fn port(&self) -> u16 {
        self.port
    }

    fn description(&self) -> String {
        format!("GraphQL server on port {}", self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graphql_mock_server_protocol() {
        let server = GraphqlMockServer::new(4000, None);
        assert_eq!(server.protocol(), Protocol::GraphQL);
    }

    #[test]
    fn test_graphql_mock_server_port() {
        let server = GraphqlMockServer::new(4000, None);
        assert_eq!(server.port(), 4000);
    }

    #[test]
    fn test_graphql_mock_server_description() {
        let server = GraphqlMockServer::new(4000, None);
        assert_eq!(server.description(), "GraphQL server on port 4000");
    }
}
