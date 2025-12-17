//! Multitenant routing middleware for hosted mocks
//!
//! Routes requests to the correct mock service based on org/project/env

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
    pub fn create_router(state: AppState) -> Router {
        Router::new()
            .route("/:org_id/:slug/*path", any(Self::route_request))
            .route("/:org_id/:slug", any(Self::route_request))
            .with_state(state)
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
        let org_id = Uuid::parse_str(&org_id_str).map_err(|_| StatusCode::BAD_REQUEST)?;

        // Find deployment
        let deployment = HostedMock::find_by_slug(state.db.pool(), org_id, &slug)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        // Check if deployment is active
        if !matches!(deployment.status(), crate::models::DeploymentStatus::Active) {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }

        // Get deployment URL
        let base_url = deployment.deployment_url.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

        // Extract path from URI
        let path = uri.path();
        let path_after_slug =
            path.strip_prefix(&format!("/{}/{}", org_id_str, slug)).unwrap_or("/");

        // Build target URL
        let mut target_url = format!("{}{}", base_url, path_after_slug);
        if let Some(query) = uri.query() {
            target_url = format!("{}?{}", target_url, query);
        }

        // Proxy request to deployed service
        let client = reqwest::Client::new();

        // Read body if present
        let body_bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        // Build request based on method
        let request_builder = match method.as_str() {
            "GET" => client.get(&target_url),
            "POST" => {
                let mut req = client.post(&target_url);
                if !body_bytes.is_empty() {
                    req = req.body(body_bytes.to_vec());
                }
                req
            }
            "PUT" => {
                let mut req = client.put(&target_url);
                if !body_bytes.is_empty() {
                    req = req.body(body_bytes.to_vec());
                }
                req
            }
            "PATCH" => {
                let mut req = client.patch(&target_url);
                if !body_bytes.is_empty() {
                    req = req.body(body_bytes.to_vec());
                }
                req
            }
            "DELETE" => client.delete(&target_url),
            _ => return Err(StatusCode::METHOD_NOT_ALLOWED),
        };

        let mut request = request_builder.timeout(std::time::Duration::from_secs(30));

        // Forward relevant headers (convert to strings for reqwest)
        if let Some(accept) = headers.get("accept") {
            if let Ok(accept_str) = accept.to_str() {
                request = request.header("accept", accept_str);
            }
        }
        if let Some(content_type) = headers.get("content-type") {
            if let Ok(content_type_str) = content_type.to_str() {
                request = request.header("content-type", content_type_str);
            }
        }
        if let Some(authorization) = headers.get("authorization") {
            if let Ok(auth_str) = authorization.to_str() {
                request = request.header("authorization", auth_str);
            }
        }

        let response = request.send().await.map_err(|_| StatusCode::BAD_GATEWAY)?;

        // Convert response - save status and headers before consuming body
        let status = StatusCode::from_u16(response.status().as_u16())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        // Collect headers before consuming response
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

        let body_bytes = response.bytes().await.map_err(|_| StatusCode::BAD_GATEWAY)?;

        // Build response with headers
        let mut response_builder = Response::builder().status(status);
        for (header_name, header_value) in response_headers {
            response_builder = response_builder.header(header_name, header_value);
        }

        let http_response = response_builder
            .body(axum::body::Body::from(body_bytes.to_vec()))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(http_response)
    }
}
