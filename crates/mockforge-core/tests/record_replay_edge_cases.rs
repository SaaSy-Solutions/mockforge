//! Edge case tests for record/replay functionality
//!
//! These tests cover error paths, edge cases, and boundary conditions
//! for record and replay handlers.

use axum::http::{HeaderMap, Method, Uri};
use mockforge_core::record_replay::{RecordHandler, RecordReplayHandler, ReplayHandler};
use mockforge_core::request_fingerprint::RequestFingerprint;
use tempfile::TempDir;

/// Test ReplayHandler with disabled replay
#[tokio::test]
async fn test_replay_handler_disabled() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = ReplayHandler::new(fixtures_dir, false);

    let method = Method::GET;
    let uri: Uri = "/api/test".parse().unwrap();
    let headers = HeaderMap::new();
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);

    assert!(!handler.has_fixture(&fingerprint).await);
    assert!(handler.load_fixture(&fingerprint).await.unwrap().is_none());
}

/// Test ReplayHandler with non-existent fixture
#[tokio::test]
async fn test_replay_handler_no_fixture() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = ReplayHandler::new(fixtures_dir, true);

    let method = Method::GET;
    let uri: Uri = "/api/nonexistent".parse().unwrap();
    let headers = HeaderMap::new();
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);

    assert!(!handler.has_fixture(&fingerprint).await);
    assert!(handler.load_fixture(&fingerprint).await.unwrap().is_none());
}

/// Test RecordHandler with disabled recording
#[tokio::test]
async fn test_record_handler_disabled() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordHandler::new(fixtures_dir.clone(), false, false);

    let method = Method::GET;
    let uri: Uri = "/api/test".parse().unwrap();
    let headers = HeaderMap::new();
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);
    let response_headers = HeaderMap::new();

    // Should not record when disabled
    assert!(!handler.should_record(&Method::GET));
    handler
        .record_request(&fingerprint, 200, &response_headers, "{}", None)
        .await
        .unwrap();

    // Verify nothing was recorded
    let replay = ReplayHandler::new(fixtures_dir, true);
    assert!(!replay.has_fixture(&fingerprint).await);
}

/// Test RecordHandler with record_get_only enabled
#[tokio::test]
async fn test_record_handler_get_only() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordHandler::new(fixtures_dir, true, true);

    assert!(handler.should_record(&Method::GET));
    assert!(!handler.should_record(&Method::POST));
    assert!(!handler.should_record(&Method::PUT));
    assert!(!handler.should_record(&Method::DELETE));
}

/// Test RecordHandler with record_get_only disabled
#[tokio::test]
async fn test_record_handler_all_methods() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordHandler::new(fixtures_dir, true, false);

    assert!(handler.should_record(&Method::GET));
    assert!(handler.should_record(&Method::POST));
    assert!(handler.should_record(&Method::PUT));
    assert!(handler.should_record(&Method::DELETE));
    assert!(handler.should_record(&Method::PATCH));
}

/// Test RecordReplayHandler creation
#[tokio::test]
async fn test_record_replay_handler_new() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

    // Should be able to access both handlers (test through functionality)
    let method = Method::GET;
    let uri: Uri = "/api/test".parse().unwrap();
    let headers = HeaderMap::new();
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);

    // Replay should work when enabled
    assert!(!handler.replay_handler().has_fixture(&fingerprint).await);

    // Record should work when enabled
    let response_headers = HeaderMap::new();
    handler
        .record_handler()
        .record_request(&fingerprint, 200, &response_headers, "{}", None)
        .await
        .unwrap();
}

/// Test RecordReplayHandler with replay disabled
#[tokio::test]
async fn test_record_replay_handler_replay_disabled() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordReplayHandler::new(fixtures_dir.clone(), false, true, false);

    let method = Method::GET;
    let uri: Uri = "/api/test".parse().unwrap();
    let headers = HeaderMap::new();
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);

    // Replay should return false when disabled
    assert!(!handler.replay_handler().has_fixture(&fingerprint).await);
    assert!(handler.replay_handler().load_fixture(&fingerprint).await.unwrap().is_none());
}

/// Test RecordReplayHandler with record disabled
#[tokio::test]
async fn test_record_replay_handler_record_disabled() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordReplayHandler::new(fixtures_dir.clone(), true, false, false);

    assert!(!handler.record_handler().should_record(&Method::GET));
}

/// Test recording with various HTTP methods
#[tokio::test]
async fn test_record_various_methods() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

    let methods = vec![
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::PATCH,
    ];
    let mut fingerprints = Vec::new();

    for method in methods {
        let uri: Uri = format!("/api/test/{}", method.as_str()).parse().unwrap();
        let headers = HeaderMap::new();
        let fingerprint = RequestFingerprint::new(method.clone(), &uri, &headers, None);
        fingerprints.push(fingerprint.clone());

        let response_headers = HeaderMap::new();
        handler
            .record_handler()
            .record_request(&fingerprint, 200, &response_headers, "{}", None)
            .await
            .unwrap();
    }

    // Verify all were recorded
    for fingerprint in fingerprints {
        assert!(handler.replay_handler().has_fixture(&fingerprint).await);
    }
}

/// Test recording with various status codes
#[tokio::test]
async fn test_record_various_status_codes() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

    let status_codes = vec![200, 201, 204, 400, 401, 403, 404, 500, 502, 503];

    for status in status_codes {
        let method = Method::GET;
        let uri: Uri = format!("/api/test/{}", status).parse().unwrap();
        let headers = HeaderMap::new();
        let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);

        let response_headers = HeaderMap::new();
        handler
            .record_handler()
            .record_request(&fingerprint, status, &response_headers, "{}", None)
            .await
            .unwrap();

        // Verify it was recorded with correct status
        let recorded = handler.replay_handler().load_fixture(&fingerprint).await.unwrap().unwrap();
        assert_eq!(recorded.status_code, status);
    }
}

/// Test recording with response headers
#[tokio::test]
async fn test_record_with_headers() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

    let method = Method::GET;
    let uri: Uri = "/api/test".parse().unwrap();
    let headers = HeaderMap::new();
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);

    let mut response_headers = HeaderMap::new();
    response_headers.insert("content-type", "application/json".parse().unwrap());
    response_headers.insert("x-custom-header", "custom-value".parse().unwrap());
    response_headers.insert("cache-control", "no-cache".parse().unwrap());

    handler
        .record_handler()
        .record_request(&fingerprint, 200, &response_headers, "{}", None)
        .await
        .unwrap();

    let recorded = handler.replay_handler().load_fixture(&fingerprint).await.unwrap().unwrap();

    assert_eq!(
        recorded.response_headers.get("content-type"),
        Some(&"application/json".to_string())
    );
    assert_eq!(
        recorded.response_headers.get("x-custom-header"),
        Some(&"custom-value".to_string())
    );
    assert_eq!(recorded.response_headers.get("cache-control"), Some(&"no-cache".to_string()));
}

/// Test recording with metadata
#[tokio::test]
async fn test_record_with_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

    let method = Method::GET;
    let uri: Uri = "/api/test".parse().unwrap();
    let headers = HeaderMap::new();
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("name".to_string(), "test-endpoint".to_string());
    metadata.insert("version".to_string(), "1.0".to_string());
    metadata.insert("tags".to_string(), "test,api".to_string());

    let response_headers = HeaderMap::new();
    handler
        .record_handler()
        .record_request(&fingerprint, 200, &response_headers, "{}", Some(metadata.clone()))
        .await
        .unwrap();

    let recorded = handler.replay_handler().load_fixture(&fingerprint).await.unwrap().unwrap();

    assert_eq!(recorded.metadata.get("name"), Some(&"test-endpoint".to_string()));
    assert_eq!(recorded.metadata.get("version"), Some(&"1.0".to_string()));
    assert_eq!(recorded.metadata.get("tags"), Some(&"test,api".to_string()));
}

/// Test recording with empty metadata
#[tokio::test]
async fn test_record_with_empty_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

    let method = Method::GET;
    let uri: Uri = "/api/test".parse().unwrap();
    let headers = HeaderMap::new();
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);

    let response_headers = HeaderMap::new();
    handler
        .record_handler()
        .record_request(&fingerprint, 200, &response_headers, "{}", None)
        .await
        .unwrap();

    let recorded = handler.replay_handler().load_fixture(&fingerprint).await.unwrap().unwrap();

    assert!(recorded.metadata.is_empty());
}

/// Test recording with large response body
#[tokio::test]
async fn test_record_large_body() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

    let method = Method::GET;
    let uri: Uri = "/api/test".parse().unwrap();
    let headers = HeaderMap::new();
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);

    // Create a large response body (10KB)
    let large_body = "x".repeat(10000);

    let response_headers = HeaderMap::new();
    handler
        .record_handler()
        .record_request(&fingerprint, 200, &response_headers, &large_body, None)
        .await
        .unwrap();

    let recorded = handler.replay_handler().load_fixture(&fingerprint).await.unwrap().unwrap();

    assert_eq!(recorded.response_body, large_body);
    assert_eq!(recorded.response_body.len(), 10000);
}

/// Test recording with special characters in path
#[tokio::test]
async fn test_record_special_characters_path() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

    let method = Method::GET;
    let uri: Uri = "/api/test%20with%20spaces".parse().unwrap();
    let headers = HeaderMap::new();
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);

    let response_headers = HeaderMap::new();
    handler
        .record_handler()
        .record_request(&fingerprint, 200, &response_headers, "{}", None)
        .await
        .unwrap();

    assert!(handler.replay_handler().has_fixture(&fingerprint).await);
}

/// Test recording with query parameters
#[tokio::test]
async fn test_record_with_query_params() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

    let method = Method::GET;
    let uri: Uri = "/api/test?page=1&limit=10&sort=name".parse().unwrap();
    let headers = HeaderMap::new();
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);

    let response_headers = HeaderMap::new();
    handler
        .record_handler()
        .record_request(&fingerprint, 200, &response_headers, "{}", None)
        .await
        .unwrap();

    assert!(handler.replay_handler().has_fixture(&fingerprint).await);

    // Verify query params are preserved in fingerprint
    assert!(fingerprint.query.contains("page=1"));
    assert!(fingerprint.query.contains("limit=10"));
    assert!(fingerprint.query.contains("sort=name"));
}

/// Test recording with request body hash
#[tokio::test]
async fn test_record_with_body_hash() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

    let method = Method::POST;
    let uri: Uri = "/api/test".parse().unwrap();
    let headers = HeaderMap::new();
    let body = Some(r#"{"key": "value"}"#.as_bytes());
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, body);

    // Body hash should be present
    assert!(fingerprint.body_hash.is_some());

    let response_headers = HeaderMap::new();
    handler
        .record_handler()
        .record_request(&fingerprint, 200, &response_headers, "{}", None)
        .await
        .unwrap();

    assert!(handler.replay_handler().has_fixture(&fingerprint).await);
}

/// Test recording multiple requests with same path but different methods
#[tokio::test]
async fn test_record_same_path_different_methods() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

    let uri: Uri = "/api/users/123".parse().unwrap();
    let headers = HeaderMap::new();

    let methods = vec![Method::GET, Method::PUT, Method::DELETE];
    let mut fingerprints = Vec::new();

    for method in methods {
        let fingerprint = RequestFingerprint::new(method.clone(), &uri, &headers, None);
        fingerprints.push(fingerprint.clone());

        let response_headers = HeaderMap::new();
        handler
            .record_handler()
            .record_request(&fingerprint, 200, &response_headers, "{}", None)
            .await
            .unwrap();
    }

    // All should be recorded separately
    for fingerprint in fingerprints {
        assert!(handler.replay_handler().has_fixture(&fingerprint).await);
    }
}

/// Test recording with invalid header values (should handle gracefully)
#[tokio::test]
async fn test_record_invalid_header_values() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let handler = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

    let method = Method::GET;
    let uri: Uri = "/api/test".parse().unwrap();
    let headers = HeaderMap::new();
    let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);

    // Headers with invalid UTF-8 should be handled
    let mut response_headers = HeaderMap::new();
    response_headers.insert("content-type", "application/json".parse().unwrap());
    // Note: HeaderMap will reject invalid header values, so we can only test valid ones

    handler
        .record_handler()
        .record_request(&fingerprint, 200, &response_headers, "{}", None)
        .await
        .unwrap();

    assert!(handler.replay_handler().has_fixture(&fingerprint).await);
}
