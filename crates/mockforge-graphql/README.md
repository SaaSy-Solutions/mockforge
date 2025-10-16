# MockForge GraphQL

GraphQL protocol support for MockForge with schema-based query execution.

This crate provides comprehensive GraphQL mocking capabilities, allowing you to define GraphQL schemas and automatically generate realistic resolvers. Perfect for frontend development, API testing, and GraphQL client development.

## Features

- **Schema-Based Mocking**: Define GraphQL schemas and auto-generate resolvers
- **Query & Mutation Support**: Handle queries, mutations, and subscriptions
- **Full Type System**: Support for scalars, objects, interfaces, unions, enums
- **Introspection**: Built-in GraphQL introspection for tooling integration
- **GraphQL Playground**: Interactive web-based query interface
- **Latency Simulation**: Configurable response delays for realistic testing
- **Error Injection**: Simulate GraphQL errors and partial responses
- **Tracing Integration**: Distributed tracing support with OpenTelemetry

## Quick Start

### Basic GraphQL Server

```rust,no_run
use mockforge_graphql::start;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Start GraphQL server on port 4000
    start(4000).await?;
    Ok(())
}
```

### Server with Custom Schema

```rust,no_run
use mockforge_graphql::{GraphQLSchema, GraphQLExecutor, create_graphql_router};
use mockforge_core::LatencyProfile;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load custom schema
    let schema = GraphQLSchema::from_file("schema.graphql").await?;

    // Configure latency simulation
    let latency = Some(LatencyProfile::fast());

    // Create and start server
    let router = create_graphql_router(latency).await?;
    // ... serve the router
}
```

## GraphQL Schema Definition

Define your GraphQL schema using standard GraphQL SDL (Schema Definition Language):

```graphql
type Query {
  user(id: ID!): User
  users(limit: Int = 10, offset: Int = 0): [User!]!
  posts(userId: ID): [Post!]!
}

type Mutation {
  createUser(input: CreateUserInput!): User!
  updateUser(id: ID!, input: UpdateUserInput!): User!
  deleteUser(id: ID!): Boolean!
}

type Subscription {
  userCreated: User!
  postAdded(userId: ID): Post!
}

type User {
  id: ID!
  name: String!
  email: String!
  avatar: String
  posts: [Post!]!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type Post {
  id: ID!
  title: String!
  content: String!
  author: User!
  tags: [String!]!
  published: Boolean!
  createdAt: DateTime!
  updatedAt: DateTime!
}

input CreateUserInput {
  name: String!
  email: String!
  avatar: String
}

input UpdateUserInput {
  name: String
  email: String
  avatar: String
}

scalar DateTime

enum UserRole {
  ADMIN
  MODERATOR
  USER
}
```

## Automatic Resolver Generation

MockForge GraphQL automatically generates resolvers with realistic data based on field names and types:

### Query Examples

```bash
# Get single user
curl -X POST http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ user(id: \"123\") { id name email avatar } }"}'

# Get users with pagination
curl -X POST http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ users(limit: 5) { id name email posts { title } } }"}'
```

### Mutation Examples

```bash
# Create user
curl -X POST http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation CreateUser($input: CreateUserInput!) { createUser(input: $input) { id name email } }",
    "variables": {
      "input": {
        "name": "Alice Johnson",
        "email": "alice@example.com",
        "avatar": "https://example.com/avatar.jpg"
      }
    }
  }'

# Update user
curl -X POST http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation UpdateUser($id: ID!, $input: UpdateUserInput!) { updateUser(id: $id, input: $input) { id name email } }",
    "variables": {
      "id": "123",
      "input": { "name": "Alice Smith" }
    }
  }'
```

## GraphQL Playground

Access the interactive GraphQL Playground at `http://localhost:4000/playground` for:

- **Schema Exploration**: Browse types, fields, and relationships
- **Query Builder**: Auto-complete with syntax highlighting
- **Documentation**: Inline field and type documentation
- **History**: Save and replay previous queries
- **Response Viewer**: Formatted JSON responses with error highlighting

## Advanced Features

### Latency Simulation

Simulate realistic network conditions:

```rust,no_run
use mockforge_graphql::start_with_latency;
use mockforge_core::LatencyProfile;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Simulate slow API
    let latency = LatencyProfile::slow(); // 300-800ms
    start_with_latency(4000, Some(latency)).await?;
    Ok(())
}
```

### Custom Latency Profiles

```rust,no_run
use mockforge_core::LatencyProfile;

// Fixed delay
let fixed_latency = LatencyProfile::with_fixed_delay(500); // 500ms

// Normal distribution
let normal_latency = LatencyProfile::with_normal_distribution(200, 50.0); // mean 200ms, std dev 50ms

// Custom range
let range_latency = LatencyProfile::with_range(100, 1000); // 100-1000ms
```

### Error Injection

Simulate GraphQL errors:

```rust,no_run
use mockforge_graphql::GraphQLExecutor;

// Configure error injection
let executor = GraphQLExecutor::new(schema)
    .with_error_rate(0.1) // 10% error rate
    .with_error_types(vec![
        "USER_NOT_FOUND",
        "VALIDATION_ERROR",
        "INTERNAL_SERVER_ERROR"
    ]);
```

### Tracing Integration

Enable distributed tracing:

```rust,no_run
use mockforge_graphql::graphql_tracing::{create_graphql_span, record_graphql_success};

// Create spans for monitoring
let span = create_graphql_span("query", "GetUser");

// Execute query...

// Record success
record_graphql_success(&span, 150); // 150ms duration
```

## Schema Registry

Manage multiple GraphQL schemas:

```rust,no_run
use mockforge_graphql::GraphQLSchemaRegistry;

// Create registry
let registry = GraphQLSchemaRegistry::new();

// Register schemas
registry.register_schema("v1", schema_v1).await?;
registry.register_schema("v2", schema_v2).await?;

// Switch between versions
registry.set_active_schema("v2").await?;
```

## Integration with MockForge

MockForge GraphQL integrates seamlessly with the broader MockForge ecosystem:

- **MockForge Core**: Shared configuration and latency profiles
- **MockForge CLI**: Command-line GraphQL server management
- **MockForge Data**: Enhanced data generation for GraphQL responses
- **MockForge Observability**: Metrics and tracing integration

## Configuration

### Server Configuration

```rust,no_run
use mockforge_graphql::GraphQLExecutor;
use mockforge_core::LatencyProfile;

// Configure executor
let executor = GraphQLExecutor::new(schema)
    .with_latency_profile(LatencyProfile::normal())
    .with_max_query_depth(10)
    .with_max_query_complexity(1000)
    .with_introspection_enabled(true)
    .with_playground_enabled(true);
```

### Environment Variables

```bash
# Server configuration
export GRAPHQL_PORT=4000
export GRAPHQL_ENABLE_PLAYGROUND=true
export GRAPHQL_ENABLE_INTROSPECTION=true

# Latency simulation
export GRAPHQL_LATENCY_PROFILE=normal
export GRAPHQL_LATENCY_FIXED_MS=200

# Error injection
export GRAPHQL_ERROR_RATE=0.05
export GRAPHQL_ERROR_TYPES="VALIDATION_ERROR,INTERNAL_ERROR"
```

## Testing GraphQL APIs

Use MockForge GraphQL for comprehensive testing:

### Unit Testing

```rust,no_run
use mockforge_graphql::GraphQLExecutor;

#[tokio::test]
async fn test_user_query() {
    let schema = GraphQLSchema::from_string(SCHEMA).await.unwrap();
    let executor = GraphQLExecutor::new(schema);

    let query = r#"
        query GetUser($id: ID!) {
            user(id: $id) {
                id
                name
                email
            }
        }
    "#;

    let variables = serde_json::json!({ "id": "123" });
    let result = executor.execute(query, Some(variables)).await.unwrap();

    assert!(result.errors.is_empty());
    assert!(result.data.is_object());
}
```

### Integration Testing

```rust,no_run
use reqwest::Client;

#[tokio::test]
async fn test_graphql_endpoint() {
    let client = Client::new();

    let query = serde_json::json!({
        "query": "{ users { id name } }",
        "variables": null
    });

    let response = client
        .post("http://localhost:4000/graphql")
        .json(&query)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let result: serde_json::Value = response.json().await.unwrap();
    assert!(result.get("data").is_some());
}
```

## Performance Considerations

- **Schema Complexity**: Large schemas may impact startup time
- **Query Depth**: Limit maximum query depth to prevent abuse
- **Caching**: Enable response caching for repeated queries
- **Connection Pooling**: Use connection pooling for database resolvers

## Examples

### Complete Server Setup

```rust,no_run
use axum::{routing::get, Router};
use mockforge_graphql::{create_graphql_router, GraphQLSchema};
use mockforge_core::LatencyProfile;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load schema
    let schema = GraphQLSchema::from_file("schema.graphql").await?;

    // Configure latency
    let latency = Some(LatencyProfile::normal());

    // Create GraphQL router
    let graphql_router = create_graphql_router(latency).await?;

    // Add CORS and other middleware
    let app = Router::new()
        .merge(graphql_router)
        .layer(CorsLayer::permissive());

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 4000));
    println!("🚀 GraphQL server running at http://{}", addr);
    println!("📖 GraphQL Playground at http://{}/playground", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}
```

### Custom Resolvers

```rust,no_run
use async_graphql::*;
use mockforge_graphql::GraphQLSchema;

// Define custom resolvers
struct Query;

#[Object]
impl Query {
    async fn custom_user(&self, ctx: &Context<'_>, id: ID) -> Result<User> {
        // Custom logic here
        Ok(User {
            id,
            name: "Custom User".to_string(),
            email: "custom@example.com".to_string(),
        })
    }
}

// Register custom resolvers
let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
    .data(custom_data)
    .finish();
```

## Troubleshooting

### Common Issues

**Schema validation errors:**
- Check GraphQL syntax in your schema files
- Ensure all referenced types are defined
- Validate field names and type references

**Query execution errors:**
- Verify query syntax
- Check variable types match schema
- Ensure query depth doesn't exceed limits

**Performance issues:**
- Profile query execution times
- Check for N+1 query problems
- Optimize resolver implementations

## Related Crates

- [`mockforge-core`](https://docs.rs/mockforge-core): Core mocking functionality
- [`mockforge-data`](https://docs.rs/mockforge-data): Synthetic data generation
- [`async-graphql`](https://docs.rs/async-graphql): Underlying GraphQL implementation

## License

Licensed under MIT OR Apache-2.0
