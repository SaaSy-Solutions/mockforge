//! # MockForge gRPC
//!
//! Flexible gRPC mocking library with dynamic service discovery and HTTP bridge.
//!
//! This crate provides comprehensive gRPC mocking capabilities with:
//!
//! - **Dynamic Service Discovery**: Auto-discover and mock services from `.proto` files
//! - **HTTP Bridge**: Expose gRPC services as REST APIs with OpenAPI documentation
//! - **gRPC Reflection**: Built-in server reflection for service discovery
//! - **Streaming Support**: Full support for unary, server, client, and bidirectional streaming
//! - **Protocol Buffer Parsing**: Runtime parsing of `.proto` files without code generation
//!
//! ## Overview
//!
//! MockForge gRPC eliminates the need to hardcode service implementations. Simply provide
//! `.proto` files, and MockForge will automatically:
//!
//! 1. Parse protobuf definitions
//! 2. Generate mock service implementations
//! 3. Handle all RPC methods (unary and streaming)
//! 4. Optionally expose as REST APIs via HTTP Bridge
//!
//! ## Quick Start
//!
//! ### Basic gRPC Server
//!
//! ```rust,no_run
//! use mockforge_grpc::start;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     // Start gRPC server on port 50051
//!     // Automatically discovers .proto files in ./proto directory
//!     start(50051).await?;
//!     Ok(())
//! }
//! ```
//!
//! ### With Custom Configuration
//!
//! ```rust,no_run
//! use mockforge_grpc::{start_with_config, DynamicGrpcConfig};
//! use mockforge_core::LatencyProfile;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! let config = DynamicGrpcConfig {
//!     proto_dir: "./my-protos".to_string(),
//!     enable_reflection: true,
//!     max_message_size: 8 * 1024 * 1024, // 8MB
//!     ..Default::default()
//! };
//!
//! let latency = Some(LatencyProfile::normal());
//! start_with_config(50051, latency, config).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### HTTP Bridge Mode
//!
//! Expose gRPC services as REST APIs:
//!
//! ```rust,no_run
//! use mockforge_grpc::{DynamicGrpcConfig, start_with_config};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! let config = DynamicGrpcConfig {
//!     proto_dir: "./proto".to_string(),
//!     enable_http_bridge: true,
//!     http_bridge_port: 8080,
//!     generate_openapi: true,
//!     ..Default::default()
//! };
//!
//! start_with_config(50051, None, config).await?;
//! // Now accessible via:
//! // - gRPC: localhost:50051
//! // - REST: http://localhost:8080/api/{service}/{method}
//! // - OpenAPI: http://localhost:8080/api/docs
//! # Ok(())
//! # }
//! ```
//!
//! ## Dynamic Service Discovery
//!
//! MockForge automatically discovers and mocks all services defined in your `.proto` files:
//!
//! ```protobuf
//! // user.proto
//! service UserService {
//!   rpc GetUser(GetUserRequest) returns (GetUserResponse);
//!   rpc ListUsers(ListUsersRequest) returns (stream User);
//!   rpc CreateUser(stream CreateUserRequest) returns (CreateUserResponse);
//!   rpc Chat(stream ChatMessage) returns (stream ChatMessage);
//! }
//! ```
//!
//! All four method types (unary, server streaming, client streaming, bidirectional) are
//! automatically supported without any code generation or manual implementation.
//!
//! ## gRPC Reflection
//!
//! Enable reflection for service discovery by gRPC clients:
//!
//! ```bash
//! # List services
//! grpcurl -plaintext localhost:50051 list
//!
//! # Describe a service
//! grpcurl -plaintext localhost:50051 describe UserService
//!
//! # Call a method
//! grpcurl -plaintext -d '{"user_id": "123"}' localhost:50051 UserService/GetUser
//! ```
//!
//! ## HTTP Bridge
//!
//! The HTTP Bridge automatically converts gRPC services to REST endpoints:
//!
//! ```bash
//! # gRPC call
//! grpcurl -d '{"user_id": "123"}' localhost:50051 UserService/GetUser
//!
//! # Equivalent HTTP call
//! curl -X POST http://localhost:8080/api/userservice/getuser \
//!   -H "Content-Type: application/json" \
//!   -d '{"user_id": "123"}'
//!
//! # OpenAPI documentation
//! curl http://localhost:8080/api/docs
//! ```
//!
//! ## Advanced Data Synthesis
//!
//! Generate realistic mock data using intelligent field inference:
//!
//! - Detects field types from names (`email`, `phone`, `id`, etc.)
//! - Maintains referential integrity across related messages
//! - Supports deterministic seeding for reproducible tests
//!
//! ## Key Modules
//!
//! - [`dynamic`]: Dynamic service discovery and mocking
//! - [`reflection`]: gRPC reflection protocol implementation
//! - [`registry`]: Service and method registry
//!
//! ## Examples
//!
//! See the [examples directory](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples)
//! for complete working examples.
//!
//! ## Related Crates
//!
//! - [`mockforge-core`](https://docs.rs/mockforge-core): Core mocking functionality
//! - [`mockforge-data`](https://docs.rs/mockforge-data): Synthetic data generation
//!
//! ## Documentation
//!
//! - [MockForge Book](https://docs.mockforge.dev/)
//! - [gRPC Mocking Guide](https://docs.mockforge.dev/user-guide/grpc-mocking.html)
//! - [API Reference](https://docs.rs/mockforge-grpc)

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
