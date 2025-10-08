//! MockForge gRPC - Flexible gRPC mocking and service discovery
//!
//! This crate provides a flexible gRPC mocking system that can dynamically
//! discover and mock services from proto files without hardcoding.

use mockforge_core::LatencyProfile;

pub mod dynamic;
pub mod reflection;
pub mod registry;

// Include generated proto code
pub mod generated {
    // Include all generated proto files
    tonic::include_proto!("mockforge.greeter");
}

pub use dynamic::proto_parser::ProtoService;
pub use dynamic::service_generator::MockResponse;
pub use dynamic::{DynamicGrpcConfig, ServiceRegistry};
pub use reflection::{MockReflectionProxy, ProxyConfig, ReflectionProxy};
pub use registry::GrpcProtoRegistry;

/// Start gRPC server with default configuration
pub async fn start(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    start_with_latency(port, None).await
}

/// Start gRPC server with latency configuration
pub async fn start_with_latency(
    port: u16,
    latency_profile: Option<LatencyProfile>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = DynamicGrpcConfig::default();
    start_with_config(port, latency_profile, config).await
}

/// Start gRPC server with custom configuration
pub async fn start_with_config(
    port: u16,
    latency_profile: Option<LatencyProfile>,
    config: DynamicGrpcConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dynamic::start_dynamic_server(port, config, latency_profile).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_grpc_config_default() {
        let _config = DynamicGrpcConfig::default();
        // Config should be created successfully
        assert!(true);
    }

    #[test]
    fn test_latency_profile_creation() {
        let profile = LatencyProfile::default();
        assert_eq!(profile.base_ms, 50);
        assert_eq!(profile.jitter_ms, 20);
        assert_eq!(profile.min_ms, 0);
    }

    #[test]
    fn test_latency_profile_custom() {
        let profile = LatencyProfile::new(100, 25);

        assert_eq!(profile.base_ms, 100);
        assert_eq!(profile.jitter_ms, 25);
        assert_eq!(profile.min_ms, 0);
    }
}
