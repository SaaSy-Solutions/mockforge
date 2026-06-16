//! CSRF protection middleware
//!
//! Provides Cross-Site Request Forgery protection for state-changing requests.
//!
//! For JWT-based APIs where tokens are sent via Authorization header (not cookies),
//! CSRF is less of a concern. However, this middleware provides additional protection
//! by validating Origin/Referer headers on state-changing requests.
//!
//! Configuration:
//! - `ALLOWED_ORIGINS`: Comma-separated list of allowed origins (default: localhost and app domain)
//! - `CSRF_ENABLED`: Set to "false" to disable CSRF protection (default: "true")

use axum::{
    extract::Request,
    http::{HeaderMap, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::OnceLock;

/// Default allowed origins for development
const DEFAULT_ALLOWED_ORIGINS: &[&str] = &[
    "http://localhost:3000",
    "http://localhost:5173",
    "http://127.0.0.1:3000",
    "http://127.0.0.1:5173",
    "https://app.mockforge.dev",
    "https://mockforge.dev",
];

/// Cached allowed origins
static ALLOWED_ORIGINS: OnceLock<Vec<String>> = OnceLock::new();

/// Check if CSRF protection is enabled
fn is_csrf_enabled() -> bool {
    std::env::var("CSRF_ENABLED")
        .map(|v| v.to_lowercase() != "false")
        .unwrap_or(true)
}

/// Get the list of allowed origins
fn get_allowed_origins() -> &'static Vec<String> {
    ALLOWED_ORIGINS.get_or_init(|| {
        std::env::var("ALLOWED_ORIGINS")
            .map(|s| s.split(',').map(|o| o.trim().to_string()).collect())
            .unwrap_or_else(|_| DEFAULT_ALLOWED_ORIGINS.iter().map(|s| s.to_string()).collect())
    })
}

/// Check if an origin is allowed against a given allowlist.
///
/// The allowlist is taken as a parameter (rather than read from the global
/// `OnceLock`) so this is unit-testable without mutating process-wide env state.
///
/// Wildcard entries (`*.base`) are matched against the **parsed host** of the
/// origin URL, never by raw string suffix. A naive `strip_suffix(".mockforge.dev")`
/// would wrongly accept `https://evilmockforge.dev` and
/// `https://app.mockforge.dev.attacker.com` (#760), so we parse the origin,
/// extract its host, and require `host == base || host.ends_with(".{base}")`.
/// Origins that don't parse or have no host are rejected.
fn is_origin_allowed_with(origin: &str, allowed: &[String]) -> bool {
    // Check for exact match first (cheap, and preserves scheme/port semantics).
    if allowed.iter().any(|o| o == origin) {
        return true;
    }

    // Any wildcard handling requires the parsed host of the request origin.
    let Ok(parsed) = url::Url::parse(origin) else {
        return false;
    };
    let Some(host) = parsed.host_str() else {
        return false;
    };

    for allowed_origin in allowed {
        if let Some(base) = allowed_origin.strip_prefix("*.") {
            // Allow the apex domain and any genuine subdomain of it. The leading
            // dot in `".{base}"` is what blocks `evil{base}` style lookalike domains.
            if host == base || host.ends_with(&format!(".{base}")) {
                return true;
            }
        }
    }

    false
}

/// Check if an origin is allowed (uses the cached global allowlist).
fn is_origin_allowed(origin: &str) -> bool {
    is_origin_allowed_with(origin, get_allowed_origins())
}

/// Extract origin from request headers
fn extract_origin(headers: &HeaderMap) -> Option<String> {
    // Try Origin header first (more reliable for CORS requests)
    if let Some(origin) = headers.get("Origin") {
        if let Ok(value) = origin.to_str() {
            if !value.is_empty() && value != "null" {
                return Some(value.to_string());
            }
        }
    }

    // Fall back to Referer header (extract origin)
    if let Some(referer) = headers.get("Referer") {
        if let Ok(value) = referer.to_str() {
            // Simple origin extraction: scheme://host[:port]
            // Find the third "/" which marks the start of the path
            if let Some(scheme_end) = value.find("://") {
                let after_scheme = &value[scheme_end + 3..];
                if let Some(path_start) = after_scheme.find('/') {
                    // Origin is everything before the path
                    return Some(value[..scheme_end + 3 + path_start].to_string());
                } else {
                    // No path, entire value is the origin
                    return Some(value.to_string());
                }
            }
        }
    }

    None
}

/// Check if a method is state-changing (requires CSRF protection)
fn is_state_changing_method(method: &Method) -> bool {
    matches!(method, &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE)
}

/// CSRF protection middleware
///
/// Validates Origin/Referer headers on state-changing requests (POST, PUT, PATCH, DELETE).
/// Allows requests from allowed origins or requests without Origin (API clients).
pub async fn csrf_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Skip if CSRF is disabled
    if !is_csrf_enabled() {
        return Ok(next.run(request).await);
    }

    // Only check state-changing methods
    if !is_state_changing_method(request.method()) {
        return Ok(next.run(request).await);
    }

    // Skip CSRF check for API requests with Authorization header
    // These are typically from API clients, not browser-based CSRF attacks
    if headers.contains_key("Authorization") {
        return Ok(next.run(request).await);
    }

    // Extract and validate origin
    let origin = extract_origin(&headers);

    match origin {
        Some(ref o) if is_origin_allowed(o) => {
            // Origin is allowed
            Ok(next.run(request).await)
        }
        Some(ref o) => {
            // Origin is not allowed
            tracing::warn!(
                origin = %o,
                path = %request.uri().path(),
                "CSRF check failed: origin not allowed"
            );
            Err(csrf_error_response().into_response())
        }
        None => {
            // No origin header - could be an API client or direct request
            // For web forms, Origin/Referer should be present
            // Allow for now but log for monitoring
            tracing::debug!(
                path = %request.uri().path(),
                "Request without Origin/Referer header"
            );
            Ok(next.run(request).await)
        }
    }
}

/// Create a CSRF error response
fn csrf_error_response() -> impl IntoResponse {
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "error": {
                "code": "CSRF_VALIDATION_FAILED",
                "message": "Cross-site request forgery validation failed. Please try again."
            }
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_state_changing_method() {
        assert!(is_state_changing_method(&Method::POST));
        assert!(is_state_changing_method(&Method::PUT));
        assert!(is_state_changing_method(&Method::PATCH));
        assert!(is_state_changing_method(&Method::DELETE));

        assert!(!is_state_changing_method(&Method::GET));
        assert!(!is_state_changing_method(&Method::HEAD));
        assert!(!is_state_changing_method(&Method::OPTIONS));
    }

    #[test]
    fn test_is_origin_allowed_exact() {
        // Test against default origins
        assert!(is_origin_allowed("http://localhost:3000"));
        assert!(is_origin_allowed("https://app.mockforge.dev"));

        // Test non-allowed origin
        assert!(!is_origin_allowed("https://evil.com"));
        assert!(!is_origin_allowed("http://localhost:9999"));
    }

    #[test]
    fn test_is_origin_allowed_wildcard_accepts_apex_and_subdomains() {
        let allowed = vec!["*.mockforge.dev".to_string()];
        // Apex and genuine subdomains are accepted.
        assert!(is_origin_allowed_with("https://app.mockforge.dev", &allowed));
        assert!(is_origin_allowed_with("https://x.mockforge.dev", &allowed));
        assert!(is_origin_allowed_with("https://mockforge.dev", &allowed));
    }

    #[test]
    fn test_is_origin_allowed_wildcard_rejects_lookalikes() {
        let allowed = vec!["*.mockforge.dev".to_string()];
        // Suffix-match smuggling that the old strip_suffix logic let through (#760).
        assert!(!is_origin_allowed_with("https://evilmockforge.dev", &allowed));
        assert!(!is_origin_allowed_with("https://mockforge.dev.evil.com", &allowed));
        assert!(!is_origin_allowed_with("https://app.mockforge.dev.attacker.com", &allowed));
    }

    #[test]
    fn test_is_origin_allowed_wildcard_rejects_port_and_userinfo_smuggling() {
        let allowed = vec!["*.mockforge.dev".to_string()];
        // Userinfo-smuggled host: the real host is attacker.com, not mockforge.dev.
        assert!(!is_origin_allowed_with("https://app.mockforge.dev@attacker.com", &allowed));
        // A port on a look-alike host must still be rejected.
        assert!(!is_origin_allowed_with("https://evilmockforge.dev:443", &allowed));
        // A real subdomain with a port is still a real subdomain → allowed.
        assert!(is_origin_allowed_with("https://app.mockforge.dev:8443", &allowed));
    }

    #[test]
    fn test_is_origin_allowed_with_exact_match() {
        let allowed = vec!["https://app.mockforge.dev".to_string()];
        assert!(is_origin_allowed_with("https://app.mockforge.dev", &allowed));
        assert!(!is_origin_allowed_with("https://evil.com", &allowed));
    }

    #[test]
    fn test_is_origin_allowed_rejects_unparsable() {
        let allowed = vec!["*.mockforge.dev".to_string()];
        assert!(!is_origin_allowed_with("not-a-url", &allowed));
        assert!(!is_origin_allowed_with("", &allowed));
    }

    #[test]
    fn test_extract_origin_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert("Origin", "https://app.mockforge.dev".parse().unwrap());

        let origin = extract_origin(&headers);
        assert_eq!(origin, Some("https://app.mockforge.dev".to_string()));
    }

    #[test]
    fn test_extract_origin_from_referer() {
        let mut headers = HeaderMap::new();
        headers.insert("Referer", "https://app.mockforge.dev/some/path".parse().unwrap());

        let origin = extract_origin(&headers);
        assert_eq!(origin, Some("https://app.mockforge.dev".to_string()));
    }

    #[test]
    fn test_extract_origin_prefers_origin_header() {
        let mut headers = HeaderMap::new();
        headers.insert("Origin", "https://origin.example.com".parse().unwrap());
        headers.insert("Referer", "https://referer.example.com/path".parse().unwrap());

        let origin = extract_origin(&headers);
        assert_eq!(origin, Some("https://origin.example.com".to_string()));
    }

    #[test]
    fn test_extract_origin_empty() {
        let headers = HeaderMap::new();
        let origin = extract_origin(&headers);
        assert!(origin.is_none());
    }

    #[test]
    fn test_extract_origin_null() {
        let mut headers = HeaderMap::new();
        headers.insert("Origin", "null".parse().unwrap());

        let origin = extract_origin(&headers);
        assert!(origin.is_none());
    }
}
