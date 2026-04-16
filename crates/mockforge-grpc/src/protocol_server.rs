//! Unified protocol server implementation for the gRPC mock server.

use async_trait::async_trait;
use mockforge_core::protocol_abstraction::Protocol;
use mockforge_core::protocol_server::MockProtocolServer;

use crate::dynamic::DynamicGrpcConfig;
use mockforge_foundation::latency::LatencyProfile;

/// A `MockProtocolServer` wrapper around the gRPC server startup.
///
/// Wraps [`crate::start_with_config`] with shutdown-signal integration.
pub struct GrpcMockServer {
    port: u16,
    latency_profile: Option<LatencyProfile>,
    config: DynamicGrpcConfig,
}

impl GrpcMockServer {
    /// Create a new `GrpcMockServer` with the given configuration.
    pub fn new(
        port: u16,
        latency_profile: Option<LatencyProfile>,
        config: DynamicGrpcConfig,
    ) -> Self {
        Self {
            port,
            latency_profile,
            config,
        }
    }
}

#[async_trait]
impl MockProtocolServer for GrpcMockServer {
    fn protocol(&self) -> Protocol {
        Protocol::Grpc
    }

    async fn start(
        &self,
        mut shutdown: tokio::sync::watch::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let port = self.port;
        let latency = self.latency_profile.clone();
        let config = self.config.clone();

        tokio::select! {
            result = crate::start_with_config(port, latency, config) => {
                result
            }
            _ = shutdown.changed() => {
                tracing::info!("Shutting down gRPC server on port {}", port);
                Ok(())
            }
        }
    }

    fn port(&self) -> u16 {
        self.port
    }

    fn description(&self) -> String {
        format!("gRPC server on port {}", self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_mock_server_protocol() {
        let server = GrpcMockServer::new(50051, None, DynamicGrpcConfig::default());
        assert_eq!(server.protocol(), Protocol::Grpc);
    }

    #[test]
    fn test_grpc_mock_server_port() {
        let server = GrpcMockServer::new(50051, None, DynamicGrpcConfig::default());
        assert_eq!(server.port(), 50051);
    }

    #[test]
    fn test_grpc_mock_server_description() {
        let server = GrpcMockServer::new(50051, None, DynamicGrpcConfig::default());
        assert_eq!(server.description(), "gRPC server on port 50051");
    }
}
