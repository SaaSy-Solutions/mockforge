//! MockForge GraphQL - Basic GraphQL mocking
//!
//! This crate provides basic GraphQL mocking capabilities for MockForge.

use mockforge_core::LatencyProfile;

pub mod executor;
pub mod graphql_tracing;
pub mod registry;
pub mod schema;

pub use executor::{create_graphql_router, start_graphql_server, GraphQLExecutor};
pub use registry::GraphQLSchemaRegistry;
pub use schema::GraphQLSchema;

// Re-export tracing utilities
pub use graphql_tracing::{
    create_graphql_span, create_resolver_span,
    record_graphql_error, record_graphql_success,
    record_resolver_error, record_resolver_success,
};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_router_without_latency() {
        let result = create_router(None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_router_with_latency() {
        let latency_profile = LatencyProfile::default();
        let result = create_router(Some(latency_profile)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_router_with_custom_latency() {
        let latency_profile = LatencyProfile::with_normal_distribution(50, 10.0);
        let result = create_router(Some(latency_profile)).await;
        assert!(result.is_ok());
    }
}
