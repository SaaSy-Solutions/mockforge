//! Security headers middleware
//!
//! Adds security headers to all HTTP responses:
//! - Content-Security-Policy (CSP)
//! - Strict-Transport-Security (HSTS)
//! - X-Frame-Options
//! - X-Content-Type-Options
//! - Referrer-Policy
//! - Permissions-Policy

use axum::{
    extract::Request,
    http::{HeaderValue, Response},
    middleware::Next,
};

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
    // Allow self, and common CDNs for assets
    // In production, this should be more restrictive
    let csp = if std::env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "development".to_string()) == "production"
    {
        // Production CSP - more restrictive
        "default-src 'self'; \
         script-src 'self' 'unsafe-inline' 'unsafe-eval' https://js.sentry-cdn.com; \
         style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; \
         font-src 'self' https://fonts.gstatic.com; \
         img-src 'self' data: https:; \
         connect-src 'self' https://*.sentry.io https://api.postmarkapp.com https://api.brevo.com; \
         frame-ancestors 'none'; \
         base-uri 'self'; \
         form-action 'self'"
    } else {
        // Development CSP - more permissive for development tools
        "default-src 'self' 'unsafe-inline' 'unsafe-eval' data: blob:; \
         connect-src 'self' http://localhost:* ws://localhost:* wss://localhost:* https://*.sentry.io; \
         img-src 'self' data: https:; \
         font-src 'self' data: https:; \
         frame-ancestors 'self'"
    };
    headers.insert(
        "content-security-policy",
        HeaderValue::from_str(csp).unwrap_or_default(),
    );

    // Strict-Transport-Security (HSTS)
    // Only in production and when using HTTPS
    if std::env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "development".to_string()) == "production"
    {
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

    // Create layers for each header
    let csp_layer = SetResponseHeader::overriding(
        "content-security-policy",
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline';",
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
