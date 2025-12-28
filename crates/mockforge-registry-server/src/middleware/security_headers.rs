//! Security headers middleware
//!
//! Adds security headers to all HTTP responses:
//! - Content-Security-Policy (CSP)
//! - Strict-Transport-Security (HSTS)
//! - X-Frame-Options
//! - X-Content-Type-Options
//! - Referrer-Policy
//! - Permissions-Policy
//!
//! # Security Note
//!
//! The CSP policy defaults to strict/production mode for security.
//! To enable development mode (permissive CSP), you must explicitly set
//! `ENVIRONMENT=development`. This fail-safe approach ensures that
//! production deployments are secure by default.

use axum::{
    extract::Request,
    http::{HeaderValue, Response},
    middleware::Next,
};
use std::sync::OnceLock;
use tracing::warn;

/// Cache the environment check result to avoid repeated env var lookups
static IS_DEVELOPMENT: OnceLock<bool> = OnceLock::new();

/// Check if running in explicit development mode.
/// Returns true ONLY if ENVIRONMENT is explicitly set to "development".
/// This is a fail-safe design: unknown or missing values default to production (strict) mode.
fn is_development_mode() -> bool {
    *IS_DEVELOPMENT.get_or_init(|| {
        let is_dev = std::env::var("ENVIRONMENT")
            .map(|v| v.to_lowercase() == "development")
            .unwrap_or(false);

        if is_dev {
            warn!(
                "Running with ENVIRONMENT=development - using permissive CSP. \
                 Set ENVIRONMENT=production for strict security headers."
            );
        }

        is_dev
    })
}

/// Security headers middleware
/// Adds security headers to all responses
pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Response<axum::body::Body> {
    let mut response = next.run(request).await;

    // Get headers from response
    let headers = response.headers_mut();

    // Content-Security-Policy
    // API servers should have restrictive CSP since they don't serve interactive HTML
    // NOTE: Defaults to strict (production) CSP unless ENVIRONMENT=development is explicitly set
    let csp = if is_development_mode() {
        // Development CSP - more permissive for development tools and hot reload
        // Only used when ENVIRONMENT=development is explicitly set
        "default-src 'self'; \
         script-src 'self' 'unsafe-inline' 'unsafe-eval'; \
         style-src 'self' 'unsafe-inline'; \
         connect-src 'self' http://localhost:* ws://localhost:* wss://localhost:* https://*.sentry.io; \
         img-src 'self' data: https:; \
         font-src 'self' data: https:; \
         frame-ancestors 'self'"
    } else {
        // Production CSP - strict policy for API server (DEFAULT)
        // No inline scripts/styles needed for JSON API responses
        "default-src 'none'; \
         script-src 'self' https://js.sentry-cdn.com; \
         style-src 'self' https://fonts.googleapis.com; \
         font-src 'self' https://fonts.gstatic.com; \
         img-src 'self' data: https:; \
         connect-src 'self' https://*.sentry.io https://api.postmarkapp.com https://api.brevo.com; \
         frame-ancestors 'none'; \
         base-uri 'self'; \
         form-action 'self'; \
         upgrade-insecure-requests"
    };
    headers.insert(
        "content-security-policy",
        HeaderValue::from_str(csp).unwrap_or_default(),
    );

    // Strict-Transport-Security (HSTS)
    // Enabled by default (production mode) unless ENVIRONMENT=development
    if !is_development_mode() {
        // HSTS: max-age=31536000 (1 year), includeSubDomains, preload
        headers.insert(
            "strict-transport-security",
            HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
        );
    }

    // X-Frame-Options: Prevent clickjacking
    headers.insert(
        "x-frame-options",
        HeaderValue::from_static("DENY"),
    );

    // X-Content-Type-Options: Prevent MIME type sniffing
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );

    // Referrer-Policy: Control referrer information
    headers.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Permissions-Policy: Control browser features
    // Restrict features that could be used for tracking or security issues
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static(
            "geolocation=(), \
             microphone=(), \
             camera=(), \
             payment=(), \
             usb=(), \
             magnetometer=(), \
             gyroscope=(), \
             accelerometer=()",
        ),
    );

    // X-XSS-Protection: Legacy header (modern browsers ignore, but doesn't hurt)
    headers.insert(
        "x-xss-protection",
        HeaderValue::from_static("1; mode=block"),
    );

    response
}

/// Create a SetResponseHeaderLayer for security headers
/// This is an alternative approach using tower-http layers
pub fn security_headers_layer() -> impl tower::Layer<axum::Router> + Clone {
    use tower::Layer;
    use tower_http::set_header::SetResponseHeader;

    // Create layers for each header (strict CSP for API server)
    let csp_layer = SetResponseHeader::overriding(
        "content-security-policy",
        HeaderValue::from_static(
            "default-src 'none'; script-src 'self'; style-src 'self'; frame-ancestors 'none'; base-uri 'self'",
        ),
    );

    let frame_options_layer = SetResponseHeader::overriding(
        "x-frame-options",
        HeaderValue::from_static("DENY"),
    );

    let content_type_layer = SetResponseHeader::overriding(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );

    let referrer_layer = SetResponseHeader::overriding(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Combine layers
    csp_layer
        .and_then(frame_options_layer)
        .and_then(content_type_layer)
        .and_then(referrer_layer)
}
