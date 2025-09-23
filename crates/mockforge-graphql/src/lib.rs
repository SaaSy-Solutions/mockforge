//! MockForge GraphQL - Basic GraphQL mocking
//!
//! This crate provides basic GraphQL mocking capabilities for MockForge.

use mockforge_core::LatencyProfile;

pub mod schema;
pub mod executor;

pub use schema::GraphQLSchema;
pub use executor::{GraphQLExecutor, start_graphql_server, create_graphql_router};

/// Start GraphQL server with default configuration
pub async fn start(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    start_with_latency(port, None).await
}

/// Start GraphQL server with latency configuration
pub async fn start_with_latency(
    port: u16,
    latency_profile: Option<LatencyProfile>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    start_graphql_server(port, latency_profile).await
}

/// Create a GraphQL router with latency support
pub async fn create_router(
    latency_profile: Option<LatencyProfile>,
) -> Result<axum::Router, Box<dyn std::error::Error + Send + Sync>> {
    create_graphql_router(latency_profile).await
}
