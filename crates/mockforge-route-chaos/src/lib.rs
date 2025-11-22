//! Per-route fault injection and latency simulation
//!
//! Provides route-specific chaos engineering capabilities that allow configuring
//! fault injection and latency on a per-route basis, with support for multiple
//! fault types and various latency distributions.
//!
//! This crate is isolated from mockforge-core to avoid Send issues. It only uses
//! thread_rng() (which is Send-safe) and does not import rng() from rand.

use mockforge_core::config::{
    LatencyDistribution, RouteConfig, RouteFaultInjectionConfig, RouteFaultType, RouteLatencyConfig,
};
use mockforge_core::{Error, Result};
use mockforge_core::priority_handler::{RouteChaosInjectorTrait, RouteFaultResponse as CoreRouteFaultResponse};
use axum::http::{Method, Uri};
use async_trait::async_trait;
use rand::thread_rng;
use rand::Rng;
use regex::Regex;
use std::collections::HashMap;
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
            if &compiled_route.method != method {
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
                        } else {
                            param_name.push(chars.next().unwrap());
                        }
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
        let full_pattern = format!("^{}$", regex_pattern);
        Regex::new(&full_pattern)
            .map_err(|e| Error::generic(format!("Invalid route pattern '{}': {}", pattern, e)))
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
                    .unwrap_or_else(|| format!("Injected HTTP error {}", status_code)),
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
                    .unwrap_or_else(|| format!("Request timeout after {}ms", duration_ms)),
                fault_type: "timeout".to_string(),
            }),
            RouteFaultType::PartialResponse { truncate_percent } => Some(RouteFaultResponse {
                status_code: 200,
                error_message: format!("Partial response (truncated at {}%)", truncate_percent),
                fault_type: "partial_response".to_string(),
            }),
            RouteFaultType::PayloadCorruption { corruption_type } => Some(RouteFaultResponse {
                status_code: 200,
                error_message: format!("Payload corruption ({})", corruption_type),
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

    #[test]
    fn test_path_pattern_compilation() {
        let pattern = RouteMatcher::compile_path_pattern("/users/{id}").unwrap();
        assert!(pattern.is_match("/users/123"));
        assert!(pattern.is_match("/users/abc"));
        assert!(!pattern.is_match("/users/123/posts"));
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
}
