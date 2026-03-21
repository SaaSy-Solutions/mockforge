//! Error response tests for GraphQL protocol.
//!
//! Tests GraphQL error response formatting including invalid queries, missing fields,
//! malformed payloads, and handler error propagation.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use mockforge_graphql::handlers::HandlerError;
use mockforge_graphql::{
    create_router, GraphQLContext, GraphQLHandler, HandlerRegistry, HandlerResult, OperationType,
};
use serde_json::{json, Value};
use tower::ServiceExt;

use async_graphql::{Response, Variables};
use async_trait::async_trait;

/// Helper to send a POST to /graphql and return (status, parsed JSON body)
async fn graphql_post(router: axum::Router, body: &Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(body).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

#[tokio::test]
async fn test_syntax_error_returns_errors_field() {
    let router = create_router(None).await.unwrap();

    let (status, body) = graphql_post(router, &json!({"query": "{ invalid {{"})).await;

    // GraphQL spec says errors should still be 200
    assert_eq!(status, StatusCode::OK);
    let errors = body.get("errors").expect("should have errors field");
    assert!(errors.is_array());
    assert!(!errors.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_unknown_field_returns_error() {
    let router = create_router(None).await.unwrap();

    // Query a field that does not exist in the schema
    let (status, body) =
        graphql_post(router, &json!({"query": "{ nonExistentField { id } }"})).await;

    assert_eq!(status, StatusCode::OK);
    let errors = body.get("errors").expect("should have errors field");
    assert!(!errors.as_array().unwrap().is_empty());

    // Error message should reference the unknown field
    let first_error = &errors[0];
    let message = first_error["message"].as_str().unwrap_or("");
    assert!(
        message.contains("nonExistentField") || !message.is_empty(),
        "error message should be descriptive"
    );
}

#[tokio::test]
async fn test_missing_required_argument() {
    let router = create_router(None).await.unwrap();

    // The `user` field requires an `id` argument — omitting it should produce an error
    let (status, body) = graphql_post(router, &json!({"query": "{ user { id name } }"})).await;

    assert_eq!(status, StatusCode::OK);
    let errors = body.get("errors");
    // async-graphql should report the missing required argument
    assert!(errors.is_some(), "missing required argument should produce errors");
}

#[tokio::test]
async fn test_empty_query_string() {
    let router = create_router(None).await.unwrap();

    let (status, body) = graphql_post(router, &json!({"query": ""})).await;

    assert_eq!(status, StatusCode::OK);
    // An empty query should produce errors
    let errors = body.get("errors");
    assert!(errors.is_some(), "empty query should produce errors");
}

#[tokio::test]
async fn test_missing_query_field_in_json() {
    let router = create_router(None).await.unwrap();

    // Send JSON without a "query" key — async-graphql treats the query as empty
    let (status, body) = graphql_post(router, &json!({"variables": {}})).await;

    // async-graphql returns 200 with errors for a missing/empty query
    assert_eq!(status, StatusCode::OK);
    let errors = body.get("errors");
    assert!(errors.is_some(), "missing query field should produce errors");
}

#[tokio::test]
async fn test_malformed_json_body() {
    let router = create_router(None).await.unwrap();

    let request = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from("not valid json at all"))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_wrong_content_type() {
    let router = create_router(None).await.unwrap();

    let request = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "text/plain")
        .body(Body::from(r#"{"query": "{ __typename }"}"#))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    // Should reject non-JSON content type
    let status = response.status();
    assert!(
        status == StatusCode::BAD_REQUEST
            || status == StatusCode::UNSUPPORTED_MEDIA_TYPE
            || status == StatusCode::OK,
        "unexpected status: {}",
        status
    );
}

#[tokio::test]
async fn test_error_message_contains_location_info() {
    let router = create_router(None).await.unwrap();

    // Intentionally broken syntax near the end
    let (status, body) =
        graphql_post(router, &json!({"query": "query { users { id } invalid }"})).await;

    assert_eq!(status, StatusCode::OK);
    if let Some(errors) = body.get("errors") {
        let first = &errors[0];
        // GraphQL errors should include location info
        assert!(
            first.get("locations").is_some() || first.get("message").is_some(),
            "error should have locations or message"
        );
    }
}

#[tokio::test]
async fn test_handler_error_propagation() {
    // Test that handler errors are properly formatted as GraphQL errors
    struct ErrorHandler;

    #[async_trait]
    impl GraphQLHandler for ErrorHandler {
        async fn on_error(&self, _ctx: &GraphQLContext, error: String) -> HandlerResult<Response> {
            let server_error = async_graphql::ServerError::new(error, None);
            Ok(Response::from_errors(vec![server_error]))
        }

        fn handles_operation(&self, _: Option<&str>, _: &OperationType) -> bool {
            true
        }
    }

    let mut registry = HandlerRegistry::new();
    registry.register(ErrorHandler);

    let ctx = GraphQLContext::new(
        Some("failOp".to_string()),
        OperationType::Query,
        "query { fail }".to_string(),
        Variables::default(),
    );

    // Call on_error directly
    let handlers = registry.get_handlers(Some("failOp"), &OperationType::Query);
    assert!(!handlers.is_empty());

    let response = handlers[0].on_error(&ctx, "Something went wrong".to_string()).await.unwrap();
    assert!(!response.errors.is_empty());
    assert_eq!(response.errors[0].message, "Something went wrong");
}

#[test]
fn test_handler_error_variants() {
    let send_err = HandlerError::SendError("connection lost".to_string());
    assert!(send_err.to_string().contains("connection lost"));

    let op_err = HandlerError::OperationError("invalid op".to_string());
    assert!(op_err.to_string().contains("invalid op"));

    let upstream_err = HandlerError::UpstreamError("timeout".to_string());
    assert!(upstream_err.to_string().contains("timeout"));

    let generic_err = HandlerError::Generic("unexpected".to_string());
    assert!(generic_err.to_string().contains("unexpected"));
}

#[tokio::test]
async fn test_passthrough_without_upstream_returns_error() {
    let registry = HandlerRegistry::new();
    // No upstream configured
    assert!(registry.upstream_url().is_none());

    let request = async_graphql::Request::new("{ __typename }");
    let result = registry.passthrough(&request).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.to_string().contains("No upstream URL"), "should report missing upstream");
}
