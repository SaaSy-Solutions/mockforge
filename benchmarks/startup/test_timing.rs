#!/usr/bin/env rust-script
//! Test the startup timing instrumentation
//!
//! ```cargo
//! [dependencies]
//! mockforge-http = { path = "../../crates/mockforge-http" }
//! mockforge-grpc = { path = "../../crates/mockforge-grpc" }
//! mockforge-core = { path = "../../crates/mockforge-core" }
//! tokio = { version = "1", features = ["full"] }
//! tracing = "0.1"
//! tracing-subscriber = { version = "0.3", features = ["env-filter"] }
//! ```

use mockforge_http::build_router;
use mockforge_grpc::{start_with_config, DynamicGrpcConfig};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("=== MockForge Startup Timing Test ===\n");

    // Test 1: HTTP with large OpenAPI spec
    println!("Test 1: HTTP Server with 100-endpoint OpenAPI spec");
    let start = Instant::now();
    let _router = build_router(
        Some("benchmarks/startup/large_api_100_endpoints.json".to_string()),
        None,
        None,
    )
    .await;
    let duration = start.elapsed();
    println!("Total time (manual measurement): {:?}\n", duration);

    // Test 2: gRPC with proto files
    println!("Test 2: gRPC Server discovery with proto files");
    let config = DynamicGrpcConfig {
        proto_dir: "benchmarks/startup/proto".to_string(),
        enable_reflection: true,
        ..Default::default()
    };

    let start = Instant::now();
    let registry = mockforge_grpc::dynamic::discover_services(&config).await?;
    let duration = start.elapsed();
    println!("Service discovery time (manual measurement): {:?}", duration);
    println!("Services discovered: {}", registry.service_names().len());

    Ok(())
}
