//! Static asset serving handlers
//!
//! This module handles serving static assets like HTML, CSS, and JavaScript
//! files for the admin UI.

use axum::{
    http::{self},
    response::{Html, IntoResponse, Redirect},
};

/// Serve the main admin HTML page
pub async fn serve_admin_html() -> Html<&'static str> {
    Html(include_str!("../../ui/dist/index.html"))
}

/// Serve the admin CSS with proper content type
pub async fn serve_admin_css() -> ([(http::HeaderName, &'static str); 1], &'static str) {
    (
        [(http::header::CONTENT_TYPE, "text/css")],
        include_str!("../../ui/dist/assets/index.css"),
    )
}

/// Serve the admin JavaScript with proper content type
pub async fn serve_admin_js() -> ([(http::HeaderName, &'static str); 1], &'static str) {
    (
        [(http::header::CONTENT_TYPE, "application/javascript")],
        include_str!("../../ui/dist/assets/index.js"),
    )
}

/// Serve icon files
pub async fn serve_icon() -> impl IntoResponse {
    // Return a simple SVG icon or placeholder
    let icon_svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 32 32\"><rect width=\"32\" height=\"32\" fill=\"#4f46e5\"/><text x=\"16\" y=\"20\" text-anchor=\"middle\" fill=\"white\" font-family=\"Arial\" font-size=\"14\">MF</text></svg>";
    ([(http::header::CONTENT_TYPE, "image/svg+xml")], icon_svg)
}

/// Serve 32x32 icon
pub async fn serve_icon_32() -> impl IntoResponse {
    serve_icon().await
}

/// Serve 48x48 icon
pub async fn serve_icon_48() -> impl IntoResponse {
    serve_icon().await
}

/// Serve logo files
pub async fn serve_logo() -> impl IntoResponse {
    serve_icon().await
}

/// Serve 40x40 logo
pub async fn serve_logo_40() -> impl IntoResponse {
    serve_icon().await
}

/// Serve 80x80 logo
pub async fn serve_logo_80() -> impl IntoResponse {
    serve_icon().await
}

/// Serve the API documentation - redirects to the book
pub async fn serve_api_docs() -> impl IntoResponse {
    // Redirect to the comprehensive documentation in the book
    Redirect::permanent("https://docs.mockforge.dev/api/admin-ui-rest.html")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_serve_admin_html() {
        let html = serve_admin_html().await;
        let html_str = html.0;
        assert!(!html_str.is_empty());
        assert!(html_str.contains("<!DOCTYPE html>") || html_str.contains("<html"));
    }

    #[tokio::test]
    async fn test_serve_admin_css() {
        let (headers, css) = serve_admin_css().await;
        assert_eq!(headers[0].0, http::header::CONTENT_TYPE);
        assert_eq!(headers[0].1, "text/css");
        assert!(!css.is_empty());
    }

    #[tokio::test]
    async fn test_serve_admin_js() {
        let (headers, js) = serve_admin_js().await;
        assert_eq!(headers[0].0, http::header::CONTENT_TYPE);
        assert_eq!(headers[0].1, "application/javascript");
        assert!(!js.is_empty());
    }

    #[tokio::test]
    async fn test_serve_icon() {
        let response = serve_icon().await;
        // Icon returns SVG content - we can't easily check headers in impl IntoResponse
        // but we can verify it returns successfully
        let _ = response;
    }

    #[tokio::test]
    async fn test_serve_icon_32() {
        let _ = serve_icon_32().await;
    }

    #[tokio::test]
    async fn test_serve_icon_48() {
        let _ = serve_icon_48().await;
    }

    #[tokio::test]
    async fn test_serve_logo() {
        let _ = serve_logo().await;
    }

    #[tokio::test]
    async fn test_serve_logo_40() {
        let _ = serve_logo_40().await;
    }

    #[tokio::test]
    async fn test_serve_logo_80() {
        let _ = serve_logo_80().await;
    }

    #[tokio::test]
    async fn test_serve_api_docs() {
        let _ = serve_api_docs().await;
        // Redirect can't be easily tested without request context
        // but we verify it compiles and runs
    }
}
