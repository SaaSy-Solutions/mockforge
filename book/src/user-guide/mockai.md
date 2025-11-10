# MockAI (Intelligent Mocking)

MockAI is MockForge's intelligent mock generation system that uses AI to create contextually appropriate, realistic API responses. It automatically learns from OpenAPI specifications and example payloads to generate intelligent behavior.

## Overview

MockAI provides:

- **Auto-Generated Rules**: Automatically infers behavioral rules from OpenAPI specs or example payloads
- **Context-Aware Responses**: Maintains session state and conversation history across requests
- **Mutation Detection**: Intelligently detects create, update, and delete operations from request changes
- **Validation Error Generation**: Generates realistic, context-aware validation error responses
- **Pagination Intelligence**: Automatically generates realistic pagination metadata and responses
- **Session Persistence**: Tracks state across multiple requests within a session

## Quick Start

### Enable MockAI

```yaml
# config.yaml
mockai:
  enabled: true
  auto_learn: true
  mutation_detection: true
  ai_validation_errors: true
  intelligent_pagination: true
```

### Start Server

```bash
mockforge serve --config config.yaml --spec api.yaml
```

MockAI will automatically:
- Learn from your OpenAPI specification
- Generate intelligent responses
- Track session state
- Handle mutations and pagination

## Configuration

### Basic Configuration

```yaml
mockai:
  enabled: true
  auto_learn: true
  mutation_detection: true
  ai_validation_errors: true
  intelligent_pagination: true
  intelligent_behavior:
    behavior_model:
      provider: "ollama"  # or "openai", "anthropic"
      model: "llama3.2"
      base_url: "http://localhost:11434"
```

### LLM Provider Configuration

#### Ollama (Local, Free)

```yaml
mockai:
  intelligent_behavior:
    behavior_model:
      provider: "ollama"
      model: "llama3.2"
      base_url: "http://localhost:11434"
```

#### OpenAI

```yaml
mockai:
  intelligent_behavior:
    behavior_model:
      provider: "openai"
      model: "gpt-3.5-turbo"
      api_key: "${OPENAI_API_KEY}"
      temperature: 0.7
      max_tokens: 1000
```

#### Anthropic

```yaml
mockai:
  intelligent_behavior:
    behavior_model:
      provider: "anthropic"
      model: "claude-3-sonnet-20240229"
      api_key: "${ANTHROPIC_API_KEY}"
```

### Performance Tuning

```yaml
mockai:
  intelligent_behavior:
    performance:
      max_history_length: 100
      cache_enabled: true
      cache_ttl_seconds: 3600
      timeout_seconds: 30
```

## CLI Commands

### Enable/Disable MockAI

```bash
# Enable globally
mockforge mockai enable

# Enable for specific endpoints
mockforge mockai enable --endpoints "/users" "/products"

# Disable globally
mockforge mockai disable

# Disable for specific endpoints
mockforge mockai disable --endpoints "/admin/*"
```

### Check Status

```bash
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
mockforge mockai generate \
  --method POST \
  --path "/users" \
  --body '{"name": "John"}'
```

## Session Management

MockAI automatically tracks sessions to maintain context across requests:

### Session Identification

Sessions are identified by:
- **Header**: `X-Session-ID: <session-id>`
- **Cookie**: `mockforge_session=<session-id>`

If no session ID is provided, MockAI generates a new one automatically.

### Example with Session

```bash
# First request - creates session
curl http://localhost:3000/users

# Response includes session ID in Set-Cookie header
# Subsequent requests use the same session

# Second request with session
curl -H "X-Session-ID: my-session-123" \
     http://localhost:3000/users
```

## Mutation Detection

MockAI automatically detects mutations (create, update, delete) by comparing request bodies:

### Create Detection

```bash
# First request - creates a new resource
curl -X POST http://localhost:3000/users \
     -H "Content-Type: application/json" \
     -d '{"name": "John", "email": "john@example.com"}'

# MockAI detects this as a create operation
# Response includes generated ID and created timestamp
```

### Update Detection

```bash
# Second request with changes - detected as update
curl -X POST http://localhost:3000/users \
     -H "Content-Type: application/json" \
     -H "X-Session-ID: my-session-123" \
     -d '{"name": "John Doe", "email": "john@example.com"}'

# MockAI detects changes and treats as update
# Response reflects updated values
```

## Validation Errors

MockAI generates realistic validation errors when requests don't match schemas:

### Missing Required Field

```bash
curl -X POST http://localhost:3000/users \
     -H "Content-Type: application/json" \
     -d '{"email": "invalid"}'  # Missing "name" field
```

Response:
```json
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

## Pagination

MockAI automatically handles pagination requests:

### Paginated Request

```bash
curl "http://localhost:3000/users?page=1&limit=10"
```

Response:
```json
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

### Create MockAI from OpenAPI

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

### Learn from Examples

```rust
use mockforge_core::intelligent_behavior::rule_generator::ExamplePair;

let examples = vec![
    ExamplePair {
        method: "POST".to_string(),
        path: "/users".to_string(),
        request: Some(json!({"name": "John"})),
        response: Some(json!({"id": 1, "name": "John"})),
    },
];

mockai.learn_from_example(examples[0]).await?;
```

## Use Cases

### Rapid Prototyping

Generate realistic API responses without writing fixtures:

```yaml
mockai:
  enabled: true
  auto_learn: true
```

### Testing Error Handling

Generate realistic validation errors:

```yaml
mockai:
  enabled: true
  ai_validation_errors: true
```

### Session-Based Testing

Test multi-step workflows with session persistence:

```bash
# Step 1: Create session
curl -X POST http://localhost:3000/sessions

# Step 2: Use session in subsequent requests
curl -H "X-Session-ID: <session-id>" \
     http://localhost:3000/users
```

## Best Practices

1. **Start with Defaults**: Begin with default configuration and adjust as needed
2. **Use Local LLMs**: For faster responses, use Ollama or similar local providers
3. **Monitor Performance**: Track response times and adjust `timeout_seconds` accordingly
4. **Session Management**: Use consistent session IDs across related requests
5. **Example Quality**: Provide high-quality examples for better rule generation

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

## Limitations

- Query parameter extraction currently requires middleware enhancement
- Session contexts are stored in memory (not persisted to disk)
- Large OpenAPI specs may take longer to initialize

## Related Documentation

- [Reality Slider](reality-slider.md) - Control MockAI via reality levels
- [Configuration Guide](../configuration/files.md) - Complete configuration reference
- [OpenAPI Integration](http-mocking/openapi.md) - OpenAPI specification support

