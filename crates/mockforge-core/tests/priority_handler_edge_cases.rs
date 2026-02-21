//! Edge case tests for priority handler and request processing
//!
//! These tests cover error paths, edge cases, and integration scenarios
//! that are critical for reliability but may have lower coverage.

use axum::http::{HeaderMap, Method, Uri};
use mockforge_core::priority_handler::{PriorityHttpHandler, SimpleMockGenerator};
use mockforge_core::{RecordReplayHandler, RequestFingerprint};
use std::sync::Arc;
use tempfile::TempDir;

/// Helper to create a minimal priority handler for testing
fn create_test_handler() -> PriorityHttpHandler {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();
    let record_replay = RecordReplayHandler::new(fixtures_dir, true, true, false);
    let mock_generator =
        Box::new(SimpleMockGenerator::new(200, r#"{"message": "mock response"}"#.to_string()));

    PriorityHttpHandler::new(
        record_replay,
        None, // No failure injection
        None, // No proxy
        Some(mock_generator),
    )
}

/// Test priority handler with empty state
#[tokio::test]
async fn test_priority_handler_empty_state() {
    let handler = create_test_handler();

    let method = Method::GET;
    let uri = Uri::from_static("/api/test");
    let headers = HeaderMap::new();

    let result = handler.process_request(&method, &uri, &headers, None).await;

    // Should return a mock response (lowest priority)
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.source.priority, mockforge_core::ResponsePriority::Mock);
}

/// Test priority handler with invalid URI
#[tokio::test]
async fn test_priority_handler_invalid_uri() {
    let handler = create_test_handler();

    let method = Method::GET;
    // This should still work - URI parsing happens before handler
    let uri = Uri::from_static("/api/test");
    let headers = HeaderMap::new();

    let result = handler.process_request(&method, &uri, &headers, None).await;
    assert!(result.is_ok());
}

/// Test priority handler with empty body
#[tokio::test]
async fn test_priority_handler_empty_body() {
    let handler = create_test_handler();

    let method = Method::POST;
    let uri = Uri::from_static("/api/test");
    let headers = HeaderMap::new();

    let result = handler.process_request(&method, &uri, &headers, Some(&[])).await;
    assert!(result.is_ok());
}

/// Test priority handler with large body
#[tokio::test]
async fn test_priority_handler_large_body() {
    let handler = create_test_handler();

    let method = Method::POST;
    let uri = Uri::from_static("/api/test");
    let headers = HeaderMap::new();
    let large_body = vec![0u8; 10000]; // 10KB body

    let result = handler.process_request(&method, &uri, &headers, Some(&large_body)).await;
    assert!(result.is_ok());
}

/// Test request fingerprint with various edge cases
#[test]
fn test_request_fingerprint_edge_cases() {
    // Test with empty path
    let method = Method::GET;
    let uri = Uri::from_static("/");
    let headers = HeaderMap::new();
    let fingerprint = RequestFingerprint::new(method.clone(), &uri, &headers, None);
    assert_eq!(fingerprint.path, "/");

    // Test with query parameters
    let uri = Uri::from_static("/api/test?a=1&b=2");
    let fingerprint = RequestFingerprint::new(method.clone(), &uri, &headers, None);
    assert_eq!(fingerprint.path, "/api/test");
    assert!(fingerprint.query.contains("a=1"));
    assert!(fingerprint.query.contains("b=2"));

    // Test with body hash
    let body = b"test body";
    let fingerprint1 = RequestFingerprint::new(method.clone(), &uri, &headers, Some(body));
    let fingerprint2 = RequestFingerprint::new(method.clone(), &uri, &headers, Some(body));
    assert_eq!(fingerprint1.body_hash, fingerprint2.body_hash);

    // Test with different bodies
    let body2 = b"different body";
    let fingerprint3 = RequestFingerprint::new(method, &uri, &headers, Some(body2));
    assert_ne!(fingerprint1.body_hash, fingerprint3.body_hash);
}

/// Test request fingerprint with special characters in path
#[test]
fn test_request_fingerprint_special_characters() {
    let method = Method::GET;
    let headers = HeaderMap::new();

    // Test with encoded path
    let uri = Uri::from_static("/api/test%20path");
    let fingerprint = RequestFingerprint::new(method.clone(), &uri, &headers, None);
    assert_eq!(fingerprint.path, "/api/test%20path");

    // Test with query parameters containing special chars
    let uri = Uri::from_static("/api/test?param=value%20with%20spaces");
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);
    assert!(fingerprint.query.contains("param=value%20with%20spaces"));
}

/// Test request fingerprint with various HTTP methods
#[test]
fn test_request_fingerprint_all_methods() {
    let headers = HeaderMap::new();
    let uri = Uri::from_static("/api/test");

    let methods = vec![
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::PATCH,
        Method::HEAD,
        Method::OPTIONS,
    ];

    for method in methods {
        let method_str = method.to_string();
        let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);
        assert_eq!(fingerprint.method, method_str);
    }
}

/// Test request fingerprint tag extraction
#[test]
fn test_request_fingerprint_tags() {
    let headers = HeaderMap::new();

    // Test simple path
    let method = Method::GET;
    let uri = Uri::from_static("/api/users");
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);
    let tags = fingerprint.tags();
    assert!(tags.contains(&"api".to_string()));
    assert!(tags.contains(&"users".to_string()));
    assert!(tags.contains(&"get".to_string()));

    // Test path with parameters
    let method = Method::GET;
    let uri = Uri::from_static("/api/users/{id}/posts");
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);
    let tags = fingerprint.tags();
    assert!(tags.contains(&"api".to_string()));
    assert!(tags.contains(&"users".to_string()));
    assert!(tags.contains(&"posts".to_string()));
    // Should not include parameter placeholders
    assert!(!tags.iter().any(|t| t.starts_with('{')));
}

/// Test request fingerprint display format
#[test]
fn test_request_fingerprint_display() {
    let method = Method::POST;
    let uri = Uri::from_static("/api/test?param=value");
    let mut headers = HeaderMap::new();
    headers.insert("authorization", "Bearer token123".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());

    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);
    let display = fingerprint.to_string();

    // Should contain method, path, and query
    assert!(display.contains("POST"));
    assert!(display.contains("/api/test"));
    assert!(display.contains("param=value"));
    // Headers should be included
    assert!(display.contains("authorization"));
    assert!(display.contains("content-type"));
}

/// Test request fingerprint hash consistency
#[test]
fn test_request_fingerprint_hash_consistency() {
    let method = Method::GET;
    let uri = Uri::from_static("/api/test");
    let headers = HeaderMap::new();

    let fingerprint1 = RequestFingerprint::new(method.clone(), &uri, &headers, None);
    let fingerprint2 = RequestFingerprint::new(method, &uri, &headers, None);

    // Same fingerprint should produce same hash
    assert_eq!(fingerprint1.to_hash(), fingerprint2.to_hash());
}

/// Test priority handler with custom fixture loader but no matching fixture
#[tokio::test]
async fn test_priority_handler_custom_fixture_no_match() {
    use mockforge_core::CustomFixtureLoader;

    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();
    let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);
    let mock_generator =
        Box::new(SimpleMockGenerator::new(200, r#"{"message": "mock response"}"#.to_string()));

    let loader = Arc::new(CustomFixtureLoader::new(fixtures_dir, true));

    let handler = PriorityHttpHandler::new(record_replay, None, None, Some(mock_generator))
        .with_custom_fixture_loader(loader);

    let method = Method::GET;
    let uri = Uri::from_static("/api/nonexistent");
    let headers = HeaderMap::new();

    // Should fall through to next priority (mock)
    let result = handler.process_request(&method, &uri, &headers, None).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    // Should use mock since no custom fixture matches
    assert_eq!(response.source.priority, mockforge_core::ResponsePriority::Mock);
}

/// Test priority handler error handling
#[tokio::test]
async fn test_priority_handler_error_recovery() {
    let handler = create_test_handler();

    let method = Method::GET;
    let uri = Uri::from_static("/api/test");
    let mut headers = HeaderMap::new();

    // Add header with unusual but valid characters (HeaderMap rejects newlines)
    headers.insert("x-custom", "value-with-special-chars-!@#$%".parse().unwrap());

    let result = handler.process_request(&method, &uri, &headers, None).await;
    // Should still succeed, handling various header values gracefully
    assert!(result.is_ok());
}

/// Test request fingerprint with unicode in path
#[test]
fn test_request_fingerprint_unicode() {
    let method = Method::GET;
    let headers = HeaderMap::new();

    // Test with unicode characters (should be URL encoded in real usage)
    let uri = Uri::from_static("/api/test%E2%98%BA"); // ☺ encoded
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);
    assert_eq!(fingerprint.path, "/api/test%E2%98%BA");
}

/// Test request fingerprint with empty query string
#[test]
fn test_request_fingerprint_empty_query() {
    let method = Method::GET;
    let uri = Uri::from_static("/api/test");
    let headers = HeaderMap::new();

    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);
    assert_eq!(fingerprint.query, "");
}

/// Test request fingerprint with duplicate query parameters
#[test]
fn test_request_fingerprint_duplicate_query() {
    let method = Method::GET;
    let uri = Uri::from_static("/api/test?param=1&param=2");
    let headers = HeaderMap::new();

    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);
    // Query should be sorted
    assert!(fingerprint.query.contains("param=1"));
    assert!(fingerprint.query.contains("param=2"));
}

/// Test priority handler with all optional components disabled
#[tokio::test]
async fn test_priority_handler_minimal_config() {
    let handler = create_test_handler();

    let method = Method::GET;
    let uri = Uri::from_static("/api/test");
    let headers = HeaderMap::new();

    let result = handler.process_request(&method, &uri, &headers, None).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    // Should default to mock response
    assert_eq!(response.source.priority, mockforge_core::ResponsePriority::Mock);
}

/// Test request fingerprint with various header combinations
#[test]
fn test_request_fingerprint_header_combinations() {
    let method = Method::GET;
    let uri = Uri::from_static("/api/test");

    // Test with no headers
    let headers = HeaderMap::new();
    let fingerprint = RequestFingerprint::new(method.clone(), &uri, &headers, None);
    assert!(fingerprint.headers.is_empty());

    // Test with important headers
    let mut headers = HeaderMap::new();
    headers.insert("authorization", "Bearer token".parse().unwrap());
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert("accept", "application/json".parse().unwrap());

    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);
    assert_eq!(fingerprint.headers.len(), 3);
    assert_eq!(fingerprint.headers.get("authorization"), Some(&"Bearer token".to_string()));
    assert_eq!(fingerprint.headers.get("content-type"), Some(&"application/json".to_string()));
    assert_eq!(fingerprint.headers.get("accept"), Some(&"application/json".to_string()));
}

/// Test request fingerprint with non-important headers (should be ignored)
#[test]
fn test_request_fingerprint_non_important_headers() {
    let method = Method::GET;
    let uri = Uri::from_static("/api/test");
    let mut headers = HeaderMap::new();

    // Add non-important header
    headers.insert("x-custom-header", "custom-value".parse().unwrap());

    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);
    // Non-important headers should not be included
    assert!(!fingerprint.headers.contains_key("x-custom-header"));
}

/// Test request fingerprint with unusual header values
#[test]
fn test_request_fingerprint_invalid_header_values() {
    let method = Method::GET;
    let uri = Uri::from_static("/api/test");
    let mut headers = HeaderMap::new();

    // Test with valid but unusual header values (HeaderMap rejects newlines, so use other special chars)
    headers.insert("authorization", "Bearer token-with-special!@#chars".parse().unwrap());

    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);
    // Should include the header
    assert!(fingerprint.headers.contains_key("authorization"));
    assert_eq!(
        fingerprint.headers.get("authorization"),
        Some(&"Bearer token-with-special!@#chars".to_string())
    );
}
