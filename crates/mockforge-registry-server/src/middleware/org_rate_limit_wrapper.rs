//! Wrapper for org rate limit middleware to work with axum's middleware system

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};

use crate::{middleware::org_rate_limit::org_rate_limit_middleware, AppState};

/// Wrapper function that extracts State and calls org_rate_limit_middleware
pub async fn org_rate_limit_wrapper(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, axum::response::IntoResponse> {
    org_rate_limit_middleware(State(state), headers, request, next).await
}
