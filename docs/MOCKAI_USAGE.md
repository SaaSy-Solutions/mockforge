# MockAI (Behavioral Mock Intelligence) Usage Guide

MockAI is MockForge's intelligent mock generation system that uses AI to create contextually appropriate, realistic API responses. It automatically learns from OpenAPI specifications and example payloads to generate intelligent behavior.

## Features

- **Auto-Generated Rules**: Automatically infers behavioral rules from OpenAPI specs or example payloads
- **Context-Aware Responses**: Maintains session state and conversation history across requests
- **Mutation Detection**: Intelligently detects create, update, and delete operations from request changes
- **Validation Error Generation**: Generates realistic, context-aware validation error responses
- **Pagination Intelligence**: Automatically generates realistic pagination metadata and responses
- **Session Persistence**: Tracks state across multiple requests within a session

## Configuration

### Basic Configuration

Enable MockAI in your `config.yaml`:

```yaml
mockai:
  enabled: true
  auto_learn: true
  mutation_detection: true
  ai_validation_errors: true
  intelligent_pagination: true
  intelligent_behavior:
    behavior_model:
      provider: "ollama"  # or "openai", "anthropic", etc.
      model: "llama3.2"
      base_url: "http://localhost:11434"
    performance:
      max_history_length: 50
      cache_enabled: true
```

### Profile-Specific Configuration

Override MockAI settings per profile:

```yaml
profiles:
  production:
    mockai:
      enabled: false  # Disable MockAI in production profile
  development:
    mockai:
      enabled: true
      intelligent_behavior:
        behavior_model:
          model: "llama3.2:1b"  # Use smaller model for faster responses
```

## CLI Commands

### Enable MockAI

```bash
# Enable MockAI globally
mockforge mockai enable

# Enable MockAI for specific endpoints
mockforge mockai enable --endpoints "/users" "/products"
```

### Disable MockAI

```bash
# Disable MockAI globally
mockforge mockai disable

# Disable MockAI for specific endpoints
mockforge mockai disable --endpoints "/admin/*"
```

### Check Status

```bash
# Check MockAI status
mockforge mockai status
```

### Learn from Examples

```bash
# Learn from example request/response pairs
mockforge mockai learn --examples examples.json
```

### Generate Response

```bash
# Generate a response for a request
mockforge mockai generate --method POST --path "/users" --body '{"name": "John"}'
```

## Usage Examples

### Starting Server with MockAI

```bash
# Start server with MockAI enabled
mockforge serve --openapi-spec api.yaml --mockai-enabled

# Or use config file
mockforge serve --config config.yaml
```

### Session Management

MockAI automatically tracks sessions via:

1. **Header**: `X-Session-ID: <session-id>`
2. **Cookie**: `mockforge_session=<session-id>`

If no session ID is provided, MockAI generates a new one automatically.

Example with session header:

```bash
curl -H "X-Session-ID: my-session-123" \
     http://localhost:3000/users
```

### Mutation Detection

MockAI automatically detects mutations (create, update, delete) by comparing request bodies:

```bash
# First request - creates a new resource
curl -X POST http://localhost:3000/users \
     -H "Content-Type: application/json" \
     -d '{"name": "John", "email": "john@example.com"}'

# Second request with changes - detected as update
curl -X POST http://localhost:3000/users \
     -H "Content-Type: application/json" \
     -H "X-Session-ID: my-session-123" \
     -d '{"name": "John Doe", "email": "john@example.com"}'
```

### Validation Errors

MockAI generates realistic validation errors when requests don't match schemas:

```bash
# Missing required field
curl -X POST http://localhost:3000/users \
     -H "Content-Type: application/json" \
     -d '{"email": "invalid"}'  # Missing "name" field

# Response includes detailed validation errors
{
  "error": "Validation failed",
  "details": [
    {
      "field": "name",
      "message": "Field 'name' is required"
    },
    {
      "field": "email",
      "message": "Invalid email format"
    }
  ]
}
```

### Pagination

MockAI automatically handles pagination requests:

```bash
# Paginated request
curl "http://localhost:3000/users?page=1&limit=10"

# Response includes pagination metadata
{
  "data": [...],
  "pagination": {
    "page": 1,
    "limit": 10,
    "total": 100,
    "total_pages": 10,
    "has_next": true,
    "has_prev": false
  }
}
```

## Programmatic Usage

### Creating MockAI from OpenAPI

```rust
use mockforge_core::intelligent_behavior::{IntelligentBehaviorConfig, MockAI};
use mockforge_core::openapi::OpenApiSpec;

// Load OpenAPI spec
let spec = OpenApiSpec::from_file("api.yaml").await?;

// Create MockAI with default config
let config = IntelligentBehaviorConfig::default();
let mockai = MockAI::from_openapi(&spec, config).await?;

// Process a request
let request = Request {
    method: "POST".to_string(),
    path: "/users".to_string(),
    body: Some(json!({"name": "John"})),
    query_params: HashMap::new(),
    headers: HashMap::new(),
};

let response = mockai.process_request(&request).await?;
```

### Creating MockAI from Examples

```rust
use mockforge_core::intelligent_behavior::{IntelligentBehaviorConfig, MockAI};
use mockforge_core::intelligent_behavior::rule_generator::ExamplePair;

let examples = vec![
    ExamplePair {
        method: "POST".to_string(),
        path: "/users".to_string(),
        request: Some(json!({"name": "John"})),
        response: Some(json!({"id": 1, "name": "John"})),
    },
];

let config = IntelligentBehaviorConfig::default();
let mockai = MockAI::from_examples(examples, config).await?;
```

### Learning from Examples

```rust
// Learn from a new example
let example = ExamplePair {
    method: "PUT".to_string(),
    path: "/users/{id}".to_string(),
    request: Some(json!({"name": "Jane"})),
    response: Some(json!({"id": 1, "name": "Jane"})),
};

mockai.learn_from_example(example).await?;
```

## Advanced Configuration

### Custom Behavior Model

```yaml
mockai:
  intelligent_behavior:
    behavior_model:
      provider: "openai"
      model: "gpt-4"
      api_key: "${OPENAI_API_KEY}"
      temperature: 0.7
      max_tokens: 1000
```

### Performance Tuning

```yaml
mockai:
  intelligent_behavior:
    performance:
      max_history_length: 100  # Keep more history
      cache_enabled: true
      cache_ttl_seconds: 3600
      timeout_seconds: 30
```

### Vector Store Configuration

```yaml
mockai:
  intelligent_behavior:
    vector_store:
      enabled: true
      provider: "postgres"  # or "qdrant", "pinecone"
      connection_string: "postgresql://localhost/mockforge"
      dimension: 384
```

## Troubleshooting

### MockAI Not Responding

1. Check if MockAI is enabled:
   ```bash
   mockforge mockai status
   ```

2. Verify LLM provider is accessible:
   ```bash
   # For Ollama
   curl http://localhost:11434/api/tags
   ```

3. Check logs for errors:
   ```bash
   mockforge serve --log-level debug
   ```

### Session Not Persisting

- Ensure session ID is sent in headers or cookies
- Check session timeout settings
- Verify session storage is not being cleared

### Slow Responses

- Use a smaller/faster model
- Enable caching
- Reduce `max_history_length`
- Use a local LLM provider (Ollama)

## Best Practices

1. **Start Simple**: Begin with default configuration and adjust as needed
2. **Use Local LLMs**: For faster responses, use Ollama or similar local providers
3. **Monitor Performance**: Track response times and adjust `timeout_seconds` accordingly
4. **Session Management**: Use consistent session IDs across related requests
5. **Example Quality**: Provide high-quality examples for better rule generation

## Limitations

- Query parameter extraction currently requires middleware enhancement (documented in code)
- Session contexts are stored in memory (not persisted to disk)
- Large OpenAPI specs may take longer to initialize

## See Also

- [Configuration Guide](../CONFIG.md)
- [OpenAPI Integration](../INTEGRATION_GUIDE.md)
- [Architecture Documentation](../ARCHITECTURE.md)
