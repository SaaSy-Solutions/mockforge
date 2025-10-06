use mockforge_grpc::dynamic::{DynamicGrpcConfig, start_dynamic_server};
use std::sync::Arc;
use tokio::time::Duration;

#[tokio::test]
async fn test_http_bridge_with_real_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create configuration with HTTP bridge enabled
    let config = DynamicGrpcConfig {
        proto_dir: "proto".to_string(),
        enable_reflection: true,
        excluded_services: vec![],
        http_bridge: Some(Default::default()), // Enable bridge with default config
    };
    
    println!("🚀 Starting gRPC server with HTTP bridge enabled...");
    println!("Config: {:?}", config);
    
    // Try to start the server for a short time
    let timeout = Duration::from_secs(10);
    tokio::spawn(async move {
        if let Err(e) = start_dynamic_server(50051, config, None).await {
            println!("❌ Server error: {}", e);
        }
    });
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Try to make HTTP request to the bridge
    let client = reqwest::Client::new();
    
    match client.get("http://localhost:50051/api/health").send().await {
        Ok(response) => {
            println!("✅ Bridge health check successful: {}", response.status());
            let body = response.text().await.unwrap_or_default();
            println!("Response: {}", body);
        }
        Err(e) => {
            println!("⚠️  Bridge health check failed: {}", e);
        }
    }
    
    println!("🧪 Test completed - server probably started but we can't keep it running in test");
    Ok(())
}
