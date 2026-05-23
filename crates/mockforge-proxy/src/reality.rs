//! Reality-slider-driven mock/proxy switching middleware (#222).
//!
//! When `MOCKFORGE_PROXY_UPSTREAM` is set on the process, this middleware
//! probabilistically forwards a fraction of incoming requests to that URL
//! based on the active workspace's `reality_continuum_ratio`. The fraction
//! is per-request: ratio 0.0 = always-mock, 1.0 = always-proxy, 0.5 =
//! coin-flip per request.
//!
//! ## Design
//!
//! The middleware is a no-op when:
//!   - `MOCKFORGE_PROXY_UPSTREAM` is unset (e.g., local dev)
//!   - the request has no associated `UnifiedState` extension (set by the
//!     consistency middleware upstream of this one)
//!   - the resolved ratio is exactly 0.0
//!
//! When proxying, the middleware reconstructs the request against the
//! upstream base URL preserving method, path, query, headers, and body,
//! then streams the upstream response back to the caller. Any failure
//! falls through to the mock chain — the mock is the durable path; the
//! upstream is best-effort.
//!
//! Wiring: insert the layer between `consistency_middleware` (which
//! injects `UnifiedState`) and the route handlers. The dependency on
//! `UnifiedState` is read-only, so ordering relative to recording or
//! tracing layers doesn't matter.
//!
//! Moved from `mockforge_http::reality_proxy` under #555 phase 8 — the
//! file's only foreign dep (`mockforge_core::consistency::UnifiedState`)
//! already lived in this crate's dep graph via `mockforge-core`.
//! `mockforge_http::reality_proxy` is now a thin shim re-exporting from
//! here.

use axum::{
    body::{to_bytes, Body},
    extract::Request,
    http::{
        header::{CONTENT_TYPE, HOST},
        HeaderName, HeaderValue, Method, StatusCode, Uri,
    },
    middleware::Next,
    response::Response,
};
use mockforge_core::consistency::UnifiedState;
use reqwest::Method as ReqwestMethod;
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

/// Cheap-to-clone handle holding the upstream base URL and a shared
/// reqwest client. Constructed once at server startup; the layer closure
/// holds an `Arc<RealityProxyConfig>` so per-request work is just an
/// arc-clone.
#[derive(Clone)]
pub struct RealityProxyConfig {
    /// Base URL — protocol + host + (optional) port, no trailing slash.
    /// Path/query are taken from the incoming request.
    pub upstream_base: String,
    /// Shared HTTP client used for all upstream requests.
    pub client: reqwest::Client,
}

impl RealityProxyConfig {
    /// Construct from `MOCKFORGE_PROXY_UPSTREAM`. Returns None when the
    /// env var is missing or empty (no-op middleware) or when the HTTP
    /// client can't be built (logged as a warning).
    pub fn from_env() -> Option<Arc<Self>> {
        let base = std::env::var("MOCKFORGE_PROXY_UPSTREAM").ok()?;
        let trimmed = base.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            return None;
        }
        let client = match reqwest::Client::builder().timeout(Duration::from_secs(30)).build() {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "RealityProxy HTTP client init failed; middleware will no-op");
                return None;
            }
        };
        Some(Arc::new(Self {
            upstream_base: trimmed.to_string(),
            client,
        }))
    }
}

/// The middleware function. Reads `reality_continuum_ratio` from the
/// `UnifiedState` request extension, rolls a per-request RNG, and either
/// forwards to upstream or hands off to the next layer (mock chain).
pub async fn reality_proxy_middleware(
    config: Arc<RealityProxyConfig>,
    req: Request,
    next: Next,
) -> Response {
    let ratio = req
        .extensions()
        .get::<UnifiedState>()
        .map(|s| s.reality_continuum_ratio)
        .unwrap_or(0.0);

    // Fast path: no upstream desired for this request.
    if ratio <= 0.0 {
        return next.run(req).await;
    }

    let should_proxy = if ratio >= 1.0 {
        true
    } else {
        rand::random::<f64>() < ratio
    };

    if !should_proxy {
        return next.run(req).await;
    }

    match forward_to_upstream(&config, req).await {
        Ok(resp) => resp,
        Err(err) => {
            // We've already consumed the request body to forward it, so
            // we can't fall back to the mock chain. Surface 502 — the
            // alternative (silent retry / synthetic mock) would hide
            // real upstream incidents from operators.
            warn!(error = %err, "Reality proxy upstream request failed");
            let body = serde_json::to_vec(&serde_json::json!({
                "error": "reality_proxy_upstream_failed",
                "message": err.to_string(),
            }))
            .unwrap_or_default();
            let mut resp = Response::new(Body::from(body));
            *resp.status_mut() = StatusCode::BAD_GATEWAY;
            resp.headers_mut()
                .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            resp
        }
    }
}

async fn forward_to_upstream(
    config: &RealityProxyConfig,
    req: Request,
) -> Result<Response, ProxyError> {
    let (parts, body) = req.into_parts();
    // Cap at 16 MiB — same as Axum's default request size limit.
    // Anything larger and we'd be holding too much in memory for a
    // simple proxy hop; better to fail loudly than swap-thrash.
    const MAX_BODY: usize = 16 * 1024 * 1024;
    let body_bytes = to_bytes(body, MAX_BODY)
        .await
        .map_err(|e| ProxyError::ReadBody(e.to_string()))?;

    let upstream_uri = build_upstream_uri(&config.upstream_base, &parts.uri)?;
    let method = reqwest_method(&parts.method);
    let mut req_builder = config.client.request(method, &upstream_uri);

    // Copy headers, dropping hop-by-hop / Host so reqwest sets correct ones.
    for (name, value) in parts.headers.iter() {
        if is_hop_by_hop(name) {
            continue;
        }
        if name == HOST {
            continue;
        }
        req_builder = req_builder.header(name.as_str(), value);
    }

    if !body_bytes.is_empty() {
        req_builder = req_builder.body(body_bytes);
    }

    let upstream_resp = req_builder.send().await.map_err(ProxyError::Send)?;
    let status = upstream_resp.status();
    let headers = upstream_resp.headers().clone();
    let resp_bytes = upstream_resp.bytes().await.map_err(ProxyError::ReadResponse)?;

    let mut response = Response::builder().status(status.as_u16());
    {
        let response_headers = response.headers_mut().expect("Response builder must have headers");
        for (name, value) in headers.iter() {
            if is_hop_by_hop_str(name.as_str()) {
                continue;
            }
            if let Ok(hname) = HeaderName::from_bytes(name.as_str().as_bytes()) {
                if let Ok(hval) = HeaderValue::from_bytes(value.as_bytes()) {
                    response_headers.insert(hname, hval);
                }
            }
        }
        response_headers.insert(
            HeaderName::from_static("x-mockforge-source"),
            HeaderValue::from_static("upstream"),
        );
    }
    response
        .body(Body::from(resp_bytes))
        .map_err(|e| ProxyError::BuildResponse(e.to_string()))
}

fn build_upstream_uri(base: &str, original: &Uri) -> Result<String, ProxyError> {
    let path = original.path();
    let query = original.query().map(|q| format!("?{}", q)).unwrap_or_default();
    Ok(format!("{}{}{}", base, path, query))
}

fn reqwest_method(m: &Method) -> ReqwestMethod {
    ReqwestMethod::from_bytes(m.as_str().as_bytes()).unwrap_or(ReqwestMethod::GET)
}

fn is_hop_by_hop(name: &HeaderName) -> bool {
    is_hop_by_hop_str(name.as_str())
}

fn is_hop_by_hop_str(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
            | "content-length"
    )
}

#[derive(Debug, thiserror::Error)]
enum ProxyError {
    #[error("failed to read request body: {0}")]
    ReadBody(String),
    #[error("upstream request send failed: {0}")]
    Send(reqwest::Error),
    #[error("upstream response read failed: {0}")]
    ReadResponse(reqwest::Error),
    #[error("response build failed: {0}")]
    BuildResponse(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_disabled_when_unset() {
        std::env::remove_var("MOCKFORGE_PROXY_UPSTREAM");
        assert!(RealityProxyConfig::from_env().is_none());
    }

    #[test]
    fn from_env_disabled_when_blank() {
        std::env::set_var("MOCKFORGE_PROXY_UPSTREAM", "   ");
        assert!(RealityProxyConfig::from_env().is_none());
        std::env::remove_var("MOCKFORGE_PROXY_UPSTREAM");
    }

    #[test]
    fn from_env_strips_trailing_slash() {
        std::env::set_var("MOCKFORGE_PROXY_UPSTREAM", "https://api.example.com/");
        let cfg = RealityProxyConfig::from_env().expect("config");
        assert_eq!(cfg.upstream_base, "https://api.example.com");
        std::env::remove_var("MOCKFORGE_PROXY_UPSTREAM");
    }

    #[test]
    fn build_upstream_uri_preserves_path_and_query() {
        let base = "https://api.example.com";
        let uri: Uri = "/users/42?role=admin".parse().unwrap();
        let result = build_upstream_uri(base, &uri).unwrap();
        assert_eq!(result, "https://api.example.com/users/42?role=admin");
    }

    #[test]
    fn build_upstream_uri_no_query() {
        let base = "https://api.example.com";
        let uri: Uri = "/health".parse().unwrap();
        let result = build_upstream_uri(base, &uri).unwrap();
        assert_eq!(result, "https://api.example.com/health");
    }

    #[test]
    fn hop_by_hop_headers_are_filtered() {
        assert!(is_hop_by_hop_str("Connection"));
        assert!(is_hop_by_hop_str("transfer-encoding"));
        assert!(is_hop_by_hop_str("UPGRADE"));
        assert!(!is_hop_by_hop_str("authorization"));
        assert!(!is_hop_by_hop_str("x-custom-header"));
    }
}
