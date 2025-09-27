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
