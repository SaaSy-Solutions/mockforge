//! Dynamic gRPC service discovery and registration
//!
//! This module provides functionality to dynamically discover and register
//! gRPC services from proto files, making the gRPC mock system flexible
//! for different applications and developers.

pub mod http_bridge;
pub mod proto_parser;
pub mod service_generator;

use crate::reflection::{MockReflectionProxy, ProxyConfig};
use proto_parser::ProtoParser;
use service_generator::DynamicGrpcService;
use std::collections::HashMap;
use std::sync::Arc;
use tonic::transport::Server;
use tonic_reflection::server::Builder as ReflectionBuilder;
use tracing::*;

/// TLS configuration for gRPC server
#[derive(Debug, Clone)]
pub struct GrpcTlsConfig {
    /// Path to the TLS certificate file (PEM format)
    pub cert_path: String,
    /// Path to the TLS private key file (PEM format)
    pub key_path: String,
    /// Optional path to CA certificate for client certificate verification (mTLS)
    pub client_ca_path: Option<String>,
}

impl GrpcTlsConfig {
    /// Create a new TLS configuration
    pub fn new(cert_path: impl Into<String>, key_path: impl Into<String>) -> Self {
        Self {
            cert_path: cert_path.into(),
            key_path: key_path.into(),
            client_ca_path: None,
        }
    }

    /// Create TLS configuration with mutual TLS (client certificate verification)
    pub fn with_mtls(
        cert_path: impl Into<String>,
        key_path: impl Into<String>,
        client_ca_path: impl Into<String>,
    ) -> Self {
        Self {
            cert_path: cert_path.into(),
            key_path: key_path.into(),
            client_ca_path: Some(client_ca_path.into()),
        }
    }

    /// Create TLS configuration from environment variables
    ///
    /// Uses:
    /// - GRPC_TLS_CERT: Path to certificate file
    /// - GRPC_TLS_KEY: Path to private key file
    /// - GRPC_TLS_CLIENT_CA: Optional path to client CA for mTLS
    pub fn from_env() -> Option<Self> {
        let cert_path = std::env::var("GRPC_TLS_CERT").ok()?;
        let key_path = std::env::var("GRPC_TLS_KEY").ok()?;
        let client_ca_path = std::env::var("GRPC_TLS_CLIENT_CA").ok();

        Some(Self {
            cert_path,
            key_path,
            client_ca_path,
        })
    }
}

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
    /// TLS configuration (None for plaintext)
    pub tls: Option<GrpcTlsConfig>,
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
            // Check for TLS configuration from environment
            tls: GrpcTlsConfig::from_env(),
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
}

/// Discover and register services from proto files
pub async fn discover_services(
    config: &DynamicGrpcConfig,
) -> Result<ServiceRegistry, Box<dyn std::error::Error + Send + Sync>> {
    use std::time::Instant;

    let discovery_start = Instant::now();
    info!("Discovering gRPC services from proto directory: {}", config.proto_dir);

    // Parse proto files
    let parse_start = Instant::now();
    let mut parser = ProtoParser::new();
    parser.parse_directory(&config.proto_dir).await?;
    let parse_duration = parse_start.elapsed();
    info!("Proto file parsing completed (took {:?})", parse_duration);

    // Create registry with the descriptor pool from the parser
    let registry_start = Instant::now();
    let mut registry = ServiceRegistry::new();
    // Extract services from parser and move descriptor pool
    let services = parser.services().clone();
    let descriptor_pool = parser.into_pool();

    registry.set_descriptor_pool(descriptor_pool);
    let registry_duration = registry_start.elapsed();
    debug!("Registry creation completed (took {:?})", registry_duration);

    // Create dynamic services from parsed proto definitions
    let service_reg_start = Instant::now();
    for (service_name, proto_service) in services {
        // Skip excluded services
        if config.excluded_services.contains(&service_name) {
            info!("Skipping excluded service: {}", service_name);
            continue;
        }

        // Create dynamic service
        let dynamic_service = DynamicGrpcService::new(proto_service.clone(), None);
        registry.register(service_name.clone(), dynamic_service);

        debug!("Registered service: {}", service_name);
    }
    let service_reg_duration = service_reg_start.elapsed();
    info!(
        "Service registration completed for {} services (took {:?})",
        registry.service_names().len(),
        service_reg_duration
    );

    let total_discovery_duration = discovery_start.elapsed();
    info!("Service discovery completed (total time: {:?})", total_discovery_duration);
    Ok(registry)
}

/// Start a dynamic server with both gRPC and HTTP bridge support
pub async fn start_dynamic_server(
    port: u16,
    config: DynamicGrpcConfig,
    latency_profile: Option<mockforge_core::LatencyProfile>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::time::Instant;

    let startup_start = Instant::now();

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
    let reflection_start = Instant::now();
    let mock_proxy = MockReflectionProxy::new(proxy_config, registry_arc.clone()).await?;
    let reflection_duration = reflection_start.elapsed();
    info!("gRPC reflection proxy created (took {:?})", reflection_duration);

    let total_startup_duration = startup_start.elapsed();
    info!("gRPC server startup completed (total time: {:?})", total_startup_duration);

    // Start HTTP server (bridge) if enabled
    // Currently, just start the gRPC server directly
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
    _mock_proxy: MockReflectionProxy,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tonic::transport::{Certificate, Identity, ServerTlsConfig};

    // Create server builder with optional TLS
    let mut server_builder = if let Some(tls_config) = &config.tls {
        info!("Configuring gRPC server with TLS");

        // Read certificate and key files
        let cert = tokio::fs::read(&tls_config.cert_path).await.map_err(|e| {
            error!("Failed to read TLS certificate from {}: {}", tls_config.cert_path, e);
            Box::<dyn std::error::Error + Send + Sync>::from(format!(
                "Failed to read TLS certificate: {}",
                e
            ))
        })?;

        let key = tokio::fs::read(&tls_config.key_path).await.map_err(|e| {
            error!("Failed to read TLS key from {}: {}", tls_config.key_path, e);
            Box::<dyn std::error::Error + Send + Sync>::from(format!(
                "Failed to read TLS key: {}",
                e
            ))
        })?;

        let identity = Identity::from_pem(cert, key);

        let mut tls = ServerTlsConfig::new().identity(identity);

        // Add client CA for mTLS if configured
        if let Some(client_ca_path) = &tls_config.client_ca_path {
            info!("Configuring mutual TLS (mTLS) with client certificate verification");
            let client_ca = tokio::fs::read(client_ca_path).await.map_err(|e| {
                error!("Failed to read client CA from {}: {}", client_ca_path, e);
                Box::<dyn std::error::Error + Send + Sync>::from(format!(
                    "Failed to read client CA: {}",
                    e
                ))
            })?;
            tls = tls.client_ca_root(Certificate::from_pem(client_ca));
        }

        Server::builder().tls_config(tls).map_err(|e| {
            error!("Failed to configure TLS: {}", e);
            Box::<dyn std::error::Error + Send + Sync>::from(format!(
                "Failed to configure TLS: {}",
                e
            ))
        })?
    } else {
        info!("gRPC server running in plaintext mode (no TLS configured)");
        Server::builder()
    };

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

    // Create a basic gRPC server that at least starts successfully
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

    use futures::StreamExt;
    use std::pin::Pin;
    use tokio_stream::wrappers::ReceiverStream;

    #[tonic::async_trait]
    impl Greeter for MockGreeterService {
        type SayHelloStreamStream =
            Pin<Box<dyn futures::Stream<Item = Result<HelloReply, Status>> + Send>>;
        type ChatStream = Pin<Box<dyn futures::Stream<Item = Result<HelloReply, Status>> + Send>>;

        async fn say_hello(
            &self,
            request: Request<HelloRequest>,
        ) -> Result<Response<HelloReply>, Status> {
            info!("gRPC say_hello request: {:?}", request);

            let req = request.into_inner();
            let reply = HelloReply {
                message: format!("Hello {}! This is a mock response from MockForge", req.name),
                metadata: None,
                items: vec![],
            };

            Ok(Response::new(reply))
        }

        /// Server streaming: Returns multiple responses for a single request
        async fn say_hello_stream(
            &self,
            request: Request<HelloRequest>,
        ) -> Result<Response<Self::SayHelloStreamStream>, Status> {
            info!("gRPC say_hello_stream request: {:?}", request);
            let req = request.into_inner();
            let name = req.name.clone();

            // Create a channel to send responses
            let (tx, rx) = tokio::sync::mpsc::channel(128);

            // Spawn a task to send multiple responses
            tokio::spawn(async move {
                for i in 1..=5 {
                    let reply = HelloReply {
                        message: format!(
                            "Hello {}! Stream message {} of 5 from MockForge",
                            name, i
                        ),
                        metadata: None,
                        items: vec![],
                    };

                    if tx.send(Ok(reply)).await.is_err() {
                        // Client disconnected
                        break;
                    }

                    // Small delay between messages to simulate streaming
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            });

            let stream = ReceiverStream::new(rx);
            Ok(Response::new(Box::pin(stream) as Self::SayHelloStreamStream))
        }

        /// Client streaming: Collects multiple requests and returns a single response
        async fn say_hello_client_stream(
            &self,
            request: Request<tonic::Streaming<HelloRequest>>,
        ) -> Result<Response<HelloReply>, Status> {
            info!("gRPC say_hello_client_stream started");

            let mut stream = request.into_inner();
            let mut names = Vec::new();
            let mut count = 0;

            // Collect all incoming requests
            while let Some(req) = stream.next().await {
                match req {
                    Ok(hello_request) => {
                        info!("Received client stream message: {:?}", hello_request);
                        names.push(hello_request.name);
                        count += 1;
                    }
                    Err(e) => {
                        error!("Error receiving client stream message: {}", e);
                        return Err(Status::internal(format!("Stream error: {}", e)));
                    }
                }
            }

            // Create aggregated response
            let message = if names.is_empty() {
                "Hello! No names received in the stream.".to_string()
            } else {
                format!(
                    "Hello {}! Received {} messages from MockForge client stream.",
                    names.join(", "),
                    count
                )
            };

            let reply = HelloReply {
                message,
                metadata: None,
                items: vec![],
            };

            Ok(Response::new(reply))
        }

        /// Bidirectional streaming: Echo back responses for each request
        async fn chat(
            &self,
            request: Request<tonic::Streaming<HelloRequest>>,
        ) -> Result<Response<Self::ChatStream>, Status> {
            info!("gRPC chat (bidirectional streaming) started");

            let mut stream = request.into_inner();
            let (tx, rx) = tokio::sync::mpsc::channel(128);

            // Spawn a task to process incoming messages and send responses
            tokio::spawn(async move {
                let mut message_count = 0;

                while let Some(req) = stream.next().await {
                    match req {
                        Ok(hello_request) => {
                            message_count += 1;
                            info!("Chat received: {:?}", hello_request);

                            let reply = HelloReply {
                                message: format!(
                                    "Chat response {}: Hello {}! from MockForge",
                                    message_count, hello_request.name
                                ),
                                metadata: None,
                                items: vec![],
                            };

                            if tx.send(Ok(reply)).await.is_err() {
                                // Client disconnected
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Chat stream error: {}", e);
                            let _ = tx
                                .send(Err(Status::internal(format!("Stream error: {}", e))))
                                .await;
                            break;
                        }
                    }
                }

                info!("Chat session ended after {} messages", message_count);
            });

            let output_stream = ReceiverStream::new(rx);
            Ok(Response::new(Box::pin(output_stream) as Self::ChatStream))
        }
    }

    let greeter = MockGreeterService;

    info!("gRPC server listening on {} with Greeter service", grpc_addr);

    // Build the server with services
    let mut router = server_builder.add_service(GreeterServer::new(greeter));

    // Add reflection service if enabled
    if config.enable_reflection {
        // Build reflection service from the descriptor pool
        let encoded_fd_set = registry_arc.descriptor_pool().encode_to_vec();
        let reflection_service = ReflectionBuilder::configure()
            .register_encoded_file_descriptor_set(&encoded_fd_set)
            .build_v1()
            .map_err(|e| {
                error!("Failed to build reflection service: {}", e);
                Box::<dyn std::error::Error + Send + Sync>::from(format!(
                    "Failed to build reflection service: {}",
                    e
                ))
            })?;

        router = router.add_service(reflection_service);
        info!("gRPC reflection service enabled");
    }

    router.serve(grpc_addr).await?;

    Ok(())
}

// start_combined_server removed - was a stub that was never implemented

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_compiles() {}
}
