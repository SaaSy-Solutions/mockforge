# MockAI OpenAPI Generation

Generate OpenAPI 3.0 specifications from recorded HTTP traffic using AI-powered pattern detection and inference.

## Overview

The OpenAPI generation feature analyzes recorded API traffic to automatically infer:
- **API paths** with parameter normalization (e.g., `/users/123` → `/users/{id}`)
- **HTTP methods** and operation definitions
- **Request/response schemas** from JSON payloads
- **Path parameters** with inferred names and types
- **Confidence scores** for each inferred path

## Features

- **Pattern-based inference**: Groups similar paths and infers parameterized patterns
- **LLM-assisted generation**: Uses AI to generate comprehensive OpenAPI specs when available
- **Confidence scoring**: Provides confidence metrics for each inferred path
- **Flexible filtering**: Filter by time range, path patterns, and minimum confidence
- **Multiple output formats**: JSON and YAML export

## Usage

### CLI Command

```bash
# Generate from default database
mockforge mockai generate-from-traffic

# With filters
mockforge mockai generate-from-traffic \
  --database ./recordings.db \
  --since "2025-01-01T00:00:00Z" \
  --until "2025-01-31T23:59:59Z" \
  --path-pattern "/api/*" \
  --min-confidence 0.7 \
  --output openapi.json
```

### API Endpoint

```bash
POST /__mockforge/api/mockai/generate-openapi
Content-Type: application/json

{
  "database_path": "./recordings.db",
  "since": "2025-01-01T00:00:00Z",
  "until": "2025-01-31T23:59:59Z",
  "path_pattern": "/api/*",
  "min_confidence": 0.7
}
```

**Response:**
```json
{
  "spec": {
    "openapi": "3.0.0",
    "info": {
      "title": "Generated API",
      "version": "1.0.0"
    },
    "paths": {
      "/users/{id}": {
        "get": {
          "summary": "Get user by ID",
          "parameters": [
            {
              "name": "id",
              "in": "path",
              "required": true,
              "schema": { "type": "string" }
            }
          ],
          "responses": {
            "200": {
              "description": "User object",
              "content": {
                "application/json": {
                  "schema": {
                    "type": "object",
                    "properties": {
                      "id": { "type": "string" },
                      "name": { "type": "string" }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  },
  "metadata": {
    "requests_analyzed": 150,
    "paths_inferred": 12,
    "path_confidence": {
      "/users/{id}": {
        "value": 0.95,
        "reason": "High pattern match count (45 examples)"
      }
    },
    "generated_at": "2025-01-15T10:30:00Z",
    "duration_ms": 1234
  }
}
```

### UI Usage

1. Navigate to **MockAI OpenAPI Generator** in the sidebar
2. Configure filters:
   - **Database Path**: Path to recorder database (defaults to `./recordings.db`)
   - **Time Range**: Start and end times for filtering traffic
   - **Path Pattern**: Wildcard pattern to filter paths (e.g., `/api/*`)
   - **Minimum Confidence**: Confidence threshold (0.0 to 1.0)
3. Click **Generate OpenAPI Spec**
4. Review the generated specification and metadata
5. Download as JSON or YAML

## How It Works

### Path Parameter Inference

The generator groups paths by their base pattern and infers parameters:

```
Input paths:
  /users/123
  /users/456
  /users/789

Inferred:
  /users/{id}
```

The parameter name is inferred from:
- Common patterns (e.g., `id`, `userId`, `user_id`)
- Path segment position
- Context from other paths

### Schema Inference

JSON request/response bodies are analyzed to generate JSON Schema:

```json
// Example response
{
  "id": "123",
  "name": "Alice",
  "email": "alice@example.com",
  "age": 30
}

// Inferred schema
{
  "type": "object",
  "properties": {
    "id": { "type": "string" },
    "name": { "type": "string" },
    "email": { "type": "string" },
    "age": { "type": "integer" }
  },
  "required": ["id", "name", "email"]
}
```

### Confidence Scoring

Confidence scores are calculated based on:
- **Pattern match count**: More examples = higher confidence
- **Consistency**: Similar patterns across examples
- **Schema completeness**: More complete schemas = higher confidence

Confidence ranges:
- **0.8-1.0**: High confidence (many examples, consistent patterns)
- **0.6-0.8**: Medium confidence (some examples, mostly consistent)
- **0.0-0.6**: Low confidence (few examples or inconsistent patterns)

## Configuration

### Environment Variables

- `MOCKFORGE_LLM_PROVIDER`: LLM provider for AI-assisted generation (default: "disabled")
- `MOCKFORGE_LLM_API_KEY`: API key for LLM provider
- `MOCKFORGE_LLM_MODEL`: Model name (default: "gpt-4")

### Generation Options

- **min_confidence**: Minimum confidence threshold (0.0 to 1.0, default: 0.7)
- **use_llm**: Enable LLM-assisted generation (requires API key)
- **max_paths**: Maximum number of paths to include (default: unlimited)

## Best Practices

1. **Record sufficient traffic**: More examples lead to better inference
2. **Use time filters**: Focus on recent traffic for current API state
3. **Review confidence scores**: Low confidence paths may need manual review
4. **Validate generated specs**: Always review and validate generated OpenAPI specs
5. **Combine with manual specs**: Use generated specs as a starting point

## Limitations

- **Path parameter names**: Inferred names may not match actual API conventions
- **Schema completeness**: Complex nested structures may be simplified
- **Authentication**: Auth requirements are not automatically inferred
- **Error responses**: Error response schemas may be incomplete
- **Non-JSON payloads**: Only JSON request/response bodies are analyzed

## Troubleshooting

### No paths inferred

- Check that the database contains recorded traffic
- Verify time range filters are correct
- Ensure path patterns match recorded paths
- Lower the minimum confidence threshold

### Low confidence scores

- Record more traffic for the paths in question
- Ensure consistent path patterns across examples
- Check for typos or inconsistent path structures

### Missing schemas

- Ensure request/response bodies are JSON
- Verify bodies are not empty or malformed
- Check that content-type headers indicate JSON

## Examples

### Basic Generation

```bash
# Generate from all recorded traffic
mockforge mockai generate-from-traffic -o api.json
```

### Filtered Generation

```bash
# Generate only API paths from last week
mockforge mockai generate-from-traffic \
  --since "$(date -u -d '7 days ago' +%Y-%m-%dT%H:%M:%SZ)" \
  --path-pattern "/api/*" \
  --min-confidence 0.8 \
  -o api.json
```

### API Integration

```typescript
const response = await fetch('/__mockforge/api/mockai/generate-openapi', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    since: '2025-01-01T00:00:00Z',
    path_pattern: '/api/v1/*',
    min_confidence: 0.75
  })
});

const { spec, metadata } = await response.json();
console.log(`Generated spec with ${metadata.paths_inferred} paths`);
```

## See Also

- [API Flight Recorder](./API_FLIGHT_RECORDER.md) - Recording API traffic
- [MockAI Usage](./MOCKAI_USAGE.md) - General MockAI features
- [AI Response Generation](./AI_RESPONSE_GENERATION.md) - AI-powered responses
