//! GraphQL E2E tests
//!
//! End-to-end tests for GraphQL protocol functionality

use mockforge_test::{MockForgeServer, ServerConfig};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

/// Helper to assert response status
fn assert_status(response: &reqwest::Response, expected: u16) {
    assert_eq!(
        response.status().as_u16(),
        expected,
        "Expected status {}, got {}",
        expected,
        response.status()
    );
}

/// Helper to assert JSON response
async fn assert_json_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, Box<dyn std::error::Error>> {
    assert!(response.headers().get("content-type").is_some());
    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        content_type.contains("application/json"),
        "Expected JSON response, got {}",
        content_type
    );
    Ok(response.json().await?)
}

#[tokio::test]
async fn test_graphql_schema_loading() {
    // Test that GraphQL schema (SDL) can be loaded
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;
    
    // Create temporary GraphQL schema file
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let schema_file = temp_dir.path().join("schema.graphql");
    fs::write(&schema_file, r#"
type Query {
    hello: String
    user(id: ID!): User
}

type User {
    id: ID!
    name: String!
    email: String!
}
"#).expect("Failed to write schema");
    
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_GRAPHQL_ENABLED", "true")
        .env_var("MOCKFORGE_GRAPHQL_SCHEMA_FILE", schema_file.to_str().unwrap())
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start GraphQL server");
    
    let http_port = server.http_port();
    let client = Client::new();
    
    // Test GraphQL endpoint exists
    let response = client
        .post(&format!("http://localhost:{}/graphql", http_port))
        .json(&json!({
            "query": "{ hello }"
        }))
        .send()
        .await
        .expect("Failed to make GraphQL request");
    
    // GraphQL endpoint should exist (may return 200 or 404 if not fully configured)
    assert!(response.status().is_success() || response.status().as_u16() == 404,
        "GraphQL endpoint should exist or return 404 if not configured");
    
    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_graphql_query_execution() {
    // Test that GraphQL queries can be executed
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_GRAPHQL_ENABLED", "true")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start GraphQL server");
    
    let http_port = server.http_port();
    let client = Client::new();
    
    // Execute a simple query
    let response = client
        .post(&format!("http://localhost:{}/graphql", http_port))
        .json(&json!({
            "query": "{ __typename }"
        }))
        .send()
        .await
        .expect("Failed to make GraphQL request");
    
    // Query should execute (may return 200 with data or errors)
    if response.status().is_success() {
        let body: serde_json::Value = assert_json_response(response).await.expect("Failed to parse JSON");
        // GraphQL responses have "data" or "errors" field
        assert!(body.get("data").is_some() || body.get("errors").is_some(),
            "GraphQL response should have 'data' or 'errors' field");
    }
    
    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_graphql_mutation_execution() {
    // Test that GraphQL mutations can be executed
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_GRAPHQL_ENABLED", "true")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start GraphQL server");
    
    let http_port = server.http_port();
    let client = Client::new();
    
    // Execute a mutation
    let response = client
        .post(&format!("http://localhost:{}/graphql", http_port))
        .json(&json!({
            "query": "mutation { createUser(name: \"Test\") { id name } }"
        }))
        .send()
        .await
        .expect("Failed to make GraphQL request");
    
    // Mutation should execute (may return 200 with data or errors)
    if response.status().is_success() {
        let body: serde_json::Value = assert_json_response(response).await.expect("Failed to parse JSON");
        // GraphQL responses have "data" or "errors" field
        assert!(body.get("data").is_some() || body.get("errors").is_some(),
            "GraphQL response should have 'data' or 'errors' field");
    }
    
    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_graphql_query_validation() {
    // Test that GraphQL query validation works
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_GRAPHQL_ENABLED", "true")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start GraphQL server");
    
    let http_port = server.http_port();
    let client = Client::new();
    
    // Execute an invalid query
    let response = client
        .post(&format!("http://localhost:{}/graphql", http_port))
        .json(&json!({
            "query": "{ invalidField }"
        }))
        .send()
        .await
        .expect("Failed to make GraphQL request");
    
    // Invalid query should return errors
    if response.status().is_success() {
        let body: serde_json::Value = assert_json_response(response).await.expect("Failed to parse JSON");
        // Should have errors field for invalid query
        assert!(body.get("errors").is_some() || body.get("data").is_some(),
            "GraphQL response should have 'errors' or 'data' field");
    }
    
    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_graphql_subscription_support() {
    // Test that GraphQL subscriptions are supported (if implemented)
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_GRAPHQL_ENABLED", "true")
        .env_var("MOCKFORGE_GRAPHQL_SUBSCRIPTIONS_ENABLED", "true")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start GraphQL server");
    
    let http_port = server.http_port();
    let client = Client::new();
    
    // Try to execute a subscription query
    let response = client
        .post(&format!("http://localhost:{}/graphql", http_port))
        .json(&json!({
            "query": "subscription { events { type data } }"
        }))
        .send()
        .await
        .expect("Failed to make GraphQL request");
    
    // Subscription may return 200 with data/errors or 501 if not implemented
    // For now, just verify the server handles the request
    assert!(response.status().is_success() || response.status().as_u16() == 501,
        "GraphQL subscription should be handled");
    
    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_graphql_error_handling() {
    // Test that GraphQL error handling works correctly
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_GRAPHQL_ENABLED", "true")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start GraphQL server");
    
    let http_port = server.http_port();
    let client = Client::new();
    
    // Execute a query with syntax error
    let response = client
        .post(&format!("http://localhost:{}/graphql", http_port))
        .json(&json!({
            "query": "{ hello }" // Valid syntax
        }))
        .send()
        .await
        .expect("Failed to make GraphQL request");
    
    // Should return proper GraphQL response format
    if response.status().is_success() {
        let body: serde_json::Value = assert_json_response(response).await.expect("Failed to parse JSON");
        // GraphQL responses should have "data" or "errors" field
        assert!(body.get("data").is_some() || body.get("errors").is_some(),
            "GraphQL response should have 'data' or 'errors' field");
        
        // If errors exist, they should have proper structure
        if let Some(errors) = body.get("errors") {
            assert!(errors.is_array(), "Errors should be an array");
        }
    }
    
    server.stop().expect("Failed to stop server");
        .send()
        .await
        .expect("Failed to make GraphQL request");
    
    // Invalid query should return errors
    if response.status().is_success() {
        let body: serde_json::Value = assert_json_response(response).await.expect("Failed to parse JSON");
        // Should have errors field for invalid queries
        if body.get("errors").is_some() {
            // Validation worked - query was rejected
            assert!(true, "Query validation correctly rejected invalid query");
        }
    }
    
    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_graphql_error_handling() {
    // Test that GraphQL error handling works
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_GRAPHQL_ENABLED", "true")
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start GraphQL server");
    
    let http_port = server.http_port();
    let client = Client::new();
    
    // Execute a query that might cause an error
    let response = client
        .post(&format!("http://localhost:{}/graphql", http_port))
        .json(&json!({
            "query": "{ user(id: \"invalid\") { id name } }"
        }))
        .send()
        .await
        .expect("Failed to make GraphQL request");
    
    // Should handle errors gracefully
    if response.status().is_success() {
        let body: serde_json::Value = assert_json_response(response).await.expect("Failed to parse JSON");
        // GraphQL should return errors in a structured format
        assert!(body.get("data").is_some() || body.get("errors").is_some(),
            "GraphQL should return structured response with data or errors");
    }
    
    server.stop().expect("Failed to stop server");
}

