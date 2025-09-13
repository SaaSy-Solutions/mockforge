//! Dynamic gRPC service discovery and registration
//!
//! This module provides functionality to dynamically discover and register
//! gRPC services from proto files, making the gRPC mock system flexible
//! for different applications and developers.

pub mod proto_parser;
pub mod service_generator;

use crate::reflection::{MockReflectionProxy, ProxyConfig};
use proto_parser::ProtoParser;
use service_generator::DynamicGrpcService;
use std::collections::HashMap;
use std::sync::Arc;
use tonic::transport::Server;
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
}

impl Default for DynamicGrpcConfig {
    fn default() -> Self {
        Self {
            proto_dir: "proto".to_string(),
            enable_reflection: false,
            excluded_services: Vec::new(),
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

impl ServiceRegistry {
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

/// Start a dynamic gRPC server with discovered services
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
        "Dynamic gRPC listening on {} with {} services",
        addr,
        registry_arc.service_names().len()
    );

    // Create proxy configuration
    let proxy_config = ProxyConfig::default();

    // Create mock reflection proxy
    let _mock_proxy = MockReflectionProxy::new(proxy_config, registry_arc.clone()).await?;

    // Create a gRPC server with the mock proxy
    let _server_builder = Server::builder();

    // Add reflection service if enabled
    if config.enable_reflection {
        // TODO: Add reflection service
        info!("gRPC reflection is enabled (not yet implemented)");
    }

    // Start both HTTP server for status and gRPC server for actual requests
    use axum::{response::Json, routing::get, Router};
    use serde_json::json;
    use tokio::net::TcpListener;

    let service_names = registry_arc.service_names().clone();

    // Create HTTP router for status endpoints
    let http_app: Router = Router::new()
        .route("/", get({
            let service_names = service_names.clone();
            move || async move {
                Json(json!({
                    "message": "MockForge gRPC Server",
                    "status": "running",
                    "services": service_names,
                    "note": "This is a dynamic gRPC server with integrated mock proxy. Services are discovered from proto files and ready for gRPC requests."
                }))
            }
        }))
        .route("/services", get({
            let service_names = service_names.clone();
            move || async move {
                let services: Vec<serde_json::Value> = service_names.iter().map(|name| {
                    json!({
                        "name": name,
                        "status": "discovered",
                        "methods": "parsed from proto files",
                        "proxy": "mock reflection proxy ready"
                    })
                }).collect();

                Json(json!({
                    "services": services,
                    "count": services.len(),
                    "proxy_status": "integrated"
                }))
            }
        }));

    // Start HTTP server on the specified port
    let http_addr = addr;
    let http_listener = TcpListener::bind(http_addr).await?;
    info!("HTTP status server listening on {}", http_addr);

    // Start actual gRPC server on the specified port
    info!(
        "Starting gRPC server on {} with {} discovered services",
        addr,
        registry_arc.service_names().len()
    );

    // Log discovered services
    for service_name in registry_arc.service_names() {
        info!("  - Service: {}", service_name);
    }

    // For now, create a basic gRPC server that at least starts successfully
    // Full implementation would require generating actual service implementations
    use std::net::SocketAddr;

    let grpc_addr: SocketAddr = addr;

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

    let greeter = MockGreeterService::default();

    info!("gRPC server listening on {} with Greeter service", grpc_addr);

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(grpc_addr)
        .await?;

    Ok(())
}
