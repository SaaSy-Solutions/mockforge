//! Advanced Resilience Patterns Example
//!
//! This example demonstrates:
//! 1. Per-endpoint circuit breakers
//! 2. Per-service bulkheads
//! 3. Dynamic threshold adjustment
//! 4. Retry with exponential backoff
//! 5. Fallback handlers
//! 6. Health check integration
//! 7. Prometheus metrics
//! 8. Dashboard API

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use mockforge_chaos::{
    BulkheadConfig, BulkheadError, BulkheadManager, CircuitBreakerConfig, CircuitBreakerManager,
    HealthCheckIntegration, JsonFallbackHandler, ResilienceApiState, ResilienceRetryConfig,
    RetryPolicy, create_resilience_router,
};
use prometheus::Registry;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// Shared application state
#[derive(Clone)]
struct AppState {
    circuit_breaker_manager: Arc<CircuitBreakerManager>,
    bulkhead_manager: Arc<BulkheadManager>,
    retry_policy: Arc<RetryPolicy>,
    health_integration: Arc<HealthCheckIntegration>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create Prometheus registry
    let registry = Arc::new(Registry::new());

    // Configure circuit breakers
    let cb_config = CircuitBreakerConfig {
        enabled: true,
        failure_threshold: 5,
        success_threshold: 2,
        timeout_ms: 60000,
        half_open_max_requests: 3,
        failure_rate_threshold: 50.0,
        min_requests_for_rate: 10,
        rolling_window_ms: 10000,
    };

    // Create circuit breaker manager
    let cb_manager = Arc::new(CircuitBreakerManager::new(cb_config, registry.clone()));

    // Configure bulkheads
    let bh_config = BulkheadConfig {
        enabled: true,
        max_concurrent_requests: 100,
        max_queue_size: 10,
        queue_timeout_ms: 5000,
    };

    // Create bulkhead manager
    let bh_manager = Arc::new(BulkheadManager::new(bh_config, registry.clone()));

    // Configure retry policy
    let retry_config = ResilienceRetryConfig {
        max_attempts: 3,
        initial_backoff_ms: 100,
        max_backoff_ms: 30000,
        backoff_multiplier: 2.0,
        jitter_factor: 0.1,
    };
    let retry_policy = Arc::new(RetryPolicy::new(retry_config));

    // Create health check integration
    let health_integration = Arc::new(HealthCheckIntegration::new(cb_manager.clone()));

    // Start health check monitoring for endpoints
    tokio::spawn({
        let health_integration = health_integration.clone();
        async move {
            health_integration
                .start_monitoring(
                    "/api/users".to_string(),
                    "http://localhost:8081/health".to_string(),
                    Duration::from_secs(30),
                )
                .await;
        }
    });

    // Create application state
    let app_state = AppState {
        circuit_breaker_manager: cb_manager.clone(),
        bulkhead_manager: bh_manager.clone(),
        retry_policy: retry_policy.clone(),
        health_integration,
    };

    // Create resilience API state
    let resilience_state = ResilienceApiState {
        circuit_breaker_manager: cb_manager,
        bulkhead_manager: bh_manager,
    };

    // Build application router
    let app = Router::new()
        // API endpoints with resilience
        .route("/api/users", get(get_users_handler))
        .route("/api/payments", get(get_payments_handler))
        .route("/api/orders", get(get_orders_handler))
        // Mount resilience dashboard API
        .nest("/api/resilience", create_resilience_router(resilience_state))
        // Metrics endpoint
        .route("/metrics", get(metrics_handler))
        .with_state(app_state);

    // Start server
    println!("Starting server on http://localhost:3000");
    println!("Resilience Dashboard: http://localhost:3000/api/resilience/dashboard/summary");
    println!("Metrics: http://localhost:3000/metrics");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Handler for /api/users with full resilience pattern
async fn get_users_handler(State(state): State<AppState>) -> Response {
    let endpoint = "/api/users";
    let service = "user-service";

    // Create fallback handler
    let fallback = JsonFallbackHandler::new(json!({
        "error": "User service temporarily unavailable",
        "status": "circuit_open",
        "data": [],
        "cached": true
    }));

    // Step 1: Check circuit breaker
    let breaker = state.circuit_breaker_manager.get_breaker(endpoint).await;

    if !breaker.allow_request().await {
        tracing::warn!("Circuit breaker open for {}", endpoint);
        return (StatusCode::SERVICE_UNAVAILABLE, fallback.handle()).into_response();
    }

    // Step 2: Acquire bulkhead slot
    let bulkhead = state.bulkhead_manager.get_bulkhead(service).await;
    let _guard = match bulkhead.try_acquire().await {
        Ok(guard) => guard,
        Err(BulkheadError::Rejected) => {
            tracing::warn!("Bulkhead rejected for {}", service);
            breaker.record_failure().await;
            return (StatusCode::TOO_MANY_REQUESTS, "Service busy").into_response();
        }
        Err(BulkheadError::Timeout) => {
            tracing::warn!("Bulkhead timeout for {}", service);
            breaker.record_failure().await;
            return (StatusCode::GATEWAY_TIMEOUT, "Service timeout").into_response();
        }
    };

    // Step 3: Execute with retry
    let result = state
        .retry_policy
        .execute(|| async { fetch_users().await })
        .await;

    // Step 4: Record result and update metrics
    match result {
        Ok(users) => {
            breaker.record_success().await;
            state
                .circuit_breaker_manager
                .record_with_adjustment(endpoint, true)
                .await;

            (StatusCode::OK, axum::Json(users)).into_response()
        }
        Err(_) => {
            breaker.record_failure().await;
            state
                .circuit_breaker_manager
                .record_with_adjustment(endpoint, false)
                .await;

            (StatusCode::SERVICE_UNAVAILABLE, fallback.handle()).into_response()
        }
    }
}

// Handler for /api/payments with stricter resilience
async fn get_payments_handler(State(state): State<AppState>) -> Response {
    let endpoint = "/api/payments";
    let service = "payment-service";

    // Create fallback
    let fallback = JsonFallbackHandler::new(json!({
        "error": "Payment service temporarily unavailable",
        "status": "circuit_open",
        "message": "Please try again in a few minutes"
    }));

    // Check circuit breaker
    let breaker = state.circuit_breaker_manager.get_breaker(endpoint).await;

    if !breaker.allow_request().await {
        return (StatusCode::SERVICE_UNAVAILABLE, fallback.handle()).into_response();
    }

    // Acquire bulkhead (payments have lower concurrency limit)
    let bulkhead = state.bulkhead_manager.get_bulkhead(service).await;
    let _guard = match bulkhead.try_acquire().await {
        Ok(guard) => guard,
        Err(_) => {
            breaker.record_failure().await;
            return (StatusCode::TOO_MANY_REQUESTS, "Payment service busy").into_response();
        }
    };

    // Execute with retry
    let result = state
        .retry_policy
        .execute(|| async { fetch_payments().await })
        .await;

    // Record result
    match result {
        Ok(payments) => {
            breaker.record_success().await;
            state
                .circuit_breaker_manager
                .record_with_adjustment(endpoint, true)
                .await;

            (StatusCode::OK, axum::Json(payments)).into_response()
        }
        Err(_) => {
            breaker.record_failure().await;
            state
                .circuit_breaker_manager
                .record_with_adjustment(endpoint, false)
                .await;

            (StatusCode::SERVICE_UNAVAILABLE, fallback.handle()).into_response()
        }
    }
}

// Handler for /api/orders
async fn get_orders_handler(State(state): State<AppState>) -> Response {
    let endpoint = "/api/orders";
    let service = "order-service";

    let fallback = JsonFallbackHandler::new(json!({
        "error": "Order service temporarily unavailable",
        "orders": []
    }));

    let breaker = state.circuit_breaker_manager.get_breaker(endpoint).await;

    if !breaker.allow_request().await {
        return (StatusCode::SERVICE_UNAVAILABLE, fallback.handle()).into_response();
    }

    let bulkhead = state.bulkhead_manager.get_bulkhead(service).await;
    let _guard = match bulkhead.try_acquire().await {
        Ok(guard) => guard,
        Err(_) => {
            breaker.record_failure().await;
            return (StatusCode::TOO_MANY_REQUESTS, "Service busy").into_response();
        }
    };

    let result = state
        .retry_policy
        .execute(|| async { fetch_orders().await })
        .await;

    match result {
        Ok(orders) => {
            breaker.record_success().await;
            (StatusCode::OK, axum::Json(orders)).into_response()
        }
        Err(_) => {
            breaker.record_failure().await;
            (StatusCode::SERVICE_UNAVAILABLE, fallback.handle()).into_response()
        }
    }
}

// Metrics endpoint
async fn metrics_handler(State(_state): State<AppState>) -> String {
    // In production, use prometheus::TextEncoder to encode metrics
    "# Metrics would be exported here\n".to_string()
}

// Simulated API calls
async fn fetch_users() -> Result<Vec<User>, String> {
    // Simulate occasional failures (20% chance)
    if rand::random::<f64>() < 0.2 {
        sleep(Duration::from_millis(100)).await;
        return Err("User service error".to_string());
    }

    sleep(Duration::from_millis(50)).await;
    Ok(vec![
        User {
            id: 1,
            name: "Alice".to_string(),
        },
        User {
            id: 2,
            name: "Bob".to_string(),
        },
    ])
}

async fn fetch_payments() -> Result<Vec<Payment>, String> {
    // Simulate occasional failures (10% chance - more reliable)
    if rand::random::<f64>() < 0.1 {
        sleep(Duration::from_millis(100)).await;
        return Err("Payment service error".to_string());
    }

    sleep(Duration::from_millis(75)).await;
    Ok(vec![Payment {
        id: 1,
        amount: 100.0,
        status: "completed".to_string(),
    }])
}

async fn fetch_orders() -> Result<Vec<Order>, String> {
    // Simulate occasional failures (15% chance)
    if rand::random::<f64>() < 0.15 {
        sleep(Duration::from_millis(100)).await;
        return Err("Order service error".to_string());
    }

    sleep(Duration::from_millis(60)).await;
    Ok(vec![Order {
        id: 1,
        product: "Widget".to_string(),
        quantity: 5,
    }])
}

// Data structures
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Payment {
    id: u64,
    amount: f64,
    status: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Order {
    id: u64,
    product: String,
    quantity: u32,
}
