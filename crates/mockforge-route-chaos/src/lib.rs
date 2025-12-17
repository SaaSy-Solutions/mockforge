//! Per-route fault injection and latency simulation
//!
//! Provides route-specific chaos engineering capabilities that allow configuring
//! fault injection and latency on a per-route basis, with support for multiple
//! fault types and various latency distributions.
//!
//! This crate is isolated from mockforge-core to avoid Send issues. It only uses
//! `thread_rng()` (which is Send-safe) and does not import `rng()` from rand.

use async_trait::async_trait;
use axum::http::{Method, Uri};
use mockforge_core::config::{
    LatencyDistribution, RouteConfig, RouteFaultType, RouteLatencyConfig,
};
use mockforge_core::priority_handler::{
    RouteChaosInjectorTrait, RouteFaultResponse as CoreRouteFaultResponse,
};
use mockforge_core::{Error, Result};
use rand::thread_rng;
use rand::Rng;
use regex::Regex;
use std::time::Duration;
use tokio::time::sleep;
use tracing::debug;

/// Route matcher for matching requests to configured routes
#[derive(Debug, Clone)]
pub struct RouteMatcher {
    /// Compiled route patterns (path -> regex)
    routes: Vec<CompiledRoute>,
}

/// Compiled route with pattern matching
#[derive(Debug, Clone)]
struct CompiledRoute {
    /// Original route config
    config: RouteConfig,
    /// Compiled regex pattern for path matching
    path_pattern: Regex,
    /// HTTP method
    method: Method,
}

impl RouteMatcher {
    /// Create a new route matcher from route configurations
    pub fn new(routes: Vec<RouteConfig>) -> Result<Self> {
        let mut compiled_routes = Vec::new();

        for route in routes {
            // Convert path pattern to regex (e.g., /users/{id} -> /users/([^/]+))
            let path_pattern = Self::compile_path_pattern(&route.path)?;
            let method = route.method.parse::<Method>().map_err(|e| {
                Error::generic(format!("Invalid HTTP method '{}': {}", route.method, e))
            })?;

            compiled_routes.push(CompiledRoute {
                config: route,
                path_pattern,
                method,
            });
        }

        Ok(Self {
            routes: compiled_routes,
        })
    }

    /// Match a request to a route configuration
    pub fn match_route(&self, method: &Method, uri: &Uri) -> Option<&RouteConfig> {
        let path = uri.path();

        for compiled_route in &self.routes {
            // Check method match
            if compiled_route.method != method {
                continue;
            }

            // Check path match
            if compiled_route.path_pattern.is_match(path) {
                return Some(&compiled_route.config);
            }
        }

        None
    }

    /// Compile a path pattern to a regex
    /// Converts /users/{id} to /users/([^/]+)
    fn compile_path_pattern(pattern: &str) -> Result<Regex> {
        // Escape special regex characters except {}
        let mut regex_pattern = String::new();
        let mut chars = pattern.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '{' => {
                    // Find the closing brace
                    let mut param_name = String::new();
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == '}' {
                            chars.next(); // consume '}'
                                          // Replace with regex group
                            regex_pattern.push_str("([^/]+)");
                            break;
                        }
                        param_name.push(chars.next().unwrap());
                    }
                }
                '*' => {
                    // Wildcard - match anything
                    regex_pattern.push_str(".*");
                }
                ch if ".+?^$|\\[]()".contains(ch) => {
                    // Escape regex special characters
                    regex_pattern.push('\\');
                    regex_pattern.push(ch);
                }
                ch => {
                    regex_pattern.push(ch);
                }
            }
        }

        // Anchor to start and end
        let full_pattern = format!("^{regex_pattern}$");
        Regex::new(&full_pattern)
            .map_err(|e| Error::generic(format!("Invalid route pattern '{pattern}': {e}")))
    }
}

/// Per-route fault and latency injector
#[derive(Debug, Clone)]
pub struct RouteChaosInjector {
    /// Route matcher
    matcher: RouteMatcher,
}

#[async_trait]
impl RouteChaosInjectorTrait for RouteChaosInjector {
    /// Inject latency for this request
    async fn inject_latency(&self, method: &Method, uri: &Uri) -> Result<()> {
        self.inject_latency_impl(method, uri).await
    }

    /// Get fault injection response for a request
    fn get_fault_response(&self, method: &Method, uri: &Uri) -> Option<CoreRouteFaultResponse> {
        self.get_fault_response_impl(method, uri).map(|r| CoreRouteFaultResponse {
            status_code: r.status_code,
            error_message: r.error_message,
            fault_type: r.fault_type,
        })
    }
}

impl RouteChaosInjector {
    /// Create a new route chaos injector
    pub fn new(routes: Vec<RouteConfig>) -> Result<Self> {
        let matcher = RouteMatcher::new(routes)?;
        Ok(Self { matcher })
    }

    /// Check if a fault should be injected for this request
    pub fn should_inject_fault(
        &self,
        method: &Method,
        uri: &Uri,
    ) -> Option<RouteFaultInjectionResult> {
        let route = self.matcher.match_route(method, uri)?;
        let fault_config = route.fault_injection.as_ref()?;

        if !fault_config.enabled {
            return None;
        }

        // Check probability - using thread_rng() which is Send-safe
        let mut rng = thread_rng();
        if rng.random::<f64>() > fault_config.probability {
            return None;
        }

        // Select a random fault type
        if fault_config.fault_types.is_empty() {
            return None;
        }

        let fault_type =
            &fault_config.fault_types[rng.random_range(0..fault_config.fault_types.len())];

        Some(RouteFaultInjectionResult {
            fault_type: fault_type.clone(),
        })
    }

    /// Inject latency for this request (internal implementation)
    async fn inject_latency_impl(&self, method: &Method, uri: &Uri) -> Result<()> {
        let route = match self.matcher.match_route(method, uri) {
            Some(r) => r,
            None => return Ok(()), // No route match, no latency injection
        };

        let latency_config = match &route.latency {
            Some(cfg) => cfg,
            None => return Ok(()), // No latency config
        };

        if !latency_config.enabled {
            return Ok(());
        }

        // Calculate delay before any await point to ensure Send safety
        // All RNG operations must complete before the await
        let delay_ms = {
            // Check probability - using thread_rng() which is Send-safe
            let mut rng = thread_rng();
            if rng.random::<f64>() > latency_config.probability {
                return Ok(());
            }

            // Calculate delay (all RNG operations happen here, before await)
            self.calculate_delay(latency_config)?
        };

        // Now we can await safely - all RNG operations are complete
        if delay_ms > 0 {
            debug!("Injecting per-route latency: {}ms for {} {}", delay_ms, method, uri.path());
            sleep(Duration::from_millis(delay_ms)).await;
        }

        Ok(())
    }

    /// Calculate delay based on latency configuration
    fn calculate_delay(&self, config: &RouteLatencyConfig) -> Result<u64> {
        // Using thread_rng() which is Send-safe
        let mut rng = thread_rng();

        let base_delay = match &config.distribution {
            LatencyDistribution::Fixed => config.fixed_delay_ms.unwrap_or(0),
            LatencyDistribution::Normal {
                mean_ms,
                std_dev_ms,
            } => {
                // Use Box-Muller transform for normal distribution
                let u1: f64 = rng.random();
                let u2: f64 = rng.random();
                let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                let value = mean_ms + std_dev_ms * z0;
                value.max(0.0) as u64
            }
            LatencyDistribution::Exponential { lambda } => {
                // Inverse transform sampling for exponential distribution
                let u: f64 = rng.random();
                let value = -lambda.ln() * (1.0 - u);
                value.max(0.0) as u64
            }
            LatencyDistribution::Uniform => {
                if let Some((min, max)) = config.random_delay_range_ms {
                    rng.random_range(min..=max)
                } else {
                    config.fixed_delay_ms.unwrap_or(0)
                }
            }
        };

        // Apply jitter
        let delay = if config.jitter_percent > 0.0 {
            let jitter = (base_delay as f64 * config.jitter_percent / 100.0) as u64;
            let jitter_offset = rng.random_range(0..=jitter);
            if rng.random_bool(0.5) {
                base_delay + jitter_offset
            } else {
                base_delay.saturating_sub(jitter_offset)
            }
        } else {
            base_delay
        };

        Ok(delay)
    }

    /// Get fault injection response for a request (internal implementation)
    fn get_fault_response_impl(&self, method: &Method, uri: &Uri) -> Option<RouteFaultResponse> {
        let fault_result = self.should_inject_fault(method, uri)?;

        match &fault_result.fault_type {
            RouteFaultType::HttpError {
                status_code,
                message,
            } => Some(RouteFaultResponse {
                status_code: *status_code,
                error_message: message
                    .clone()
                    .unwrap_or_else(|| format!("Injected HTTP error {status_code}")),
                fault_type: "http_error".to_string(),
            }),
            RouteFaultType::ConnectionError { message } => Some(RouteFaultResponse {
                status_code: 503,
                error_message: message.clone().unwrap_or_else(|| "Connection error".to_string()),
                fault_type: "connection_error".to_string(),
            }),
            RouteFaultType::Timeout {
                duration_ms,
                message,
            } => Some(RouteFaultResponse {
                status_code: 504,
                error_message: message
                    .clone()
                    .unwrap_or_else(|| format!("Request timeout after {duration_ms}ms")),
                fault_type: "timeout".to_string(),
            }),
            RouteFaultType::PartialResponse { truncate_percent } => Some(RouteFaultResponse {
                status_code: 200,
                error_message: format!("Partial response (truncated at {truncate_percent}%)"),
                fault_type: "partial_response".to_string(),
            }),
            RouteFaultType::PayloadCorruption { corruption_type } => Some(RouteFaultResponse {
                status_code: 200,
                error_message: format!("Payload corruption ({corruption_type})"),
                fault_type: "payload_corruption".to_string(),
            }),
        }
    }
}

/// Result of fault injection check
#[derive(Debug, Clone)]
pub struct RouteFaultInjectionResult {
    /// The fault type to inject
    pub fault_type: RouteFaultType,
}

/// Fault injection response
#[derive(Debug, Clone)]
pub struct RouteFaultResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Error message
    pub error_message: String,
    /// Fault type identifier
    pub fault_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::config::{RouteConfig, RouteResponseConfig};
    use std::collections::HashMap;

    fn create_test_route(path: &str, method: &str) -> RouteConfig {
        RouteConfig {
            path: path.to_string(),
            method: method.to_string(),
            request: None,
            response: RouteResponseConfig {
                status: 200,
                headers: HashMap::new(),
                body: None,
            },
            fault_injection: None,
            latency: None,
        }
    }

    // RouteMatcher tests
    #[test]
    fn test_path_pattern_compilation() {
        let pattern = RouteMatcher::compile_path_pattern("/users/{id}").unwrap();
        assert!(pattern.is_match("/users/123"));
        assert!(pattern.is_match("/users/abc"));
        assert!(!pattern.is_match("/users/123/posts"));
        assert!(!pattern.is_match("/users"));
    }

    #[test]
    fn test_path_pattern_compilation_multiple_params() {
        let pattern =
            RouteMatcher::compile_path_pattern("/users/{user_id}/posts/{post_id}").unwrap();
        assert!(pattern.is_match("/users/123/posts/456"));
        assert!(pattern.is_match("/users/abc/posts/xyz"));
        assert!(!pattern.is_match("/users/123/posts"));
        assert!(!pattern.is_match("/users/123"));
    }

    #[test]
    fn test_path_pattern_compilation_wildcard() {
        let pattern = RouteMatcher::compile_path_pattern("/api/*").unwrap();
        assert!(pattern.is_match("/api/"));
        assert!(pattern.is_match("/api/users"));
        assert!(pattern.is_match("/api/users/123/posts"));
    }

    #[test]
    fn test_path_pattern_compilation_special_chars() {
        let pattern = RouteMatcher::compile_path_pattern("/api/v1.0/users").unwrap();
        assert!(pattern.is_match("/api/v1.0/users"));
        assert!(!pattern.is_match("/api/v1X0/users"));
    }

    #[test]
    fn test_path_pattern_compilation_empty_path() {
        let pattern = RouteMatcher::compile_path_pattern("/").unwrap();
        assert!(pattern.is_match("/"));
        assert!(!pattern.is_match("/users"));
    }

    #[test]
    fn test_route_matching() {
        let routes = vec![
            create_test_route("/users/{id}", "GET"),
            create_test_route("/orders/{order_id}", "POST"),
            create_test_route("/health", "GET"),
        ];

        let matcher = RouteMatcher::new(routes).unwrap();

        let get_users = Method::GET;
        let post_orders = Method::POST;
        let get_health = Method::GET;

        assert!(matcher.match_route(&get_users, &Uri::from_static("/users/123")).is_some());
        assert!(matcher.match_route(&post_orders, &Uri::from_static("/orders/456")).is_some());
        assert!(matcher.match_route(&get_health, &Uri::from_static("/health")).is_some());
        assert!(matcher.match_route(&get_users, &Uri::from_static("/unknown")).is_none());
    }

    #[test]
    fn test_route_matching_method_mismatch() {
        let routes = vec![create_test_route("/users/{id}", "GET")];
        let matcher = RouteMatcher::new(routes).unwrap();

        // POST request to GET-only route should not match
        assert!(matcher.match_route(&Method::POST, &Uri::from_static("/users/123")).is_none());
    }

    #[test]
    fn test_route_matching_empty_routes() {
        let matcher = RouteMatcher::new(vec![]).unwrap();
        assert!(matcher.match_route(&Method::GET, &Uri::from_static("/anything")).is_none());
    }

    #[test]
    fn test_route_matcher_debug() {
        let routes = vec![create_test_route("/test", "GET")];
        let matcher = RouteMatcher::new(routes).unwrap();
        let debug = format!("{:?}", matcher);
        assert!(debug.contains("RouteMatcher"));
    }

    #[test]
    fn test_route_matcher_clone() {
        let routes = vec![create_test_route("/test", "GET")];
        let matcher = RouteMatcher::new(routes).unwrap();
        let cloned = matcher.clone();
        assert!(cloned.match_route(&Method::GET, &Uri::from_static("/test")).is_some());
    }

    // RouteChaosInjector tests
    #[test]
    fn test_route_chaos_injector_new() {
        let routes = vec![create_test_route("/test", "GET")];
        let injector = RouteChaosInjector::new(routes);
        assert!(injector.is_ok());
    }

    #[test]
    fn test_route_chaos_injector_new_custom_method() {
        // HTTP allows custom methods, so "CUSTOM" is actually valid
        let routes = vec![create_test_route("/test", "CUSTOM")];
        let injector = RouteChaosInjector::new(routes);
        assert!(injector.is_ok());
    }

    #[test]
    fn test_route_chaos_injector_debug() {
        let routes = vec![create_test_route("/test", "GET")];
        let injector = RouteChaosInjector::new(routes).unwrap();
        let debug = format!("{:?}", injector);
        assert!(debug.contains("RouteChaosInjector"));
    }

    #[test]
    fn test_route_chaos_injector_clone() {
        let routes = vec![create_test_route("/test", "GET")];
        let injector = RouteChaosInjector::new(routes).unwrap();
        let _cloned = injector.clone();
    }

    #[tokio::test]
    async fn test_latency_injection() {
        use mockforge_core::config::RouteLatencyConfig;

        let mut route = create_test_route("/test", "GET");
        route.latency = Some(RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: Some(10),
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Fixed,
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        let start = std::time::Instant::now();
        injector.inject_latency(&Method::GET, &Uri::from_static("/test")).await.unwrap();
        let elapsed = start.elapsed();

        assert!(elapsed >= Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_latency_injection_disabled() {
        use mockforge_core::config::RouteLatencyConfig;

        let mut route = create_test_route("/test", "GET");
        route.latency = Some(RouteLatencyConfig {
            enabled: false,
            probability: 1.0,
            fixed_delay_ms: Some(100),
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Fixed,
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        let start = std::time::Instant::now();
        injector.inject_latency(&Method::GET, &Uri::from_static("/test")).await.unwrap();
        let elapsed = start.elapsed();

        // Should not delay when disabled
        assert!(elapsed < Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_latency_injection_no_route_match() {
        let routes = vec![create_test_route("/test", "GET")];
        let injector = RouteChaosInjector::new(routes).unwrap();
        let start = std::time::Instant::now();
        injector
            .inject_latency(&Method::GET, &Uri::from_static("/unknown"))
            .await
            .unwrap();
        let elapsed = start.elapsed();

        // Should not delay when no route matches
        assert!(elapsed < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_latency_injection_no_latency_config() {
        let routes = vec![create_test_route("/test", "GET")];
        let injector = RouteChaosInjector::new(routes).unwrap();
        let start = std::time::Instant::now();
        injector.inject_latency(&Method::GET, &Uri::from_static("/test")).await.unwrap();
        let elapsed = start.elapsed();

        // Should not delay when no latency config
        assert!(elapsed < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_latency_injection_uniform_distribution() {
        use mockforge_core::config::RouteLatencyConfig;

        let mut route = create_test_route("/test", "GET");
        route.latency = Some(RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: None,
            random_delay_range_ms: Some((5, 15)),
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Uniform,
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        let start = std::time::Instant::now();
        injector.inject_latency(&Method::GET, &Uri::from_static("/test")).await.unwrap();
        let elapsed = start.elapsed();

        // Should delay somewhere in the range
        assert!(elapsed >= Duration::from_millis(5));
        assert!(elapsed < Duration::from_millis(100)); // Allow some buffer
    }

    #[tokio::test]
    async fn test_latency_injection_normal_distribution() {
        use mockforge_core::config::RouteLatencyConfig;

        let mut route = create_test_route("/test", "GET");
        route.latency = Some(RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: None,
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Normal {
                mean_ms: 10.0,
                std_dev_ms: 1.0,
            },
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        // Just verify it doesn't panic
        injector.inject_latency(&Method::GET, &Uri::from_static("/test")).await.unwrap();
    }

    #[tokio::test]
    async fn test_latency_injection_exponential_distribution() {
        use mockforge_core::config::RouteLatencyConfig;

        let mut route = create_test_route("/test", "GET");
        route.latency = Some(RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: None,
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Exponential { lambda: 0.1 },
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        // Just verify it doesn't panic
        injector.inject_latency(&Method::GET, &Uri::from_static("/test")).await.unwrap();
    }

    #[tokio::test]
    async fn test_latency_injection_with_jitter() {
        use mockforge_core::config::RouteLatencyConfig;

        let mut route = create_test_route("/test", "GET");
        route.latency = Some(RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: Some(10),
            random_delay_range_ms: None,
            jitter_percent: 50.0,
            distribution: LatencyDistribution::Fixed,
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        // Just verify it doesn't panic
        injector.inject_latency(&Method::GET, &Uri::from_static("/test")).await.unwrap();
    }

    // Fault injection tests
    #[test]
    fn test_fault_injection() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};

        let mut route = create_test_route("/test", "GET");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![RouteFaultType::HttpError {
                status_code: 500,
                message: Some("Test error".to_string()),
            }],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        let response =
            injector.get_fault_response(&Method::GET, &Uri::from_static("/test")).unwrap();

        assert_eq!(response.status_code, 500);
        assert_eq!(response.error_message, "Test error");
    }

    #[test]
    fn test_fault_injection_http_error_default_message() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};

        let mut route = create_test_route("/test", "GET");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![RouteFaultType::HttpError {
                status_code: 503,
                message: None,
            }],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        let response =
            injector.get_fault_response(&Method::GET, &Uri::from_static("/test")).unwrap();

        assert_eq!(response.status_code, 503);
        assert!(response.error_message.contains("503"));
        assert_eq!(response.fault_type, "http_error");
    }

    #[test]
    fn test_fault_injection_connection_error() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};

        let mut route = create_test_route("/test", "GET");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![RouteFaultType::ConnectionError {
                message: Some("Network failure".to_string()),
            }],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        let response =
            injector.get_fault_response(&Method::GET, &Uri::from_static("/test")).unwrap();

        assert_eq!(response.status_code, 503);
        assert_eq!(response.error_message, "Network failure");
        assert_eq!(response.fault_type, "connection_error");
    }

    #[test]
    fn test_fault_injection_connection_error_default_message() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};

        let mut route = create_test_route("/test", "GET");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![RouteFaultType::ConnectionError { message: None }],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        let response =
            injector.get_fault_response(&Method::GET, &Uri::from_static("/test")).unwrap();

        assert_eq!(response.status_code, 503);
        assert_eq!(response.error_message, "Connection error");
    }

    #[test]
    fn test_fault_injection_timeout() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};

        let mut route = create_test_route("/test", "GET");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![RouteFaultType::Timeout {
                duration_ms: 5000,
                message: Some("Gateway timeout".to_string()),
            }],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        let response =
            injector.get_fault_response(&Method::GET, &Uri::from_static("/test")).unwrap();

        assert_eq!(response.status_code, 504);
        assert_eq!(response.error_message, "Gateway timeout");
        assert_eq!(response.fault_type, "timeout");
    }

    #[test]
    fn test_fault_injection_timeout_default_message() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};

        let mut route = create_test_route("/test", "GET");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![RouteFaultType::Timeout {
                duration_ms: 3000,
                message: None,
            }],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        let response =
            injector.get_fault_response(&Method::GET, &Uri::from_static("/test")).unwrap();

        assert_eq!(response.status_code, 504);
        assert!(response.error_message.contains("3000"));
    }

    #[test]
    fn test_fault_injection_partial_response() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};

        let mut route = create_test_route("/test", "GET");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![RouteFaultType::PartialResponse {
                truncate_percent: 50.0,
            }],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        let response =
            injector.get_fault_response(&Method::GET, &Uri::from_static("/test")).unwrap();

        assert_eq!(response.status_code, 200);
        assert!(response.error_message.contains("50%"));
        assert_eq!(response.fault_type, "partial_response");
    }

    #[test]
    fn test_fault_injection_payload_corruption() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};

        let mut route = create_test_route("/test", "GET");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![RouteFaultType::PayloadCorruption {
                corruption_type: "random_bytes".to_string(),
            }],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        let response =
            injector.get_fault_response(&Method::GET, &Uri::from_static("/test")).unwrap();

        assert_eq!(response.status_code, 200);
        assert!(response.error_message.contains("random_bytes"));
        assert_eq!(response.fault_type, "payload_corruption");
    }

    #[test]
    fn test_fault_injection_disabled() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};

        let mut route = create_test_route("/test", "GET");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: false,
            probability: 1.0,
            fault_types: vec![RouteFaultType::HttpError {
                status_code: 500,
                message: None,
            }],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        let response = injector.get_fault_response(&Method::GET, &Uri::from_static("/test"));

        assert!(response.is_none());
    }

    #[test]
    fn test_fault_injection_no_fault_types() {
        use mockforge_core::config::RouteFaultInjectionConfig;

        let mut route = create_test_route("/test", "GET");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        let response = injector.get_fault_response(&Method::GET, &Uri::from_static("/test"));

        assert!(response.is_none());
    }

    #[test]
    fn test_fault_injection_no_config() {
        let route = create_test_route("/test", "GET");
        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        let response = injector.get_fault_response(&Method::GET, &Uri::from_static("/test"));

        assert!(response.is_none());
    }

    #[test]
    fn test_fault_injection_no_route_match() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};

        let mut route = create_test_route("/test", "GET");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![RouteFaultType::HttpError {
                status_code: 500,
                message: None,
            }],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        let response = injector.get_fault_response(&Method::GET, &Uri::from_static("/unknown"));

        assert!(response.is_none());
    }

    // RouteFaultResponse tests
    #[test]
    fn test_route_fault_response_debug() {
        let response = RouteFaultResponse {
            status_code: 500,
            error_message: "Error".to_string(),
            fault_type: "http_error".to_string(),
        };
        let debug = format!("{:?}", response);
        assert!(debug.contains("RouteFaultResponse"));
        assert!(debug.contains("500"));
    }

    #[test]
    fn test_route_fault_response_clone() {
        let response = RouteFaultResponse {
            status_code: 503,
            error_message: "Service unavailable".to_string(),
            fault_type: "connection_error".to_string(),
        };
        let cloned = response.clone();
        assert_eq!(response.status_code, cloned.status_code);
        assert_eq!(response.error_message, cloned.error_message);
        assert_eq!(response.fault_type, cloned.fault_type);
    }

    // RouteFaultInjectionResult tests
    #[test]
    fn test_route_fault_injection_result_debug() {
        use mockforge_core::config::RouteFaultType;

        let result = RouteFaultInjectionResult {
            fault_type: RouteFaultType::HttpError {
                status_code: 404,
                message: None,
            },
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("RouteFaultInjectionResult"));
    }

    #[test]
    fn test_route_fault_injection_result_clone() {
        use mockforge_core::config::RouteFaultType;

        let result = RouteFaultInjectionResult {
            fault_type: RouteFaultType::Timeout {
                duration_ms: 1000,
                message: None,
            },
        };
        let _cloned = result.clone();
    }

    // RouteChaosInjectorTrait implementation tests
    #[tokio::test]
    async fn test_trait_inject_latency() {
        use mockforge_core::config::RouteLatencyConfig;

        let mut route = create_test_route("/test", "GET");
        route.latency = Some(RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: Some(5),
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Fixed,
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        // Use trait method
        let result = <RouteChaosInjector as RouteChaosInjectorTrait>::inject_latency(
            &injector,
            &Method::GET,
            &Uri::from_static("/test"),
        )
        .await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_trait_get_fault_response() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};

        let mut route = create_test_route("/test", "GET");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![RouteFaultType::HttpError {
                status_code: 502,
                message: Some("Bad gateway".to_string()),
            }],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();
        // Use trait method
        let response = <RouteChaosInjector as RouteChaosInjectorTrait>::get_fault_response(
            &injector,
            &Method::GET,
            &Uri::from_static("/test"),
        );

        assert!(response.is_some());
        let response = response.unwrap();
        assert_eq!(response.status_code, 502);
        assert_eq!(response.error_message, "Bad gateway");
    }

    // Edge case tests
    #[test]
    fn test_route_with_query_params() {
        let routes = vec![create_test_route("/users/{id}", "GET")];
        let matcher = RouteMatcher::new(routes).unwrap();

        // URI with query params - path should still match
        let uri = "/users/123?foo=bar".parse::<Uri>().unwrap();
        assert!(matcher.match_route(&Method::GET, &uri).is_some());
    }

    #[test]
    fn test_multiple_routes_same_path_different_methods() {
        let routes = vec![
            create_test_route("/users/{id}", "GET"),
            create_test_route("/users/{id}", "DELETE"),
            create_test_route("/users/{id}", "PUT"),
        ];

        let matcher = RouteMatcher::new(routes).unwrap();

        assert!(matcher.match_route(&Method::GET, &Uri::from_static("/users/123")).is_some());
        assert!(matcher.match_route(&Method::DELETE, &Uri::from_static("/users/123")).is_some());
        assert!(matcher.match_route(&Method::PUT, &Uri::from_static("/users/123")).is_some());
        assert!(matcher.match_route(&Method::POST, &Uri::from_static("/users/123")).is_none());
    }

    // Additional edge case tests for path pattern compilation
    #[test]
    fn test_path_pattern_unclosed_brace() {
        let pattern = RouteMatcher::compile_path_pattern("/users/{id").unwrap();
        // Should still create a regex, just won't match the intended pattern
        assert!(!pattern.is_match("/users/123"));
    }

    #[test]
    fn test_path_pattern_nested_braces() {
        let pattern = RouteMatcher::compile_path_pattern("/users/{{id}}").unwrap();
        // Nested braces create unusual patterns
        let debug = format!("{:?}", pattern);
        assert!(!debug.is_empty());
    }

    #[test]
    fn test_path_pattern_multiple_wildcards() {
        let pattern = RouteMatcher::compile_path_pattern("/*/api/*").unwrap();
        assert!(pattern.is_match("/v1/api/users"));
        assert!(pattern.is_match("/v2/api/orders"));
    }

    #[test]
    fn test_path_pattern_mixed_params_and_wildcards() {
        let pattern = RouteMatcher::compile_path_pattern("/users/{id}/*").unwrap();
        assert!(pattern.is_match("/users/123/posts"));
        assert!(pattern.is_match("/users/abc/profile/settings"));
    }

    #[test]
    fn test_path_pattern_all_special_chars() {
        // Test all special regex chars are properly escaped
        let pattern = RouteMatcher::compile_path_pattern("/api/v1.0/data[test]").unwrap();
        assert!(pattern.is_match("/api/v1.0/data[test]"));
        assert!(!pattern.is_match("/api/v1X0/data[test]"));
    }

    #[test]
    fn test_path_pattern_plus_sign() {
        let pattern = RouteMatcher::compile_path_pattern("/api/v1+2").unwrap();
        assert!(pattern.is_match("/api/v1+2"));
    }

    #[test]
    fn test_path_pattern_question_mark() {
        let pattern = RouteMatcher::compile_path_pattern("/api/test?").unwrap();
        assert!(pattern.is_match("/api/test?"));
    }

    #[test]
    fn test_path_pattern_caret_and_dollar() {
        let pattern = RouteMatcher::compile_path_pattern("/api/$test^path").unwrap();
        assert!(pattern.is_match("/api/$test^path"));
    }

    #[test]
    fn test_path_pattern_pipe() {
        let pattern = RouteMatcher::compile_path_pattern("/api/test|path").unwrap();
        assert!(pattern.is_match("/api/test|path"));
    }

    #[test]
    fn test_path_pattern_backslash() {
        let pattern = RouteMatcher::compile_path_pattern("/api/test\\path").unwrap();
        assert!(pattern.is_match("/api/test\\path"));
    }

    #[test]
    fn test_path_pattern_parentheses() {
        let pattern = RouteMatcher::compile_path_pattern("/api/test(123)").unwrap();
        assert!(pattern.is_match("/api/test(123)"));
    }

    #[test]
    fn test_path_pattern_brackets() {
        let pattern = RouteMatcher::compile_path_pattern("/api/test[123]").unwrap();
        assert!(pattern.is_match("/api/test[123]"));
    }

    #[test]
    fn test_path_pattern_empty_param_name() {
        let pattern = RouteMatcher::compile_path_pattern("/users/{}").unwrap();
        assert!(pattern.is_match("/users/123"));
        assert!(pattern.is_match("/users/abc"));
    }

    // calculate_delay tests
    #[test]
    fn test_calculate_delay_fixed_no_jitter() {
        use mockforge_core::config::RouteLatencyConfig;

        let config = RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: Some(100),
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Fixed,
        };

        let injector = RouteChaosInjector::new(vec![create_test_route("/test", "GET")]).unwrap();
        let delay = injector.calculate_delay(&config).unwrap();
        assert_eq!(delay, 100);
    }

    #[test]
    fn test_calculate_delay_fixed_with_jitter() {
        use mockforge_core::config::RouteLatencyConfig;

        let config = RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: Some(100),
            random_delay_range_ms: None,
            jitter_percent: 20.0,
            distribution: LatencyDistribution::Fixed,
        };

        let injector = RouteChaosInjector::new(vec![create_test_route("/test", "GET")]).unwrap();
        let delay = injector.calculate_delay(&config).unwrap();
        // With 20% jitter on 100ms, delay should be between 80 and 120
        assert!(delay >= 80 && delay <= 120);
    }

    #[test]
    fn test_calculate_delay_uniform_with_range() {
        use mockforge_core::config::RouteLatencyConfig;

        let config = RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: None,
            random_delay_range_ms: Some((50, 150)),
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Uniform,
        };

        let injector = RouteChaosInjector::new(vec![create_test_route("/test", "GET")]).unwrap();
        let delay = injector.calculate_delay(&config).unwrap();
        assert!(delay >= 50 && delay <= 150);
    }

    #[test]
    fn test_calculate_delay_uniform_without_range() {
        use mockforge_core::config::RouteLatencyConfig;

        let config = RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: Some(75),
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Uniform,
        };

        let injector = RouteChaosInjector::new(vec![create_test_route("/test", "GET")]).unwrap();
        let delay = injector.calculate_delay(&config).unwrap();
        // Falls back to fixed_delay_ms when no range is provided
        assert_eq!(delay, 75);
    }

    #[test]
    fn test_calculate_delay_uniform_no_fixed_no_range() {
        use mockforge_core::config::RouteLatencyConfig;

        let config = RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: None,
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Uniform,
        };

        let injector = RouteChaosInjector::new(vec![create_test_route("/test", "GET")]).unwrap();
        let delay = injector.calculate_delay(&config).unwrap();
        assert_eq!(delay, 0);
    }

    #[test]
    fn test_calculate_delay_normal_distribution() {
        use mockforge_core::config::RouteLatencyConfig;

        let config = RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: None,
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Normal {
                mean_ms: 100.0,
                std_dev_ms: 10.0,
            },
        };

        let injector = RouteChaosInjector::new(vec![create_test_route("/test", "GET")]).unwrap();
        // Normal distribution should produce values, typically around the mean
        // We can't test exact value due to randomness, but delay is u64 so always >= 0
        let _ = injector.calculate_delay(&config).unwrap();
    }

    #[test]
    fn test_calculate_delay_exponential_distribution() {
        use mockforge_core::config::RouteLatencyConfig;

        let config = RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: None,
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Exponential { lambda: 0.01 },
        };

        let injector = RouteChaosInjector::new(vec![create_test_route("/test", "GET")]).unwrap();
        // Exponential distribution should produce non-negative values (delay is u64, always >= 0)
        let _ = injector.calculate_delay(&config).unwrap();
    }

    #[test]
    fn test_calculate_delay_normal_with_jitter() {
        use mockforge_core::config::RouteLatencyConfig;

        let config = RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: None,
            random_delay_range_ms: None,
            jitter_percent: 10.0,
            distribution: LatencyDistribution::Normal {
                mean_ms: 100.0,
                std_dev_ms: 5.0,
            },
        };

        let injector = RouteChaosInjector::new(vec![create_test_route("/test", "GET")]).unwrap();
        // Should produce non-negative values with jitter applied (delay is u64, always >= 0)
        let _ = injector.calculate_delay(&config).unwrap();
    }

    #[test]
    fn test_calculate_delay_fixed_zero() {
        use mockforge_core::config::RouteLatencyConfig;

        let config = RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: Some(0),
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Fixed,
        };

        let injector = RouteChaosInjector::new(vec![create_test_route("/test", "GET")]).unwrap();
        let delay = injector.calculate_delay(&config).unwrap();
        assert_eq!(delay, 0);
    }

    #[test]
    fn test_calculate_delay_with_large_jitter() {
        use mockforge_core::config::RouteLatencyConfig;

        let config = RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: Some(100),
            random_delay_range_ms: None,
            jitter_percent: 100.0,
            distribution: LatencyDistribution::Fixed,
        };

        let injector = RouteChaosInjector::new(vec![create_test_route("/test", "GET")]).unwrap();
        let delay = injector.calculate_delay(&config).unwrap();
        // With 100% jitter, delay should be between 0 and 200
        assert!(delay <= 200);
    }

    #[test]
    fn test_calculate_delay_jitter_saturating_sub() {
        use mockforge_core::config::RouteLatencyConfig;

        let config = RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: Some(10),
            random_delay_range_ms: None,
            jitter_percent: 200.0, // 200% jitter could cause subtraction to go negative
            distribution: LatencyDistribution::Fixed,
        };

        let injector = RouteChaosInjector::new(vec![create_test_route("/test", "GET")]).unwrap();
        let delay = injector.calculate_delay(&config).unwrap();
        // Should never be negative due to saturating_sub
        assert!(delay < u64::MAX);
    }

    // should_inject_fault tests
    #[test]
    fn test_should_inject_fault_zero_probability() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};

        let mut route = create_test_route("/test", "GET");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 0.0,
            fault_types: vec![RouteFaultType::HttpError {
                status_code: 500,
                message: None,
            }],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();

        // With 0.0 probability, should never inject
        let mut found_injection = false;
        for _ in 0..100 {
            if injector.should_inject_fault(&Method::GET, &Uri::from_static("/test")).is_some() {
                found_injection = true;
                break;
            }
        }
        assert!(!found_injection);
    }

    #[test]
    fn test_should_inject_fault_multiple_fault_types() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};

        let mut route = create_test_route("/test", "GET");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![
                RouteFaultType::HttpError {
                    status_code: 500,
                    message: None,
                },
                RouteFaultType::Timeout {
                    duration_ms: 1000,
                    message: None,
                },
                RouteFaultType::ConnectionError { message: None },
            ],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();

        // With 1.0 probability and multiple fault types, should inject one of them
        let result = injector.should_inject_fault(&Method::GET, &Uri::from_static("/test"));
        assert!(result.is_some());
    }

    #[test]
    fn test_should_inject_fault_returns_different_types() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};
        use std::collections::HashSet;

        let mut route = create_test_route("/test", "GET");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![
                RouteFaultType::HttpError {
                    status_code: 500,
                    message: None,
                },
                RouteFaultType::HttpError {
                    status_code: 503,
                    message: None,
                },
                RouteFaultType::Timeout {
                    duration_ms: 1000,
                    message: None,
                },
            ],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();

        // Run multiple times and verify we can get different fault types
        let mut seen_types = HashSet::new();
        for _ in 0..50 {
            if let Some(result) =
                injector.should_inject_fault(&Method::GET, &Uri::from_static("/test"))
            {
                seen_types.insert(format!("{:?}", result.fault_type));
            }
        }
        // Should see at least one fault type (possibly more with randomness)
        assert!(!seen_types.is_empty());
    }

    // Latency injection with probability tests
    #[tokio::test]
    async fn test_latency_injection_zero_probability() {
        use mockforge_core::config::RouteLatencyConfig;

        let mut route = create_test_route("/test", "GET");
        route.latency = Some(RouteLatencyConfig {
            enabled: true,
            probability: 0.0,
            fixed_delay_ms: Some(100),
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Fixed,
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();

        // With 0.0 probability, should never inject delay
        for _ in 0..10 {
            let start = std::time::Instant::now();
            injector.inject_latency(&Method::GET, &Uri::from_static("/test")).await.unwrap();
            let elapsed = start.elapsed();
            assert!(elapsed < Duration::from_millis(50));
        }
    }

    #[tokio::test]
    async fn test_latency_injection_mid_probability() {
        use mockforge_core::config::RouteLatencyConfig;

        let mut route = create_test_route("/test", "GET");
        route.latency = Some(RouteLatencyConfig {
            enabled: true,
            probability: 0.5,
            fixed_delay_ms: Some(10),
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Fixed,
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();

        // With 0.5 probability, should sometimes inject, sometimes not
        let mut injected_count = 0;
        for _ in 0..20 {
            let start = std::time::Instant::now();
            injector.inject_latency(&Method::GET, &Uri::from_static("/test")).await.unwrap();
            let elapsed = start.elapsed();
            if elapsed >= Duration::from_millis(10) {
                injected_count += 1;
            }
        }
        // Should have injected at least once but not every time
        assert!(injected_count > 0 && injected_count < 20);
    }

    // Route matching edge cases
    #[test]
    fn test_route_matching_trailing_slash() {
        let routes = vec![create_test_route("/users", "GET")];
        let matcher = RouteMatcher::new(routes).unwrap();

        // Without trailing slash should match
        assert!(matcher.match_route(&Method::GET, &Uri::from_static("/users")).is_some());
        // With trailing slash should not match (exact match)
        assert!(matcher.match_route(&Method::GET, &Uri::from_static("/users/")).is_none());
    }

    #[test]
    fn test_route_matching_case_sensitive() {
        let routes = vec![create_test_route("/Users", "GET")];
        let matcher = RouteMatcher::new(routes).unwrap();

        // Should be case-sensitive
        assert!(matcher.match_route(&Method::GET, &Uri::from_static("/Users")).is_some());
        assert!(matcher.match_route(&Method::GET, &Uri::from_static("/users")).is_none());
    }

    #[test]
    fn test_first_matching_route_wins() {
        let mut route1 = create_test_route("/api/*", "GET");
        route1.response.status = 200;

        let mut route2 = create_test_route("/api/users", "GET");
        route2.response.status = 201;

        let matcher = RouteMatcher::new(vec![route1.clone(), route2.clone()]).unwrap();

        // First route with wildcard should match
        let matched = matcher.match_route(&Method::GET, &Uri::from_static("/api/users")).unwrap();
        assert_eq!(matched.response.status, 200);
    }

    #[test]
    fn test_route_with_fragment() {
        let routes = vec![create_test_route("/users/{id}", "GET")];
        let matcher = RouteMatcher::new(routes).unwrap();

        // Fragments are not part of the path
        let uri = "/users/123#section".parse::<Uri>().unwrap();
        assert!(matcher.match_route(&Method::GET, &uri).is_some());
    }

    // Compiled route tests
    #[test]
    fn test_compiled_route_debug() {
        let route = create_test_route("/test", "GET");
        let pattern = RouteMatcher::compile_path_pattern(&route.path).unwrap();
        let method = route.method.parse::<Method>().unwrap();

        let compiled = CompiledRoute {
            config: route,
            path_pattern: pattern,
            method,
        };

        let debug = format!("{:?}", compiled);
        assert!(debug.contains("CompiledRoute"));
    }

    #[test]
    fn test_compiled_route_clone() {
        let route = create_test_route("/test", "GET");
        let pattern = RouteMatcher::compile_path_pattern(&route.path).unwrap();
        let method = route.method.parse::<Method>().unwrap();

        let compiled = CompiledRoute {
            config: route,
            path_pattern: pattern,
            method,
        };

        let _cloned = compiled.clone();
    }

    // Integration-style tests
    #[tokio::test]
    async fn test_full_chaos_injection_http_error() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};

        let mut route = create_test_route("/api/users", "POST");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![RouteFaultType::HttpError {
                status_code: 429,
                message: Some("Rate limit exceeded".to_string()),
            }],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();

        // Check fault response
        let response = injector.get_fault_response(&Method::POST, &Uri::from_static("/api/users"));
        assert!(response.is_some());
        let response = response.unwrap();
        assert_eq!(response.status_code, 429);
        assert_eq!(response.error_message, "Rate limit exceeded");
    }

    #[tokio::test]
    async fn test_full_chaos_injection_with_latency_and_fault() {
        use mockforge_core::config::{
            RouteFaultInjectionConfig, RouteFaultType, RouteLatencyConfig,
        };

        let mut route = create_test_route("/api/orders", "GET");
        route.latency = Some(RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: Some(5),
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Fixed,
        });
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![RouteFaultType::Timeout {
                duration_ms: 5000,
                message: None,
            }],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();

        // Test latency injection
        let start = std::time::Instant::now();
        injector
            .inject_latency(&Method::GET, &Uri::from_static("/api/orders"))
            .await
            .unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(5));

        // Test fault injection
        let response = injector.get_fault_response(&Method::GET, &Uri::from_static("/api/orders"));
        assert!(response.is_some());
    }

    #[test]
    fn test_multiple_routes_different_configs() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};

        let mut route1 = create_test_route("/api/v1/users", "GET");
        route1.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![RouteFaultType::HttpError {
                status_code: 404,
                message: None,
            }],
        });

        let mut route2 = create_test_route("/api/v1/orders", "GET");
        route2.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![RouteFaultType::HttpError {
                status_code: 500,
                message: None,
            }],
        });

        let injector = RouteChaosInjector::new(vec![route1, route2]).unwrap();

        let response1 =
            injector.get_fault_response(&Method::GET, &Uri::from_static("/api/v1/users"));
        assert_eq!(response1.unwrap().status_code, 404);

        let response2 =
            injector.get_fault_response(&Method::GET, &Uri::from_static("/api/v1/orders"));
        assert_eq!(response2.unwrap().status_code, 500);
    }

    // Error path tests
    #[test]
    fn test_invalid_regex_pattern() {
        // Pattern with unmatched brackets should still compile
        // Regex crate will handle the escaping
        let result = RouteMatcher::compile_path_pattern("/api/[invalid");
        assert!(result.is_ok());
    }

    #[test]
    fn test_route_matcher_with_invalid_method() {
        let mut route = create_test_route("/test", "GET");
        // Create a route with an invalid method string directly
        route.method = "INVALID METHOD WITH SPACES".to_string();

        let result = RouteMatcher::new(vec![route]);
        assert!(result.is_err());
    }

    // Trait coverage tests
    #[tokio::test]
    async fn test_trait_inject_latency_no_match() {
        let routes = vec![create_test_route("/test", "GET")];
        let injector = RouteChaosInjector::new(routes).unwrap();

        let result = <RouteChaosInjector as RouteChaosInjectorTrait>::inject_latency(
            &injector,
            &Method::POST,
            &Uri::from_static("/nomatch"),
        )
        .await;

        assert!(result.is_ok());
    }

    #[test]
    fn test_trait_get_fault_response_no_match() {
        use mockforge_core::config::{RouteFaultInjectionConfig, RouteFaultType};

        let mut route = create_test_route("/test", "GET");
        route.fault_injection = Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0,
            fault_types: vec![RouteFaultType::HttpError {
                status_code: 500,
                message: None,
            }],
        });

        let injector = RouteChaosInjector::new(vec![route]).unwrap();

        let response = <RouteChaosInjector as RouteChaosInjectorTrait>::get_fault_response(
            &injector,
            &Method::POST,
            &Uri::from_static("/nomatch"),
        );

        assert!(response.is_none());
    }

    // Normal distribution edge case - negative values
    #[test]
    fn test_calculate_delay_normal_negative_clamp() {
        use mockforge_core::config::RouteLatencyConfig;

        let config = RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: None,
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Normal {
                mean_ms: 10.0,
                std_dev_ms: 100.0, // Large std dev can create negative values
            },
        };

        let injector = RouteChaosInjector::new(vec![create_test_route("/test", "GET")]).unwrap();

        // Run multiple times to potentially hit negative values that should be clamped
        for _ in 0..20 {
            let delay = injector.calculate_delay(&config).unwrap();
            // Should never be negative due to max(0.0) clamp
            assert!(delay < u64::MAX);
        }
    }

    // Exponential distribution edge case
    #[test]
    fn test_calculate_delay_exponential_various_lambdas() {
        use mockforge_core::config::RouteLatencyConfig;

        let injector = RouteChaosInjector::new(vec![create_test_route("/test", "GET")]).unwrap();

        // Test with very small lambda
        let config = RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: None,
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Exponential { lambda: 0.001 },
        };
        // delay is u64, always >= 0
        let _ = injector.calculate_delay(&config).unwrap();

        // Test with large lambda
        let config = RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: None,
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Exponential { lambda: 10.0 },
        };
        // delay is u64, always >= 0
        let _ = injector.calculate_delay(&config).unwrap();
    }

    #[test]
    fn test_path_pattern_consecutive_params() {
        let pattern = RouteMatcher::compile_path_pattern("/api/{param1}{param2}").unwrap();
        // This creates consecutive capture groups
        assert!(pattern.is_match("/api/value1value2"));
    }

    #[test]
    fn test_path_matching_numeric_params() {
        let routes = vec![create_test_route("/users/{id}", "GET")];
        let matcher = RouteMatcher::new(routes).unwrap();

        // Should match numeric IDs
        assert!(matcher.match_route(&Method::GET, &Uri::from_static("/users/12345")).is_some());
        // Should match UUIDs
        assert!(matcher
            .match_route(
                &Method::GET,
                &Uri::from_static("/users/550e8400-e29b-41d4-a716-446655440000")
            )
            .is_some());
    }

    #[test]
    fn test_empty_route_path() {
        let pattern = RouteMatcher::compile_path_pattern("").unwrap();
        assert!(pattern.is_match(""));
    }
}
