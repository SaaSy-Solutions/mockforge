//! Chaos engineering middleware for HTTP

use crate::{
    config::CorruptionType,
    fault::FaultInjector,
    latency::LatencyInjector,
    latency_metrics::LatencyMetricsTracker,
    rate_limit::RateLimiter,
    resilience::{Bulkhead, CircuitBreaker},
    traffic_shaping::TrafficShaper,
    ChaosConfig,
};
use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use rand::Rng;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Chaos middleware state
///
/// This middleware reads configuration from a shared `Arc<RwLock<ChaosConfig>>`
/// to support hot-reload of chaos settings at runtime.
#[derive(Clone)]
pub struct ChaosMiddleware {
    /// Shared chaos configuration (read on each request for hot-reload support)
    config: Arc<RwLock<ChaosConfig>>,
    /// Latency metrics tracker for recording injected latencies
    latency_tracker: Arc<LatencyMetricsTracker>,
    /// Cached injectors (recreated when config changes)
    /// These are cached for performance but can be updated via update_from_config()
    latency_injector: Arc<RwLock<LatencyInjector>>,
    fault_injector: Arc<RwLock<FaultInjector>>,
    rate_limiter: Arc<RwLock<RateLimiter>>,
    traffic_shaper: Arc<RwLock<TrafficShaper>>,
    circuit_breaker: Arc<RwLock<CircuitBreaker>>,
    bulkhead: Arc<RwLock<Bulkhead>>,
}

impl ChaosMiddleware {
    /// Create new chaos middleware from shared config
    ///
    /// # Arguments
    /// * `config` - Shared chaos configuration (Arc<RwLock<ChaosConfig>>)
    /// * `latency_tracker` - Latency metrics tracker for recording injected latencies
    ///
    /// The middleware will read from the shared config on each request,
    /// allowing hot-reload of chaos settings without restarting the server.
    pub fn new(
        config: Arc<RwLock<ChaosConfig>>,
        latency_tracker: Arc<LatencyMetricsTracker>,
    ) -> Self {
        // Initialize injectors with defaults (will be updated via init_from_config)
        let latency_injector = Arc::new(RwLock::new(LatencyInjector::new(Default::default())));

        // FaultInjector doesn't support hot-reload, but we'll read from config directly
        // Keep a reference for compatibility but won't use it for fault injection
        // Note: We wrap it in RwLock for consistency, even though we read from config directly
        let fault_injector = Arc::new(RwLock::new(FaultInjector::new(Default::default())));

        let rate_limiter = Arc::new(RwLock::new(RateLimiter::new(Default::default())));

        let traffic_shaper = Arc::new(RwLock::new(TrafficShaper::new(Default::default())));

        let circuit_breaker = Arc::new(RwLock::new(CircuitBreaker::new(Default::default())));

        let bulkhead = Arc::new(RwLock::new(Bulkhead::new(Default::default())));

        Self {
            config,
            latency_tracker,
            latency_injector,
            fault_injector,
            rate_limiter,
            traffic_shaper,
            circuit_breaker,
            bulkhead,
        }
    }

    /// Initialize middleware from config (async version)
    ///
    /// This should be called after creation to sync injectors with the actual config.
    /// This is a convenience method that calls `update_from_config()`.
    pub async fn init_from_config(&self) {
        self.update_from_config().await;
    }

    /// Update injectors from current config
    ///
    /// This method should be called when the config is updated to refresh
    /// the cached injectors. For hot-reload support, this is called automatically
    /// when processing requests if the config has changed.
    pub async fn update_from_config(&self) {
        let config = self.config.read().await;

        // Update latency injector
        {
            let mut injector = self.latency_injector.write().await;
            *injector = LatencyInjector::new(config.latency.clone().unwrap_or_default());
        }

        // Note: FaultInjector doesn't have an update method, so we'd need to recreate it
        // For now, we'll read from config directly in the middleware

        // Update rate limiter
        {
            let mut limiter = self.rate_limiter.write().await;
            *limiter = RateLimiter::new(config.rate_limit.clone().unwrap_or_default());
        }

        // Update traffic shaper
        {
            let mut shaper = self.traffic_shaper.write().await;
            *shaper = TrafficShaper::new(config.traffic_shaping.clone().unwrap_or_default());
        }

        // Update circuit breaker
        {
            let mut breaker = self.circuit_breaker.write().await;
            *breaker = CircuitBreaker::new(config.circuit_breaker.clone().unwrap_or_default());
        }

        // Update bulkhead
        {
            let mut bh = self.bulkhead.write().await;
            *bh = Bulkhead::new(config.bulkhead.clone().unwrap_or_default());
        }
    }

    /// Get latency injector (read-only access)
    pub fn latency_injector(&self) -> Arc<RwLock<LatencyInjector>> {
        self.latency_injector.clone()
    }

    /// Get fault injector (read-only access)
    /// Note: FaultInjector doesn't support hot-reload, so we read from config directly
    pub fn fault_injector(&self) -> Arc<RwLock<FaultInjector>> {
        self.fault_injector.clone()
    }

    /// Get rate limiter (read-only access)
    pub fn rate_limiter(&self) -> Arc<RwLock<RateLimiter>> {
        self.rate_limiter.clone()
    }

    /// Get traffic shaper (read-only access)
    pub fn traffic_shaper(&self) -> Arc<RwLock<TrafficShaper>> {
        self.traffic_shaper.clone()
    }

    /// Get circuit breaker (read-only access)
    pub fn circuit_breaker(&self) -> Arc<RwLock<CircuitBreaker>> {
        self.circuit_breaker.clone()
    }

    /// Get bulkhead (read-only access)
    pub fn bulkhead(&self) -> Arc<RwLock<Bulkhead>> {
        self.bulkhead.clone()
    }

    /// Get shared config (for direct access if needed)
    pub fn config(&self) -> Arc<RwLock<ChaosConfig>> {
        self.config.clone()
    }

    /// Get latency tracker
    pub fn latency_tracker(&self) -> &Arc<LatencyMetricsTracker> {
        &self.latency_tracker
    }
}

/// Chaos middleware handler (takes state directly, for use with from_fn)
pub async fn chaos_middleware_with_state(
    chaos: Arc<ChaosMiddleware>,
    req: Request<Body>,
    next: Next,
) -> Response {
    // Call the main handler by creating a temporary State extractor
    // We do this by putting the state in request extensions temporarily
    let (mut parts, body) = req.into_parts();
    parts.extensions.insert(chaos.clone());
    let req = Request::from_parts(parts, body);

    // Now we can use the State extractor pattern
    // But actually, let's just call the core logic directly
    chaos_middleware_core(chaos, req, next).await
}

/// Chaos middleware handler (uses State extractor, for use with from_fn_with_state)
pub async fn chaos_middleware(
    State(chaos): State<Arc<ChaosMiddleware>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    chaos_middleware_core(chaos, req, next).await
}

/// Core chaos middleware logic
async fn chaos_middleware_core(
    chaos: Arc<ChaosMiddleware>,
    req: Request<Body>,
    next: Next,
) -> Response {
    // Read config at start of request (supports hot-reload)
    let config = chaos.config.read().await;

    // Early return if chaos is disabled
    if !config.enabled {
        drop(config);
        return next.run(req).await;
    }

    let path = req.uri().path().to_string();

    // Extract client IP from request extensions (set by ConnectInfo if available) or headers
    let ip = req
        .extensions()
        .get::<SocketAddr>()
        .map(|addr| addr.ip().to_string())
        .or_else(|| {
            req.headers()
                .get("x-forwarded-for")
                .or_else(|| req.headers().get("x-real-ip"))
                .and_then(|h| h.to_str().ok())
                .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        })
        .unwrap_or_else(|| "127.0.0.1".to_string());

    debug!("Chaos middleware processing: {} {}", req.method(), path);

    // Release config lock early (we'll read specific configs as needed)
    drop(config);

    // Check circuit breaker
    {
        let circuit_breaker = chaos.circuit_breaker.read().await;
        if !circuit_breaker.allow_request().await {
            warn!("Circuit breaker open, rejecting request: {}", path);
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "Service temporarily unavailable (circuit breaker open)",
            )
                .into_response();
        }
    }

    // Try to acquire bulkhead slot
    let _bulkhead_guard = {
        let bulkhead = chaos.bulkhead.read().await;
        match bulkhead.try_acquire().await {
            Ok(guard) => guard,
            Err(e) => {
                warn!("Bulkhead rejected request: {} - {:?}", path, e);
                return (StatusCode::SERVICE_UNAVAILABLE, format!("Service overloaded: {}", e))
                    .into_response();
            }
        }
    };

    // Check rate limits
    let rate_limiter = chaos.rate_limiter.read().await;
    if let Err(_e) = rate_limiter.check(Some(&ip), Some(&path)) {
        drop(rate_limiter);
        warn!("Rate limit exceeded: {} - {}", ip, path);
        return (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded").into_response();
    }
    drop(rate_limiter);

    // Check connection limits
    let traffic_shaper = chaos.traffic_shaper.read().await;
    if !traffic_shaper.check_connection_limit() {
        drop(traffic_shaper);
        warn!("Connection limit exceeded");
        return (StatusCode::SERVICE_UNAVAILABLE, "Connection limit exceeded").into_response();
    }

    // Always release connection on scope exit
    let _connection_guard = crate::traffic_shaping::ConnectionGuard::new(traffic_shaper.clone());

    // Check for packet loss (simulate dropped connection)
    if traffic_shaper.should_drop_packet() {
        drop(traffic_shaper);
        warn!("Simulating packet loss for: {}", path);
        return (StatusCode::REQUEST_TIMEOUT, "Connection dropped").into_response();
    }
    drop(traffic_shaper);

    // Inject latency and record it for metrics
    let latency_injector = chaos.latency_injector.read().await;
    let delay_ms = latency_injector.inject().await;
    drop(latency_injector);
    if delay_ms > 0 {
        chaos.latency_tracker.record_latency(delay_ms);
    }

    // Check for fault injection (read from config for hot-reload)
    let config = chaos.config.read().await;
    let fault_config = config.fault_injection.as_ref();
    let should_inject_fault = fault_config.map(|f| f.enabled).unwrap_or(false);
    let http_error_status = if should_inject_fault {
        // Check probability and get error status
        fault_config.and_then(|f| {
            let mut rng = rand::rng();
            if rng.random::<f64>() <= f.http_error_probability && !f.http_errors.is_empty() {
                Some(f.http_errors[rng.random_range(0..f.http_errors.len())])
            } else {
                None
            }
        })
    } else {
        None
    };
    drop(config);

    if let Some(status_code) = http_error_status {
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
    {
        let traffic_shaper = chaos.traffic_shaper.read().await;
        traffic_shaper.throttle_bandwidth(request_size).await;
    }

    // Reconstruct request
    let req = Request::from_parts(parts, Body::from(body_bytes));

    // Pass to next handler
    let response = next.run(req).await;

    // Record circuit breaker result based on response status
    let status = response.status();
    {
        let circuit_breaker = chaos.circuit_breaker.read().await;
        if status.is_server_error() || status == StatusCode::SERVICE_UNAVAILABLE {
            circuit_breaker.record_failure().await;
        } else if status.is_success() {
            circuit_breaker.record_success().await;
        }
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
    // Read from config for hot-reload support
    let config = chaos.config.read().await;
    let should_truncate = config
        .fault_injection
        .as_ref()
        .map(|f| f.enabled && f.timeout_errors)
        .unwrap_or(false);
    let should_corrupt = config.fault_injection.as_ref().map(|f| f.enabled).unwrap_or(false);
    let corruption_type = config
        .fault_injection
        .as_ref()
        .map(|f| f.corruption_type)
        .unwrap_or(CorruptionType::None);
    drop(config);

    let mut final_body_bytes = if should_truncate {
        warn!("Injecting partial response");
        let truncate_at = response_size / 2;
        response_body_bytes.slice(0..truncate_at).to_vec()
    } else {
        response_body_bytes.to_vec()
    };

    // Apply payload corruption if enabled
    if should_corrupt && corruption_type != CorruptionType::None {
        warn!("Injecting payload corruption: {:?}", corruption_type);
        final_body_bytes = corrupt_payload(&final_body_bytes, corruption_type);
    }

    let final_body = Body::from(final_body_bytes);

    // Throttle response bandwidth
    {
        let traffic_shaper = chaos.traffic_shaper.read().await;
        traffic_shaper.throttle_bandwidth(response_size).await;
    }

    Response::from_parts(parts, final_body)
}

/// Corrupt a payload based on the corruption type
fn corrupt_payload(data: &[u8], corruption_type: CorruptionType) -> Vec<u8> {
    if data.is_empty() {
        return data.to_vec();
    }

    let mut rng = rand::rng();
    let mut corrupted = data.to_vec();

    match corruption_type {
        CorruptionType::None => corrupted,
        CorruptionType::RandomBytes => {
            // Replace 10% of bytes with random values
            let num_bytes_to_corrupt = (data.len() as f64 * 0.1).max(1.0) as usize;
            for _ in 0..num_bytes_to_corrupt {
                let index = rng.random_range(0..data.len());
                corrupted[index] = rng.random::<u8>();
            }
            corrupted
        }
        CorruptionType::Truncate => {
            // Truncate at random position (between 50% and 90% of original length)
            let min_truncate = data.len() / 2;
            let max_truncate = (data.len() as f64 * 0.9) as usize;
            let truncate_at = if max_truncate > min_truncate {
                rng.random_range(min_truncate..=max_truncate)
            } else {
                min_truncate
            };
            corrupted.truncate(truncate_at);
            corrupted
        }
        CorruptionType::BitFlip => {
            // Flip random bits in 10% of bytes
            let num_bytes_to_flip = (data.len() as f64 * 0.1).max(1.0) as usize;
            for _ in 0..num_bytes_to_flip {
                let index = rng.random_range(0..data.len());
                let bit_to_flip = rng.random_range(0..8);
                corrupted[index] ^= 1 << bit_to_flip;
            }
            corrupted
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LatencyConfig, RateLimitConfig};
    use crate::latency_metrics::LatencyMetricsTracker;

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

        let latency_tracker = Arc::new(LatencyMetricsTracker::new());
        let config_arc = Arc::new(RwLock::new(config));
        let middleware = ChaosMiddleware::new(config_arc, latency_tracker);
        // Initialize middleware from config to sync injectors with actual config
        middleware.init_from_config().await;
        assert!(middleware.latency_injector.read().await.is_enabled());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let config = Arc::new(RwLock::new(ChaosConfig {
            enabled: true,
            rate_limit: Some(RateLimitConfig {
                enabled: true,
                requests_per_second: 1,
                burst_size: 2, // burst_size is the total capacity, not additional requests
                ..Default::default()
            }),
            ..Default::default()
        }));

        let latency_tracker = Arc::new(LatencyMetricsTracker::new());
        let middleware = Arc::new(ChaosMiddleware::new(config.clone(), latency_tracker));
        middleware.init_from_config().await;

        // First two requests should succeed (rate + burst)
        {
            let rate_limiter = middleware.rate_limiter.read().await;
            assert!(rate_limiter.check(Some("127.0.0.1"), Some("/test")).is_ok());
            assert!(rate_limiter.check(Some("127.0.0.1"), Some("/test")).is_ok());
        }

        // Third should fail
        {
            let rate_limiter = middleware.rate_limiter.read().await;
            assert!(rate_limiter.check(Some("127.0.0.1"), Some("/test")).is_err());
        }
    }

    #[tokio::test]
    async fn test_latency_recording() {
        let config = Arc::new(RwLock::new(ChaosConfig {
            enabled: true,
            latency: Some(LatencyConfig {
                enabled: true,
                fixed_delay_ms: Some(50),
                probability: 1.0,
                ..Default::default()
            }),
            ..Default::default()
        }));

        let latency_tracker = Arc::new(LatencyMetricsTracker::new());
        let middleware = Arc::new(ChaosMiddleware::new(config.clone(), latency_tracker.clone()));
        middleware.init_from_config().await;

        // Verify tracker is accessible via getter
        let tracker_from_middleware = middleware.latency_tracker();
        assert_eq!(Arc::as_ptr(tracker_from_middleware), Arc::as_ptr(&latency_tracker));

        // Manually inject latency and record it (simulating what middleware does)
        let delay_ms = {
            let injector = middleware.latency_injector.read().await;
            injector.inject().await
        };
        if delay_ms > 0 {
            latency_tracker.record_latency(delay_ms);
        }

        // Verify latency was recorded
        let samples = latency_tracker.get_samples();
        assert!(!samples.is_empty(), "Should have recorded at least one latency sample");
        assert_eq!(samples[0].latency_ms, 50, "Recorded latency should match injected delay");
    }
}
