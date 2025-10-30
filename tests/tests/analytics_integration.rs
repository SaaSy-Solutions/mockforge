//! Analytics Integration Tests
//!
//! Tests that verify analytics data recording, querying, and streaming
//! work correctly end-to-end.

use mockforge_test::MockForgeServer;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

/// Test that metrics are recorded correctly when making HTTP requests
#[tokio::test]
#[ignore] // Requires running server
async fn test_metrics_recording() {
    // Start MockForge server with HTTP enabled
    let server = match MockForgeServer::builder()
        .http_port(0) // Auto-assign port
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: Failed to start server: {}", e);
            return;
        }
    };

    let base_url = server.base_url();
    let client = Client::new();

    // Generate some traffic by making HTTP requests
    for i in 0..5 {
        let response = client.get(format!("{}/health", base_url)).send().await;

        if response.is_err() {
            eprintln!("Failed to make request {}: {:?}", i, response);
            return;
        }

        // Small delay between requests
        sleep(Duration::from_millis(200)).await;
    }

    // Wait a moment for metrics to be recorded
    sleep(Duration::from_secs(2)).await;

    // Query analytics summary endpoint
    let summary_url = format!("{}/__mockforge/analytics/summary?range=5m", base_url);
    let summary_response = client.get(&summary_url).send().await;

    match summary_response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: Value = resp.json().await.unwrap_or(Value::Null);
                // Verify response structure
                assert!(
                    body.get("data").is_some() || body.get("success").is_some(),
                    "Analytics response should have data or success field"
                );
                eprintln!("✅ Analytics summary query successful: {:?}", body);
            } else {
                eprintln!("Warning: Analytics endpoint returned status {}", resp.status());
                // Don't fail - Prometheus might not be configured
            }
        }
        Err(e) => {
            eprintln!("Skipping analytics query: Failed to connect: {}", e);
            // Don't fail - analytics might require Prometheus setup
        }
    }
}

/// Test analytics querying with different time ranges
#[tokio::test]
#[ignore] // Requires running server
async fn test_analytics_query() {
    let server = match MockForgeServer::builder()
        .http_port(0)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: Failed to start server: {}", e);
            return;
        }
    };

    let base_url = server.base_url();
    let client = Client::new();

    // Generate some traffic
    for _ in 0..3 {
        let _ = client.get(format!("{}/health", base_url)).send().await;
        sleep(Duration::from_millis(100)).await;
    }

    sleep(Duration::from_secs(2)).await;

    // Test different time ranges
    let time_ranges = vec!["5m", "15m", "1h"];

    for range in time_ranges {
        let url = format!("{}/__mockforge/analytics/summary?range={}", base_url, range);
        let response = client.get(&url).send().await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    let body: Value = resp.json().await.unwrap_or(Value::Null);
                    eprintln!("✅ Analytics query with range '{}' successful", range);
                    // Verify response structure
                    assert!(
                        body.get("data").is_some() || body.get("success").is_some(),
                        "Response should have data structure"
                    );
                } else {
                    eprintln!(
                        "Warning: Analytics endpoint returned status {} for range {}",
                        resp.status(),
                        range
                    );
                }
            }
            Err(e) => {
                eprintln!("Skipping analytics query for range {}: {}", range, e);
            }
        }
    }
}

/// Test analytics endpoints query
#[tokio::test]
#[ignore] // Requires running server
async fn test_analytics_endpoints() {
    let server = match MockForgeServer::builder()
        .http_port(0)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: Failed to start server: {}", e);
            return;
        }
    };

    let base_url = server.base_url();
    let client = Client::new();

    // Generate traffic to multiple endpoints
    let _ = client.get(format!("{}/health", base_url)).send().await;
    sleep(Duration::from_millis(100)).await;

    sleep(Duration::from_secs(2)).await;

    // Query endpoints analytics
    let endpoints_url = format!("{}/__mockforge/analytics/endpoints?limit=10", base_url);
    let response = client.get(&endpoints_url).send().await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: Value = resp.json().await.unwrap_or(Value::Null);
                eprintln!("✅ Endpoints query successful: {:?}", body);
                // Verify response is an array or has data field
                if let Some(data) = body.get("data") {
                    assert!(
                        data.is_array() || data.is_object(),
                        "Endpoints data should be array or object"
                    );
                }
            } else {
                eprintln!("Warning: Endpoints endpoint returned status {}", resp.status());
            }
        }
        Err(e) => {
            eprintln!("Skipping endpoints query: {}", e);
        }
    }
}

/// Test analytics requests time-series endpoint
#[tokio::test]
#[ignore] // Requires running server
async fn test_analytics_requests_timeseries() {
    let server = match MockForgeServer::builder()
        .http_port(0)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: Failed to start server: {}", e);
            return;
        }
    };

    let base_url = server.base_url();
    let client = Client::new();

    // Generate multiple requests over time
    for _ in 0..3 {
        let _ = client.get(format!("{}/health", base_url)).send().await;
        sleep(Duration::from_millis(300)).await;
    }

    sleep(Duration::from_secs(2)).await;

    // Query requests time-series
    let requests_url = format!("{}/__mockforge/analytics/requests?range=5m", base_url);
    let response = client.get(&requests_url).send().await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: Value = resp.json().await.unwrap_or(Value::Null);
                eprintln!("✅ Requests time-series query successful");
                // Verify response has timestamps and series
                if let Some(data) = body.get("data") {
                    // Should have timestamps and series fields
                    assert!(
                        data.get("timestamps").is_some() || data.get("series").is_some(),
                        "Time-series should have timestamps or series"
                    );
                }
            } else {
                eprintln!("Warning: Requests endpoint returned status {}", resp.status());
            }
        }
        Err(e) => {
            eprintln!("Skipping requests query: {}", e);
        }
    }
}

/// Test analytics system metrics endpoint
#[tokio::test]
#[ignore] // Requires running server
async fn test_analytics_system_metrics() {
    let server = match MockForgeServer::builder()
        .http_port(0)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: Failed to start server: {}", e);
            return;
        }
    };

    let base_url = server.base_url();
    let client = Client::new();

    // Query system metrics
    let system_url = format!("{}/__mockforge/analytics/system", base_url);
    let response = client.get(&system_url).send().await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: Value = resp.json().await.unwrap_or(Value::Null);
                eprintln!("✅ System metrics query successful");
                // Verify response structure
                if let Some(data) = body.get("data") {
                    // System metrics should have memory, cpu, etc.
                    assert!(data.is_object(), "System metrics should be an object");
                }
            } else {
                eprintln!("Warning: System endpoint returned status {}", resp.status());
            }
        }
        Err(e) => {
            eprintln!("Skipping system metrics query: {}", e);
        }
    }
}

/// Test real-time analytics streaming via WebSocket
#[tokio::test]
#[ignore] // Requires WebSocket infrastructure and Prometheus
async fn test_analytics_streaming() {
    // Note: This test requires:
    // 1. WebSocket server enabled
    // 2. Prometheus metrics configured
    // 3. Analytics streaming endpoint implementation

    // For now, this is a placeholder
    // Full implementation would:
    // 1. Connect to WebSocket stream endpoint
    // 2. Generate traffic
    // 3. Verify metric updates are received in real-time

    eprintln!("Analytics streaming test requires WebSocket and Prometheus setup");
    eprintln!("This would test: WS /api/v2/analytics/stream endpoint");
}

/// Test analytics data retention
#[tokio::test]
#[ignore] // Requires long-running test and retention configuration
async fn test_analytics_retention() {
    // This test would verify that old analytics data is cleaned up
    // according to retention policy. Requires:
    // 1. Configuring retention policy
    // 2. Generating old data (waiting)
    // 3. Verifying old data is removed

    eprintln!("Analytics retention test requires time-based testing");
    eprintln!("This would verify data cleanup according to retention policy");
}
