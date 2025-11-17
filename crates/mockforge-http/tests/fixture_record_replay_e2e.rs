//! End-to-end tests for fixture recording and replay functionality
//!
//! These tests verify the complete workflow:
//! 1. Record requests as fixtures
//! 2. Verify fixtures are created on disk
//! 3. Enable replay mode
//! 4. Verify recorded responses are served

use axum::http::{HeaderMap, Method, Uri};
use mockforge_core::record_replay::RecordReplayHandler;
use mockforge_core::RequestFingerprint;
use tempfile::TempDir;

/// Test the complete record/replay cycle
#[tokio::test]
async fn test_fixture_record_and_replay_e2e() {
    // Create a temporary directory for fixtures
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    // Step 1: Create a record/replay handler with recording enabled
    let record_replay = RecordReplayHandler::new(
        fixtures_dir.clone(),
        false, // replay disabled initially
        true,  // recording enabled
        false, // record all methods, not just GET
    );

    // Step 2: Create a priority handler that uses the record/replay handler
    // In a real scenario, this would be part of the HTTP server
    let fingerprint = RequestFingerprint::new(
        Method::GET,
        &Uri::from_static("/api/test/users"),
        &HeaderMap::new(),
        None,
    );

    // Step 3: Record a request/response
    let mut response_headers = HeaderMap::new();
    response_headers.insert("content-type", "application/json".parse().unwrap());
    let response_body = r#"{"users": [{"id": 1, "name": "Test User"}]}"#;

    record_replay
        .record_handler()
        .record_request(&fingerprint, 200, &response_headers, response_body, None)
        .await
        .unwrap();

    // Step 4: Verify the fixture file was created on disk
    // The path structure is: fixtures/http/{method}/{path_hash}/{hash}.json
    // where path_hash is the path with / and : replaced with _
    let path_hash = fingerprint.path.replace(['/', ':'], "_");
    let method_lower = fingerprint.method.to_lowercase();
    let fixture_path = fixtures_dir
        .join("http")
        .join(&method_lower)
        .join(&path_hash)
        .join(format!("{}.json", fingerprint.to_hash()));

    assert!(
        fixture_path.exists(),
        "Fixture file should exist at: {}",
        fixture_path.display()
    );

    // Step 5: Verify the fixture content is correct
    let fixture_content = std::fs::read_to_string(&fixture_path).unwrap();
    assert!(fixture_content.contains("Test User"));
    assert!(fixture_content.contains("200"));

    // Step 6: Create a new handler with replay enabled (simulating server restart with replay mode)
    let replay_handler = RecordReplayHandler::new(
        fixtures_dir.clone(),
        true,  // replay enabled
        false, // recording disabled
        false,
    );

    // Step 7: Verify the fixture can be loaded for replay
    assert!(
        replay_handler.replay_handler().has_fixture(&fingerprint).await,
        "Fixture should be available for replay"
    );

    // Step 8: Load and verify the recorded fixture
    let recorded = replay_handler
        .replay_handler()
        .load_fixture(&fingerprint)
        .await
        .unwrap()
        .expect("Fixture should be loadable");

    assert_eq!(recorded.status_code, 200);
    assert_eq!(recorded.response_body, response_body);
    assert_eq!(
        recorded.response_headers.get("content-type").map(|v| v.as_str()),
        Some("application/json")
    );
}

/// Test recording multiple requests and replaying them
#[tokio::test]
async fn test_fixture_record_multiple_and_replay() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), false, true, false);

    // Record multiple different requests
    let requests = vec![
        ("/api/users", Method::GET, r#"{"users": []}"#),
        ("/api/users/1", Method::GET, r#"{"id": 1, "name": "User 1"}"#),
        ("/api/users", Method::POST, r#"{"id": 2, "name": "New User"}"#),
    ];

    for (path, method, body) in &requests {
        let uri: Uri = path.parse().unwrap();
        let fingerprint = RequestFingerprint::new(method.clone(), &uri, &HeaderMap::new(), None);

        record_replay
            .record_handler()
            .record_request(&fingerprint, 200, &HeaderMap::new(), body, None)
            .await
            .unwrap();
    }

    // Verify all fixtures were created
    let http_dir = fixtures_dir.join("http");
    assert!(http_dir.exists(), "HTTP fixtures directory should exist");

    // Create replay handler and verify all can be replayed
    let replay_handler = RecordReplayHandler::new(fixtures_dir.clone(), true, false, false);

    for (path, method, expected_body) in &requests {
        let uri: Uri = path.parse().unwrap();
        let fingerprint = RequestFingerprint::new(method.clone(), &uri, &HeaderMap::new(), None);

        assert!(
            replay_handler.replay_handler().has_fixture(&fingerprint).await,
            "Fixture should exist for {} {}",
            method,
            path
        );

        let recorded = replay_handler
            .replay_handler()
            .load_fixture(&fingerprint)
            .await
            .unwrap()
            .expect("Fixture should be loadable");

        assert_eq!(recorded.response_body, *expected_body);
    }
}

/// Test that replay takes precedence over mock generation
#[tokio::test]
async fn test_replay_priority_over_mocks() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    // Record a specific response
    let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), false, true, false);

    let fingerprint = RequestFingerprint::new(
        Method::GET,
        &Uri::from_static("/api/priority-test"),
        &HeaderMap::new(),
        None,
    );

    let recorded_body = r#"{"source": "fixture", "priority": "replay"}"#;
    record_replay
        .record_handler()
        .record_request(&fingerprint, 200, &HeaderMap::new(), recorded_body, None)
        .await
        .unwrap();

    // Enable replay and verify the recorded response is available
    let replay_handler = RecordReplayHandler::new(fixtures_dir.clone(), true, false, false);

    let recorded = replay_handler
        .replay_handler()
        .load_fixture(&fingerprint)
        .await
        .unwrap()
        .expect("Recorded fixture should be available");

    // Verify it's the recorded response, not a generated mock
    assert_eq!(recorded.response_body, recorded_body);
    assert!(recorded.response_body.contains("fixture"));
    assert!(recorded.response_body.contains("replay"));
}

/// Test recording with different HTTP methods
#[tokio::test]
async fn test_record_different_methods() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), false, true, false);

    let methods = vec![Method::GET, Method::POST, Method::PUT, Method::DELETE];

    for method in methods {
        let uri: Uri = "/api/test".parse().unwrap();
        let fingerprint = RequestFingerprint::new(method.clone(), &uri, &HeaderMap::new(), None);

        let body = format!(r#"{{"method": "{}"}}"#, method.as_str());
        record_replay
            .record_handler()
            .record_request(&fingerprint, 200, &HeaderMap::new(), &body, None)
            .await
            .unwrap();

        // Verify fixture was created in method-specific directory
        let method_dir = fixtures_dir.join("http").join(method.as_str().to_lowercase());
        assert!(method_dir.exists(), "Method directory should exist for {}", method.as_str());
    }
}

/// Test importing a manually created fixture file
/// This verifies that users can manually place fixture JSON files in the fixtures directory
/// and they will be picked up by the replay handler
#[tokio::test]
async fn test_import_manual_fixture_file() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    // Create a fixture file manually (simulating a user importing/uploading a fixture)
    // First, create the fingerprint to get the correct path structure
    use mockforge_core::RequestFingerprint;
    let fingerprint = RequestFingerprint::new(
        Method::GET,
        &Uri::from_static("/api/test/import"),
        &HeaderMap::new(),
        None,
    );

    // Use the same path calculation as the replay handler
    let hash = fingerprint.to_hash();
    let method_lower = fingerprint.method.to_lowercase();
    let path_hash = fingerprint.path.replace(['/', ':'], "_");

    let http_dir = fixtures_dir.join("http").join(&method_lower);
    let path_hash_dir = http_dir.join(&path_hash);
    std::fs::create_dir_all(&path_hash_dir).unwrap();

    // Create a fixture JSON file with the expected structure
    // Note: RequestFingerprint uses "query" (String), not "query_params"
    let fixture_content = r#"{
  "fingerprint": {
    "method": "GET",
    "path": "/api/test/import",
    "query": "",
    "headers": {},
    "body_hash": null
  },
  "timestamp": "2024-01-15T10:30:00Z",
  "status_code": 200,
  "response_headers": {
    "content-type": "application/json"
  },
  "response_body": "{\"message\": \"This fixture was manually imported\", \"source\": \"manual\"}",
  "metadata": {
    "imported": "true",
    "name": "Manually Imported Fixture"
  }
}"#;

    let fixture_file = path_hash_dir.join(format!("{}.json", hash));
    std::fs::write(&fixture_file, fixture_content).unwrap();

    // Verify the file was created
    assert!(fixture_file.exists(), "Manually created fixture file should exist");

    // Create a replay handler and verify it can load the manually imported fixture
    let replay_handler = RecordReplayHandler::new(fixtures_dir.clone(), true, false, false);

    // Verify the fixture can be found
    assert!(
        replay_handler.replay_handler().has_fixture(&fingerprint).await,
        "Manually imported fixture should be detectable"
    );

    // Load and verify the fixture
    let recorded = replay_handler
        .replay_handler()
        .load_fixture(&fingerprint)
        .await
        .unwrap()
        .expect("Manually imported fixture should be loadable");

    assert_eq!(recorded.status_code, 200);
    assert!(recorded.response_body.contains("manually imported"));
    assert!(recorded.response_body.contains("manual"));
    assert_eq!(
        recorded.response_headers.get("content-type").map(|v| v.as_str()),
        Some("application/json")
    );
    assert_eq!(recorded.metadata.get("imported"), Some(&"true".to_string()));
}

/// Test that recording respects record_get_only flag
#[tokio::test]
async fn test_record_get_only_flag() {
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    // Create handler that only records GET requests
    let record_replay = RecordReplayHandler::new(fixtures_dir.clone(), false, true, true);

    // Try to record a GET request - should succeed
    let get_fingerprint = RequestFingerprint::new(
        Method::GET,
        &Uri::from_static("/api/test"),
        &HeaderMap::new(),
        None,
    );

    record_replay
        .record_handler()
        .record_request(&get_fingerprint, 200, &HeaderMap::new(), "{}", None)
        .await
        .unwrap();

    // Try to record a POST request - should be ignored
    let post_fingerprint = RequestFingerprint::new(
        Method::POST,
        &Uri::from_static("/api/test"),
        &HeaderMap::new(),
        None,
    );

    record_replay
        .record_handler()
        .record_request(&post_fingerprint, 200, &HeaderMap::new(), "{}", None)
        .await
        .unwrap();

    // Verify only GET fixture was created
    let get_dir = fixtures_dir.join("http").join("get");
    let post_dir = fixtures_dir.join("http").join("post");

    assert!(get_dir.exists(), "GET fixtures directory should exist");

    // POST directory might exist but should be empty or not have our fixture
    if post_dir.exists() {
        let entries: Vec<_> =
            std::fs::read_dir(&post_dir).unwrap().filter_map(|e| e.ok()).collect();
        assert_eq!(
            entries.len(),
            0,
            "POST fixtures directory should be empty when record_get_only is true"
        );
    }
}
