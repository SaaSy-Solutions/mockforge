//! # MockForge GraphQL
//!
//! GraphQL mocking library for MockForge with schema-based query execution.
//!
//! This crate provides GraphQL mocking capabilities including:
//!
//! - **Schema-Based Mocking**: Define GraphQL schemas and automatically generate resolvers
//! - **Query & Mutation Support**: Handle queries, mutations, and subscriptions
//! - **Type System**: Full GraphQL type system support (scalars, objects, interfaces, unions)
//! - **Introspection**: Built-in introspection queries for tooling
//! - **Playground Integration**: GraphQL Playground UI for interactive testing
//!
//! ## Overview
//!
//! MockForge GraphQL allows you to define GraphQL schemas and automatically mock
//! resolvers with realistic data. Perfect for frontend development and integration testing.
//!
//! ## Quick Start
//!
//! ### Basic GraphQL Server
//!
//! ```rust,no_run
//! use mockforge_graphql::start;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     // Start GraphQL server on port 4000
//!     start(4000).await?;
//!     Ok(())
//! }
//! ```
//!
//! ### With Custom Schema
//!
//! ```rust,no_run
//! use mockforge_graphql::{GraphQLSchema, GraphQLExecutor, create_graphql_router};
//! use mockforge_core::LatencyProfile;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! let schema = GraphQLSchema::from_file("schema.graphql").await?;
//! let latency = Some(LatencyProfile::fast());
//! let router = create_graphql_router(latency).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## GraphQL Schema Example
//!
//! Define your GraphQL schema:
//!
//! ```graphql
//! type Query {
//!   user(id: ID!): User
//!   users(limit: Int = 10): [User!]!
//! }
//!
//! type Mutation {
//!   createUser(input: CreateUserInput!): User!
//!   updateUser(id: ID!, input: UpdateUserInput!): User!
//! }
//!
//! type User {
//!   id: ID!
//!   name: String!
//!   email: String!
//!   posts: [Post!]!
//! }
//!
//! type Post {
//!   id: ID!
//!   title: String!
//!   content: String!
//!   author: User!
//! }
//!
//! input CreateUserInput {
//!   name: String!
//!   email: String!
//! }
//! ```
//!
//! MockForge automatically generates resolvers with realistic data:
//!
//! ```bash
//! # Query
//! curl -X POST http://localhost:4000/graphql \
//!   -H "Content-Type: application/json" \
//!   -d '{"query": "{ user(id: \"123\") { id name email } }"}'
//!
//! # Mutation
//! curl -X POST http://localhost:4000/graphql \
//!   -H "Content-Type: application/json" \
//!   -d '{"query": "mutation { createUser(input: {name: \"Alice\", email: \"alice@example.com\"}) { id } }"}'
//! ```
//!
//! ## GraphQL Playground
//!
//! Access the interactive GraphQL Playground at:
//! ```
//! http://localhost:4000/playground
//! ```
//!
//! The Playground provides:
//! - Schema explorer
//! - Query editor with auto-complete
//! - Response viewer
//! - Request history
//!
//! ## Features
//!
//! ### Automatic Resolver Generation
//! - Generates realistic data based on field names and types
//! - Maintains referential integrity between related types
//! - Supports nested queries and relationships
//!
//! ### Latency Simulation
//! - Simulate network delays for realistic testing
//! - Per-resolver latency configuration
//! - Random or fixed latency profiles
//!
//! ### Error Injection
//! - Simulate GraphQL errors and partial responses
//! - Configure error rates per resolver
//! - Test error handling in client applications
//!
//! ## Key Modules
//!
//! - [`executor`]: GraphQL query execution engine
//! - [`schema`]: Schema parsing and validation
//! - [`registry`]: Type and resolver registry
//! - [`graphql_tracing`]: Distributed tracing integration
//!
//! ## Examples
//!
//! See the [examples directory](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples)
//! for complete working examples.
//!
//! ## Related Crates
//!
//! - [`mockforge-core`](https://docs.rs/mockforge-core): Core mocking functionality
//! - [`mockforge-data`](https://docs.rs/mockforge-data): Synthetic data generation
//!
//! ## Documentation
//!
//! - [MockForge Book](https://docs.mockforge.dev/)
//! - [GraphQL Mocking Guide](https://docs.mockforge.dev/user-guide/graphql-mocking.html)
//! - [API Reference](https://docs.rs/mockforge-graphql)

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
    create_graphql_span, create_resolver_span, record_graphql_error, record_graphql_success,
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
