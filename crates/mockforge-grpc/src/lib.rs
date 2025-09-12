//! MockForge gRPC - Flexible gRPC mocking and service discovery
//!
//! This crate provides a flexible gRPC mocking system that can dynamically
//! discover and mock services from proto files without hardcoding.

use mockforge_core::LatencyProfile;

pub mod reflection;
pub mod dynamic;

pub use reflection::{ProxyConfig, ReflectionProxy, MockReflectionProxy};
pub use dynamic::{
    DynamicGrpcConfig,
    ServiceRegistry,
};
pub use dynamic::proto_parser::ProtoService;
pub use dynamic::service_generator::MockResponse;

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
