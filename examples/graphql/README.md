# GraphQL Operation Handlers Example

This example demonstrates MockForge's native GraphQL operation handlers, similar to MSW's `graphql.query` and `graphql.mutation` patterns.

## Overview

MockForge provides a flexible handler-based system for GraphQL operations:

- **Schema-based**: Load GraphQL schemas from `.graphql` or `.gql` files
- **Operation matching**: Match handlers by query/mutation name
- **Variable filtering**: Filter operations based on variable values
- **Upstream passthrough**: Forward requests to a real GraphQL server
- **Lifecycle hooks**: Intercept operations before/after execution

## Quick Start

### 1. Start the GraphQL Server

```bash
# With a schema file
mockforge serve --graphql ./schema.graphql --graphql-port 4000

# With upstream passthrough
mockforge serve --graphql ./schema.graphql --graphql-upstream http://api.example.com/graphql
```

### 2. Access GraphQL Playground

Open your browser to [http://localhost:4000/playground](http://localhost:4000/playground) to interact with the GraphQL API.

### 3. Run Example Queries

**Query users:**
```graphql
query GetUsers {
  users(limit: 5) {
    id
    name
    email
    role
  }
}
```

**Create a user:**
```graphql
mutation CreateUser {
  createUser(input: {
    name: "Alice Smith"
    email: "alice@example.com"
    password: "secret123"
  }) {
    id
    name
    email
  }
}
```

**Query products with variables:**
```graphql
query GetProducts($category: String!, $maxPrice: Float) {
  products(category: $category, maxPrice: $maxPrice) {
    id
    name
    price
    inStock
  }
}
```

Variables:
```json
{
  "category": "electronics",
  "maxPrice": 1000.0
}
```

## Configuration

### Config File (`mockforge.yaml`)

```yaml
graphql:
  enabled: true
  port: 4000
  schema_path: ./examples/graphql/schema.graphql
  handlers_dir: ./examples/graphql/handlers
  playground_enabled: true
  upstream_url: null  # Optional: http://api.example.com/graphql
  introspection_enabled: true
```

### CLI Flags

```bash
# Schema file
--graphql <path>              # Path to GraphQL schema file

# Port configuration
--graphql-port <port>         # GraphQL server port (default: 4000)

# Upstream passthrough
--graphql-upstream <url>      # Upstream GraphQL server URL
```

## Handler System

MockForge's GraphQL handlers work similarly to WebSocket handlers:

### Handler Trait

```rust
use mockforge_graphql::{GraphQLHandler, GraphQLContext, HandlerResult};
use async_graphql::{Response, Value};
use async_trait::async_trait;

struct UserQueryHandler;

#[async_trait]
impl GraphQLHandler for UserQueryHandler {
    async fn on_operation(&self, ctx: &GraphQLContext) -> HandlerResult<Option<Response>> {
        // Return custom response or None to use default
        if ctx.operation_name.as_deref() == Some("getUser") {
            // Check variables
            if let Some(Value::String(id)) = ctx.get_variable("id") {
                // Return custom mock data
                return Ok(Some(Response::new(serde_json::json!({
                    "id": id,
                    "name": "Mock User",
                    "email": "mock@example.com",
                    "role": "USER"
                }))));
            }
        }
        Ok(None) // Use default resolution
    }

    fn handles_operation(&self, operation_name: Option<&str>, _: &OperationType) -> bool {
        operation_name == Some("getUser")
    }

    fn priority(&self) -> i32 {
        10 // Higher priority executes first
    }
}
```

### Variable Matching

```rust
use mockforge_graphql::{VariableMatcher, VariablePattern};
use async_graphql::Value;

// Match specific variable values
let matcher = VariableMatcher::new()
    .with_pattern("id".to_string(), VariablePattern::Exact(Value::String("123".to_string())))
    .with_pattern("role".to_string(), VariablePattern::Present);

if matcher.matches(&ctx.variables) {
    // Handle this specific operation
}

// Pattern types:
// - VariablePattern::Any - Matches anything
// - VariablePattern::Exact(value) - Exact match
// - VariablePattern::Regex(pattern) - Regex match for strings
// - VariablePattern::Present - Variable must be present
// - VariablePattern::Null - Variable must be null or absent
```

### Handler Registry

```rust
use mockforge_graphql::HandlerRegistry;

let mut registry = HandlerRegistry::new();

// Register handlers
registry.register(UserQueryHandler);
registry.register(ProductQueryHandler);
registry.register(OrderMutationHandler);

// With upstream passthrough
let registry = HandlerRegistry::with_upstream(
    Some("http://api.example.com/graphql".to_string())
);
```

## Operation Types

MockForge supports all GraphQL operation types:

### Queries
- Read-only operations
- Default mock data generation based on schema
- Custom handlers for specific queries

### Mutations
- Write operations (create, update, delete)
- Can be mocked or passed through to upstream
- Support for optimistic responses

### Subscriptions
- WebSocket-based real-time updates
- Event-driven mock responses
- Integration with subscription servers

## Features

### ✅ Implemented

- [x] Schema loading from `.graphql` files
- [x] Handler trait with lifecycle hooks
- [x] Handler registry with priority
- [x] Variable matching and filtering
- [x] Upstream passthrough
- [x] GraphQL Playground UI
- [x] Introspection queries
- [x] CLI flags and configuration

### 🚧 Coming Soon

- [ ] Dynamic handler loading from TypeScript/JavaScript
- [ ] Hot-reloading of schema and handlers
- [ ] Response caching and memoization
- [ ] Advanced subscription support
- [ ] GraphQL federation support
- [ ] Performance monitoring per operation

## Testing with MockForge

### Integration Tests

```rust
use mockforge_graphql::{HandlerRegistry, GraphQLContext, OperationType};

#[tokio::test]
async fn test_user_query_handler() {
    let mut registry = HandlerRegistry::new();
    registry.register(UserQueryHandler);

    let ctx = GraphQLContext::new(
        Some("getUser".to_string()),
        OperationType::Query,
        "query { user(id: \"123\") { id name } }".to_string(),
        Variables::default(),
    );

    let result = registry.execute_operation(&ctx).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}
```

### cURL Examples

```bash
# Query
curl -X POST http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ users { id name email } }"}'

# Mutation
curl -X POST http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation { createUser(input: {name: \"Bob\", email: \"bob@test.com\", password: \"pass\"}) { id name } }"
  }'

# With variables
curl -X POST http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query GetUser($id: ID!) { user(id: $id) { id name email } }",
    "variables": {"id": "123"}
  }'
```

## Comparison with MSW

MockForge's GraphQL handlers are inspired by [Mock Service Worker (MSW)](https://mswjs.io/):

| Feature | MSW | MockForge |
|---------|-----|-----------|
| **Language** | JavaScript/TypeScript | Rust (with JS support planned) |
| **Schema** | Optional | Required (`.graphql` files) |
| **Handlers** | `graphql.query`, `graphql.mutation` | `GraphQLHandler` trait |
| **Variables** | Direct access | `VariableMatcher` pattern |
| **Passthrough** | Via `ctx.fetch()` | `--graphql-upstream` flag |
| **Playground** | No | Yes (built-in) |
| **Performance** | Node.js runtime | Native binary (faster) |

### MSW Example

```typescript
// MSW style
import { graphql } from 'msw';

export const handlers = [
  graphql.query('GetUser', (req, res, ctx) => {
    const { id } = req.variables;
    return res(
      ctx.data({
        user: { id, name: 'Mock User', email: 'mock@example.com' }
      })
    );
  }),
];
```

### MockForge Equivalent

```rust
// MockForge style
struct GetUserHandler;

#[async_trait]
impl GraphQLHandler for GetUserHandler {
    async fn on_operation(&self, ctx: &GraphQLContext) -> HandlerResult<Option<Response>> {
        if let Some(Value::String(id)) = ctx.get_variable("id") {
            Ok(Some(Response::new(json!({
                "id": id,
                "name": "Mock User",
                "email": "mock@example.com"
            }))))
        } else {
            Ok(None)
        }
    }

    fn handles_operation(&self, operation_name: Option<&str>, _: &OperationType) -> bool {
        operation_name == Some("GetUser")
    }
}
```

## Examples in This Directory

- **`schema.graphql`** - Complete e-commerce GraphQL schema
- **`README.md`** - This documentation
- **`handlers.rs`** (coming soon) - Example Rust handlers
- **`mockforge.yaml`** (coming soon) - Configuration example

## Related Documentation

- [MockForge Book](https://docs.mockforge.dev/)
- [GraphQL Specification](https://spec.graphql.org/)
- [async-graphql Documentation](https://async-graphql.github.io/async-graphql/en/)

## Troubleshooting

### Schema Not Found

```bash
Error: Schema file not found: ./schema.graphql
```

**Solution:** Ensure the schema file path is correct:
```bash
mockforge serve --graphql ./examples/graphql/schema.graphql
```

### Port Already in Use

```bash
Error: Address already in use (port 4000)
```

**Solution:** Use a different port:
```bash
mockforge serve --graphql-port 4001 --graphql ./schema.graphql
```

### Upstream Connection Failed

```bash
Error: Upstream error: connection refused
```

**Solution:** Check that the upstream server is running and accessible.

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines on adding new handlers or examples.

## License

MIT OR Apache-2.0
