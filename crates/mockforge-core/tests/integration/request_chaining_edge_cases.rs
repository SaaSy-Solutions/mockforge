//! Edge case tests for request chaining
//!
//! These tests cover error paths, edge cases, and boundary conditions
//! for request chaining functionality.

use mockforge_core::request_chaining::{ChainContext, ChainResponse, RequestBody};
use serde_json::json;

/// Test ChainContext creation and basic operations
#[test]
fn test_chain_context_new() {
    let context = ChainContext::new();
    assert!(context.responses.is_empty());
    assert!(context.variables.is_empty());
    assert!(context.metadata.is_empty());
}

/// Test ChainContext default
#[test]
fn test_chain_context_default() {
    let context = ChainContext::default();
    assert!(context.responses.is_empty());
}

/// Test storing and retrieving responses
#[test]
fn test_chain_context_store_get_response() {
    let mut context = ChainContext::new();

    let response = ChainResponse {
        status: 200,
        headers: std::collections::HashMap::new(),
        body: Some(json!({"id": 123})),
        duration_ms: 150,
        executed_at: "2023-01-01T00:00:00Z".to_string(),
        error: None,
    };

    context.store_response("login".to_string(), response.clone());

    let retrieved = context.get_response("login");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().status, 200);

    // Test non-existent response
    assert!(context.get_response("nonexistent").is_none());
}

/// Test storing and retrieving variables
#[test]
fn test_chain_context_variables() {
    let mut context = ChainContext::new();

    context.set_variable("user_id".to_string(), json!(12345));
    context.set_variable("token".to_string(), json!("abc123"));

    assert_eq!(context.get_variable("user_id"), Some(&json!(12345)));
    assert_eq!(context.get_variable("token"), Some(&json!("abc123")));
    assert!(context.get_variable("nonexistent").is_none());
}

/// Test storing and retrieving metadata
#[test]
fn test_chain_context_metadata() {
    let mut context = ChainContext::new();

    context.set_metadata("chain_id".to_string(), "chain-123".to_string());
    context.set_metadata("execution_mode".to_string(), "sequential".to_string());

    assert_eq!(context.get_metadata("chain_id"), Some(&"chain-123".to_string()));
    assert_eq!(context.get_metadata("execution_mode"), Some(&"sequential".to_string()));
    assert!(context.get_metadata("nonexistent").is_none());
}

/// Test RequestBody JSON creation
#[test]
fn test_request_body_json() {
    let body = RequestBody::json(json!({"name": "test", "value": 42}));

    match body {
        RequestBody::Json(value) => {
            assert_eq!(value.get("name"), Some(&json!("test")));
            assert_eq!(value.get("value"), Some(&json!(42)));
        }
        _ => panic!("Expected Json variant"),
    }
}

/// Test RequestBody binary file creation
#[test]
fn test_request_body_binary_file() {
    let body = RequestBody::binary_file(
        "/path/to/file.bin".to_string(),
        Some("application/octet-stream".to_string()),
    );

    match body {
        RequestBody::BinaryFile { path, content_type } => {
            assert_eq!(path, "/path/to/file.bin");
            assert_eq!(content_type, Some("application/octet-stream".to_string()));
        }
        _ => panic!("Expected BinaryFile variant"),
    }
}

/// Test RequestBody binary file without content type
#[test]
fn test_request_body_binary_file_no_content_type() {
    let body = RequestBody::binary_file("/path/to/file.bin".to_string(), None);

    match body {
        RequestBody::BinaryFile { path, content_type } => {
            assert_eq!(path, "/path/to/file.bin");
            assert!(content_type.is_none());
        }
        _ => panic!("Expected BinaryFile variant"),
    }
}

/// Test RequestBody content type for JSON
#[test]
fn test_request_body_content_type_json() {
    let body = RequestBody::json(json!({}));
    assert_eq!(body.content_type(), Some("application/json"));
}

/// Test RequestBody content type for binary file
#[test]
fn test_request_body_content_type_binary() {
    let body =
        RequestBody::binary_file("/path/to/file.bin".to_string(), Some("image/png".to_string()));
    assert_eq!(body.content_type(), Some("image/png"));
}

/// Test RequestBody content type for binary file without specified type
#[test]
fn test_request_body_content_type_binary_none() {
    let body = RequestBody::binary_file("/path/to/file.bin".to_string(), None);
    assert_eq!(body.content_type(), None);
}

/// Test ChainResponse with all fields
#[test]
fn test_chain_response_complete() {
    let mut headers = std::collections::HashMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());

    let response = ChainResponse {
        status: 201,
        headers,
        body: Some(json!({"created": true})),
        duration_ms: 250,
        executed_at: "2023-01-01T12:00:00Z".to_string(),
        error: None,
    };

    assert_eq!(response.status, 201);
    assert_eq!(response.headers.len(), 1);
    assert_eq!(response.body, Some(json!({"created": true})));
    assert_eq!(response.duration_ms, 250);
    assert_eq!(response.executed_at, "2023-01-01T12:00:00Z");
    assert!(response.error.is_none());
}

/// Test ChainResponse with error
#[test]
fn test_chain_response_with_error() {
    let response = ChainResponse {
        status: 500,
        headers: std::collections::HashMap::new(),
        body: None,
        duration_ms: 100,
        executed_at: "2023-01-01T12:00:00Z".to_string(),
        error: Some("Connection timeout".to_string()),
    };

    assert_eq!(response.status, 500);
    assert!(response.body.is_none());
    assert_eq!(response.error, Some("Connection timeout".to_string()));
}

/// Test ChainResponse with empty body
#[test]
fn test_chain_response_empty_body() {
    let response = ChainResponse {
        status: 204,
        headers: std::collections::HashMap::new(),
        body: None,
        duration_ms: 50,
        executed_at: "2023-01-01T12:00:00Z".to_string(),
        error: None,
    };

    assert_eq!(response.status, 204);
    assert!(response.body.is_none());
}

/// Test ChainContext overwriting responses
#[test]
fn test_chain_context_overwrite_response() {
    let mut context = ChainContext::new();

    let response1 = ChainResponse {
        status: 200,
        headers: std::collections::HashMap::new(),
        body: Some(json!({"version": 1})),
        duration_ms: 100,
        executed_at: "2023-01-01T00:00:00Z".to_string(),
        error: None,
    };

    let response2 = ChainResponse {
        status: 200,
        headers: std::collections::HashMap::new(),
        body: Some(json!({"version": 2})),
        duration_ms: 150,
        executed_at: "2023-01-01T01:00:00Z".to_string(),
        error: None,
    };

    context.store_response("update".to_string(), response1);
    context.store_response("update".to_string(), response2);

    // Should have the latest response
    let retrieved = context.get_response("update").unwrap();
    assert_eq!(retrieved.body, Some(json!({"version": 2})));
}

/// Test ChainContext overwriting variables
#[test]
fn test_chain_context_overwrite_variable() {
    let mut context = ChainContext::new();

    context.set_variable("counter".to_string(), json!(1));
    context.set_variable("counter".to_string(), json!(2));

    assert_eq!(context.get_variable("counter"), Some(&json!(2)));
}

/// Test ChainContext with multiple responses
#[test]
fn test_chain_context_multiple_responses() {
    let mut context = ChainContext::new();

    let response1 = ChainResponse {
        status: 200,
        headers: std::collections::HashMap::new(),
        body: Some(json!({"step": "login"})),
        duration_ms: 100,
        executed_at: "2023-01-01T00:00:00Z".to_string(),
        error: None,
    };

    let response2 = ChainResponse {
        status: 201,
        headers: std::collections::HashMap::new(),
        body: Some(json!({"step": "create"})),
        duration_ms: 200,
        executed_at: "2023-01-01T00:00:01Z".to_string(),
        error: None,
    };

    context.store_response("step1".to_string(), response1);
    context.store_response("step2".to_string(), response2);

    assert_eq!(context.responses.len(), 2);
    assert_eq!(context.get_response("step1").unwrap().status, 200);
    assert_eq!(context.get_response("step2").unwrap().status, 201);
}

/// Test ChainContext with complex JSON variables
#[test]
fn test_chain_context_complex_variables() {
    let mut context = ChainContext::new();

    let complex_value = json!({
        "user": {
            "id": 123,
            "name": "test",
            "roles": ["admin", "user"]
        },
        "metadata": {
            "created": "2023-01-01",
            "tags": ["important"]
        }
    });

    context.set_variable("user_data".to_string(), complex_value.clone());

    let retrieved = context.get_variable("user_data").unwrap();
    assert_eq!(retrieved, &complex_value);
}

/// Test ChainResponse with various status codes
#[test]
fn test_chain_response_status_codes() {
    let status_codes = vec![200, 201, 204, 400, 401, 403, 404, 500, 502, 503];

    for status in status_codes {
        let response = ChainResponse {
            status,
            headers: std::collections::HashMap::new(),
            body: None,
            duration_ms: 0,
            executed_at: "2023-01-01T00:00:00Z".to_string(),
            error: None,
        };

        assert_eq!(response.status, status);
    }
}

/// Test ChainResponse with large body
#[test]
fn test_chain_response_large_body() {
    let large_array: Vec<serde_json::Value> = (0..1000).map(|i| json!({"id": i})).collect();
    let body = json!(large_array);

    let response = ChainResponse {
        status: 200,
        headers: std::collections::HashMap::new(),
        body: Some(body.clone()),
        duration_ms: 500,
        executed_at: "2023-01-01T00:00:00Z".to_string(),
        error: None,
    };

    assert_eq!(response.body, Some(body));
}

/// Test ChainContext with empty string keys
#[test]
fn test_chain_context_empty_keys() {
    let mut context = ChainContext::new();

    let response = ChainResponse {
        status: 200,
        headers: std::collections::HashMap::new(),
        body: None,
        duration_ms: 0,
        executed_at: "2023-01-01T00:00:00Z".to_string(),
        error: None,
    };

    context.store_response("".to_string(), response);
    context.set_variable("".to_string(), json!("empty"));
    context.set_metadata("".to_string(), "empty".to_string());

    // Empty keys should work (though not recommended)
    assert!(context.get_response("").is_some());
    assert!(context.get_variable("").is_some());
    assert!(context.get_metadata("").is_some());
}

/// Test ChainContext with unicode in keys
#[test]
fn test_chain_context_unicode_keys() {
    let mut context = ChainContext::new();

    let response = ChainResponse {
        status: 200,
        headers: std::collections::HashMap::new(),
        body: None,
        duration_ms: 0,
        executed_at: "2023-01-01T00:00:00Z".to_string(),
        error: None,
    };

    context.store_response("测试".to_string(), response);
    context.set_variable("ユーザー".to_string(), json!("value"));

    assert!(context.get_response("测试").is_some());
    assert_eq!(context.get_variable("ユーザー"), Some(&json!("value")));
}

/// Test RequestBody JSON with various value types
#[test]
fn test_request_body_json_types() {
    // Test with string
    let body1 = RequestBody::json(json!("string value"));
    assert_eq!(body1.content_type(), Some("application/json"));

    // Test with number
    let body2 = RequestBody::json(json!(42));
    assert_eq!(body2.content_type(), Some("application/json"));

    // Test with boolean
    let body3 = RequestBody::json(json!(true));
    assert_eq!(body3.content_type(), Some("application/json"));

    // Test with array
    let body4 = RequestBody::json(json!([1, 2, 3]));
    assert_eq!(body4.content_type(), Some("application/json"));

    // Test with null
    let body5 = RequestBody::json(json!(null));
    assert_eq!(body5.content_type(), Some("application/json"));
}

/// Test ChainResponse with various header combinations
#[test]
fn test_chain_response_headers() {
    let mut headers = std::collections::HashMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());
    headers.insert("x-request-id".to_string(), "req-123".to_string());
    headers.insert("cache-control".to_string(), "no-cache".to_string());

    let response = ChainResponse {
        status: 200,
        headers: headers.clone(),
        body: None,
        duration_ms: 0,
        executed_at: "2023-01-01T00:00:00Z".to_string(),
        error: None,
    };

    assert_eq!(response.headers.len(), 3);
    assert_eq!(response.headers.get("content-type"), Some(&"application/json".to_string()));
    assert_eq!(response.headers.get("x-request-id"), Some(&"req-123".to_string()));
    assert_eq!(response.headers.get("cache-control"), Some(&"no-cache".to_string()));
}

/// Test ChainContext variable types
#[test]
fn test_chain_context_variable_types() {
    let mut context = ChainContext::new();

    context.set_variable("string".to_string(), json!("text"));
    context.set_variable("number".to_string(), json!(42));
    context.set_variable("float".to_string(), json!(3.125));
    context.set_variable("boolean".to_string(), json!(true));
    context.set_variable("null".to_string(), json!(null));
    context.set_variable("array".to_string(), json!([1, 2, 3]));
    context.set_variable("object".to_string(), json!({"key": "value"}));

    assert_eq!(context.get_variable("string"), Some(&json!("text")));
    assert_eq!(context.get_variable("number"), Some(&json!(42)));
    assert_eq!(context.get_variable("float"), Some(&json!(3.125)));
    assert_eq!(context.get_variable("boolean"), Some(&json!(true)));
    assert_eq!(context.get_variable("null"), Some(&json!(null)));
    assert_eq!(context.get_variable("array"), Some(&json!([1, 2, 3])));
    assert_eq!(context.get_variable("object"), Some(&json!({"key": "value"})));
}
