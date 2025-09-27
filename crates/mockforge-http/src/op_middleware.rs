//! Middleware/utilities to apply latency/failure and overrides per operation.
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::{extract::State, middleware::Next, response::Response};
use serde_json::Value;

use crate::latency_profiles::LatencyProfiles;
use mockforge_core::{FailureInjector, Overrides, TrafficShaper};

#[derive(Clone)]
pub struct OperationMeta {
    pub id: String,
    pub tags: Vec<String>,
    pub path: String,
}

#[derive(Clone)]
pub struct Shared {
    pub profiles: LatencyProfiles,
    pub overrides: Overrides,
    pub failure_injector: Option<FailureInjector>,
    pub traffic_shaper: Option<TrafficShaper>,
    pub overrides_enabled: bool,
    pub traffic_shaping_enabled: bool,
}

pub async fn add_shared_extension(
    State(shared): State<Shared>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    req.extensions_mut().insert(shared);
    next.run(req).await
}

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
