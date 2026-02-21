//! Integration tests for playground API endpoints

use mockforge_ui::handlers::playground::*;
use mockforge_ui::handlers::AdminState;
use serde_json::json;

#[tokio::test]
async fn test_list_playground_endpoints() {
    // Create a minimal AdminState for testing
    let state = AdminState::new(
        Some("127.0.0.1:3000".parse().unwrap()),
        None,
        None,
        Some("127.0.0.1:4000".parse().unwrap()),
        true,
        8080,
        None,
        None,
        None,
        None,
        None,
    );

    // Note: This test requires a running HTTP server, so it's more of an integration test
    // In a real scenario, we'd mock the HTTP client or use a test server
    let response = list_playground_endpoints(
        axum::extract::State(state),
        axum::extract::Query(std::collections::HashMap::new()),
    )
    .await;

    // Verify response structure
    let response_value = serde_json::to_value(&*response).unwrap();
    assert!(response_value.get("success").is_some() || response_value.get("data").is_some());
}

#[tokio::test]
async fn test_code_snippet_generation_rest() {
    let state = AdminState::new(
        Some("127.0.0.1:3000".parse().unwrap()),
        None,
        None,
        None,
        true,
        8080,
        None,
        None,
        None,
        None,
        None,
    );

    let request = CodeSnippetRequest {
        protocol: "rest".to_string(),
        method: Some("POST".to_string()),
        path: "/api/users".to_string(),
        headers: Some({
            let mut h = std::collections::HashMap::new();
            h.insert("Content-Type".to_string(), "application/json".to_string());
            h
        }),
        body: Some(json!({ "name": "John" })),
        graphql_query: None,
        graphql_variables: None,
        base_url: "http://localhost:3000".to_string(),
    };

    let response =
        generate_code_snippet(axum::extract::State(state), axum::extract::Json(request)).await;

    let response_value = serde_json::to_value(&*response).unwrap();
    let data = response_value.get("data").and_then(|d| d.get("snippets"));

    assert!(data.is_some());
    let snippets = data.unwrap().as_object().unwrap();

    // Verify curl snippet exists
    assert!(snippets.contains_key("curl"));

    // Verify JavaScript snippet exists
    assert!(snippets.contains_key("javascript"));

    // Verify Python snippet exists
    assert!(snippets.contains_key("python"));

    // Verify curl snippet contains expected elements
    let curl_snippet = snippets.get("curl").unwrap().as_str().unwrap();
    assert!(curl_snippet.contains("curl"));
    assert!(curl_snippet.contains("POST"));
    assert!(curl_snippet.contains("/api/users"));
}

#[tokio::test]
async fn test_code_snippet_generation_graphql() {
    let state = AdminState::new(
        None,
        None,
        None,
        Some("127.0.0.1:4000".parse().unwrap()),
        true,
        8080,
        None,
        None,
        None,
        None,
        None,
    );

    let request = CodeSnippetRequest {
        protocol: "graphql".to_string(),
        method: None,
        path: "/graphql".to_string(),
        headers: None,
        body: None,
        graphql_query: Some("query { user(id: 1) { name } }".to_string()),
        graphql_variables: None,
        base_url: "http://localhost:4000".to_string(),
    };

    let response =
        generate_code_snippet(axum::extract::State(state), axum::extract::Json(request)).await;

    let response_value = serde_json::to_value(&*response).unwrap();
    let data = response_value.get("data").and_then(|d| d.get("snippets"));

    assert!(data.is_some());
    let snippets = data.unwrap().as_object().unwrap();

    // Verify curl snippet exists
    assert!(snippets.contains_key("curl"));

    // Verify JavaScript snippet exists
    assert!(snippets.contains_key("javascript"));

    // Verify GraphQL query is in snippets
    let curl_snippet = snippets.get("curl").unwrap().as_str().unwrap();
    assert!(curl_snippet.contains("graphql"));
}
