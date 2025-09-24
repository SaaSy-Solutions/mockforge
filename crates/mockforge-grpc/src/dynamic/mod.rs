//! Dynamic gRPC service discovery and registration
//!
//! This module provides functionality to dynamically discover and register
//! gRPC services from proto files, making the gRPC mock system flexible
//! for different applications and developers.

pub mod proto_parser;
pub mod service_generator;
pub mod http_bridge;

use crate::reflection::{MockReflectionProxy, ProxyConfig};
use proto_parser::ProtoParser;
use service_generator::DynamicGrpcService;
use std::collections::HashMap;
use std::sync::Arc;
use tonic::transport::Server;
use tonic_reflection::server::Builder as ReflectionBuilder;
use tracing::*;

/// Configuration for dynamic gRPC service discovery
#[derive(Debug, Clone)]
pub struct DynamicGrpcConfig {
    /// Directory containing proto files
    pub proto_dir: String,
    /// Whether to enable reflection
    pub enable_reflection: bool,
    /// Services to exclude from discovery
    pub excluded_services: Vec<String>,
    /// HTTP bridge configuration
    pub http_bridge: Option<http_bridge::HttpBridgeConfig>,
}

impl Default for DynamicGrpcConfig {
    fn default() -> Self {
        Self {
            proto_dir: "proto".to_string(),
            enable_reflection: false,
            excluded_services: Vec::new(),
            http_bridge: Some(http_bridge::HttpBridgeConfig {
                enabled: true,
                ..Default::default()
            }),
        }
    }
}

/// A registry of discovered gRPC services
#[derive(Clone)]
pub struct ServiceRegistry {
    /// Map of service names to their implementations
    services: HashMap<String, Arc<DynamicGrpcService>>,
    /// Descriptor pool containing parsed proto definitions
    descriptor_pool: prost_reflect::DescriptorPool,
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceRegistry {
    /// Get the descriptor pool
    pub fn descriptor_pool(&self) -> &prost_reflect::DescriptorPool {
        &self.descriptor_pool
    }

    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
            descriptor_pool: prost_reflect::DescriptorPool::new(),
        }
    }

    /// Create a service registry with a descriptor pool
    pub fn with_descriptor_pool(descriptor_pool: prost_reflect::DescriptorPool) -> Self {
        Self {
            services: HashMap::new(),
            descriptor_pool,
        }
    }

    /// Set the descriptor pool (useful when building registry incrementally)
    pub fn set_descriptor_pool(&mut self, pool: prost_reflect::DescriptorPool) {
        self.descriptor_pool = pool;
    }

    /// Register a service implementation
    pub fn register(&mut self, name: String, service: DynamicGrpcService) {
        self.services.insert(name, Arc::new(service));
    }

    /// Get a service by name
    pub fn get(&self, name: &str) -> Option<&Arc<DynamicGrpcService>> {
        self.services.get(name)
    }

    /// List all registered service names
    pub fn service_names(&self) -> Vec<String> {
        self.services.keys().cloned().collect()
    }

    /// Get the descriptor pool
    pub fn descriptor_pool(&self) -> &prost_reflect::DescriptorPool {
        &self.descriptor_pool
    }
}

/// Discover and register services from proto files
pub async fn discover_services(
    config: &DynamicGrpcConfig,
) -> Result<ServiceRegistry, Box<dyn std::error::Error + Send + Sync>> {
    info!("Discovering gRPC services from proto directory: {}", config.proto_dir);

    // Parse proto files
    let mut parser = ProtoParser::new();
    parser.parse_directory(&config.proto_dir).await?;

    // Create registry with the descriptor pool from the parser
    let mut registry = ServiceRegistry::new();
    // Extract services from parser and move descriptor pool
    let services = parser.services().clone();
    let descriptor_pool = parser.into_pool();

    registry.set_descriptor_pool(descriptor_pool);

    // Create dynamic services from parsed proto definitions
    for (service_name, proto_service) in services {
        // Skip excluded services
        if config.excluded_services.contains(&service_name) {
            info!("Skipping excluded service: {}", service_name);
            continue;
        }

        // Create dynamic service
        let dynamic_service = DynamicGrpcService::new(proto_service.clone(), None);
        registry.register(service_name.clone(), dynamic_service);

        info!("Registered service: {}", service_name);
    }

    info!("Successfully registered {} services", registry.service_names().len());
    Ok(registry)
}

/// Start a dynamic server with both gRPC and HTTP bridge support
pub async fn start_dynamic_server(
    port: u16,
    config: DynamicGrpcConfig,
    latency_profile: Option<mockforge_core::LatencyProfile>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(feature = "data-faker")]
    mockforge_data::provider::register_core_faker_provider();

    let _latency_injector = latency_profile
        .map(|profile| mockforge_core::latency::LatencyInjector::new(profile, Default::default()));

    // Discover services
    let registry = discover_services(&config).await?;
    let registry_arc = Arc::new(registry);

    // Use shared server utilities for consistent address creation
    let addr = mockforge_core::wildcard_socket_addr(port);
    info!(
        "Dynamic server listening on {} with {} services",
        addr,
        registry_arc.service_names().len()
    );

    // Create proxy configuration
    let proxy_config = ProxyConfig::default();

    // Create mock reflection proxy
    let mock_proxy = MockReflectionProxy::new(proxy_config, registry_arc.clone()).await?;

    // Start HTTP server (bridge) if enabled
    // For now, just start the gRPC server directly
    // HTTP bridge functionality is disabled
    start_grpc_only_server(port, &config, registry_arc.clone(), mock_proxy).await?;

    // HTTP bridge is disabled, no server handle to wait for

    Ok(())
}

/// Start a gRPC-only server (for backward compatibility)
pub async fn start_dynamic_grpc_server(
    port: u16,
    config: DynamicGrpcConfig,
    latency_profile: Option<mockforge_core::LatencyProfile>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Disable HTTP bridge
    let mut grpc_only_config = config;
    grpc_only_config.http_bridge = None;

    start_dynamic_server(port, grpc_only_config, latency_profile).await
}

/// Start the gRPC-only server implementation
async fn start_grpc_only_server(
    port: u16,
    config: &DynamicGrpcConfig,
    registry_arc: Arc<ServiceRegistry>,
    mock_proxy: MockReflectionProxy,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create a gRPC server with the mock proxy
    let mut server_builder = Server::builder();

    // Add reflection service if enabled
    if config.enable_reflection {
        // TODO: Implement proper reflection service setup with current tonic-reflection API
        // The prost-reflect API has changed and encode_file_descriptor_set is no longer available
        warn!("gRPC reflection service temporarily disabled due to API changes");
    }

    // Start actual gRPC server on the specified port
    info!(
        "Starting gRPC server on {} with {} discovered services",
        mockforge_core::wildcard_socket_addr(port),
        registry_arc.service_names().len()
    );

    // Log discovered services
    for service_name in registry_arc.service_names() {
        info!("  - Service: {}", service_name);
    }

    // For now, create a basic gRPC server that at least starts successfully
    // Full implementation would require generating actual service implementations
    use std::net::SocketAddr;

    let grpc_addr: SocketAddr = mockforge_core::wildcard_socket_addr(port);

    info!("gRPC server listening on {} (basic implementation)", grpc_addr);
    info!("Discovered services are logged but not yet fully implemented:");
    for service_name in registry_arc.service_names() {
        info!("  - {}", service_name);
    }

    // Create a basic gRPC server with the discovered services
    use crate::generated::greeter_server::{Greeter, GreeterServer};
    use crate::generated::{HelloReply, HelloRequest};
    use tonic::{Request, Response, Status};

    // Basic implementation of the Greeter service
    #[derive(Debug, Default)]
    pub struct MockGreeterService;

    #[tonic::async_trait]
    impl Greeter for MockGreeterService {
        type SayHelloStreamStream = futures::stream::Empty<Result<HelloReply, Status>>;
        type ChatStream = futures::stream::Empty<Result<HelloReply, Status>>;

        async fn say_hello(
            &self,
            request: Request<HelloRequest>,
        ) -> Result<Response<HelloReply>, Status> {
            println!("Got a request: {:?}", request);

            let req = request.into_inner();
            let reply = HelloReply {
                message: format!("Hello {}! This is a mock response from MockForge", req.name),
                metadata: None,
                items: vec![],
            };

            Ok(Response::new(reply))
        }

        async fn say_hello_stream(
            &self,
            _request: Request<HelloRequest>,
        ) -> Result<Response<Self::SayHelloStreamStream>, Status> {
            Err(Status::unimplemented("say_hello_stream not yet implemented"))
        }

        async fn say_hello_client_stream(
            &self,
            _request: Request<tonic::Streaming<HelloRequest>>,
        ) -> Result<Response<HelloReply>, Status> {
            Err(Status::unimplemented("say_hello_client_stream not yet implemented"))
        }

        async fn chat(
            &self,
            _request: Request<tonic::Streaming<HelloRequest>>,
        ) -> Result<Response<Self::ChatStream>, Status> {
            Err(Status::unimplemented("chat not yet implemented"))
        }
    }

    let greeter = MockGreeterService;

    info!("gRPC server listening on {} with Greeter service", grpc_addr);

    server_builder
        .add_service(GreeterServer::new(greeter))
        .serve(grpc_addr)
        .await?;

    Ok(())
}

/// Start combined gRPC + HTTP server
async fn start_combined_server(
    _port: u16,
    _config: &DynamicGrpcConfig,
    _registry_arc: Arc<ServiceRegistry>,
    _mock_proxy: MockReflectionProxy,
) -> Result<tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>, Box<dyn std::error::Error + Send + Sync>> {
    // HTTP bridge temporarily disabled for compilation
    Err("HTTP bridge not yet implemented".into())
}
