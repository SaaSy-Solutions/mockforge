//! End-to-end regression: a real HTTP client (`reqwest`) posts a GraphQL
//! query to the mock server and validates the JSON response against the
//! schema.
//!
//! Existing tests in this crate exercise the Rust-level handler registry
//! and schema builders but never hit the `/graphql` HTTP endpoint with a
//! real request, so a regression in the router / executor / serializer
//! could ship silently. This locks in the on-the-wire contract.

use mockforge_graphql::create_graphql_router;
use serde_json::json;
use std::time::Duration;

async fn spawn_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = create_graphql_router(None).await.expect("router builds cleanly");
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    // Give axum a tick to start accepting.
    tokio::time::sleep(Duration::from_millis(50)).await;
    format!("http://{addr}/graphql")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_real_client_users_query_returns_structured_list() {
    let url = spawn_server().await;

    let client = reqwest::Client::new();
    let body = json!({
        "query": "query($limit: Int!) { users(limit: $limit) { id name email } }",
        "variables": { "limit": 3 },
    });
    let response =
        tokio::time::timeout(Duration::from_secs(5), client.post(&url).json(&body).send())
            .await
            .expect("request should complete within 5s")
            .expect("reqwest transport ok");

    assert!(response.status().is_success(), "HTTP {}", response.status());
    let value: serde_json::Value = response.json().await.expect("response must be JSON");

    // No errors field (or if present, empty array).
    if let Some(errs) = value.get("errors") {
        assert!(
            errs.as_array().map(|a| a.is_empty()).unwrap_or(false),
            "expected no GraphQL errors, got: {errs}"
        );
    }

    let users = value
        .pointer("/data/users")
        .and_then(|v| v.as_array())
        .expect("response.data.users must be an array");
    assert_eq!(users.len(), 3, "limit=3 was requested; got {} users", users.len());

    for (i, u) in users.iter().enumerate() {
        let id = u.pointer("/id").and_then(|v| v.as_str()).unwrap();
        let name = u.pointer("/name").and_then(|v| v.as_str()).unwrap();
        let email = u.pointer("/email").and_then(|v| v.as_str()).unwrap();
        assert_eq!(id, format!("user-{i}"));
        assert_eq!(name, format!("User {i}"));
        assert!(email.ends_with("@example.com"), "unexpected email shape: {email}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_real_client_user_by_id_returns_single_record() {
    let url = spawn_server().await;
    let client = reqwest::Client::new();
    let body = json!({
        "query": r#"{ user(id: "42") { id name email } }"#,
    });
    let response = client.post(&url).json(&body).send().await.expect("reqwest transport ok");
    assert!(response.status().is_success());
    let value: serde_json::Value = response.json().await.unwrap();

    let user = value.pointer("/data/user").expect("data.user present");
    assert_eq!(user.pointer("/id").and_then(|v| v.as_str()), Some("42"));
    assert!(user.pointer("/name").and_then(|v| v.as_str()).is_some());
}

/// Standard introspection query (abridged) exercising `__schema`,
/// `types`, `queryType`, and field shapes — the shape Apollo / GraphiQL /
/// Relay compiler send during schema discovery. Covers the over-the-wire
/// path specifically; existing handler-level coverage lives in
/// `tests/introspection_test.rs` which bypasses axum routing.
const INTROSPECTION_QUERY: &str = r#"
    query IntrospectionQuery {
      __schema {
        queryType { name }
        mutationType { name }
        subscriptionType { name }
        types {
          name
          kind
          fields(includeDeprecated: true) { name }
        }
      }
    }
"#;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_real_client_introspection_query_returns_schema() {
    let url = spawn_server().await;
    let client = reqwest::Client::new();
    let body = json!({ "query": INTROSPECTION_QUERY });
    let response =
        tokio::time::timeout(Duration::from_secs(5), client.post(&url).json(&body).send())
            .await
            .expect("request should complete within 5s")
            .expect("reqwest transport ok");
    assert!(response.status().is_success(), "HTTP {}", response.status());

    let value: serde_json::Value = response.json().await.expect("response must be JSON");
    if let Some(errs) = value.get("errors") {
        assert!(
            errs.as_array().map(|a| a.is_empty()).unwrap_or(false),
            "expected no GraphQL errors, got: {errs}"
        );
    }

    let schema = value.pointer("/data/__schema").expect("response.data.__schema must be present");
    assert_eq!(
        schema.pointer("/queryType/name").and_then(|v| v.as_str()),
        Some("QueryRoot"),
        "queryType name should be QueryRoot"
    );

    // Default schema has EmptyMutation/EmptySubscription, which async-graphql
    // renders as a null type in introspection (not an empty type).
    assert!(
        schema.pointer("/mutationType").map(|v| v.is_null()).unwrap_or(false),
        "default schema has no mutation root; mutationType should be null"
    );
    assert!(
        schema.pointer("/subscriptionType").map(|v| v.is_null()).unwrap_or(false),
        "default schema has no subscription root; subscriptionType should be null"
    );

    let types = schema
        .pointer("/types")
        .and_then(|v| v.as_array())
        .expect("__schema.types must be an array");
    let names: std::collections::HashSet<&str> = types
        .iter()
        .filter_map(|t| t.pointer("/name").and_then(|v| v.as_str()))
        .collect();
    for required in ["QueryRoot", "User", "Post"] {
        assert!(
            names.contains(required),
            "introspection types missing `{required}`; returned: {names:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_syntactically_invalid_query_surfaces_error_field() {
    let url = spawn_server().await;
    let client = reqwest::Client::new();
    let body = json!({ "query": "{ this is not graphql" });
    let response = client.post(&url).json(&body).send().await.expect("reqwest transport ok");
    // GraphQL returns 200 with an errors array for parse errors (per spec).
    let value: serde_json::Value = response.json().await.unwrap();
    let errors = value.pointer("/errors").and_then(|v| v.as_array());
    assert!(
        errors.is_some_and(|e| !e.is_empty()),
        "expected non-empty errors array for invalid query, got: {value}"
    );
}
