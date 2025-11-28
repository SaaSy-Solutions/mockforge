//! Simple tunnel server implementation for testing
//!
//! This module provides a basic tunnel server that can be used for integration testing
//! and local development. It's not production-ready but sufficient for testing.

use crate::{TunnelConfig, TunnelStatus};
use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri},
    response::{Json, Response},
    routing::{any, delete, get, post},
    Router,
};
use chrono::Utc;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::result::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Trait for tunnel storage backends
#[async_trait]
pub trait TunnelStoreTrait: Send + Sync {
    async fn create_tunnel(&self, config: &TunnelConfig) -> crate::Result<TunnelStatus>;
    async fn get_tunnel(&self, tunnel_id: &str) -> crate::Result<TunnelStatus>;
    async fn delete_tunnel(&self, tunnel_id: &str) -> crate::Result<()>;
    async fn list_tunnels(&self) -> Vec<TunnelStatus>;
    async fn get_tunnel_by_subdomain(&self, subdomain: &str) -> crate::Result<TunnelStatus>;
    async fn get_tunnel_by_id(&self, tunnel_id: &str) -> crate::Result<TunnelStatus>;
    async fn record_request(&self, tunnel_id: &str, bytes: u64);
    async fn cleanup_expired(&self) -> crate::Result<u64>;
}

/// Wrapper type for tunnel store trait objects
/// This allows us to use trait objects with Axum's State while maintaining Clone
#[derive(Clone)]
pub struct TunnelStoreWrapper {
    inner: Arc<dyn TunnelStoreTrait>,
}

impl TunnelStoreWrapper {
    pub fn new(store: Arc<dyn TunnelStoreTrait>) -> Self {
        Self { inner: store }
    }
}

#[async_trait]
impl TunnelStoreTrait for TunnelStoreWrapper {
    async fn create_tunnel(&self, config: &TunnelConfig) -> crate::Result<TunnelStatus> {
        self.inner.create_tunnel(config).await
    }

    async fn get_tunnel(&self, tunnel_id: &str) -> crate::Result<TunnelStatus> {
        self.inner.get_tunnel(tunnel_id).await
    }

    async fn delete_tunnel(&self, tunnel_id: &str) -> crate::Result<()> {
        self.inner.delete_tunnel(tunnel_id).await
    }

    async fn list_tunnels(&self) -> Vec<TunnelStatus> {
        self.inner.list_tunnels().await
    }

    async fn get_tunnel_by_subdomain(&self, subdomain: &str) -> crate::Result<TunnelStatus> {
        self.inner.get_tunnel_by_subdomain(subdomain).await
    }

    async fn get_tunnel_by_id(&self, tunnel_id: &str) -> crate::Result<TunnelStatus> {
        self.inner.get_tunnel_by_id(tunnel_id).await
    }

    async fn record_request(&self, tunnel_id: &str, bytes: u64) {
        self.inner.record_request(tunnel_id, bytes).await
    }

    async fn cleanup_expired(&self) -> crate::Result<u64> {
        self.inner.cleanup_expired().await
    }
}

/// In-memory tunnel store
#[derive(Clone)]
pub struct TunnelStore {
    tunnels: Arc<RwLock<HashMap<String, TunnelStatus>>>,
    subdomains: Arc<RwLock<HashMap<String, String>>>, // subdomain -> tunnel_id
}

impl TunnelStore {
    pub fn new() -> Self {
        Self {
            tunnels: Arc::new(RwLock::new(HashMap::new())),
            subdomains: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_tunnel(&self, config: &TunnelConfig) -> crate::Result<TunnelStatus> {
        let tunnel_id = Uuid::new_v4().to_string();

        // Generate public URL
        let public_url = if let Some(subdomain) = &config.subdomain {
            // Check if subdomain is available
            let subdomains = self.subdomains.read().await;
            if subdomains.contains_key(subdomain) {
                return Err(crate::TunnelError::AlreadyExists(format!(
                    "Subdomain '{}' is already in use",
                    subdomain
                )));
            }
            drop(subdomains);

            // Reserve subdomain
            let mut subdomains = self.subdomains.write().await;
            subdomains.insert(subdomain.clone(), tunnel_id.clone());

            format!("https://{}.tunnel.mockforge.test", subdomain)
        } else {
            // Auto-generate subdomain
            let subdomain = format!("tunnel-{}", &tunnel_id[..8]);
            let mut subdomains = self.subdomains.write().await;
            subdomains.insert(subdomain.clone(), tunnel_id.clone());

            format!("https://{}.tunnel.mockforge.test", subdomain)
        };

        let status = TunnelStatus {
            public_url: public_url.clone(),
            tunnel_id: tunnel_id.clone(),
            active: true,
            request_count: 0,
            bytes_transferred: 0,
            created_at: Some(Utc::now()),
            expires_at: None,
            local_url: Some(config.local_url.clone()),
        };

        let mut tunnels = self.tunnels.write().await;
        tunnels.insert(tunnel_id, status.clone());

        Ok(status)
    }

    pub async fn get_tunnel(&self, tunnel_id: &str) -> crate::Result<TunnelStatus> {
        let tunnels = self.tunnels.read().await;
        tunnels
            .get(tunnel_id)
            .cloned()
            .ok_or_else(|| crate::TunnelError::NotFound(tunnel_id.to_string()))
    }

    pub async fn delete_tunnel(&self, tunnel_id: &str) -> crate::Result<()> {
        let mut tunnels = self.tunnels.write().await;
        if tunnels.remove(tunnel_id).is_some() {
            // Clean up subdomain if exists
            let mut subdomains = self.subdomains.write().await;
            subdomains.retain(|_, id| id != tunnel_id);
            Ok(())
        } else {
            Err(crate::TunnelError::NotFound(tunnel_id.to_string()))
        }
    }

    pub async fn list_tunnels(&self) -> Vec<TunnelStatus> {
        let tunnels = self.tunnels.read().await;
        tunnels.values().cloned().collect()
    }

    /// Get tunnel by subdomain
    pub async fn get_tunnel_by_subdomain(&self, subdomain: &str) -> crate::Result<TunnelStatus> {
        let subdomains = self.subdomains.read().await;
        let tunnel_id = subdomains
            .get(subdomain)
            .ok_or_else(|| {
                crate::TunnelError::NotFound(format!("Subdomain not found: {}", subdomain))
            })?
            .clone();
        drop(subdomains);

        self.get_tunnel(&tunnel_id).await
    }

    /// Get tunnel by tunnel_id (used for path-based routing)
    pub async fn get_tunnel_by_id(&self, tunnel_id: &str) -> crate::Result<TunnelStatus> {
        self.get_tunnel(tunnel_id).await
    }

    /// Increment request count and bytes for a tunnel
    pub async fn record_request(&self, tunnel_id: &str, bytes: u64) {
        let mut tunnels = self.tunnels.write().await;
        if let Some(status) = tunnels.get_mut(tunnel_id) {
            status.request_count += 1;
            status.bytes_transferred += bytes;
        }
    }
}

#[async_trait]
impl TunnelStoreTrait for TunnelStore {
    async fn create_tunnel(&self, config: &TunnelConfig) -> crate::Result<TunnelStatus> {
        self.create_tunnel(config).await
    }

    async fn get_tunnel(&self, tunnel_id: &str) -> crate::Result<TunnelStatus> {
        self.get_tunnel(tunnel_id).await
    }

    async fn delete_tunnel(&self, tunnel_id: &str) -> crate::Result<()> {
        self.delete_tunnel(tunnel_id).await
    }

    async fn list_tunnels(&self) -> Vec<TunnelStatus> {
        self.list_tunnels().await
    }

    async fn get_tunnel_by_subdomain(&self, subdomain: &str) -> crate::Result<TunnelStatus> {
        self.get_tunnel_by_subdomain(subdomain).await
    }

    async fn get_tunnel_by_id(&self, tunnel_id: &str) -> crate::Result<TunnelStatus> {
        self.get_tunnel_by_id(tunnel_id).await
    }

    async fn record_request(&self, tunnel_id: &str, bytes: u64) {
        self.record_request(tunnel_id, bytes).await
    }

    async fn cleanup_expired(&self) -> crate::Result<u64> {
        // In-memory store doesn't need cleanup
        Ok(0)
    }
}

/// Create tunnel API handler
async fn create_tunnel_handler(
    State(store): State<TunnelStoreWrapper>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<TunnelStatus>, (StatusCode, String)> {
    let local_url = payload["local_url"]
        .as_str()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "local_url required".to_string()))?;

    let config = TunnelConfig {
        local_url: local_url.to_string(),
        subdomain: payload["subdomain"].as_str().map(|s| s.to_string()),
        custom_domain: payload["custom_domain"].as_str().map(|s| s.to_string()),
        protocol: payload["protocol"].as_str().unwrap_or("http").to_string(),
        websocket_enabled: payload["websocket_enabled"].as_bool().unwrap_or(true),
        http2_enabled: payload["http2_enabled"].as_bool().unwrap_or(true),
        ..Default::default()
    };

    match store.create_tunnel(&config).await {
        Ok(status) => Ok(Json(status)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// Get tunnel status API handler
async fn get_tunnel_handler(
    State(store): State<TunnelStoreWrapper>,
    axum::extract::Path(tunnel_id): axum::extract::Path<String>,
) -> Result<Json<TunnelStatus>, (StatusCode, String)> {
    match store.get_tunnel(&tunnel_id).await {
        Ok(status) => Ok(Json(status)),
        Err(e) => Err((StatusCode::NOT_FOUND, e.to_string())),
    }
}

/// Delete tunnel API handler
async fn delete_tunnel_handler(
    State(store): State<TunnelStoreWrapper>,
    axum::extract::Path(tunnel_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match store.delete_tunnel(&tunnel_id).await {
        Ok(_) => Ok(Json(serde_json::json!({"success": true}))),
        Err(e) => Err((StatusCode::NOT_FOUND, e.to_string())),
    }
}

/// List tunnels API handler
async fn list_tunnels_handler(State(store): State<TunnelStoreWrapper>) -> Json<Vec<TunnelStatus>> {
    let tunnels = store.list_tunnels().await;
    Json(tunnels)
}

/// Health check handler
async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "mockforge-tunnel-server"
    }))
}

/// Extract subdomain from Host header
/// Handles formats like: "subdomain.example.com" or "subdomain.tunnel.mockforge.test"
fn extract_subdomain(host: &str) -> Option<String> {
    // Remove port if present (e.g., "host:8080" -> "host")
    let host_without_port = host.split(':').next().unwrap_or(host);

    // Split by dots
    let parts: Vec<&str> = host_without_port.split('.').collect();

    // For test domain format: "subdomain.tunnel.mockforge.test" -> take first part
    if parts.len() >= 4 && parts[parts.len() - 3] == "tunnel" {
        return Some(parts[0].to_string());
    }

    // For other formats, try to find subdomain (first part if multiple parts)
    if parts.len() >= 2 {
        // Check if this looks like a subdomain (not just a domain)
        // If we have 2+ parts and first part doesn't look like a TLD, it's probably a subdomain
        let first = parts[0];
        if !matches!(first, "www" | "api" | "app") || parts.len() > 2 {
            return Some(first.to_string());
        }
    }

    None
}

/// Path-based proxy handler for /tunnel/<tunnel_id>/<path>
async fn path_based_proxy_handler(
    State(store): State<TunnelStoreWrapper>,
    axum::extract::Path((tunnel_id, path)): axum::extract::Path<(String, String)>,
    method: Method,
    uri: Uri,
    request: Request<Body>,
) -> Result<Response<Body>, StatusCode> {
    // Axum's {*path} captures everything after the prefix without leading slash
    // For /tunnel/{id}/test, path will be "test"
    // For /tunnel/{id}/api/data, path will be "api/data"
    info!(
        "Path-based proxy request: {} {} (tunnel: {}, extracted path: '{}')",
        method,
        uri.path(),
        &tunnel_id,
        &path
    );

    match store.get_tunnel_by_id(&tunnel_id).await {
        Ok(tunnel) => {
            // Normalize path: ensure it starts with / for proper URL construction
            // Axum's {*path} captures without leading slash:
            // - /tunnel/{id}/test -> path = "test" -> normalized = "/test"
            // - /tunnel/{id}/ -> path = "" (empty) -> normalized = "/"
            // - /tunnel/{id} -> path = "" (empty) -> normalized = "/"
            let normalized_path = if path.is_empty() || path == "/" {
                "/".to_string()
            } else if path.starts_with('/') {
                path.clone()
            } else {
                format!("/{}", path)
            };
            info!("Normalized path: '{}' -> '{}'", path, normalized_path);
            forward_request(&store, &tunnel, &method, &uri, request, &normalized_path).await
        }
        Err(_) => {
            warn!("Tunnel not found by ID: {}", tunnel_id);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// Host-header-based proxy handler (fallback)
async fn host_header_proxy_handler(
    State(store): State<TunnelStoreWrapper>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    request: Request<Body>,
) -> Result<Response<Body>, StatusCode> {
    debug!("Host-header proxy request: {} {}", method, uri.path());

    // Try Host-header-based routing
    if let Some(host_header) = headers.get("host").and_then(|h| h.to_str().ok()) {
        if let Some(subdomain) = extract_subdomain(host_header) {
            debug!("Extracted subdomain from Host header: {}", subdomain);
            match store.get_tunnel_by_subdomain(&subdomain).await {
                Ok(tunnel) => {
                    return forward_request(&store, &tunnel, &method, &uri, request, "").await;
                }
                Err(_) => {
                    warn!("Tunnel not found for subdomain: {}", subdomain);
                }
            }
        }
    }

    // If we reach here, routing failed
    error!("Could not route request: {} {}", method, uri.path());
    Err(StatusCode::NOT_FOUND)
}

/// Forward a request to the tunnel's local URL
async fn forward_request(
    store: &TunnelStoreWrapper,
    tunnel: &TunnelStatus,
    method: &Method,
    uri: &Uri,
    request: Request<Body>,
    path_override: &str,
) -> Result<Response<Body>, StatusCode> {
    // Get local URL from tunnel
    let local_url = tunnel.local_url.as_ref().ok_or_else(|| {
        error!("Tunnel {} has no local_url", tunnel.tunnel_id);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Build target URL
    // path_override is already normalized (starts with /) when coming from path_based_proxy_handler
    let target_path = if !path_override.is_empty() && path_override != "/" {
        // Path-based routing: use the normalized path (already has leading /)
        path_override.to_string()
    } else if !path_override.is_empty() {
        // Root path
        "/".to_string()
    } else {
        // Host-header routing: use the full path from original request
        uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/").to_string()
    };

    // Construct full target URL
    let target_url = if target_path.starts_with("http://") || target_path.starts_with("https://") {
        target_path.clone()
    } else {
        // Ensure local_url doesn't end with / and path starts with /
        let base_url = local_url.trim_end_matches('/');
        format!("{}{}", base_url, target_path)
    };

    debug!(
        "Forwarding {} {} -> {} (tunnel: {}, path_override: {:?}, target_path: {:?})",
        method, uri, target_url, tunnel.tunnel_id, path_override, &target_path
    );

    // Read request body
    let body_bytes = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
        Ok(bytes) => bytes.to_vec(),
        Err(e) => {
            error!("Failed to read request body: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Create reqwest client
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| {
            error!("Failed to create HTTP client: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Convert method
    let reqwest_method = match *method {
        Method::GET => reqwest::Method::GET,
        Method::POST => reqwest::Method::POST,
        Method::PUT => reqwest::Method::PUT,
        Method::DELETE => reqwest::Method::DELETE,
        Method::PATCH => reqwest::Method::PATCH,
        Method::HEAD => reqwest::Method::HEAD,
        Method::OPTIONS => reqwest::Method::OPTIONS,
        _ => {
            warn!("Unsupported method: {}", method);
            return Err(StatusCode::METHOD_NOT_ALLOWED);
        }
    };

    // Build request
    let mut forward_request = client.request(reqwest_method.clone(), &target_url);

    // Copy headers from original request (excluding hop-by-hop headers)
    // Note: We need to extract headers from the Request, but we've already consumed it
    // For now, we'll make a minimal request. In production, you'd want to preserve headers.
    let excluded_headers = [
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailer",
        "transfer-encoding",
        "upgrade",
        "host", // Replace with target host
    ];

    // Set Content-Type if body is present
    if !body_bytes.is_empty() && method != &Method::GET && method != &Method::HEAD {
        forward_request = forward_request.header("Content-Type", "application/json");
        forward_request = forward_request.body(body_bytes.clone());
    }

    // Send request
    let response = forward_request.send().await.map_err(|e| {
        error!(
            "Failed to forward request to {}: {} (local_url: {}, path_override: {:?})",
            target_url, e, local_url, path_override
        );
        StatusCode::BAD_GATEWAY
    })?;

    // Get response status
    let status = StatusCode::from_u16(response.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    // Extract headers before consuming response body
    let mut response_headers = HeaderMap::new();
    for (name, value) in response.headers() {
        let name_str = name.as_str().to_lowercase();
        if !excluded_headers.contains(&name_str.as_str()) {
            if let (Ok(header_name), Ok(header_value)) =
                (HeaderName::try_from(name.as_str()), HeaderValue::try_from(value.as_bytes()))
            {
                response_headers.insert(header_name, header_value);
            }
        }
    }

    // Read response body
    let response_body = response.bytes().await.map_err(|e| {
        error!("Failed to read response body: {}", e);
        StatusCode::BAD_GATEWAY
    })?;

    let response_body_size = response_body.len() as u64;
    let request_body_size = body_bytes.len() as u64;
    let total_bytes = request_body_size + response_body_size;

    // Record request stats
    store.record_request(&tunnel.tunnel_id, total_bytes).await;

    // Build response
    let mut response_builder = Response::builder().status(status);

    // Add extracted headers
    for (name, value) in response_headers.iter() {
        response_builder = response_builder.header(name, value);
    }

    // Add CORS headers for testing
    response_builder = response_builder
        .header("access-control-allow-origin", "*")
        .header("access-control-allow-methods", "GET, POST, PUT, DELETE, PATCH, OPTIONS")
        .header("access-control-allow-headers", "*");

    response_builder.body(Body::from(response_body.to_vec())).map_err(|e| {
        error!("Failed to build response: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

/// Root path handler for /tunnel/<tunnel_id> or /tunnel/<tunnel_id>/
async fn root_path_proxy_handler(
    State(store): State<TunnelStoreWrapper>,
    axum::extract::Path(tunnel_id): axum::extract::Path<String>,
    method: Method,
    uri: Uri,
    request: Request<Body>,
) -> Result<Response<Body>, StatusCode> {
    info!("Root path proxy request: {} {} (tunnel: {})", method, uri.path(), &tunnel_id);

    match store.get_tunnel_by_id(&tunnel_id).await {
        Ok(tunnel) => {
            // Forward to root path
            forward_request(&store, &tunnel, &method, &uri, request, "/").await
        }
        Err(_) => {
            warn!("Tunnel not found by ID: {}", tunnel_id);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// Create the tunnel server router
pub fn create_tunnel_server_router() -> Router<TunnelStoreWrapper> {
    Router::new()
        .route("/health", get(health_handler))
        .route("/api/tunnels", post(create_tunnel_handler))
        .route("/api/tunnels", get(list_tunnels_handler))
        .route("/api/tunnels/{tunnel_id}", get(get_tunnel_handler))
        .route("/api/tunnels/{tunnel_id}", delete(delete_tunnel_handler))
        // Root path proxy routes: /tunnel/<tunnel_id> or /tunnel/<tunnel_id>/
        // Note: We need both routes because axum treats trailing slash differently
        .route("/tunnel/{tunnel_id}", any(root_path_proxy_handler))
        .route("/tunnel/{tunnel_id}/", any(root_path_proxy_handler))
        // Path-based proxy route: /tunnel/<tunnel_id>/<path> (with actual path segments)
        .route("/tunnel/{tunnel_id}/{*path}", any(path_based_proxy_handler))
        // Catch-all for Host-header-based routing (when subdomain is used)
        .fallback(host_header_proxy_handler)
}

/// Start a test tunnel server
pub async fn start_test_server(port: u16) -> crate::Result<SocketAddr> {
    let store = TunnelStore::new();
    let store_wrapper = TunnelStoreWrapper::new(Arc::new(store));
    let router = create_tunnel_server_router().with_state(store_wrapper);

    let addr = if port == 0 {
        // Bind to any available port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.map_err(|e| {
            crate::TunnelError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to bind: {}", e),
            ))
        })?;
        let actual_addr = listener.local_addr().map_err(|e| {
            crate::TunnelError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get local address: {}", e),
            ))
        })?;

        tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .map_err(|e| {
                    eprintln!("Tunnel server error: {}", e);
                })
                .ok();
        });

        actual_addr
    } else {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
            crate::TunnelError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to bind to {}: {}", addr, e),
            ))
        })?;

        tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .map_err(|e| {
                    eprintln!("Tunnel server error: {}", e);
                })
                .ok();
        });

        addr
    };

    // Give server a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    Ok(addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tunnel_store_create() {
        let store = TunnelStore::new();
        let config = TunnelConfig::new("http://localhost:3000").with_subdomain("test-api");

        let status = store.create_tunnel(&config).await.unwrap();
        assert_eq!(status.local_url, Some("http://localhost:3000".to_string()));
        assert!(status.active);
        assert!(status.public_url.contains("test-api"));
    }

    #[tokio::test]
    async fn test_tunnel_store_get() {
        let store = TunnelStore::new();
        let config = TunnelConfig::new("http://localhost:3000");

        let status = store.create_tunnel(&config).await.unwrap();
        let retrieved = store.get_tunnel(&status.tunnel_id).await.unwrap();
        assert_eq!(retrieved.tunnel_id, status.tunnel_id);
    }

    #[tokio::test]
    async fn test_tunnel_store_delete() {
        let store = TunnelStore::new();
        let config = TunnelConfig::new("http://localhost:3000");

        let status = store.create_tunnel(&config).await.unwrap();
        store.delete_tunnel(&status.tunnel_id).await.unwrap();

        let result = store.get_tunnel(&status.tunnel_id).await;
        assert!(result.is_err());
    }
}
