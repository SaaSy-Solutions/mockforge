//! Middleware/utilities to apply latency/failure and overrides per operation.
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{Json, Response};
use serde_json::Value;

use crate::latency_profiles::LatencyProfiles;
use mockforge_core::{FailureInjector, Overrides, TrafficShaper};

/// Metadata for the current OpenAPI operation
#[derive(Clone)]
pub struct OperationMeta {
    /// OpenAPI operation ID
    pub id: String,
    /// Tags associated with this operation
    pub tags: Vec<String>,
    /// API path pattern
    pub path: String,
}

/// Shared state for operation middleware
#[derive(Clone)]
pub struct Shared {
    /// Latency profiles for request simulation
    pub profiles: LatencyProfiles,
    /// Response overrides configuration
    pub overrides: Overrides,
    /// Optional failure injector for chaos engineering
    pub failure_injector: Option<FailureInjector>,
    /// Optional traffic shaper for bandwidth/loss simulation
    pub traffic_shaper: Option<TrafficShaper>,
    /// Whether overrides are enabled
    pub overrides_enabled: bool,
    /// Whether traffic shaping is enabled
    pub traffic_shaping_enabled: bool,
}

/// Middleware to add shared state to request extensions
pub async fn add_shared_extension(
    State(shared): State<Shared>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    req.extensions_mut().insert(shared);
    next.run(req).await
}

/// Middleware to apply fault injection before processing request
pub async fn fault_then_next(req: Request<Body>, next: Next) -> Response {
    let shared = req.extensions().get::<Shared>().unwrap().clone();
    let op = req.extensions().get::<OperationMeta>().cloned();

    // First, check the new enhanced failure injection system
    if let Some(failure_injector) = &shared.failure_injector {
        let tags = op.as_ref().map(|o| o.tags.as_slice()).unwrap_or(&[]);
        if let Some((status_code, error_message)) = failure_injector.process_request(tags) {
            let mut res = Response::new(axum::body::Body::from(error_message));
            *res.status_mut() =
                StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            return res;
        }
    }

    // Fallback to legacy latency profiles system for backward compatibility
    if let Some(op) = &op {
        if let Some((code, msg)) = shared
            .profiles
            .maybe_fault(&op.id, &op.tags.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .await
        {
            let mut res = Response::new(axum::body::Body::from(msg));
            *res.status_mut() =
                StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            return res;
        }
    }

    // Apply traffic shaping (bandwidth throttling and burst loss) to the request
    if shared.traffic_shaping_enabled {
        if let Some(traffic_shaper) = &shared.traffic_shaper {
            // Calculate request size for bandwidth throttling
            let request_size = calculate_request_size(&req);

            let tags = op.as_ref().map(|o| o.tags.as_slice()).unwrap_or(&[]);

            // Apply traffic shaping
            match traffic_shaper.process_transfer(request_size, tags).await {
                Ok(Some(_timeout)) => {
                    // Request was "lost" due to burst loss - return timeout error
                    let mut res = Response::new(axum::body::Body::from(
                        "Request timeout due to traffic shaping",
                    ));
                    *res.status_mut() = StatusCode::REQUEST_TIMEOUT;
                    return res;
                }
                Ok(None) => {
                    // Transfer allowed, continue
                }
                Err(e) => {
                    // Traffic shaping error - return internal server error
                    let mut res = Response::new(axum::body::Body::from(format!(
                        "Traffic shaping error: {}",
                        e
                    )));
                    *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    return res;
                }
            }
        }
    }

    let (parts, body) = req.into_parts();
    let req = Request::from_parts(parts, body);

    let response = next.run(req).await;

    // Apply traffic shaping to the response
    if shared.traffic_shaping_enabled {
        if let Some(traffic_shaper) = &shared.traffic_shaper {
            // Calculate response size for bandwidth throttling
            let response_size = calculate_response_size(&response);

            let tags = op.as_ref().map(|o| o.tags.as_slice()).unwrap_or(&[]);

            // Apply traffic shaping to response
            match traffic_shaper.process_transfer(response_size, tags).await {
                Ok(Some(_timeout)) => {
                    // Response was "lost" due to burst loss - return timeout error
                    let mut res = Response::new(axum::body::Body::from(
                        "Response timeout due to traffic shaping",
                    ));
                    *res.status_mut() = StatusCode::GATEWAY_TIMEOUT;
                    return res;
                }
                Ok(None) => {
                    // Transfer allowed, continue
                }
                Err(e) => {
                    // Traffic shaping error - return internal server error
                    let mut res = Response::new(axum::body::Body::from(format!(
                        "Traffic shaping error: {}",
                        e
                    )));
                    *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    return res;
                }
            }
        }
    }

    response
}

/// Apply response overrides to a JSON body based on operation metadata
///
/// # Arguments
/// * `shared` - Shared middleware state containing override configuration
/// * `op` - Optional operation metadata for override matching
/// * `body` - JSON response body to modify in-place
pub fn apply_overrides(shared: &Shared, op: Option<&OperationMeta>, body: &mut Value) {
    if shared.overrides_enabled {
        if let Some(op) = op {
            shared.overrides.apply(
                &op.id,
                &op.tags.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                &op.path,
                body,
            );
        }
    }
}

/// Calculate the approximate size of an HTTP request for bandwidth throttling
fn calculate_request_size<B>(req: &Request<B>) -> u64 {
    let mut size = 0u64;

    // Add header sizes (rough estimate)
    for (name, value) in req.headers() {
        size += name.as_str().len() as u64;
        size += value.as_bytes().len() as u64;
    }

    // Add URI size
    size += req.uri().to_string().len() as u64;

    // Add body size (if available)
    // Note: This is a rough estimate since we can't easily get the body size here
    // without consuming the body. In practice, this would need to be implemented
    // differently to get accurate body sizes.
    size += 1024; // Rough estimate for body size

    size
}

/// Calculate the approximate size of an HTTP response for bandwidth throttling
fn calculate_response_size(res: &Response) -> u64 {
    let mut size = 0u64;

    // Add header sizes
    for (name, value) in res.headers() {
        size += name.as_str().len() as u64;
        size += value.as_bytes().len() as u64;
    }

    // Add status line size (rough estimate)
    size += 50;

    // Add body size (rough estimate)
    // Similar to request, this is a rough estimate
    size += 2048; // Rough estimate for response body size

    size
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Request, Response, StatusCode};
    use serde_json::json;

    #[test]
    fn test_operation_meta_creation() {
        let meta = OperationMeta {
            id: "getUserById".to_string(),
            tags: vec!["users".to_string(), "public".to_string()],
            path: "/users/{id}".to_string(),
        };

        assert_eq!(meta.id, "getUserById");
        assert_eq!(meta.tags.len(), 2);
        assert_eq!(meta.path, "/users/{id}");
    }

    #[test]
    fn test_shared_creation() {
        let shared = Shared {
            profiles: LatencyProfiles::default(),
            overrides: Overrides::default(),
            failure_injector: None,
            traffic_shaper: None,
            overrides_enabled: false,
            traffic_shaping_enabled: false,
        };

        assert!(!shared.overrides_enabled);
        assert!(!shared.traffic_shaping_enabled);
        assert!(shared.failure_injector.is_none());
        assert!(shared.traffic_shaper.is_none());
    }

    #[test]
    fn test_shared_with_failure_injector() {
        let failure_injector = FailureInjector::new(None, true);
        let shared = Shared {
            profiles: LatencyProfiles::default(),
            overrides: Overrides::default(),
            failure_injector: Some(failure_injector),
            traffic_shaper: None,
            overrides_enabled: false,
            traffic_shaping_enabled: false,
        };

        assert!(shared.failure_injector.is_some());
    }

    #[test]
    fn test_apply_overrides_disabled() {
        let shared = Shared {
            profiles: LatencyProfiles::default(),
            overrides: Overrides::default(),
            failure_injector: None,
            traffic_shaper: None,
            overrides_enabled: false,
            traffic_shaping_enabled: false,
        };

        let op = OperationMeta {
            id: "getUser".to_string(),
            tags: vec![],
            path: "/users".to_string(),
        };

        let mut body = json!({"name": "John"});
        let original = body.clone();

        apply_overrides(&shared, Some(&op), &mut body);

        // Should not modify body when overrides are disabled
        assert_eq!(body, original);
    }

    #[test]
    fn test_apply_overrides_enabled_no_rules() {
        let shared = Shared {
            profiles: LatencyProfiles::default(),
            overrides: Overrides::default(),
            failure_injector: None,
            traffic_shaper: None,
            overrides_enabled: true,
            traffic_shaping_enabled: false,
        };

        let op = OperationMeta {
            id: "getUser".to_string(),
            tags: vec![],
            path: "/users".to_string(),
        };

        let mut body = json!({"name": "John"});
        let original = body.clone();

        apply_overrides(&shared, Some(&op), &mut body);

        // Should not modify body when there are no override rules
        assert_eq!(body, original);
    }

    #[test]
    fn test_apply_overrides_with_none_operation() {
        let shared = Shared {
            profiles: LatencyProfiles::default(),
            overrides: Overrides::default(),
            failure_injector: None,
            traffic_shaper: None,
            overrides_enabled: true,
            traffic_shaping_enabled: false,
        };

        let mut body = json!({"name": "John"});
        let original = body.clone();

        apply_overrides(&shared, None, &mut body);

        // Should not modify body when operation is None
        assert_eq!(body, original);
    }

    #[test]
    fn test_calculate_request_size_basic() {
        let req = Request::builder()
            .uri("/test")
            .header("content-type", "application/json")
            .body(())
            .unwrap();

        let size = calculate_request_size(&req);

        // Should be > 0 (includes headers + URI + body estimate)
        assert!(size > 0);
        // Should include at least the URI and header sizes
        assert!(size >= "/test".len() as u64 + "content-type".len() as u64);
    }

    #[test]
    fn test_calculate_request_size_with_multiple_headers() {
        let req = Request::builder()
            .uri("/api/users")
            .header("content-type", "application/json")
            .header("authorization", "Bearer token123")
            .header("user-agent", "test-client")
            .body(())
            .unwrap();

        let size = calculate_request_size(&req);

        // Should account for all headers
        assert!(size > 100); // Reasonable size with multiple headers
    }

    #[test]
    fn test_calculate_response_size_basic() {
        let res = Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(axum::body::Body::empty())
            .unwrap();

        let size = calculate_response_size(&res);

        // Should be > 0 (includes status line + headers + body estimate)
        assert!(size > 0);
        // Should include at least the status line estimate (50) and header sizes
        assert!(size >= 50);
    }

    #[test]
    fn test_calculate_response_size_with_multiple_headers() {
        let res = Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .header("cache-control", "no-cache")
            .header("x-request-id", "123-456-789")
            .body(axum::body::Body::empty())
            .unwrap();

        let size = calculate_response_size(&res);

        // Should account for all headers
        assert!(size > 100);
    }

    #[test]
    fn test_shared_clone() {
        let shared = Shared {
            profiles: LatencyProfiles::default(),
            overrides: Overrides::default(),
            failure_injector: None,
            traffic_shaper: None,
            overrides_enabled: true,
            traffic_shaping_enabled: true,
        };

        let cloned = shared.clone();

        assert_eq!(shared.overrides_enabled, cloned.overrides_enabled);
        assert_eq!(shared.traffic_shaping_enabled, cloned.traffic_shaping_enabled);
    }

    #[test]
    fn test_operation_meta_clone() {
        let meta = OperationMeta {
            id: "testOp".to_string(),
            tags: vec!["tag1".to_string()],
            path: "/test".to_string(),
        };

        let cloned = meta.clone();

        assert_eq!(meta.id, cloned.id);
        assert_eq!(meta.tags, cloned.tags);
        assert_eq!(meta.path, cloned.path);
    }
}
