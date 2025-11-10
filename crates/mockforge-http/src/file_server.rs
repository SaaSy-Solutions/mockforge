//! File serving for MockForge generated files
//!
//! This module provides HTTP endpoints to serve generated mock files
//! from the mock-files directory.

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use std::path::PathBuf;
use tracing::{error, warn};

/// Serve a generated file from the mock-files directory
pub async fn serve_mock_file(
    axum::extract::Path(file_path): axum::extract::Path<String>,
) -> Result<Response, StatusCode> {
    // Security: Prevent path traversal
    if file_path.contains("..") || file_path.contains("//") {
        warn!("Path traversal attempt detected: {}", file_path);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Parse the file path: {route_id}/{filename} or {route_id}/{subdir}/{filename}
    let parts: Vec<&str> = file_path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        warn!("Invalid file path format: {}", file_path);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Get base directory from environment or use default
    let base_dir =
        std::env::var("MOCKFORGE_MOCK_FILES_DIR").unwrap_or_else(|_| "mock-files".to_string());

    // Reconstruct the full path preserving subdirectories
    let full_file_path = PathBuf::from(&base_dir).join(&file_path);

    // Check if file exists
    if !full_file_path.exists() {
        warn!("File not found: {:?}", full_file_path);
        return Err(StatusCode::NOT_FOUND);
    }

    // Read file content
    let content = match tokio::fs::read(&full_file_path).await {
        Ok(content) => content,
        Err(e) => {
            error!("Failed to read file {:?}: {}", full_file_path, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Get filename from path for Content-Disposition header
    let filename = full_file_path.file_name().and_then(|n| n.to_str()).unwrap_or("file");

    // Determine content type from file extension
    let content_type = full_file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| match ext.to_lowercase().as_str() {
            "pdf" => "application/pdf",
            "csv" => "text/csv",
            "json" => "application/json",
            "xml" => "application/xml",
            "txt" => "text/plain",
            _ => "application/octet-stream",
        })
        .unwrap_or("application/octet-stream");

    // Build response with appropriate headers
    let headers = [
        (header::CONTENT_TYPE, content_type),
        (header::CONTENT_DISPOSITION, &format!("attachment; filename=\"{}\"", filename)),
    ];

    Ok((StatusCode::OK, headers, content).into_response())
}

/// Create router for file serving endpoints
pub fn file_serving_router() -> axum::Router {
    use axum::routing::get;

    axum::Router::new().route("/mock-files/{*path}", get(serve_mock_file))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_serve_mock_file_path_traversal() {
        // Test path traversal protection
        use axum::extract::Path;
        let result = serve_mock_file(Path("../etc/passwd".to_string())).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_serve_mock_file_invalid_format() {
        // Test invalid path format (empty string results in empty parts)
        use axum::extract::Path;
        let result = serve_mock_file(Path("".to_string())).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }
}
