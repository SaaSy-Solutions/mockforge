//! Integration tests for GraphQL handlers

use async_graphql::{Response, Value, Variables};
use async_trait::async_trait;
use mockforge_graphql::{
    GraphQLContext, GraphQLHandler, HandlerRegistry, HandlerResult, OperationType,
};

/// Test handler that returns mock data for "getUser" operation
struct TestUserHandler;

#[async_trait]
impl GraphQLHandler for TestUserHandler {
    async fn on_operation(&self, ctx: &GraphQLContext) -> HandlerResult<Option<Response>> {
        if ctx.operation_name.as_deref() == Some("getUser") {
            // Return mock user data
            Ok(Some(Response::new(Value::Null))) // Simplified for testing
        } else {
            Ok(None)
        }
    }

    fn handles_operation(&self, operation_name: Option<&str>, _: &OperationType) -> bool {
        operation_name == Some("getUser")
    }

    fn priority(&self) -> i32 {
        10
    }
}

#[tokio::test]
async fn test_handler_registry_creation() {
    let registry = HandlerRegistry::new();
    assert!(registry.upstream_url().is_none());
}

#[tokio::test]
async fn test_handler_registry_with_upstream() {
    let upstream = "http://example.com/graphql".to_string();
    let registry = HandlerRegistry::with_upstream(Some(upstream.clone()));
    assert_eq!(registry.upstream_url(), Some(upstream.as_str()));
}

#[tokio::test]
async fn test_handler_registration() {
    let mut registry = HandlerRegistry::new();
    registry.register(TestUserHandler);

    // Get handlers for the operation
    let handlers = registry.get_handlers(Some("getUser"), &OperationType::Query);
    assert_eq!(handlers.len(), 1);
}

#[tokio::test]
async fn test_handler_execution() {
    let mut registry = HandlerRegistry::new();
    registry.register(TestUserHandler);

    let ctx = GraphQLContext::new(
        Some("getUser".to_string()),
        OperationType::Query,
        "query { user(id: \"123\") { id name } }".to_string(),
        Variables::default(),
    );

    let result = registry.execute_operation(&ctx).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.is_some());
}

#[tokio::test]
async fn test_handler_does_not_match() {
    let mut registry = HandlerRegistry::new();
    registry.register(TestUserHandler);

    let ctx = GraphQLContext::new(
        Some("getProduct".to_string()),
        OperationType::Query,
        "query { product(id: \"456\") { id name } }".to_string(),
        Variables::default(),
    );

    let result = registry.execute_operation(&ctx).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    // Should return None since no handler matches "getProduct"
    assert!(response.is_none());
}

#[tokio::test]
async fn test_graphql_context_creation() {
    let ctx = GraphQLContext::new(
        Some("getUser".to_string()),
        OperationType::Query,
        "query { user { id } }".to_string(),
        Variables::default(),
    );

    assert_eq!(ctx.operation_name, Some("getUser".to_string()));
    assert_eq!(ctx.operation_type, OperationType::Query);
    assert!(ctx.metadata.is_empty());
    assert!(ctx.data.is_empty());
}

#[tokio::test]
async fn test_graphql_context_metadata() {
    let mut ctx = GraphQLContext::new(
        Some("getUser".to_string()),
        OperationType::Query,
        "query { user { id } }".to_string(),
        Variables::default(),
    );

    ctx.set_metadata("Authorization".to_string(), "Bearer token".to_string());
    assert_eq!(ctx.get_metadata("Authorization"), Some(&"Bearer token".to_string()));
}

#[tokio::test]
async fn test_graphql_context_custom_data() {
    let mut ctx = GraphQLContext::new(
        Some("getUser".to_string()),
        OperationType::Query,
        "query { user { id } }".to_string(),
        Variables::default(),
    );

    ctx.set_data("custom_key".to_string(), serde_json::json!({"test": "value"}));
    assert_eq!(ctx.get_data("custom_key"), Some(&serde_json::json!({"test": "value"})));
}

#[test]
fn test_operation_type_equality() {
    assert_eq!(OperationType::Query, OperationType::Query);
    assert_eq!(OperationType::Mutation, OperationType::Mutation);
    assert_eq!(OperationType::Subscription, OperationType::Subscription);
    assert_ne!(OperationType::Query, OperationType::Mutation);
}
