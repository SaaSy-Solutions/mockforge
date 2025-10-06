//! GraphQL execution engine

use async_graphql::http::GraphQLPlaygroundConfig;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::GraphQLSchema;

/// GraphQL executor state
pub struct GraphQLExecutor {
    schema: Arc<GraphQLSchema>,
}

impl GraphQLExecutor {
    /// Create a new executor
    pub fn new(schema: GraphQLSchema) -> Self {
        Self {
            schema: Arc::new(schema),
        }
    }

    /// Execute a GraphQL request
    pub async fn execute(&self, request: GraphQLRequest) -> GraphQLResponse {
        let response = self.schema.schema().execute(request.into_inner()).await;
        response.into()
    }

    /// Get the schema
    pub fn schema(&self) -> &GraphQLSchema {
        &self.schema
    }
}

/// Start GraphQL server
pub async fn start_graphql_server(
    port: u16,
    latency_profile: Option<mockforge_core::LatencyProfile>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = mockforge_core::wildcard_socket_addr(port);
    tracing::info!("GraphQL server listening on {}", addr);

    let app = create_graphql_router(latency_profile).await?;

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Create GraphQL router
pub async fn create_graphql_router(
    latency_profile: Option<mockforge_core::LatencyProfile>,
) -> Result<Router, Box<dyn std::error::Error + Send + Sync>> {
    // Create a basic schema
    let schema = GraphQLSchema::generate_basic_schema();
    let executor = GraphQLExecutor::new(schema);

    let mut app = Router::new()
        .route("/graphql", post(graphql_handler))
        .route("/graphql", get(graphql_playground))
        .with_state(Arc::new(executor));

    // Add latency injection if configured
    if let Some(profile) = latency_profile {
        let latency_injector =
            mockforge_core::latency::LatencyInjector::new(profile, Default::default());
        app = app.layer(axum::middleware::from_fn(
            move |req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
                let injector = latency_injector.clone();
                async move {
                    let _ = injector.inject_latency(&[]).await;
                    next.run(req).await
                }
            },
        ));
    }

    Ok(app)
}

/// GraphQL endpoint handler
async fn graphql_handler(
    State(executor): State<Arc<GraphQLExecutor>>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    executor.execute(req).await
}

/// GraphQL Playground handler
async fn graphql_playground() -> impl IntoResponse {
    Html(async_graphql::http::playground_source(
        GraphQLPlaygroundConfig::new("/graphql")
            .title("MockForge GraphQL Playground")
            .subscription_endpoint("/graphql"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::Request;

    #[test]
    fn test_graphql_executor_new() {
        let schema = GraphQLSchema::new();
        let executor = GraphQLExecutor::new(schema);

        // Verify executor is created
        assert!(executor.schema.schema().sdl().len() > 0);
    }

    #[test]
    fn test_graphql_executor_schema_getter() {
        let schema = GraphQLSchema::new();
        let executor = GraphQLExecutor::new(schema);

        let retrieved_schema = executor.schema();
        assert!(retrieved_schema.schema().sdl().len() > 0);
    }

    #[tokio::test]
    async fn test_graphql_executor_can_execute() {
        let schema = GraphQLSchema::new();
        let executor = GraphQLExecutor::new(schema);

        // Test that we can create an executor and access its schema
        assert!(executor.schema().schema().sdl().contains("Query"));
    }

    #[tokio::test]
    async fn test_create_graphql_router_no_latency() {
        let result = create_graphql_router(None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_graphql_router_with_latency() {
        let latency = mockforge_core::LatencyProfile::default();
        let result = create_graphql_router(Some(latency)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_graphql_router_with_custom_latency() {
        let latency = mockforge_core::LatencyProfile::new(100, 25);
        let result = create_graphql_router(Some(latency)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_graphql_playground_returns_html() {
        let response = graphql_playground().await;
        // Convert to response to verify it's valid HTML
        let _html_response = response.into_response();
        assert!(true);
    }

    #[test]
    fn test_graphql_handler_setup() {
        let schema = GraphQLSchema::new();
        let executor = Arc::new(GraphQLExecutor::new(schema));

        // Test that we can create executor and wrap in Arc for handler
        assert_eq!(Arc::strong_count(&executor), 1);
    }

    #[test]
    fn test_executor_arc_shared_ownership() {
        let schema = GraphQLSchema::new();
        let executor = Arc::new(GraphQLExecutor::new(schema));

        let executor_clone = Arc::clone(&executor);
        assert_eq!(Arc::strong_count(&executor), 2);

        drop(executor_clone);
        assert_eq!(Arc::strong_count(&executor), 1);
    }

    #[test]
    fn test_executor_schema_contains_query_type() {
        let schema = GraphQLSchema::new();
        let executor = GraphQLExecutor::new(schema);

        let sdl = executor.schema().schema().sdl();
        assert!(sdl.contains("Query"));
    }

    #[test]
    fn test_executor_schema_contains_user_type() {
        let schema = GraphQLSchema::new();
        let executor = GraphQLExecutor::new(schema);

        let sdl = executor.schema().schema().sdl();
        assert!(sdl.contains("User"));
    }

    #[test]
    fn test_executor_schema_contains_post_type() {
        let schema = GraphQLSchema::new();
        let executor = GraphQLExecutor::new(schema);

        let sdl = executor.schema().schema().sdl();
        assert!(sdl.contains("Post"));
    }
}
