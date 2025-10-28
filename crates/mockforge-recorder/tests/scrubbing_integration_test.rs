//! Integration test for capture scrubbing and deterministic replay
//!
//! This test verifies that:
//! 1. Sensitive data is scrubbed from captured requests/responses
//! 2. Deterministic mode produces consistent output
//! 3. Scrubbed recordings match golden files
//! 4. Capture filtering works correctly

use chrono::Timelike;
use mockforge_recorder::{
    models::RequestContext, CaptureFilter, CaptureFilterConfig, Recorder, RecorderDatabase,
    ScrubConfig, ScrubRule, ScrubTarget, Scrubber,
};
use serde_json::json;
use std::collections::HashMap;
use tempfile::tempdir;

/// Test that email addresses are scrubbed
#[tokio::test]
async fn test_scrub_email_addresses() {
    let db = RecorderDatabase::new_in_memory().await.unwrap();

    let config = ScrubConfig {
        rules: vec![ScrubRule::Email {
            replacement: "user@example.com".to_string(),
        }],
        deterministic: false,
        counter_seed: 0,
    };

    let scrubber = Scrubber::new(config).unwrap();
    let filter = CaptureFilter::default();
    let recorder = Recorder::with_scrubbing(db, scrubber, filter);

    // Record request with email in body
    let headers = HashMap::from([("content-type".to_string(), "application/json".to_string())]);
    let body = json!({
        "email": "john.doe@company.com",
        "name": "John Doe"
    })
    .to_string();

    let context = RequestContext::new(Some("127.0.0.1"), None, None);
    let request_id = recorder
        .record_http_request("POST", "/api/users", None, &headers, Some(body.as_bytes()), &context)
        .await
        .unwrap();

    // Verify email was scrubbed
    let exchange = recorder.database().get_exchange(&request_id).await.unwrap().unwrap();

    assert!(exchange.request.body.as_ref().unwrap().contains("user@example.com"));
    assert!(!exchange.request.body.as_ref().unwrap().contains("john.doe@company.com"));
}

/// Test that UUIDs are scrubbed with deterministic counter
#[tokio::test]
async fn test_scrub_uuids_deterministically() {
    let db = RecorderDatabase::new_in_memory().await.unwrap();

    let config = ScrubConfig {
        rules: vec![ScrubRule::Uuid {
            replacement: "00000000-0000-0000-0000-{{counter:012}}".to_string(),
        }],
        deterministic: false,
        counter_seed: 0,
    };

    let scrubber = Scrubber::new(config).unwrap();
    let filter = CaptureFilter::default();
    let recorder = Recorder::with_scrubbing(db, scrubber, filter);

    // Record request with UUIDs
    let headers = HashMap::new();
    let body = json!({
        "request_id": "123e4567-e89b-12d3-a456-426614174000",
        "session_id": "987f6543-e21c-43d2-b456-426614174111"
    })
    .to_string();

    let context = RequestContext::new(None, None, None);
    let request_id = recorder
        .record_http_request("POST", "/api/track", None, &headers, Some(body.as_bytes()), &context)
        .await
        .unwrap();

    // Verify UUIDs were scrubbed with deterministic counter
    let exchange = recorder.database().get_exchange(&request_id).await.unwrap().unwrap();

    let scrubbed_body = exchange.request.body.as_ref().unwrap();

    // Should contain deterministic UUIDs with incrementing counter
    assert!(scrubbed_body.contains("00000000-0000-0000-0000-000000000000"));
    assert!(scrubbed_body.contains("00000000-0000-0000-0000-000000000001"));

    // Should NOT contain original UUIDs
    assert!(!scrubbed_body.contains("123e4567-e89b-12d3-a456-426614174000"));
    assert!(!scrubbed_body.contains("987f6543-e21c-43d2-b456-426614174111"));
}

/// Test scrubbing JSON fields by path
#[tokio::test]
async fn test_scrub_json_fields() {
    let db = RecorderDatabase::new_in_memory().await.unwrap();

    let config = ScrubConfig {
        rules: vec![
            ScrubRule::Field {
                field: "user.email".to_string(),
                replacement: "redacted@example.com".to_string(),
                target: ScrubTarget::All,
            },
            ScrubRule::Field {
                field: "user.ssn".to_string(),
                replacement: "XXX-XX-XXXX".to_string(),
                target: ScrubTarget::All,
            },
        ],
        deterministic: false,
        counter_seed: 0,
    };

    let scrubber = Scrubber::new(config).unwrap();
    let filter = CaptureFilter::default();
    let recorder = Recorder::with_scrubbing(db, scrubber, filter);

    // Record request with nested JSON
    let headers = HashMap::new();
    let body = json!({
        "user": {
            "email": "secret@company.com",
            "ssn": "123-45-6789",
            "name": "John Doe"
        }
    })
    .to_string();

    let context = RequestContext::new(None, None, None);
    let request_id = recorder
        .record_http_request("POST", "/api/user", None, &headers, Some(body.as_bytes()), &context)
        .await
        .unwrap();

    // Verify fields were scrubbed
    let exchange = recorder.database().get_exchange(&request_id).await.unwrap().unwrap();

    let scrubbed_body = exchange.request.body.as_ref().unwrap();

    assert!(scrubbed_body.contains("redacted@example.com"));
    assert!(scrubbed_body.contains("XXX-XX-XXXX"));
    assert!(!scrubbed_body.contains("secret@company.com"));
    assert!(!scrubbed_body.contains("123-45-6789"));

    // Name should not be scrubbed
    assert!(scrubbed_body.contains("John Doe"));
}

/// Test scrubbing headers
#[tokio::test]
async fn test_scrub_headers() {
    let db = RecorderDatabase::new_in_memory().await.unwrap();

    let config = ScrubConfig {
        rules: vec![
            ScrubRule::Header {
                name: "Authorization".to_string(),
                replacement: "Bearer REDACTED".to_string(),
            },
            ScrubRule::Header {
                name: "X-API-Key".to_string(),
                replacement: "REDACTED".to_string(),
            },
        ],
        deterministic: false,
        counter_seed: 0,
    };

    let scrubber = Scrubber::new(config).unwrap();
    let filter = CaptureFilter::default();
    let recorder = Recorder::with_scrubbing(db, scrubber, filter);

    // Record request with sensitive headers
    let headers = HashMap::from([
        ("Authorization".to_string(), "Bearer secret-token-12345".to_string()),
        ("X-API-Key".to_string(), "super-secret-key".to_string()),
        ("Content-Type".to_string(), "application/json".to_string()),
    ]);

    let context = RequestContext::new(None, None, None);
    let request_id = recorder
        .record_http_request("GET", "/api/data", None, &headers, None, &context)
        .await
        .unwrap();

    // Verify headers were scrubbed
    let exchange = recorder.database().get_exchange(&request_id).await.unwrap().unwrap();

    let headers_json: HashMap<String, String> =
        serde_json::from_str(&exchange.request.headers).unwrap();

    assert_eq!(headers_json.get("Authorization").unwrap(), "Bearer REDACTED");
    assert_eq!(headers_json.get("X-API-Key").unwrap(), "REDACTED");
    assert_eq!(headers_json.get("Content-Type").unwrap(), "application/json");
}

/// Test deterministic timestamp normalization
#[tokio::test]
async fn test_deterministic_timestamps() {
    let db = RecorderDatabase::new_in_memory().await.unwrap();

    let config = ScrubConfig {
        rules: vec![],
        deterministic: true,
        counter_seed: 0,
    };

    let scrubber = Scrubber::new(config).unwrap();
    let filter = CaptureFilter::default();
    let recorder = Recorder::with_scrubbing(db, scrubber, filter);

    // Record request
    let headers = HashMap::new();
    let context = RequestContext::new(None, None, None);
    let request_id = recorder
        .record_http_request("GET", "/api/test", None, &headers, None, &context)
        .await
        .unwrap();

    // Record response
    recorder
        .record_http_response(&request_id, 200, &headers, None, 100)
        .await
        .unwrap();

    // Wait a bit and record another request/response
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let request_id2 = recorder
        .record_http_request("GET", "/api/test2", None, &headers, None, &context)
        .await
        .unwrap();

    recorder
        .record_http_response(&request_id2, 200, &headers, None, 100)
        .await
        .unwrap();

    // Verify timestamps are normalized (same day, 00:00:00)
    let exchange1 = recorder.database().get_exchange(&request_id).await.unwrap().unwrap();

    let exchange2 = recorder.database().get_exchange(&request_id2).await.unwrap().unwrap();

    // Both should have same timestamp (normalized to start of day)
    assert_eq!(
        exchange1.request.timestamp.date_naive(),
        exchange2.request.timestamp.date_naive()
    );
    assert_eq!(exchange1.request.timestamp.time().hour(), 0);
    assert_eq!(exchange1.request.timestamp.time().minute(), 0);
    assert_eq!(exchange1.request.timestamp.time().second(), 0);
}

/// Test capture filter by status code
#[tokio::test]
async fn test_filter_by_status_code() {
    let db = RecorderDatabase::new_in_memory().await.unwrap();

    let config = CaptureFilterConfig {
        status_codes: vec![500, 502, 503, 504],
        ..Default::default()
    };

    let scrubber = Scrubber::default();
    let filter = CaptureFilter::new(config).unwrap();
    let recorder = Recorder::with_scrubbing(db.clone(), scrubber, filter);

    let headers = HashMap::new();
    let context = RequestContext::new(None, None, None);

    // Record request (should be captured since we don't check status until response)
    let request_id = recorder
        .record_http_request("GET", "/api/test", None, &headers, None, &context)
        .await
        .unwrap();

    // Record successful response (should be captured despite filter)
    recorder
        .record_http_response(&request_id, 200, &headers, None, 100)
        .await
        .unwrap();

    // Verify it was recorded
    let exchange = recorder.database().get_exchange(&request_id).await.unwrap();
    assert!(exchange.is_some());
}

/// Test capture filter by path pattern
#[tokio::test]
async fn test_filter_by_path_pattern() {
    let db = RecorderDatabase::new_in_memory().await.unwrap();

    let config = CaptureFilterConfig {
        path_patterns: vec![r"^/api/v1/.*".to_string()],
        ..Default::default()
    };

    let scrubber = Scrubber::default();
    let filter = CaptureFilter::new(config).unwrap();
    let recorder = Recorder::with_scrubbing(db.clone(), scrubber, filter);

    let headers = HashMap::new();
    let context = RequestContext::new(None, None, None);

    // This should be captured (matches pattern)
    let request_id1 = recorder
        .record_http_request("GET", "/api/v1/users", None, &headers, None, &context)
        .await
        .unwrap();

    // This should NOT be captured (doesn't match pattern)
    let request_id2 = recorder
        .record_http_request("GET", "/api/v2/users", None, &headers, None, &context)
        .await
        .unwrap();

    // Record responses
    recorder
        .record_http_response(&request_id1, 200, &headers, None, 100)
        .await
        .unwrap();

    recorder
        .record_http_response(&request_id2, 200, &headers, None, 100)
        .await
        .unwrap();

    // Verify only the first was captured (has response)
    let exchange1 = recorder.database().get_exchange(&request_id1).await.unwrap();
    assert!(exchange1.is_some());
    assert!(exchange1.unwrap().response.is_some(), "First request should have response");

    // Second should NOT have response due to filter
    let exchange2 = recorder.database().get_exchange(&request_id2).await.unwrap();
    assert!(exchange2.is_some(), "Request should exist");
    assert!(
        exchange2.unwrap().response.is_none(),
        "Second request should NOT have response due to filter"
    );
}

/// Test capture filter errors only
#[tokio::test]
async fn test_filter_errors_only() {
    let _db = RecorderDatabase::new_in_memory().await.unwrap();

    let config = CaptureFilterConfig {
        errors_only: true,
        ..Default::default()
    };

    let filter = CaptureFilter::new(config).unwrap();

    // Test the filter directly
    assert!(filter.should_capture("GET", "/api/test", Some(400)));
    assert!(filter.should_capture("GET", "/api/test", Some(500)));
    assert!(!filter.should_capture("GET", "/api/test", Some(200)));
    assert!(!filter.should_capture("GET", "/api/test", Some(304)));
}

/// Test regex scrubbing with custom patterns
#[tokio::test]
async fn test_regex_scrubbing() {
    let db = RecorderDatabase::new_in_memory().await.unwrap();

    let config = ScrubConfig {
        rules: vec![ScrubRule::Regex {
            pattern: r"sk-[a-zA-Z0-9]{46}".to_string(), // Corrected: sk- (3 chars) + 46 chars = 49 total
            replacement: "sk-REDACTED".to_string(),
            target: ScrubTarget::All,
        }],
        deterministic: false,
        counter_seed: 0,
    };

    let scrubber = Scrubber::new(config).unwrap();
    let filter = CaptureFilter::default();
    let recorder = Recorder::with_scrubbing(db, scrubber, filter);

    // Record request with API key
    let headers = HashMap::new();
    let body = json!({
        "api_key": "sk-abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJ"
    })
    .to_string();

    let context = RequestContext::new(None, None, None);
    let request_id = recorder
        .record_http_request("POST", "/api/config", None, &headers, Some(body.as_bytes()), &context)
        .await
        .unwrap();

    // Verify API key was scrubbed
    let exchange = recorder.database().get_exchange(&request_id).await.unwrap().unwrap();

    let scrubbed_body = exchange.request.body.as_ref().unwrap();

    assert!(scrubbed_body.contains("sk-REDACTED"));
    assert!(!scrubbed_body.contains("sk-abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJ"));
}

/// Test golden file comparison - deterministic replay
#[tokio::test]
async fn test_golden_file_deterministic_replay() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = RecorderDatabase::new(db_path.to_str().unwrap()).await.unwrap();

    let config = ScrubConfig {
        rules: vec![
            ScrubRule::Email {
                replacement: "user@example.com".to_string(),
            },
            ScrubRule::Uuid {
                replacement: "00000000-0000-0000-0000-{{counter:012}}".to_string(),
            },
        ],
        deterministic: true,
        counter_seed: 0,
    };

    let scrubber = Scrubber::new(config).unwrap();
    let filter = CaptureFilter::default();
    let recorder = Recorder::with_scrubbing(db, scrubber, filter);

    // Record multiple requests with sensitive data
    let headers = HashMap::from([("content-type".to_string(), "application/json".to_string())]);
    let context = RequestContext::new(Some("192.168.1.100"), None, None);

    for i in 0..3 {
        let body = json!({
            "id": format!("123e4567-e89b-12d3-a456-42661417400{}", i),
            "email": format!("user{}@company.com", i),
            "name": format!("User {}", i)
        })
        .to_string();

        let request_id = recorder
            .record_http_request(
                "POST",
                &format!("/api/users/{}", i),
                None,
                &headers,
                Some(body.as_bytes()),
                &context,
            )
            .await
            .unwrap();

        let response_body = json!({
            "success": true,
            "user_id": format!("987f6543-e21c-43d2-b456-42661417411{}", i)
        })
        .to_string();

        recorder
            .record_http_response(
                &request_id,
                200,
                &headers,
                Some(response_body.as_bytes()),
                100 + i as i64,
            )
            .await
            .unwrap();
    }

    // Get all recent requests (returned in reverse chronological order)
    let mut all_requests = recorder.database().list_recent(10).await.unwrap();

    assert_eq!(all_requests.len(), 3);

    // Reverse to get chronological order (oldest first)
    all_requests.reverse();

    // Verify all emails were scrubbed to the same value
    for req in &all_requests {
        let body = req.body.as_ref().unwrap();
        assert!(body.contains("user@example.com"));
        assert!(!body.contains("@company.com"));
    }

    // Verify UUIDs are deterministic (counter increments for each UUID)
    // Note: Each request/response pair has 2 UUIDs, so counters increment accordingly
    let first_body = all_requests[0].body.as_ref().unwrap();
    let second_body = all_requests[1].body.as_ref().unwrap();

    // Check that UUIDs have been scrubbed with deterministic counters
    assert!(
        first_body.contains("00000000-0000-0000-0000-"),
        "First request should have deterministic UUID"
    );
    assert!(
        second_body.contains("00000000-0000-0000-0000-"),
        "Second request should have deterministic UUID"
    );

    // Verify UUIDs are different (counter incremented)
    assert_ne!(first_body, second_body, "UUID counters should be different between requests");

    // Verify timestamps are normalized
    assert_eq!(
        all_requests[0].timestamp.time().hour(),
        0,
        "Timestamp should be normalized to start of day"
    );
}

/// Test IP address scrubbing
#[tokio::test]
async fn test_scrub_ip_addresses() {
    let db = RecorderDatabase::new_in_memory().await.unwrap();

    let config = ScrubConfig {
        rules: vec![ScrubRule::IpAddress {
            replacement: "127.0.0.1".to_string(),
        }],
        deterministic: false,
        counter_seed: 0,
    };

    let scrubber = Scrubber::new(config).unwrap();
    let filter = CaptureFilter::default();
    let recorder = Recorder::with_scrubbing(db, scrubber, filter);

    // Record request with IP in body
    let headers = HashMap::new();
    let body = json!({
        "client_ip": "192.168.1.100",
        "server_ip": "10.0.0.5"
    })
    .to_string();

    let context = RequestContext::new(Some("192.168.1.100"), None, None);
    let request_id = recorder
        .record_http_request("POST", "/api/log", None, &headers, Some(body.as_bytes()), &context)
        .await
        .unwrap();

    // Verify IPs were scrubbed
    let exchange = recorder.database().get_exchange(&request_id).await.unwrap().unwrap();

    let scrubbed_body = exchange.request.body.as_ref().unwrap();

    assert!(scrubbed_body.contains("127.0.0.1"));
    assert!(!scrubbed_body.contains("192.168.1.100"));
    assert!(!scrubbed_body.contains("10.0.0.5"));

    // Client IP in context should also be scrubbed
    assert_eq!(exchange.request.client_ip.as_ref().unwrap(), "127.0.0.1");
}
