//! Static asset optimization middleware
//!
//! Adds caching headers, compression hints, and optimization for static assets
//! Also handles CDN URL rewriting when CDN is configured

use axum::{
    extract::{Request, State},
    http::{HeaderValue, Response},
    middleware::Next,
};
use crate::AppState;

/// Static asset optimization middleware
/// Adds caching headers, compression hints, and CDN URL rewriting for static assets
pub async fn static_assets_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    let path = request.uri().path();

    // Check if this is a static asset
    let is_static_asset = path.starts_with("/assets/")
        || path.ends_with(".js")
        || path.ends_with(".css")
        || path.ends_with(".png")
        || path.ends_with(".jpg")
        || path.ends_with(".jpeg")
        || path.ends_with(".svg")
        || path.ends_with(".webp")
        || path.ends_with(".woff")
        || path.ends_with(".woff2")
        || path.ends_with(".ttf")
        || path.ends_with(".eot")
        || path.ends_with(".ico");

    if is_static_asset {
        // Cache-Control: Long cache for hashed assets, shorter for unhashed
        // Check if path looks like a hashed asset (contains long alphanumeric strings)
        let is_hashed = path.split('/').any(|segment| {
            segment.len() > 20 && segment.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        });

        let cache_control = if is_hashed {
            // Hashed assets can be cached forever (browser will check hash)
            "public, max-age=31536000, immutable"
        } else {
            // Unhashed assets need shorter cache with revalidation
            "public, max-age=3600, must-revalidate"
        };

        headers.insert(
            "Cache-Control",
            HeaderValue::from_str(cache_control).unwrap_or_default(),
        );

        // ETag support (for conditional requests)
        // Note: In production, you might want to generate ETags based on file content
        // For now, we'll rely on Cache-Control

        // Compression hints
        if path.ends_with(".js") || path.ends_with(".css") || path.ends_with(".json") {
            headers.insert(
                "Vary",
                HeaderValue::from_static("Accept-Encoding"),
            );
        }

        // Image optimization hints
        if path.ends_with(".png") || path.ends_with(".jpg") || path.ends_with(".jpeg") {
            // Suggest WebP if supported (handled by CDN or client)
            headers.insert(
                "Accept",
                HeaderValue::from_static("image/webp,image/*,*/*"),
            );
        }

        // Preload hints for critical assets (use CDN URL if configured)
        if path.ends_with("/index.js") || path.ends_with("/index.css") {
            let cdn_css_url = state.config.get_cdn_url("/assets/index.css");
            let cdn_js_url = state.config.get_cdn_url("/assets/index.js");
            let link_header = format!(
                "<{}>; rel=preload; as=style, <{}>; rel=preload; as=script",
                cdn_css_url, cdn_js_url
            );
            if let Ok(header_value) = HeaderValue::from_str(&link_header) {
                headers.insert("Link", header_value);
            }
        }

        // Add CDN header to indicate if CDN is being used
        if state.config.cdn_base_url.is_some()
            || state.config.cdn_assets_url.is_some()
            || state.config.cdn_images_url.is_some() {
            headers.insert(
                "X-CDN-Enabled",
                HeaderValue::from_static("true"),
            );
        }
    }

    response
}
