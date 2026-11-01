//! Edge case tests for stateful handler
//!
//! These tests cover error paths, edge cases, and boundary conditions
//! for stateful response handling.

use axum::http::{HeaderMap, Method, Uri};
use mockforge_core::stateful_handler::{
    ResourceIdExtract, StateResponse, StatefulConfig, StatefulResponseHandler, TransitionTrigger,
};
use std::collections::HashMap;

/// Test StatefulResponseHandler creation
#[tokio::test]
async fn test_stateful_handler_new() {
    let handler = StatefulResponseHandler::new();
    assert!(handler.is_ok());
}

// Note: path_matches is a private method, tested through can_handle

/// Test can_handle with no configs
#[tokio::test]
async fn test_can_handle_no_configs() {
    let handler = StatefulResponseHandler::new().unwrap();
    assert!(!handler.can_handle(&Method::GET, "/api/users").await);
}

/// Test can_handle with matching config
#[tokio::test]
async fn test_can_handle_with_config() {
    let handler = StatefulResponseHandler::new().unwrap();

    let mut state_responses = HashMap::new();
    state_responses.insert(
        "initial".to_string(),
        StateResponse {
            status_code: 200,
            headers: HashMap::new(),
            body_template: "{}".to_string(),
            content_type: "application/json".to_string(),
        },
    );

    let config = StatefulConfig {
        resource_id_extract: ResourceIdExtract::PathParam {
            param: "id".to_string(),
        },
        resource_type: "order".to_string(),
        state_responses,
        transitions: vec![],
    };

    handler.add_config("/orders/{id}".to_string(), config).await;
    assert!(handler.can_handle(&Method::GET, "/orders/123").await);
    assert!(!handler.can_handle(&Method::GET, "/api/users").await);
}

// Note: extract_resource_id is a private method, tested through process_request

/// Test process_request with no matching config
#[tokio::test]
async fn test_process_request_no_config() {
    let handler = StatefulResponseHandler::new().unwrap();
    let uri: Uri = "/api/users".parse().unwrap();
    let headers = HeaderMap::new();

    let result = handler.process_request(&Method::GET, &uri, &headers, None).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

/// Test process_request with matching config using PathParam
#[tokio::test]
async fn test_process_request_with_config_path_param() {
    let handler = StatefulResponseHandler::new().unwrap();

    let mut state_responses = HashMap::new();
    state_responses.insert(
        "initial".to_string(),
        StateResponse {
            status_code: 200,
            headers: HashMap::new(),
            body_template: r#"{"state": "initial", "id": "test"}"#.to_string(),
            content_type: "application/json".to_string(),
        },
    );

    let config = StatefulConfig {
        resource_id_extract: ResourceIdExtract::PathParam {
            param: "id".to_string(),
        },
        resource_type: "order".to_string(),
        state_responses,
        transitions: vec![],
    };

    handler.add_config("/orders/{id}".to_string(), config).await;

    let uri: Uri = "/orders/123".parse().unwrap();
    let headers = HeaderMap::new();
    let result = handler.process_request(&Method::GET, &uri, &headers, None).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.is_some());
    let response = response.unwrap();
    assert_eq!(response.status_code, 200);
    assert_eq!(response.state, "initial");
    assert_eq!(response.resource_id, "123");
}

/// Test process_request with Header extraction
#[tokio::test]
async fn test_process_request_with_header_extraction() {
    let handler = StatefulResponseHandler::new().unwrap();

    let mut state_responses = HashMap::new();
    state_responses.insert(
        "initial".to_string(),
        StateResponse {
            status_code: 200,
            headers: HashMap::new(),
            body_template: "{}".to_string(),
            content_type: "application/json".to_string(),
        },
    );

    let config = StatefulConfig {
        resource_id_extract: ResourceIdExtract::Header {
            name: "x-resource-id".to_string(),
        },
        resource_type: "order".to_string(),
        state_responses,
        transitions: vec![],
    };

    handler.add_config("/test".to_string(), config).await;

    let uri: Uri = "/test".parse().unwrap();
    let mut headers = HeaderMap::new();
    headers.insert("x-resource-id", "header-123".parse().unwrap());
    let result = handler.process_request(&Method::GET, &uri, &headers, None).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.is_some());
    assert_eq!(response.unwrap().resource_id, "header-123");
}

/// Test process_request with QueryParam extraction
#[tokio::test]
async fn test_process_request_with_query_param_extraction() {
    let handler = StatefulResponseHandler::new().unwrap();

    let mut state_responses = HashMap::new();
    state_responses.insert(
        "initial".to_string(),
        StateResponse {
            status_code: 200,
            headers: HashMap::new(),
            body_template: "{}".to_string(),
            content_type: "application/json".to_string(),
        },
    );

    let config = StatefulConfig {
        resource_id_extract: ResourceIdExtract::QueryParam {
            param: "id".to_string(),
        },
        resource_type: "order".to_string(),
        state_responses,
        transitions: vec![],
    };

    handler.add_config("/test".to_string(), config).await;

    let uri: Uri = "/test?id=query-456".parse().unwrap();
    let headers = HeaderMap::new();
    let result = handler.process_request(&Method::GET, &uri, &headers, None).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.is_some());
    assert_eq!(response.unwrap().resource_id, "query-456");
}

/// Test process_request with JsonPath extraction
#[tokio::test]
async fn test_process_request_with_json_path_extraction() {
    let handler = StatefulResponseHandler::new().unwrap();

    let mut state_responses = HashMap::new();
    state_responses.insert(
        "initial".to_string(),
        StateResponse {
            status_code: 200,
            headers: HashMap::new(),
            body_template: "{}".to_string(),
            content_type: "application/json".to_string(),
        },
    );

    let config = StatefulConfig {
        resource_id_extract: ResourceIdExtract::JsonPath {
            path: "user.id".to_string(),
        },
        resource_type: "order".to_string(),
        state_responses,
        transitions: vec![],
    };

    handler.add_config("/test".to_string(), config).await;

    let uri: Uri = "/test".parse().unwrap();
    let headers = HeaderMap::new();
    let body = Some(r#"{"user": {"id": "json-789"}}"#.as_bytes());
    let result = handler.process_request(&Method::POST, &uri, &headers, body).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.is_some());
    assert_eq!(response.unwrap().resource_id, "json-789");
}

/// Test process_request with Composite extraction
#[tokio::test]
async fn test_process_request_with_composite_extraction() {
    let handler = StatefulResponseHandler::new().unwrap();

    let mut state_responses = HashMap::new();
    state_responses.insert(
        "initial".to_string(),
        StateResponse {
            status_code: 200,
            headers: HashMap::new(),
            body_template: "{}".to_string(),
            content_type: "application/json".to_string(),
        },
    );

    let config = StatefulConfig {
        resource_id_extract: ResourceIdExtract::Composite {
            extractors: vec![
                ResourceIdExtract::PathParam {
                    param: "id".to_string(),
                },
                ResourceIdExtract::Header {
                    name: "x-resource-id".to_string(),
                },
            ],
        },
        resource_type: "order".to_string(),
        state_responses,
        transitions: vec![],
    };

    handler.add_config("/orders/{id}".to_string(), config).await;

    // Test with path param (first extractor succeeds)
    let uri: Uri = "/orders/123".parse().unwrap();
    let headers = HeaderMap::new();
    let result = handler.process_request(&Method::GET, &uri, &headers, None).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.is_some());
    assert_eq!(response.unwrap().resource_id, "123");

    // The composite extractor tries extractors in order, so PathParam will succeed first
    // This test verifies that the first successful extractor is used
}

/// Test process_request with transition
#[tokio::test]
async fn test_process_request_with_transition() {
    let handler = StatefulResponseHandler::new().unwrap();

    let mut state_responses = HashMap::new();
    state_responses.insert(
        "initial".to_string(),
        StateResponse {
            status_code: 200,
            headers: HashMap::new(),
            body_template: "{}".to_string(),
            content_type: "application/json".to_string(),
        },
    );
    state_responses.insert(
        "processed".to_string(),
        StateResponse {
            status_code: 200,
            headers: HashMap::new(),
            body_template: "{}".to_string(),
            content_type: "application/json".to_string(),
        },
    );

    // Note: Transition triggers need to match the same path pattern as the config
    // The transition is checked within the same config, so both requests need to match
    // the same pattern. Let's use a simpler approach with a single pattern.
    let transitions = vec![TransitionTrigger {
        method: Method::POST,
        path_pattern: "/orders/{id}".to_string(), // Same pattern as config
        from_state: "initial".to_string(),
        to_state: "processed".to_string(),
        condition: None,
    }];

    let config = StatefulConfig {
        resource_id_extract: ResourceIdExtract::PathParam {
            param: "id".to_string(),
        },
        resource_type: "order".to_string(),
        state_responses,
        transitions,
    };

    handler.add_config("/orders/{id}".to_string(), config).await;

    // First request - initial state (GET)
    let uri: Uri = "/orders/123".parse().unwrap();
    let headers = HeaderMap::new();
    let result1 = handler.process_request(&Method::GET, &uri, &headers, None).await;
    assert!(result1.is_ok());
    let response1 = result1.unwrap();
    assert!(response1.is_some());
    assert_eq!(response1.unwrap().state, "initial");

    // Transition request (POST to same path)
    let result2 = handler.process_request(&Method::POST, &uri, &headers, None).await;
    assert!(result2.is_ok());
    let response2 = result2.unwrap();
    assert!(response2.is_some());
    assert_eq!(response2.unwrap().state, "processed");

    // Verify state persisted (GET again)
    let result3 = handler.process_request(&Method::GET, &uri, &headers, None).await;
    assert!(result3.is_ok());
    let response3 = result3.unwrap();
    assert!(response3.is_some());
    assert_eq!(response3.unwrap().state, "processed");
}

// Note: path_matches is a private method, tested through can_handle

/// Test multiple configs with different patterns
#[tokio::test]
async fn test_multiple_configs() {
    let handler = StatefulResponseHandler::new().unwrap();

    let mut state_responses1 = HashMap::new();
    state_responses1.insert(
        "initial".to_string(),
        StateResponse {
            status_code: 200,
            headers: HashMap::new(),
            body_template: "{}".to_string(),
            content_type: "application/json".to_string(),
        },
    );

    let config1 = StatefulConfig {
        resource_id_extract: ResourceIdExtract::PathParam {
            param: "id".to_string(),
        },
        resource_type: "order".to_string(),
        state_responses: state_responses1,
        transitions: vec![],
    };

    let mut state_responses2 = HashMap::new();
    state_responses2.insert(
        "initial".to_string(),
        StateResponse {
            status_code: 200,
            headers: HashMap::new(),
            body_template: "{}".to_string(),
            content_type: "application/json".to_string(),
        },
    );

    let config2 = StatefulConfig {
        resource_id_extract: ResourceIdExtract::PathParam {
            param: "id".to_string(),
        },
        resource_type: "user".to_string(),
        state_responses: state_responses2,
        transitions: vec![],
    };

    handler.add_config("/orders/{id}".to_string(), config1).await;
    handler.add_config("/users/{id}".to_string(), config2).await;

    assert!(handler.can_handle(&Method::GET, "/orders/123").await);
    assert!(handler.can_handle(&Method::GET, "/users/456").await);
    assert!(!handler.can_handle(&Method::GET, "/products/789").await);
}
