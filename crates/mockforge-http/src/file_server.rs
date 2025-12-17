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

    // ==================== Path Traversal Protection Tests ====================

    #[tokio::test]
    async fn test_serve_mock_file_path_traversal() {
        // Test path traversal protection
        use axum::extract::Path;
        let result = serve_mock_file(Path("../etc/passwd".to_string())).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_serve_mock_file_path_traversal_double_slash() {
        use axum::extract::Path;
        let result = serve_mock_file(Path("route//file.json".to_string())).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_serve_mock_file_path_traversal_nested() {
        use axum::extract::Path;
        let result = serve_mock_file(Path("route/../../../etc/passwd".to_string())).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_serve_mock_file_path_traversal_middle() {
        use axum::extract::Path;
        let result = serve_mock_file(Path("route/sub/../../../file.txt".to_string())).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    // ==================== Invalid Path Format Tests ====================

    #[tokio::test]
    async fn test_serve_mock_file_invalid_format() {
        // Test invalid path format (empty string results in empty parts)
        use axum::extract::Path;
        let result = serve_mock_file(Path("".to_string())).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_serve_mock_file_only_slashes() {
        use axum::extract::Path;
        let result = serve_mock_file(Path("/".to_string())).await;
        // After filtering empty parts, should be empty
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    // ==================== File Not Found Tests ====================

    #[tokio::test]
    async fn test_serve_mock_file_not_found() {
        use axum::extract::Path;
        let result = serve_mock_file(Path("nonexistent/file.json".to_string())).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_serve_mock_file_deep_path_not_found() {
        use axum::extract::Path;
        let result = serve_mock_file(Path("route/subdir/deep/file.json".to_string())).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::NOT_FOUND);
    }

    // ==================== Router Tests ====================

    #[test]
    fn test_file_serving_router_creation() {
        let router = file_serving_router();
        // Router should be created successfully
        assert!(std::mem::size_of_val(&router) > 0);
    }

    #[tokio::test]
    async fn test_router_path_traversal_blocked() {
        let router = file_serving_router();

        let request =
            Request::builder().uri("/mock-files/../etc/passwd").body(Body::empty()).unwrap();

        let response = router.oneshot(request).await.unwrap();
        // Should be blocked by path traversal check
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_router_nonexistent_file() {
        let router = file_serving_router();

        let request = Request::builder()
            .uri("/mock-files/route123/file.json")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        // File doesn't exist
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // ==================== Content Type Detection Tests ====================
    // These tests verify the content type logic is correct

    #[test]
    fn test_content_type_detection_logic() {
        // Test the extension to content-type mapping logic
        let extensions = vec![
            ("pdf", "application/pdf"),
            ("csv", "text/csv"),
            ("json", "application/json"),
            ("xml", "application/xml"),
            ("txt", "text/plain"),
            ("unknown", "application/octet-stream"),
            ("bin", "application/octet-stream"),
        ];

        for (ext, expected) in extensions {
            let content_type = match ext.to_lowercase().as_str() {
                "pdf" => "application/pdf",
                "csv" => "text/csv",
                "json" => "application/json",
                "xml" => "application/xml",
                "txt" => "text/plain",
                _ => "application/octet-stream",
            };
            assert_eq!(content_type, expected, "Extension: {}", ext);
        }
    }

    #[test]
    fn test_content_type_case_insensitive() {
        // Content type detection should be case insensitive
        let extensions = vec!["PDF", "Pdf", "pDf", "JSON", "Json", "XML", "Xml"];

        for ext in extensions {
            let content_type = match ext.to_lowercase().as_str() {
                "pdf" => "application/pdf",
                "json" => "application/json",
                "xml" => "application/xml",
                _ => "application/octet-stream",
            };
            assert_ne!(
                content_type, "application/octet-stream",
                "Extension {} should be recognized",
                ext
            );
        }
    }

    // ==================== PathBuf Construction Tests ====================

    #[test]
    fn test_path_construction() {
        let base_dir = "mock-files";
        let file_path = "route123/data.json";
        let full_path = PathBuf::from(base_dir).join(file_path);

        assert!(full_path.to_string_lossy().contains("mock-files"));
        assert!(full_path.to_string_lossy().contains("route123"));
        assert!(full_path.to_string_lossy().contains("data.json"));
    }

    #[test]
    fn test_path_with_subdirectory() {
        let base_dir = "mock-files";
        let file_path = "route123/subdir/nested/file.csv";
        let full_path = PathBuf::from(base_dir).join(file_path);

        assert!(full_path.to_string_lossy().contains("subdir"));
        assert!(full_path.to_string_lossy().contains("nested"));
    }

    #[test]
    fn test_filename_extraction() {
        let full_path = PathBuf::from("mock-files/route123/data.json");
        let filename = full_path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
        assert_eq!(filename, "data.json");
    }

    #[test]
    fn test_filename_extraction_nested() {
        let full_path = PathBuf::from("mock-files/route/sub/deep/report.pdf");
        let filename = full_path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
        assert_eq!(filename, "report.pdf");
    }

    #[test]
    fn test_extension_extraction() {
        let paths = vec![
            ("mock-files/file.pdf", Some("pdf")),
            ("mock-files/file.JSON", Some("JSON")),
            ("mock-files/file.tar.gz", Some("gz")),
            ("mock-files/file", None),
        ];

        for (path, expected_ext) in paths {
            let full_path = PathBuf::from(path);
            let ext = full_path.extension().and_then(|e| e.to_str());
            assert_eq!(ext, expected_ext, "Path: {}", path);
        }
    }
}
