//! Static asset serving handlers
//!
//! This module handles serving static assets like HTML, CSS, and JavaScript
//! files for the admin UI.

use axum::{
    http::{self, StatusCode},
    response::{Html, IntoResponse},
};

/// Serve the main admin HTML page
pub async fn serve_admin_html() -> Html<&'static str> {
    Html(include_str!("../../ui/dist/index.html"))
}

/// Serve the admin CSS with proper content type
pub async fn serve_admin_css() -> ([(http::HeaderName, &'static str); 1], &'static str) {
    ([(http::header::CONTENT_TYPE, "text/css")], include_str!("../../ui/dist/assets/index.css"))
}

/// Serve the admin JavaScript with proper content type
pub async fn serve_admin_js() -> ([(http::HeaderName, &'static str); 1], &'static str) {
    ([(http::header::CONTENT_TYPE, "application/javascript")], include_str!("../../ui/dist/assets/index.js"))
}

/// Serve icon files
pub async fn serve_icon() -> impl IntoResponse {
    // Return a simple SVG icon or placeholder
    let icon_svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><rect width="32" height="32" fill="#4f46e5"/><text x="16" y="20" text-anchor="middle" fill="white" font-family="Arial" font-size="14">MF</text></svg>"#;
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
