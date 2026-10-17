//! Fail-closed CORS policy for the HTTP mock server (#1017).
//!
//! The origin policy is built from `HttpCorsConfig::allowed_origins`:
//!
//! * no entries, or no CORS config at all: no cross-origin access;
//! * `"*"`: any origin, and credentials are always disabled;
//! * `"localhost"`: explicit dev mode, any `http(s)` origin on `localhost`,
//!   `127.0.0.1` or `[::1]` (any port);
//! * `scheme://host[:port]`: that exact origin (several are allowed);
//! * `"null"`: the opaque `null` origin, only when listed explicitly.
//!
//! One malformed entry rejects the whole list (deny-all plus an error log)
//! instead of widening it, so a typo can never turn into a wildcard.

use http::{HeaderValue, Method};
use mockforge_core::config::HttpCorsConfig;
use std::collections::HashSet;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

/// Keyword enabling the localhost-only dev mode in `allowed_origins`.
pub const LOCALHOST_KEYWORD: &str = "localhost";

/// Which origins may read cross-origin responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginPolicy {
    /// No cross-origin access.
    Deny,
    /// Any origin (`*`). Never combined with credentials.
    Wildcard,
    /// Exact origins, plus every localhost origin when `localhost` is set.
    List {
        /// Normalized exact origins (lowercase scheme and host, no trailing slash).
        exact: HashSet<String>,
        /// Allow `http(s)://localhost|127.0.0.1|[::1][:port]`.
        localhost: bool,
    },
}

/// Why an `allowed_origins` entry was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid CORS origin {entry:?}: {reason}")]
pub struct InvalidOrigin {
    /// The offending entry.
    pub entry: String,
    /// Why it was rejected.
    pub reason: &'static str,
}

/// Normalize a configured origin to `scheme://host[:port]`.
pub fn normalize_origin(entry: &str) -> Result<String, InvalidOrigin> {
    let bad = |reason| InvalidOrigin {
        entry: entry.to_string(),
        reason,
    };
    let trimmed = entry.trim();
    if trimmed.eq_ignore_ascii_case("null") {
        return Ok("null".to_string());
    }
    let without_slash = trimmed.strip_suffix('/').unwrap_or(trimmed);
    let (scheme, rest) = without_slash.split_once("://").ok_or_else(|| bad("missing scheme"))?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(bad("scheme must be http or https"));
    }
    if rest.is_empty() {
        return Err(bad("missing host"));
    }
    if rest.contains(['/', '?', '#', '@', '*', ' ', ',']) {
        return Err(bad("origin must be scheme://host[:port] with no path or wildcard"));
    }
    let normalized = format!("{scheme}://{}", rest.to_ascii_lowercase());
    let uri: http::Uri = normalized.parse().map_err(|_| bad("not a valid URL"))?;
    if uri.host().is_none_or(str::is_empty) {
        return Err(bad("missing host"));
    }
    HeaderValue::from_str(&normalized).map_err(|_| bad("not a valid header value"))?;
    Ok(normalized)
}

/// Build the origin policy from configured entries. Fails on any bad entry.
pub fn origin_policy(entries: &[String]) -> Result<OriginPolicy, InvalidOrigin> {
    let mut exact = HashSet::new();
    let mut localhost = false;
    let mut wildcard = false;
    for entry in entries {
        let trimmed = entry.trim();
        if trimmed == "*" {
            wildcard = true;
        } else if trimmed.eq_ignore_ascii_case(LOCALHOST_KEYWORD) {
            localhost = true;
        } else {
            exact.insert(normalize_origin(trimmed)?);
        }
    }
    if wildcard {
        return Ok(OriginPolicy::Wildcard);
    }
    if exact.is_empty() && !localhost {
        return Ok(OriginPolicy::Deny);
    }
    Ok(OriginPolicy::List { exact, localhost })
}

/// True for `http(s)://localhost`, `127.0.0.1` or `[::1]`, with any port.
pub fn is_localhost_origin(origin: &str) -> bool {
    let Some((scheme, rest)) = origin.split_once("://") else {
        return false;
    };
    if scheme != "http" && scheme != "https" {
        return false;
    }
    let host = if let Some(v6) = rest.strip_prefix('[') {
        match v6.split_once(']') {
            Some((h, tail)) if tail.is_empty() || is_port(tail) => return h == "::1",
            _ => return false,
        }
    } else {
        match rest.split_once(':') {
            Some((h, port)) if is_port(&format!(":{port}")) => h,
            Some(_) => return false,
            None => rest,
        }
    };
    host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1"
}

fn is_port(tail: &str) -> bool {
    tail.strip_prefix(':')
        .is_some_and(|p| !p.is_empty() && p.len() <= 5 && p.bytes().all(|b| b.is_ascii_digit()))
}

impl OriginPolicy {
    /// Whether a request `Origin` header value is allowed.
    pub fn allows(&self, origin: &str) -> bool {
        match self {
            OriginPolicy::Deny => false,
            OriginPolicy::Wildcard => true,
            OriginPolicy::List { exact, localhost } => {
                exact.contains(&origin.to_ascii_lowercase())
                    || (*localhost && is_localhost_origin(origin))
            }
        }
    }

    fn allow_origin(&self) -> Option<AllowOrigin> {
        match self {
            OriginPolicy::Deny => None,
            OriginPolicy::Wildcard => Some(AllowOrigin::from(Any)),
            OriginPolicy::List { .. } => {
                let policy = self.clone();
                Some(AllowOrigin::predicate(move |origin: &HeaderValue, _| {
                    origin.to_str().is_ok_and(|o| policy.allows(o))
                }))
            }
        }
    }
}

/// Build the CORS layer for the HTTP mock server. `None` means no CORS config:
/// no cross-origin access.
pub fn build_cors_layer(config: Option<&HttpCorsConfig>) -> CorsLayer {
    let Some(config) = config else {
        tracing::info!("No CORS config: cross-origin requests are not allowed");
        return CorsLayer::new();
    };

    let policy = origin_policy(&config.allowed_origins).unwrap_or_else(|e| {
        tracing::error!("{e}; CORS denies all cross-origin requests until it is fixed");
        OriginPolicy::Deny
    });

    let methods: Vec<Method> =
        config.allowed_methods.iter().filter_map(|m| m.parse().ok()).collect();
    let methods = if methods.is_empty() {
        vec![
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ]
    } else {
        methods
    };
    let headers: Vec<http::HeaderName> =
        config.allowed_headers.iter().filter_map(|h| h.parse().ok()).collect();
    let headers = if headers.is_empty() {
        vec![http::header::CONTENT_TYPE, http::header::AUTHORIZATION]
    } else {
        headers
    };

    // Credentials are never combined with a wildcard origin.
    let credentials = config.allow_credentials && !matches!(policy, OriginPolicy::Wildcard);
    if config.allow_credentials && !credentials {
        tracing::warn!("CORS allow_credentials ignored: not allowed with a wildcard origin");
    }

    let mut layer = CorsLayer::new()
        .allow_methods(methods)
        .allow_headers(headers)
        .allow_credentials(credentials);
    if let Some(allow) = policy.allow_origin() {
        layer = layer.allow_origin(allow);
    }
    tracing::info!("CORS origin policy: {policy:?} (credentials: {credentials})");
    layer
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    fn cfg(origins: &[&str], credentials: bool) -> HttpCorsConfig {
        HttpCorsConfig {
            enabled: true,
            allowed_origins: origins.iter().map(|s| s.to_string()).collect(),
            allowed_methods: vec![],
            allowed_headers: vec![],
            allow_credentials: credentials,
        }
    }

    fn app(config: Option<&HttpCorsConfig>) -> Router {
        Router::new()
            .route("/r", get(|| async { "ok" }))
            .layer(build_cors_layer(config))
    }

    async fn get_with_origin(
        config: Option<&HttpCorsConfig>,
        origin: &str,
    ) -> (StatusCode, Option<String>, Option<String>) {
        let res = app(config)
            .oneshot(
                Request::builder()
                    .uri("/r")
                    .header(header::ORIGIN, origin)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let h = |n| res.headers().get(n).map(|v: &HeaderValue| v.to_str().unwrap().to_string());
        (
            res.status(),
            h(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            h(header::ACCESS_CONTROL_ALLOW_CREDENTIALS),
        )
    }

    async fn preflight(
        config: Option<&HttpCorsConfig>,
        origin: &str,
    ) -> (Option<String>, Option<String>) {
        let res = app(config)
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/r")
                    .header(header::ORIGIN, origin)
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                    .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let h = |n| res.headers().get(n).map(|v: &HeaderValue| v.to_str().unwrap().to_string());
        (
            h(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            h(header::ACCESS_CONTROL_ALLOW_CREDENTIALS),
        )
    }

    #[tokio::test]
    async fn configured_origin_is_allowed() {
        let c = cfg(&["https://app.example.com", "https://admin.example.com/"], false);
        for o in ["https://app.example.com", "https://admin.example.com"] {
            let (status, acao, _) = get_with_origin(Some(&c), o).await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(acao.as_deref(), Some(o));
        }
    }

    #[tokio::test]
    async fn unknown_origin_is_rejected() {
        let c = cfg(&["https://app.example.com"], true);
        for o in [
            "https://evil.example.com",
            "https://app.example.com.evil.com",
            "http://app.example.com",
            "https://app.example.com:8443",
        ] {
            let (_, acao, _) = get_with_origin(Some(&c), o).await;
            assert_eq!(acao, None, "{o}");
            assert_eq!(preflight(Some(&c), o).await.0, None, "{o}");
        }
    }

    #[tokio::test]
    async fn null_origin_is_rejected_unless_listed() {
        let c = cfg(&["https://app.example.com"], false);
        assert_eq!(get_with_origin(Some(&c), "null").await.1, None);
        let c = cfg(&["null"], false);
        assert_eq!(get_with_origin(Some(&c), "null").await.1.as_deref(), Some("null"));
    }

    #[tokio::test]
    async fn no_config_and_empty_origins_deny_cross_origin() {
        assert_eq!(get_with_origin(None, "https://a.example").await.1, None);
        assert_eq!(preflight(None, "https://a.example").await.0, None);
        let c = cfg(&[], false);
        assert_eq!(get_with_origin(Some(&c), "https://a.example").await.1, None);
        assert_eq!(get_with_origin(Some(&c), "http://localhost:5173").await.1, None);
    }

    #[tokio::test]
    async fn preflight_with_credentials_echoes_exact_origin() {
        let c = cfg(&["https://app.example.com"], true);
        let (acao, creds) = preflight(Some(&c), "https://app.example.com").await;
        assert_eq!(acao.as_deref(), Some("https://app.example.com"));
        assert_eq!(creds.as_deref(), Some("true"));
    }

    #[tokio::test]
    async fn wildcard_is_never_combined_with_credentials() {
        for origins in [&["*"][..], &["*", "https://app.example.com"][..]] {
            let c = cfg(origins, true);
            let (_, acao, creds) = get_with_origin(Some(&c), "https://evil.example").await;
            assert_eq!(acao.as_deref(), Some("*"));
            assert_eq!(creds, None);
            let (acao, creds) = preflight(Some(&c), "https://evil.example").await;
            assert_eq!(acao.as_deref(), Some("*"));
            assert_eq!(creds, None);
        }
    }

    #[tokio::test]
    async fn malformed_entry_fails_closed_not_open() {
        for bad in [
            "app.example.com",
            "https://app.example.com/path",
            "ftp://x",
            "https://*.x.com",
        ] {
            let c = cfg(&["https://good.example", bad], false);
            assert!(origin_policy(&c.allowed_origins).is_err(), "{bad}");
            for o in ["https://good.example", "https://evil.example"] {
                assert_eq!(get_with_origin(Some(&c), o).await.1, None, "{bad} / {o}");
            }
        }
    }

    #[tokio::test]
    async fn localhost_dev_mode_is_explicit_and_narrow() {
        let c = cfg(&["localhost"], false);
        for o in [
            "http://localhost:5173",
            "http://127.0.0.1:3000",
            "http://[::1]:8080",
            "https://localhost",
        ] {
            assert_eq!(get_with_origin(Some(&c), o).await.1.as_deref(), Some(o), "{o}");
        }
        for o in [
            "http://localhost.evil.com",
            "http://evil.com:localhost",
            "http://127.0.0.1.nip.io",
            "http://localhost:5173@evil.com",
            "null",
        ] {
            assert_eq!(get_with_origin(Some(&c), o).await.1, None, "{o}");
        }
    }

    #[tokio::test]
    async fn existing_deployments_keep_working() {
        // Demo and hosted mocks set MOCKFORGE_CORS_ALLOWED_ORIGINS="*" explicitly:
        // a public, credential-free API readable from any page, as before.
        let hosted = cfg(&["*"], false);
        let (status, acao, creds) = get_with_origin(Some(&hosted), "https://customer.app").await;
        assert_eq!((status, acao.as_deref(), creds), (StatusCode::OK, Some("*"), None));
        // A deployment that lists its front-ends keeps them.
        let listed = cfg(&["https://app.mockforge.dev", "https://mockforge.dev"], false);
        let (_, acao, _) = get_with_origin(Some(&listed), "https://mockforge.dev").await;
        assert_eq!(acao.as_deref(), Some("https://mockforge.dev"));
        // Same-origin requests (no Origin header) are unaffected by a deny policy.
        let res = app(None)
            .oneshot(Request::builder().uri("/r").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[test]
    fn normalize_origin_cases() {
        assert_eq!(
            normalize_origin("HTTPS://App.Example.com/").as_deref(),
            Ok("https://app.example.com")
        );
        assert_eq!(
            normalize_origin("http://localhost:3000").as_deref(),
            Ok("http://localhost:3000")
        );
        assert!(normalize_origin("https://").is_err());
        assert!(normalize_origin("https://a.com?x=1").is_err());
        assert!(normalize_origin("https://user@a.com").is_err());
    }
}
