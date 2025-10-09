//! Chaos engineering middleware for HTTP

use crate::{
    fault::FaultInjector, latency::LatencyInjector, rate_limit::RateLimiter,
    resilience::{Bulkhead, CircuitBreaker},
    traffic_shaping::TrafficShaper, ChaosConfig,
};
use axum::{
    body::Body,
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use std::{net::SocketAddr, sync::Arc};
use tracing::{debug, warn};

/// Chaos middleware state
#[derive(Clone)]
pub struct ChaosMiddleware {
    latency_injector: Arc<LatencyInjector>,
    fault_injector: Arc<FaultInjector>,
    rate_limiter: Arc<RateLimiter>,
    traffic_shaper: Arc<TrafficShaper>,
    circuit_breaker: Arc<CircuitBreaker>,
    bulkhead: Arc<Bulkhead>,
}

impl ChaosMiddleware {
    /// Create new chaos middleware from config
    pub fn new(config: ChaosConfig) -> Self {
        let latency_injector = Arc::new(LatencyInjector::new(
            config.latency.clone().unwrap_or_default(),
        ));

        let fault_injector = Arc::new(FaultInjector::new(
            config.fault_injection.clone().unwrap_or_default(),
        ));

        let rate_limiter = Arc::new(RateLimiter::new(
            config.rate_limit.clone().unwrap_or_default(),
        ));

        let traffic_shaper = Arc::new(TrafficShaper::new(
            config.traffic_shaping.clone().unwrap_or_default(),
        ));

        let circuit_breaker = Arc::new(CircuitBreaker::new(
            config.circuit_breaker.clone().unwrap_or_default(),
        ));

        let bulkhead = Arc::new(Bulkhead::new(
            config.bulkhead.clone().unwrap_or_default(),
        ));

        Self {
            latency_injector,
            fault_injector,
            rate_limiter,
            traffic_shaper,
            circuit_breaker,
            bulkhead,
        }
    }

    /// Get latency injector
    pub fn latency_injector(&self) -> &Arc<LatencyInjector> {
        &self.latency_injector
    }

    /// Get fault injector
    pub fn fault_injector(&self) -> &Arc<FaultInjector> {
        &self.fault_injector
    }

    /// Get rate limiter
    pub fn rate_limiter(&self) -> &Arc<RateLimiter> {
        &self.rate_limiter
    }

    /// Get traffic shaper
    pub fn traffic_shaper(&self) -> &Arc<TrafficShaper> {
        &self.traffic_shaper
    }

    /// Get circuit breaker
    pub fn circuit_breaker(&self) -> &Arc<CircuitBreaker> {
        &self.circuit_breaker
    }

    /// Get bulkhead
    pub fn bulkhead(&self) -> &Arc<Bulkhead> {
        &self.bulkhead
    }
}

/// Chaos middleware handler
pub async fn chaos_middleware(
    chaos: Arc<ChaosMiddleware>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    let ip = addr.ip().to_string();

    debug!("Chaos middleware processing: {} {}", req.method(), path);

    // Check circuit breaker
    if !chaos.circuit_breaker.allow_request().await {
        warn!("Circuit breaker open, rejecting request: {}", path);
        return (StatusCode::SERVICE_UNAVAILABLE, "Service temporarily unavailable (circuit breaker open)").into_response();
    }

    // Try to acquire bulkhead slot
    let _bulkhead_guard = match chaos.bulkhead.try_acquire().await {
        Ok(guard) => guard,
        Err(e) => {
            warn!("Bulkhead rejected request: {} - {:?}", path, e);
            return (StatusCode::SERVICE_UNAVAILABLE, format!("Service overloaded: {}", e)).into_response();
        }
    };

    // Check rate limits
    if let Err(e) = chaos.rate_limiter.check(Some(&ip), Some(&path)) {
        warn!("Rate limit exceeded: {} - {}", ip, path);
        return (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded").into_response();
    }

    // Check connection limits
    if !chaos.traffic_shaper.check_connection_limit() {
        warn!("Connection limit exceeded");
        return (StatusCode::SERVICE_UNAVAILABLE, "Connection limit exceeded").into_response();
    }

    // Always release connection on scope exit
    let _connection_guard = crate::traffic_shaping::ConnectionGuard::new(chaos.traffic_shaper.as_ref().clone());

    // Check for packet loss (simulate dropped connection)
    if chaos.traffic_shaper.should_drop_packet() {
        warn!("Simulating packet loss for: {}", path);
        return (StatusCode::REQUEST_TIMEOUT, "Connection dropped").into_response();
    }

    // Inject latency
    chaos.latency_injector.inject().await;

    // Check for fault injection
    if let Some(status_code) = chaos.fault_injector.get_http_error_status() {
        warn!("Injecting HTTP error: {}", status_code);
        return (
            StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            format!("Injected error: {}", status_code),
        )
            .into_response();
    }

    // Extract body size for bandwidth throttling
    let (parts, body) = req.into_parts();
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            warn!("Failed to read request body: {}", e);
            return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response();
        }
    };

    let request_size = body_bytes.len();

    // Throttle request bandwidth
    chaos.traffic_shaper.throttle_bandwidth(request_size).await;

    // Reconstruct request
    let req = Request::from_parts(parts, Body::from(body_bytes));

    // Pass to next handler
    let response = next.run(req).await;

    // Record circuit breaker result based on response status
    let status = response.status();
    if status.is_server_error() || status == StatusCode::SERVICE_UNAVAILABLE {
        chaos.circuit_breaker.record_failure().await;
    } else if status.is_success() {
        chaos.circuit_breaker.record_success().await;
    }

    // Extract response body size for bandwidth throttling
    let (parts, body) = response.into_parts();
    let response_body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            warn!("Failed to read response body: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read response body")
                .into_response();
        }
    };

    let response_size = response_body_bytes.len();

    // Check if should truncate response (partial response simulation)
    let final_body = if chaos.fault_injector.should_truncate_response() {
        warn!("Injecting partial response");
        let truncate_at = response_size / 2;
        Body::from(response_body_bytes.slice(0..truncate_at))
    } else {
        Body::from(response_body_bytes)
    };

    // Throttle response bandwidth
    chaos.traffic_shaper.throttle_bandwidth(response_size).await;

    Response::from_parts(parts, final_body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LatencyConfig, RateLimitConfig};

    #[tokio::test]
    async fn test_middleware_creation() {
        let config = ChaosConfig {
            enabled: true,
            latency: Some(LatencyConfig {
                enabled: true,
                fixed_delay_ms: Some(10),
                ..Default::default()
            }),
            ..Default::default()
        };

        let middleware = ChaosMiddleware::new(config);
        assert!(middleware.latency_injector.is_enabled());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let config = ChaosConfig {
            enabled: true,
            rate_limit: Some(RateLimitConfig {
                enabled: true,
                requests_per_second: 1,
                burst_size: 2,  // burst_size is the total capacity, not additional requests
                ..Default::default()
            }),
            ..Default::default()
        };

        let middleware = Arc::new(ChaosMiddleware::new(config));

        // First two requests should succeed (rate + burst)
        assert!(middleware
            .rate_limiter
            .check(Some("127.0.0.1"), Some("/test"))
            .is_ok());
        assert!(middleware
            .rate_limiter
            .check(Some("127.0.0.1"), Some("/test"))
            .is_ok());

        // Third should fail
        assert!(middleware
            .rate_limiter
            .check(Some("127.0.0.1"), Some("/test"))
            .is_err());
    }
}
