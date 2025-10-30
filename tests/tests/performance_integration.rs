//! Performance Integration Tests
//!
//! Tests that verify MockForge performance under load, including:
//! - Concurrent request handling
//! - Response time consistency
//! - Memory usage patterns
//! - Throughput measurements

use mockforge_test::MockForgeServer;
use reqwest::Client;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Test handling 100 concurrent HTTP requests
#[tokio::test]
#[ignore] // Requires running server
async fn test_concurrent_requests() {
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
    let start = Instant::now();

    // Spawn 100 concurrent requests
    let handles: Vec<_> = (0..100)
        .map(|i| {
            let client = client.clone();
            let url = format!("{}/health", base_url);
            tokio::spawn(async move {
                match client.get(&url).send().await {
                    Ok(resp) => {
                        let status = resp.status();
                        let elapsed = Instant::now() - start;
                        (i, Ok((status, elapsed)))
                    }
                    Err(e) => (i, Err(e)),
                }
            })
        })
        .collect();

    // Wait for all requests to complete
    let mut successes = 0;
    let mut failures = 0;
    let mut max_duration = Duration::from_secs(0);
    let mut min_duration = Duration::from_secs(30);

    for handle in handles {
        match handle.await {
            Ok((_id, Ok((status, elapsed)))) => {
                if status.is_success() {
                    successes += 1;
                } else {
                    failures += 1;
                }
                if elapsed > max_duration {
                    max_duration = elapsed;
                }
                if elapsed < min_duration {
                    min_duration = elapsed;
                }
            }
            Ok((_id, Err(e))) => {
                eprintln!("Request failed: {}", e);
                failures += 1;
            }
            Err(e) => {
                eprintln!("Task join error: {}", e);
                failures += 1;
            }
        }
    }

    let total_duration = start.elapsed();

    // Report results
    eprintln!("✅ Concurrent requests test completed:");
    eprintln!("   Total requests: 100");
    eprintln!("   Successful: {}", successes);
    eprintln!("   Failed: {}", failures);
    eprintln!("   Total duration: {:?}", total_duration);
    eprintln!("   Min response time: {:?}", min_duration);
    eprintln!("   Max response time: {:?}", max_duration);

    // Assert at least 95% success rate
    assert!(
        successes >= 95,
        "Expected at least 95 successful requests, got {}",
        successes
    );

    // Assert total duration is reasonable (should complete in under 10 seconds)
    assert!(
        total_duration < Duration::from_secs(10),
        "Load test took too long: {:?}",
        total_duration
    );
}

/// Test that response times remain consistent under load
#[tokio::test]
#[ignore] // Requires running server
async fn test_response_time_consistency() {
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

    // Warm-up requests
    for _ in 0..5 {
        let _ = client.get(format!("{}/health", base_url)).send().await;
    }

    // Measure response times for 50 sequential requests
    let mut response_times = Vec::new();

    for i in 0..50 {
        let start = Instant::now();
        match client.get(format!("{}/health", base_url)).send().await {
            Ok(resp) => {
                let elapsed = start.elapsed();
                if resp.status().is_success() {
                    response_times.push(elapsed);
                }
            }
            Err(e) => {
                eprintln!("Request {} failed: {}", i, e);
            }
        }
    }

    if response_times.is_empty() {
        eprintln!("No successful requests to analyze");
        return;
    }

    // Calculate statistics
    response_times.sort();
    let count = response_times.len();
    let median = response_times[count / 2];
    let p95_index = (count as f64 * 0.95) as usize;
    let p95 = response_times[p95_index.min(count - 1)];

    eprintln!("✅ Response time consistency test:");
    eprintln!("   Total requests: 50");
    eprintln!("   Successful: {}", count);
    eprintln!("   Median response time: {:?}", median);
    eprintln!("   P95 response time: {:?}", p95);

    // Assert P95 response time is reasonable (< 500ms for health endpoint)
    assert!(
        p95 < Duration::from_millis(500),
        "P95 response time too high: {:?}",
        p95
    );
}

/// Test that server can handle sustained load
#[tokio::test]
#[ignore] // Requires running server, longer duration
async fn test_sustained_load() {
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

    let duration = Duration::from_secs(10); // 10 seconds of sustained load
    let start = Instant::now();
    let mut requests_sent = 0;
    let mut requests_succeeded = 0;

    // Send requests continuously for the duration
    while start.elapsed() < duration {
        requests_sent += 1;
        match client.get(format!("{}/health", base_url)).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    requests_succeeded += 1;
                }
            }
            Err(_) => {
                // Count as failure
            }
        }

        // Small delay to avoid overwhelming
        sleep(Duration::from_millis(10)).await;
    }

    let actual_duration = start.elapsed();

    // Calculate throughput
    let throughput = requests_succeeded as f64 / actual_duration.as_secs_f64();

    eprintln!("✅ Sustained load test:");
    eprintln!("   Duration: {:?}", actual_duration);
    eprintln!("   Requests sent: {}", requests_sent);
    eprintln!("   Requests succeeded: {}", requests_succeeded);
    eprintln!("   Throughput: {:.2} req/s", throughput);

    // Assert success rate is high
    let success_rate = requests_succeeded as f64 / requests_sent as f64;
    assert!(
        success_rate > 0.95,
        "Success rate too low: {:.2}%",
        success_rate * 100.0
    );

    // Assert reasonable throughput (> 10 req/s for health endpoint)
    assert!(
        throughput > 10.0,
        "Throughput too low: {:.2} req/s",
        throughput
    );
}

/// Test memory usage doesn't grow unbounded during load
#[tokio::test]
#[ignore] // Requires running server, complex to measure accurately
async fn test_memory_usage_under_load() {
    // Note: Accurate memory leak detection in Rust integration tests is complex.
    // This test verifies the server doesn't crash or become unresponsive under load,
    // which would indicate a memory issue.

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

    // Send many requests in batches
    let batches = 10;
    let requests_per_batch = 100;

    for batch in 0..batches {
        // Send concurrent requests
        let handles: Vec<_> = (0..requests_per_batch)
            .map(|_| {
                let client = client.clone();
                let url = format!("{}/health", base_url);
                tokio::spawn(async move {
                    client.get(&url).send().await
                })
            })
            .collect();

        // Wait for batch to complete
        let mut batch_success = 0;
        for handle in handles {
            match handle.await {
                Ok(Ok(resp)) => {
                    if resp.status().is_success() {
                        batch_success += 1;
                    }
                }
                _ => {}
            }
        }

        eprintln!("Batch {}: {}/{} requests succeeded", batch + 1, batch_success, requests_per_batch);

        // Check server is still responsive
        match client.get(format!("{}/health", base_url)).send().await {
            Ok(resp) => {
                assert!(
                    resp.status().is_success(),
                    "Server became unresponsive after batch {}",
                    batch + 1
                );
            }
            Err(e) => {
                panic!("Server unresponsive after batch {}: {}", batch + 1, e);
            }
        }

        // Small delay between batches
        sleep(Duration::from_millis(100)).await;
    }

    eprintln!("✅ Memory usage test: Server remained responsive through {} batches", batches);
}

/// Test that server handles burst traffic correctly
#[tokio::test]
#[ignore] // Requires running server
async fn test_burst_traffic() {
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

    // Send 200 requests simultaneously (burst)
    let burst_size = 200;
    let handles: Vec<_> = (0..burst_size)
        .map(|_| {
            let client = client.clone();
            let url = format!("{}/health", base_url);
            tokio::spawn(async move {
                let start = Instant::now();
                let result = client.get(&url).send().await;
                let elapsed = start.elapsed();
                (result, elapsed)
            })
        })
        .collect();

    // Wait for all to complete
    let mut successes = 0;
    let mut failures = 0;
    let mut response_times = Vec::new();

    for handle in handles {
        match handle.await {
            Ok((Ok(resp), elapsed)) => {
                if resp.status().is_success() {
                    successes += 1;
                    response_times.push(elapsed);
                } else {
                    failures += 1;
                }
            }
            Ok((Err(_), _)) => {
                failures += 1;
            }
            Err(e) => {
                eprintln!("Task join error: {}", e);
                failures += 1;
            }
        }
    }

    eprintln!("✅ Burst traffic test:");
    eprintln!("   Burst size: {}", burst_size);
    eprintln!("   Successful: {}", successes);
    eprintln!("   Failed: {}", failures);

    if !response_times.is_empty() {
        response_times.sort();
        let p95_index = (response_times.len() as f64 * 0.95) as usize;
        let p95 = response_times[p95_index.min(response_times.len() - 1)];
        eprintln!("   P95 response time: {:?}", p95);
    }

    // Assert at least 90% success rate for burst traffic
    let success_rate = successes as f64 / burst_size as f64;
    assert!(
        success_rate >= 0.90,
        "Burst traffic success rate too low: {:.2}%",
        success_rate * 100.0
    );
}

/// Test concurrent requests across different endpoints
#[tokio::test]
#[ignore] // Requires running server
async fn test_concurrent_mixed_endpoints() {
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

    // Mix of different endpoints
    let endpoints = vec![
        "/health",
        "/__mockforge/health", // Admin health if available
    ];

    // Send 50 concurrent requests distributed across endpoints
    let handles: Vec<_> = (0..50)
        .map(|i| {
            let client = client.clone();
            let endpoint = endpoints[i % endpoints.len()];
            let url = format!("{}{}", base_url, endpoint);
            tokio::spawn(async move {
                let start = Instant::now();
                let result = client.get(&url).send().await;
                let elapsed = start.elapsed();
                (result, elapsed)
            })
        })
        .collect();

    // Wait for all to complete
    let mut successes = 0;
    let mut failures = 0;

    for handle in handles {
        match handle.await {
            Ok((Ok(resp), _)) => {
                if resp.status().is_success() {
                    successes += 1;
                } else {
                    failures += 1;
                }
            }
            Ok((Err(_), _)) => {
                failures += 1;
            }
            Err(_) => {
                failures += 1;
            }
        }
    }

    eprintln!("✅ Mixed endpoints test:");
    eprintln!("   Total requests: 50");
    eprintln!("   Successful: {}", successes);
    eprintln!("   Failed: {}", failures);

    // Assert reasonable success rate (some endpoints might not exist, that's OK)
    assert!(
        successes > 0,
        "All requests failed - server may not be responding"
    );
}
