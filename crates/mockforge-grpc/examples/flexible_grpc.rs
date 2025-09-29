//! Example demonstrating flexible gRPC service discovery
//!
//! This example shows how to use MockForge's flexible gRPC system
//! to discover and mock services from any proto files.

use mockforge_grpc::{start_with_config, DynamicGrpcConfig};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get proto directory from environment variable or use default
    let proto_dir = env::var("MOCKFORGE_PROTO_DIR")
        .unwrap_or_else(|_| "crates/mockforge-grpc/proto".to_string());

    // Create dynamic configuration
    let config = DynamicGrpcConfig {
        proto_dir,
        enable_reflection: true,
        excluded_services: vec!["grpc.reflection.v1alpha.ServerReflection".to_string()],
        http_bridge: None,
    };

    println!("Starting MockForge gRPC server with flexible proto discovery");
    println!("Proto directory: {}", config.proto_dir);
    println!("Reflection enabled: {}", config.enable_reflection);

    // Start the server with dynamic configuration
    start_with_config(50051, None, config).await?;

    Ok(())
}
