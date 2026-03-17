//! Multitenant routing middleware for hosted mocks
//!
//! Routes requests to the correct mock service based on org/project/env.
//! Also handles custom-domain routing for `*.mocks.mockforge.dev`.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, Method, StatusCode, Uri},
    response::Response,
    routing::any,
    Router,
};
use uuid::Uuid;

use crate::models::HostedMock;
use crate::AppState;

/// Multitenant router that routes requests to deployed mock services
pub struct MultitenantRouter;

impl MultitenantRouter {
    /// Create router for multitenant mock routing
    /// Routes are nested under `/mocks/` to avoid conflicts with API routes
    pub fn create_router() -> Router<AppState> {
        Router::new()
            .route("/mocks/{org_id}/{slug}/{*path}", any(Self::route_request))
            .route("/mocks/{org_id}/{slug}", any(Self::route_request))
    }

    /// Route request to the appropriate mock service
    async fn route_request(
        State(state): State<AppState>,
        method: Method,
        Path((org_id_str, slug)): Path<(String, String)>,
        uri: Uri,
        headers: HeaderMap,
        body: axum::body::Body,
    ) -> Result<Response, StatusCode> {
        // Parse org_id
        let org_id = Uuid::parse_str(&org_id_str).map_err(|e| {
            tracing::warn!("Invalid org_id '{}': {}", org_id_str, e);
            StatusCode::BAD_REQUEST
        })?;

        // Find deployment
        let deployment = HostedMock::find_by_slug(state.db.pool(), org_id, &slug)
            .await
            .map_err(|e| {
                tracing::error!("Database error looking up deployment {}/{}: {}", org_id, slug, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;

        // Check if deployment is active
        if !matches!(deployment.status(), crate::models::DeploymentStatus::Active) {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }

        // Get the target base URL (prefer internal_url for Fly.io internal routing)
        let base_url = deployment
            .internal_url
            .as_ref()
            .or(deployment.deployment_url.as_ref())
            .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

        // Extract path from URI (strip /mocks/:org_id/:slug prefix)
        let path = uri.path();
        let path_after_slug =
            path.strip_prefix(&format!("/mocks/{}/{}", org_id_str, slug)).unwrap_or("/");

        // Build target URL
        let target_url = build_target_url(base_url, path_after_slug, uri.query());

        proxy_request(method, headers, body, &target_url).await
    }
}

/// Custom domain fallback handler.
///
/// When `MOCKFORGE_MOCKS_DOMAIN` is set (e.g., `mocks.mockforge.dev`), the
/// registry server handles requests to `<slug>.mocks.mockforge.dev` by looking
/// up the deployment by slug and proxying to its internal URL.
///
/// This is used as the router's fallback handler so it only fires when no
/// other route matches.
pub async fn custom_domain_fallback(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: axum::body::Body,
) -> Result<Response, StatusCode> {
    let mocks_domain = match std::env::var("MOCKFORGE_MOCKS_DOMAIN") {
        Ok(d) => d,
        Err(_) => return Err(StatusCode::NOT_FOUND),
    };

    // Extract host from headers (strip port if present)
    let host = headers.get("host").and_then(|v| v.to_str().ok()).unwrap_or("");
    let host = host.split(':').next().unwrap_or(host);

    // Check if host matches <slug>.<mocks_domain>
    let slug = match host.strip_suffix(&format!(".{}", mocks_domain)) {
        Some(s) if !s.is_empty() && !s.contains('.') => s,
        _ => return Err(StatusCode::NOT_FOUND),
    };

    tracing::debug!("Custom domain proxy: {} -> slug '{}'", host, slug);

    // Find deployment by slug across all orgs
    let deployment = HostedMock::find_active_by_slug(state.db.pool(), slug)
        .await
        .map_err(|e| {
            tracing::error!("Database error looking up deployment by slug '{}': {}", slug, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get the target base URL (prefer internal_url for Fly.io internal routing)
    let base_url = deployment
        .internal_url
        .as_ref()
        .or(deployment.deployment_url.as_ref())
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let target_url = build_target_url(base_url, uri.path(), uri.query());

    proxy_request(method, headers, body, &target_url).await
}

/// Build the full target URL from base, path, and optional query string
fn build_target_url(base_url: &str, path: &str, query: Option<&str>) -> String {
    let mut url = format!("{}{}", base_url, path);
    if let Some(q) = query {
        url = format!("{}?{}", url, q);
    }
    url
}

/// Proxy an HTTP request to a target URL and return the response
async fn proxy_request(
    method: Method,
    headers: HeaderMap,
    body: axum::body::Body,
    target_url: &str,
) -> Result<Response, StatusCode> {
    let client = reqwest::Client::new();

    // Read body if present
    let body_bytes = axum::body::to_bytes(body, usize::MAX).await.map_err(|e| {
        tracing::warn!("Failed to read request body: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    // Build request based on method
    let request_builder = match method.as_str() {
        "GET" => client.get(target_url),
        "HEAD" => client.head(target_url),
        "POST" => {
            let mut req = client.post(target_url);
            if !body_bytes.is_empty() {
                req = req.body(body_bytes.to_vec());
            }
            req
        }
        "PUT" => {
            let mut req = client.put(target_url);
            if !body_bytes.is_empty() {
                req = req.body(body_bytes.to_vec());
            }
            req
        }
        "PATCH" => {
            let mut req = client.patch(target_url);
            if !body_bytes.is_empty() {
                req = req.body(body_bytes.to_vec());
            }
            req
        }
        "DELETE" => client.delete(target_url),
        _ => return Err(StatusCode::METHOD_NOT_ALLOWED),
    };

    let mut request = request_builder.timeout(std::time::Duration::from_secs(30));

    // Forward relevant headers
    for header_name in ["accept", "content-type", "authorization", "x-request-id"] {
        if let Some(value) = headers.get(header_name) {
            if let Ok(value_str) = value.to_str() {
                request = request.header(header_name, value_str);
            }
        }
    }

    let response = request.send().await.map_err(|e| {
        tracing::error!("Failed to proxy request to {}: {}", target_url, e);
        StatusCode::BAD_GATEWAY
    })?;

    // Convert response
    let status = StatusCode::from_u16(response.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    let mut response_headers = Vec::new();
    for (key, value) in response.headers() {
        if let (Ok(header_name), Ok(value_str)) =
            (key.as_str().parse::<axum::http::HeaderName>(), value.to_str())
        {
            if let Ok(header_value) = axum::http::HeaderValue::from_str(value_str) {
                response_headers.push((header_name, header_value));
            }
        }
    }

    let resp_body = response.bytes().await.map_err(|e| {
        tracing::error!("Failed to read proxy response body: {}", e);
        StatusCode::BAD_GATEWAY
    })?;

    let mut response_builder = Response::builder().status(status);
    for (header_name, header_value) in response_headers {
        response_builder = response_builder.header(header_name, header_value);
    }

    response_builder.body(axum::body::Body::from(resp_body.to_vec())).map_err(|e| {
        tracing::error!("Failed to build proxy response: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}
