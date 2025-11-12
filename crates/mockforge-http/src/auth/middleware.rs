//! Authentication middleware
//!
//! This module provides the Axum middleware for handling authentication
//! across HTTP requests.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::{extract::State, middleware::Next, response::Response};
use tracing::{debug, error, warn};

use super::authenticator::authenticate_request;
use super::state::AuthState;
use super::types::AuthResult;
use mockforge_core::security::{
    emit_security_event, EventActor, EventOutcome, EventTarget, SecurityEvent, SecurityEventType,
};

/// Authentication middleware function
pub async fn auth_middleware(
    State(state): State<AuthState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    let _method = req.method().clone();

    // Skip authentication for health checks and admin endpoints
    if path.starts_with("/health") || path.starts_with("/__mockforge") {
        return next.run(req).await;
    }

    // Extract authentication information from request
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let api_key_header = req
        .headers()
        .get(
            state
                .config
                .api_key
                .as_ref()
                .map(|c| c.header_name.clone())
                .unwrap_or_else(|| "X-API-Key".to_string()),
        )
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let api_key_query = req.uri().query().and_then(|q| {
        state
            .config
            .api_key
            .as_ref()
            .and_then(|c| c.query_name.as_ref())
            .and_then(|param| {
                url::form_urlencoded::parse(q.as_bytes())
                    .find(|(k, _)| k == param)
                    .map(|(_, v)| v.to_string())
            })
    });

    // Extract IP address and user agent for security events
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

    // Try to authenticate using various methods
    let auth_result =
        authenticate_request(&state, &auth_header, &api_key_header, &api_key_query).await;

    match auth_result {
        AuthResult::Success(claims) => {
            debug!("Authentication successful for user: {:?}", claims.sub);

            // Emit security event for successful authentication
            let event = SecurityEvent::new(SecurityEventType::AuthSuccess, None, None)
                .with_actor(EventActor {
                    user_id: claims.sub.clone(),
                    username: claims.sub.clone(),
                    ip_address: ip_address.clone(),
                    user_agent: user_agent.clone(),
                })
                .with_target(EventTarget {
                    resource_type: Some("api".to_string()),
                    resource_id: Some(path.clone()),
                    method: Some(req.method().to_string()),
                })
                .with_outcome(EventOutcome {
                    success: true,
                    reason: None,
                })
                .with_metadata("auth_method".to_string(), serde_json::json!("jwt"));
            emit_security_event(event).await;

            // Add claims to request extensions for downstream handlers
            let mut req = req;
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        AuthResult::Failure(reason) => {
            warn!("Authentication failed: {}", reason);

            // Emit security event for authentication failure
            let event = SecurityEvent::new(SecurityEventType::AuthFailure, None, None)
                .with_actor(EventActor {
                    user_id: None,
                    username: auth_header
                        .as_ref()
                        .and_then(|h| h.strip_prefix("Bearer "))
                        .or_else(|| auth_header.as_ref().and_then(|h| h.strip_prefix("Basic ")))
                        .map(|s| s.to_string()),
                    ip_address: ip_address.clone(),
                    user_agent: user_agent.clone(),
                })
                .with_target(EventTarget {
                    resource_type: Some("api".to_string()),
                    resource_id: Some(path.clone()),
                    method: Some(req.method().to_string()),
                })
                .with_outcome(EventOutcome {
                    success: false,
                    reason: Some(reason.clone()),
                })
                .with_metadata("failure_reason".to_string(), serde_json::json!(reason));
            emit_security_event(event).await;
            let mut res = Response::new(axum::body::Body::from(
                serde_json::json!({
                    "error": "Authentication failed",
                    "message": reason
                })
                .to_string(),
            ));
            *res.status_mut() = StatusCode::UNAUTHORIZED;
            res.headers_mut().insert("www-authenticate", "Bearer".parse().unwrap());
            res
        }
        AuthResult::NetworkError(reason) => {
            error!("Authentication network error: {}", reason);
            let mut res = Response::new(axum::body::Body::from(
                serde_json::json!({
                    "error": "Authentication service unavailable",
                    "message": "Unable to verify token due to network issues"
                })
                .to_string(),
            ));
            *res.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
            res
        }
        AuthResult::ServerError(reason) => {
            error!("Authentication server error: {}", reason);
            let mut res = Response::new(axum::body::Body::from(
                serde_json::json!({
                    "error": "Authentication service error",
                    "message": "Unable to verify token due to server issues"
                })
                .to_string(),
            ));
            *res.status_mut() = StatusCode::BAD_GATEWAY;
            res
        }
        AuthResult::TokenExpired => {
            warn!("Token expired");

            // Emit security event for token expiration
            let event = SecurityEvent::new(SecurityEventType::AuthTokenExpired, None, None)
                .with_actor(EventActor {
                    user_id: None,
                    username: None,
                    ip_address: ip_address.clone(),
                    user_agent: user_agent.clone(),
                })
                .with_target(EventTarget {
                    resource_type: Some("api".to_string()),
                    resource_id: Some(path.clone()),
                    method: Some(req.method().to_string()),
                })
                .with_outcome(EventOutcome {
                    success: false,
                    reason: Some("Token expired".to_string()),
                });
            emit_security_event(event).await;
            let mut res = Response::new(axum::body::Body::from(
                serde_json::json!({
                    "error": "Token expired",
                    "message": "The provided token has expired"
                })
                .to_string(),
            ));
            *res.status_mut() = StatusCode::UNAUTHORIZED;
            res.headers_mut().insert(
                "www-authenticate",
                "Bearer error=\"invalid_token\", error_description=\"The token has expired\""
                    .parse()
                    .unwrap(),
            );
            res
        }
        AuthResult::TokenInvalid(reason) => {
            warn!("Token invalid: {}", reason);

            // Emit security event for invalid token
            let event = SecurityEvent::new(SecurityEventType::AuthFailure, None, None)
                .with_actor(EventActor {
                    user_id: None,
                    username: None,
                    ip_address: ip_address.clone(),
                    user_agent: user_agent.clone(),
                })
                .with_target(EventTarget {
                    resource_type: Some("api".to_string()),
                    resource_id: Some(path.clone()),
                    method: Some(req.method().to_string()),
                })
                .with_outcome(EventOutcome {
                    success: false,
                    reason: Some(format!("Invalid token: {}", reason)),
                })
                .with_metadata("token_invalid".to_string(), serde_json::json!(true));
            emit_security_event(event).await;
            let mut res = Response::new(axum::body::Body::from(
                serde_json::json!({
                    "error": "Invalid token",
                    "message": reason
                })
                .to_string(),
            ));
            *res.status_mut() = StatusCode::UNAUTHORIZED;
            res.headers_mut()
                .insert("www-authenticate", "Bearer error=\"invalid_token\"".parse().unwrap());
            res
        }
        AuthResult::None => {
            if state.config.require_auth {
                // Emit security event for missing authentication
                let event = SecurityEvent::new(SecurityEventType::AuthzAccessDenied, None, None)
                    .with_actor(EventActor {
                        user_id: None,
                        username: None,
                        ip_address: ip_address.clone(),
                        user_agent: user_agent.clone(),
                    })
                    .with_target(EventTarget {
                        resource_type: Some("api".to_string()),
                        resource_id: Some(path.clone()),
                        method: Some(req.method().to_string()),
                    })
                    .with_outcome(EventOutcome {
                        success: false,
                        reason: Some("Authentication required but not provided".to_string()),
                    });
                emit_security_event(event).await;

                let mut res = Response::new(axum::body::Body::from(
                    serde_json::json!({
                        "error": "Authentication required"
                    })
                    .to_string(),
                ));
                *res.status_mut() = StatusCode::UNAUTHORIZED;
                res.headers_mut().insert("www-authenticate", "Bearer".parse().unwrap());
                res
            } else {
                debug!("No authentication provided, proceeding without auth");
                next.run(req).await
            }
        }
    }
}
