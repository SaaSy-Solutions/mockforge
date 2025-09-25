//! Authentication middleware
//!
//! This module provides the Axum middleware for handling authentication
//! across HTTP requests.

use axum::http::{Request, StatusCode};
use axum::{extract::State, middleware::Next, response::Response};
use axum::body::Body;
use tracing::{debug, warn, error};

use super::state::AuthState;
use super::types::AuthResult;
use super::authenticator::authenticate_request;

/// Authentication middleware function
pub async fn auth_middleware(
    State(state): State<AuthState>,
    req: Request<Body>,
    next: Next,
) -> Response
{
    let path = req.uri().path().to_string();
    let method = req.method().clone();

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
        .get(state.config.api_key.as_ref().map(|c| c.header_name.clone()).unwrap_or_else(|| "X-API-Key".to_string()))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let api_key_query = req
        .uri()
        .query()
        .and_then(|q| {
            state.config.api_key.as_ref().and_then(|c| c.query_name.as_ref()).and_then(|param| {
                url::form_urlencoded::parse(q.as_bytes())
                    .find(|(k, _)| k == param)
                    .map(|(_, v)| v.to_string())
            })
        });

    // Try to authenticate using various methods
    let auth_result = authenticate_request(
        &state,
        &auth_header,
        &api_key_header,
        &api_key_query,
    ).await;

    match auth_result {
        AuthResult::Success(claims) => {
            debug!("Authentication successful for user: {:?}", claims.sub);
            // Add claims to request extensions for downstream handlers
            let mut req = req;
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        AuthResult::Failure(reason) => {
            warn!("Authentication failed: {}", reason);
            let mut res = Response::new(axum::body::Body::from(
                serde_json::json!({
                    "error": "Authentication failed",
                    "message": reason
                }).to_string()
            ));
            *res.status_mut() = StatusCode::UNAUTHORIZED;
            res.headers_mut().insert(
                "www-authenticate",
                "Bearer".parse().unwrap()
            );
            res
        }
        AuthResult::NetworkError(reason) => {
            error!("Authentication network error: {}", reason);
            let mut res = Response::new(axum::body::Body::from(
                serde_json::json!({
                    "error": "Authentication service unavailable",
                    "message": "Unable to verify token due to network issues"
                }).to_string()
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
                }).to_string()
            ));
            *res.status_mut() = StatusCode::BAD_GATEWAY;
            res
        }
        AuthResult::TokenExpired => {
            warn!("Token expired");
            let mut res = Response::new(axum::body::Body::from(
                serde_json::json!({
                    "error": "Token expired",
                    "message": "The provided token has expired"
                }).to_string()
            ));
            *res.status_mut() = StatusCode::UNAUTHORIZED;
            res.headers_mut().insert(
                "www-authenticate",
                "Bearer error=\"invalid_token\", error_description=\"The token has expired\"".parse().unwrap()
            );
            res
        }
        AuthResult::TokenInvalid(reason) => {
            warn!("Token invalid: {}", reason);
            let mut res = Response::new(axum::body::Body::from(
                serde_json::json!({
                    "error": "Invalid token",
                    "message": reason
                }).to_string()
            ));
            *res.status_mut() = StatusCode::UNAUTHORIZED;
            res.headers_mut().insert(
                "www-authenticate",
                "Bearer error=\"invalid_token\"".parse().unwrap()
            );
            res
        }
        AuthResult::None => {
            if state.config.require_auth {
                let mut res = Response::new(axum::body::Body::from(
                    serde_json::json!({
                        "error": "Authentication required"
                    }).to_string()
                ));
                *res.status_mut() = StatusCode::UNAUTHORIZED;
                res.headers_mut().insert(
                    "www-authenticate",
                    "Bearer".parse().unwrap()
                );
                res
            } else {
                debug!("No authentication provided, proceeding without auth");
                next.run(req).await
            }
        }
    }
}
