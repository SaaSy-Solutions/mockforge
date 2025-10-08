//! Startup performance benchmark example
//!
//! This example demonstrates and measures the startup time of MockForge
//! with a large OpenAPI specification.
//!
//! Run with: cargo run --example startup_benchmark --release

use mockforge_http::build_router;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging to see timing information
    tracing_subscriber::fmt()
        .with_env_filter("info,mockforge_http=info,mockforge_grpc=info")
        .init();

    println!("=== MockForge Startup Performance Benchmark ===\n");

    // Test 1: Baseline (no spec)
    println!("--- Test 1: Baseline (no OpenAPI spec) ---");
    let start = Instant::now();
    let _router = build_router(None, None, None).await;
    let duration = start.elapsed();
    println!("Baseline startup time: {:?}\n", duration);

    // Test 2: Large OpenAPI spec
    println!("--- Test 2: Large OpenAPI spec (100 endpoints) ---");
    let spec_path = "benchmarks/startup/large_api_100_endpoints.json";

    // Check if file exists
    if !std::path::Path::new(spec_path).exists() {
        eprintln!("Error: Benchmark spec not found at {}", spec_path);
        eprintln!("Please run from the project root directory");
        return Ok(());
    }

    let start = Instant::now();
    let _router = build_router(Some(spec_path.to_string()), None, None).await;
    let duration = start.elapsed();
    println!("\nLarge spec startup time: {:?}", duration);

    println!("\n=== Benchmark Complete ===");
    println!("\nCheck the logs above for detailed timing breakdown:");
    println!("  - OpenAPI spec loading");
    println!("  - Route registry creation");
    println!("  - Route extraction");
    println!("  - Router building");
    println!("  - Total startup time");

    Ok(())
}
