//! Security middleware for HTTP requests
//!
//! This middleware emits security events for all HTTP requests, tracking access granted/denied
//! and other security-relevant events.

use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use axum::middleware::Next;
use mockforge_core::security::{
    emit_security_event, EventActor, EventOutcome, EventTarget, SecurityEvent, SecurityEventType,
};
use tracing::debug;

/// Security middleware that emits security events for HTTP requests
///
/// This middleware tracks:
/// - Access granted/denied based on response status
/// - Request metadata (IP, user agent, method, path)
/// - Response status codes
pub async fn security_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    // Extract IP address and user agent
    let ip_address = req
        .headers()
        .get("x-forwarded-for")
        .or_else(|| req.headers().get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| {
            req.extensions()
                .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                .map(|addr| addr.ip().to_string())
        });

    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Extract user ID from request extensions (set by auth middleware)
    // Note: This would need to be adjusted based on actual auth implementation
    let user_id: Option<String> = None; // TODO: Extract from auth middleware when available

    // Process request
    let response = next.run(req).await;
    let status = response.status();

    // Determine if access was granted or denied based on status code
    let is_success = status.is_success();
    let is_client_error = status.is_client_error();
    let is_server_error = status.is_server_error();

    // Emit security event based on response status
    if is_success {
        // Access granted
        let event = SecurityEvent::new(SecurityEventType::AuthzAccessGranted, None, None)
            .with_actor(EventActor {
                user_id: user_id.clone(),
                username: user_id.clone(),
                ip_address: ip_address.clone(),
                user_agent: user_agent.clone(),
            })
            .with_target(EventTarget {
                resource_type: Some("api".to_string()),
                resource_id: Some(path.clone()),
                method: Some(method.to_string()),
            })
            .with_outcome(EventOutcome {
                success: true,
                reason: None,
            })
            .with_metadata("status_code".to_string(), serde_json::json!(status.as_u16()));
        emit_security_event(event).await;
    } else if is_client_error && status == StatusCode::FORBIDDEN {
        // Access denied (403)
        let event = SecurityEvent::new(SecurityEventType::AuthzAccessDenied, None, None)
            .with_actor(EventActor {
                user_id: user_id.clone(),
                username: user_id.clone(),
                ip_address: ip_address.clone(),
                user_agent: user_agent.clone(),
            })
            .with_target(EventTarget {
                resource_type: Some("api".to_string()),
                resource_id: Some(path.clone()),
                method: Some(method.to_string()),
            })
            .with_outcome(EventOutcome {
                success: false,
                reason: Some(format!("Access denied: {}", status)),
            })
            .with_metadata("status_code".to_string(), serde_json::json!(status.as_u16()));
        emit_security_event(event).await;
    } else if is_server_error {
        // Server error - could indicate security issue
        debug!("Server error detected: {} for {}", status, path);
        // Note: We don't emit security events for all server errors, only specific ones
        // This could be extended to detect specific security-related errors
    }

    response
}
