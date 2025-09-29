# GraphQL Mocking

MockForge provides comprehensive GraphQL API mocking capabilities, allowing you to create realistic GraphQL endpoints with schema-driven response generation, introspection support, and custom resolvers.

## Overview

MockForge's GraphQL support includes:

- **Schema-Driven Mocking**: Generate responses based on GraphQL schema definitions
- **Introspection Support**: Full GraphQL introspection query support
- **Custom Resolvers**: Implement custom logic for specific fields
- **Query Validation**: Validate incoming GraphQL queries against schema
- **Subscription Support**: Mock GraphQL subscriptions with real-time updates
- **Schema Stitching**: Combine multiple schemas into unified endpoints
- **Performance Simulation**: Configurable latency and complexity limits

## Getting Started

### Basic Setup

Enable GraphQL mocking in your MockForge configuration:

```yaml
# config.yaml
graphql:
  enabled: true
  endpoint: "/graphql"
  schema_file: "schema.graphql"
  introspection: true
  playground: true
  
server:
  http_port: 3000
```

Start MockForge with GraphQL support:

```bash
mockforge serve --config config.yaml
```

Access your GraphQL endpoint:
- **GraphQL Endpoint**: `http://localhost:3000/graphql`
- **GraphQL Playground**: `http://localhost:3000/graphql/playground`

### Schema Definition

Create a GraphQL schema file:

```graphql
# schema.graphql
type User {
  id: ID!
  name: String!
  email: String!
  age: Int
  posts: [Post!]!
  profile: UserProfile
}

type Post {
  id: ID!
  title: String!
  content: String!
  published: Boolean!
  author: User!
  createdAt: String!
  tags: [String!]!
}

type UserProfile {
  bio: String
  website: String
  location: String
  avatarUrl: String
}

type Query {
  users: [User!]!
  user(id: ID!): User
  posts: [Post!]!
  post(id: ID!): Post
  searchUsers(query: String!): [User!]!
}

type Mutation {
  createUser(input: CreateUserInput!): User!
  updateUser(id: ID!, input: UpdateUserInput!): User!
  deleteUser(id: ID!): Boolean!
  createPost(input: CreatePostInput!): Post!
}

type Subscription {
  userCreated: User!
  postPublished: Post!
  userOnline(userId: ID!): Boolean!
}

input CreateUserInput {
  name: String!
  email: String!
  age: Int
}

input UpdateUserInput {
  name: String
  email: String
  age: Int
}

input CreatePostInput {
  title: String!
  content: String!
  authorId: ID!
  tags: [String!]
}
```

## Configuration Options

### Basic Configuration

```yaml
graphql:
  # Enable GraphQL support
  enabled: true
  
  # GraphQL endpoint path
  endpoint: "/graphql"
  
  # Schema configuration
  schema_file: "schema.graphql"
  schema_url: "https://api.example.com/schema"  # Alternative: fetch from URL
  
  # Development features
  introspection: true
  playground: true
  playground_endpoint: "/graphql/playground"
  
  # Response generation
  mock_responses: true
  default_list_length: 5
  
  # Validation
  validate_queries: true
  max_query_depth: 10
  max_query_complexity: 1000
```

### Advanced Configuration

```yaml
graphql:
  # Performance settings
  performance:
    enable_query_complexity_analysis: true
    max_query_depth: 15
    max_query_complexity: 1000
    timeout_ms: 30000
    
  # Caching
  caching:
    enabled: true
    ttl_seconds: 300
    max_cache_size: 1000
    
  # Custom resolvers
  resolvers:
    directory: "./graphql/resolvers"
    auto_load: true
    
  # Subscription settings
  subscriptions:
    enabled: true
    transport: "websocket"
    heartbeat_interval: 30
    
  # Error handling
  errors:
    include_stack_trace: true
    include_extensions: true
    custom_error_codes: true
```

## Response Generation

### Automatic Response Generation

MockForge automatically generates realistic responses based on your schema:

```graphql
# Query
query GetUsers {
  users {
    id
    name
    email
    age
    posts {
      title
      published
    }
  }
}
```

```json
{
  "data": {
    "users": [
      {
        "id": "1a2b3c4d",
        "name": "Alice Johnson",
        "email": "alice.johnson@example.com",
        "age": 29,
        "posts": [
          {
            "title": "Getting Started with GraphQL",
            "published": true
          },
          {
            "title": "Advanced Query Techniques",
            "published": false
          }
        ]
      },
      {
        "id": "2b3c4d5e",
        "name": "Bob Smith",
        "email": "bob.smith@example.com",
        "age": 34,
        "posts": [
          {
            "title": "Building Scalable APIs",
            "published": true
          }
        ]
      }
    ]
  }
}
```

### Template-Based Responses

Use templates for more control over response data:

```yaml
# graphql/responses/user.yaml
query: "query GetUser($id: ID!)"
response:
  data:
    user:
      id: "{{args.id}}"
      name: "{{faker.name.fullName}}"
      email: "{{faker.internet.email}}"
      age: "{{randInt 18 65}}"
      profile:
        bio: "{{faker.lorem.sentence}}"
        website: "{{faker.internet.url}}"
        location: "{{faker.address.city}}, {{faker.address.state}}"
        avatarUrl: "https://api.dicebear.com/7.x/avataaars/svg?seed={{uuid}}"
```

### Custom Field Resolvers

Create custom resolvers for specific fields:

```javascript
// graphql/resolvers/user.js
module.exports = {
  User: {
    // Custom resolver for posts field
    posts: (parent, args, context) => {
      return context.dataSources.posts.getByAuthorId(parent.id);
    },
    
    // Computed field
    fullName: (parent) => {
      return `${parent.firstName} ${parent.lastName}`;
    },
    
    // Async resolver with external data
    socialStats: async (parent, args, context) => {
      return await context.dataSources.social.getStats(parent.id);
    }
  },
  
  Query: {
    // Custom query resolver
    searchUsers: (parent, args, context) => {
      const { query, limit = 10 } = args;
      return context.dataSources.users.search(query, limit);
    }
  },
  
  Mutation: {
    // Custom mutation resolver
    createUser: (parent, args, context) => {
      const { input } = args;
      const user = {
        id: uuid(),
        ...input,
        createdAt: new Date().toISOString()
      };
      
      context.dataSources.users.create(user);
      
      // Trigger subscription
      context.pubsub.publish('USER_CREATED', { userCreated: user });
      
      return user;
    }
  }
};
```

## Data Sources

### CSV Data Source

Connect GraphQL resolvers to CSV data:

```yaml
# config.yaml
graphql:
  data_sources:
    users:
      type: "csv"
      file: "data/users.csv"
      key_field: "id"
    
    posts:
      type: "csv"
      file: "data/posts.csv"
      key_field: "id"
      relationships:
        author_id: "users.id"
```

```csv
# data/users.csv
id,name,email,age
1,Alice Johnson,alice@example.com,29
2,Bob Smith,bob@example.com,34
3,Carol Davis,carol@example.com,27
```

### REST API Data Source

Fetch data from external REST APIs:

```yaml
graphql:
  data_sources:
    users:
      type: "rest"
      base_url: "https://jsonplaceholder.typicode.com"
      endpoints:
        getAll: "/users"
        getById: "/users/{id}"
        create: 
          method: "POST"
          url: "/users"
    
    posts:
      type: "rest"
      base_url: "https://jsonplaceholder.typicode.com"
      endpoints:
        getAll: "/posts"
        getByUserId: "/posts?userId={userId}"
```

### Database Data Source

Connect to databases for realistic data:

```yaml
graphql:
  data_sources:
    database:
      type: "postgresql"
      connection_string: "postgresql://user:pass@localhost/mockdb"
      tables:
        users:
          table: "users"
          key_field: "id"
        posts:
          table: "posts"
          key_field: "id"
          relationships:
            author_id: "users.id"
```

## Subscriptions

### WebSocket Subscriptions

Enable real-time GraphQL subscriptions:

```yaml
graphql:
  subscriptions:
    enabled: true
    transport: "websocket"
    endpoint: "/graphql/ws"
    heartbeat_interval: 30
    connection_timeout: 60
```

### Subscription Resolvers

```javascript
// graphql/resolvers/subscriptions.js
module.exports = {
  Subscription: {
    userCreated: {
      subscribe: (parent, args, context) => {
        return context.pubsub.asyncIterator('USER_CREATED');
      }
    },
    
    postPublished: {
      subscribe: (parent, args, context) => {
        return context.pubsub.asyncIterator('POST_PUBLISHED');
      }
    },
    
    userOnline: {
      subscribe: (parent, args, context) => {
        const { userId } = args;
        return context.pubsub.asyncIterator(`USER_ONLINE_${userId}`);
      }
    }
  }
};
```

### Triggering Subscriptions

Trigger subscriptions from mutations or external events:

```javascript
// In mutation resolver
createPost: (parent, args, context) => {
  const post = createNewPost(args.input);
  
  // Trigger subscription
  context.pubsub.publish('POST_PUBLISHED', { 
    postPublished: post 
  });
  
  return post;
}
```

## Schema Stitching

Combine multiple GraphQL schemas:

```yaml
graphql:
  schema_stitching:
    enabled: true
    schemas:
      - name: "users"
        file: "schemas/users.graphql"
        endpoint: "http://users-service/graphql"
      
      - name: "posts"
        file: "schemas/posts.graphql"
        endpoint: "http://posts-service/graphql"
      
      - name: "comments"
        file: "schemas/comments.graphql"
        endpoint: "http://comments-service/graphql"
    
    # Type extensions for stitching
    extensions:
      - |
        extend type User {
          posts: [Post]
        }
      - |
        extend type Post {
          comments: [Comment]
        }
```

## Error Handling

### Custom Error Responses

Configure custom error handling:

```yaml
graphql:
  errors:
    # Include detailed error information
    include_stack_trace: true
    include_extensions: true
    
    # Custom error codes
    custom_error_codes:
      INVALID_INPUT: 400
      UNAUTHORIZED: 401
      FORBIDDEN: 403
      NOT_FOUND: 404
      RATE_LIMITED: 429
```

### Error Response Format

```json
{
  "errors": [
    {
      "message": "User not found",
      "locations": [
        {
          "line": 2,
          "column": 3
        }
      ],
      "path": ["user"],
      "extensions": {
        "code": "NOT_FOUND",
        "userId": "invalid-id",
        "timestamp": "2024-01-01T00:00:00Z"
      }
    }
  ],
  "data": {
    "user": null
  }
}
```

## Performance & Optimization

### Query Complexity Analysis

Prevent expensive queries:

```yaml
graphql:
  performance:
    enable_query_complexity_analysis: true
    max_query_depth: 10
    max_query_complexity: 1000
    complexity_scalarCost: 1
    complexity_objectCost: 2
    complexity_listFactor: 10
    complexity_introspectionCost: 100
```

### Caching

Cache responses for improved performance:

```yaml
graphql:
  caching:
    enabled: true
    ttl_seconds: 300
    max_cache_size: 1000
    cache_key_strategy: "query_and_variables"
    
    # Cache per resolver
    resolver_cache:
      "Query.users": 600  # Cache for 10 minutes
      "Query.posts": 300  # Cache for 5 minutes
```

### Latency Simulation

Simulate real-world latency:

```yaml
graphql:
  latency:
    enabled: true
    default_delay_ms: 100
    
    # Per-field latency
    field_delays:
      "Query.users": 200
      "User.posts": 150
      "Post.comments": 100
    
    # Random latency ranges
    random_delay:
      min_ms: 50
      max_ms: 500
```

## Testing & Development

### GraphQL Playground

The built-in GraphQL Playground provides:

- **Interactive Query Editor**: Write and test GraphQL queries
- **Schema Documentation**: Browse your schema structure
- **Query Variables**: Test with different variable values
- **Response Headers**: View response metadata
- **Subscription Testing**: Test real-time subscriptions

### Query Examples

Test your GraphQL API with these examples:

```graphql
# Simple query
query GetAllUsers {
  users {
    id
    name
    email
  }
}

# Query with variables
query GetUser($userId: ID!) {
  user(id: $userId) {
    id
    name
    email
    posts {
      title
      published
    }
  }
}

# Mutation
mutation CreateUser($input: CreateUserInput!) {
  createUser(input: $input) {
    id
    name
    email
  }
}

# Subscription
subscription UserUpdates {
  userCreated {
    id
    name
    email
  }
}
```

### Integration with HTTP Mocking

Combine GraphQL with REST API mocking:

```yaml
# config.yaml
http:
  enabled: true
  spec: "openapi.yaml"

graphql:
  enabled: true
  schema_file: "schema.graphql"
  
# Use REST endpoints in GraphQL resolvers
graphql:
  data_sources:
    rest_api:
      type: "rest"
      base_url: "http://localhost:3000"  # MockForge HTTP server
      endpoints:
        users: "/api/users"
        posts: "/api/posts"
```

## Best Practices

### Schema Design

1. **Use Descriptive Names**: Choose clear, self-documenting field names
2. **Follow Conventions**: Use camelCase for fields, PascalCase for types
3. **Document Your Schema**: Add descriptions to types and fields
4. **Version Carefully**: Use field deprecation instead of breaking changes

### Performance

1. **Implement Caching**: Cache expensive resolver operations
2. **Limit Query Depth**: Prevent deeply nested queries
3. **Use DataLoaders**: Batch and cache data fetching
4. **Monitor Complexity**: Track query complexity metrics

### Testing

1. **Test Query Variations**: Test different query structures and variables
2. **Validate Error Cases**: Ensure proper error handling
3. **Test Subscriptions**: Verify real-time functionality
4. **Performance Testing**: Test with realistic query loads

## Troubleshooting

### Common Issues

#### Schema Loading Errors

```bash
# Validate GraphQL schema
mockforge graphql validate --schema schema.graphql

# Check schema syntax
graphql-schema-linter schema.graphql
```

#### Resolver Errors

```bash
# Enable debug logging
RUST_LOG=mockforge_graphql=debug mockforge serve

# Test individual resolvers
mockforge graphql test-resolver Query.users
```

#### Subscription Issues

```bash
# Test WebSocket connection
wscat -c ws://localhost:3000/graphql/ws

# Check subscription resolver
mockforge graphql test-subscription userCreated
```

This comprehensive GraphQL support makes MockForge a powerful tool for mocking modern GraphQL APIs with realistic data and behavior.