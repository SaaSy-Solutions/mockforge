# HTTP Mocking

MockForge provides comprehensive HTTP API mocking capabilities with OpenAPI specification support, dynamic response generation, and advanced request matching. This guide covers everything you need to create realistic REST API mocks.

## OpenAPI Integration

MockForge uses OpenAPI (formerly Swagger) specifications as the foundation for HTTP API mocking. This industry-standard approach ensures your mocks accurately reflect real API contracts.

### Loading OpenAPI Specifications

```bash
# Load from JSON file
mockforge serve --spec api-spec.json --http-port 3000

# Load from YAML file
mockforge serve --spec api-spec.yaml --http-port 3000

# Load from URL
mockforge serve --spec https://api.example.com/openapi.json --http-port 3000
```

### OpenAPI Specification Structure

MockForge supports OpenAPI 3.0+ specifications with the following key components:

- **Paths**: API endpoint definitions
- **Methods**: HTTP verbs (GET, POST, PUT, DELETE, PATCH)
- **Parameters**: Path, query, and header parameters
- **Request Bodies**: JSON/XML payload schemas
- **Responses**: Status codes and response schemas
- **Components**: Reusable schemas and examples

### Example OpenAPI Specification

```yaml
openapi: 3.0.3
info:
  title: User Management API
  version: 1.0.0
paths:
  /users:
    get:
      summary: List users
      parameters:
        - name: limit
          in: query
          schema:
            type: integer
            default: 10
      responses:
        '200':
          description: Successful response
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/User'
    post:
      summary: Create user
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/UserInput'
      responses:
        '201':
          description: User created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/User'

  /users/{id}:
    get:
      summary: Get user by ID
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: User found
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/User'
        '404':
          description: User not found

components:
  schemas:
    User:
      type: object
      properties:
        id:
          type: string
          format: uuid
        name:
          type: string
        email:
          type: string
          format: email
        createdAt:
          type: string
          format: date-time
    UserInput:
      type: object
      required:
        - name
        - email
      properties:
        name:
          type: string
        email:
          type: string
```

## Dynamic Response Generation

MockForge generates realistic responses automatically based on your OpenAPI schemas, with support for dynamic data through templates.

### Automatic Response Generation

For basic use cases, MockForge can generate responses directly from your OpenAPI schemas:

```bash
# Start server with automatic response generation
mockforge serve --spec api-spec.json --http-port 3000
```

This generates:
- **UUIDs** for ID fields
- **Random data** for string/number fields
- **Current timestamps** for date-time fields
- **Valid email addresses** for email fields

### Template-Enhanced Responses

For more control, use MockForge's template system in your OpenAPI examples:

```yaml
paths:
  /users:
    get:
      responses:
        '200':
          description: List of users
          content:
            application/json:
              example:
                users:
                  - id: "{{uuid}}"
                    name: "John Doe"
                    email: "john@example.com"
                    createdAt: "{{now}}"
                    lastLogin: "{{now-1d}}"
                  - id: "{{uuid}}"
                    name: "Jane Smith"
                    email: "jane@example.com"
                    createdAt: "{{now-7d}}"
                    lastLogin: "{{now-2h}}"
```

### Template Functions

#### Data Generation Templates
- `{{uuid}}` - Generate unique UUID
- `{{now}}` - Current timestamp
- `{{now+1h}}` - Future timestamp
- `{{now-1d}}` - Past timestamp
- `{{randInt 1 100}}` - Random integer
- `{{randFloat 0.0 1.0}}` - Random float

#### Request Data Templates
- `{{request.path.id}}` - Access path parameters
- `{{request.query.limit}}` - Access query parameters
- `{{request.header.Authorization}}` - Access headers
- `{{request.body.name}}` - Access request body fields

## Request Matching and Routing

MockForge uses sophisticated matching to route requests to appropriate responses.

### Matching Priority

1. **Exact Path + Method Match**
2. **Parameterized Path Match** (e.g., `/users/{id}`)
3. **Query Parameter Conditions**
4. **Header-Based Conditions**
5. **Request Body Matching**
6. **Default Response** (catch-all)

### Path Parameter Handling

```yaml
/users/{id}:
  get:
    parameters:
      - name: id
        in: path
        required: true
        schema:
          type: string
    responses:
      '200':
        content:
          application/json:
            example:
              id: "{{request.path.id}}"
              name: "User {{request.path.id}}"
              retrievedAt: "{{now}}"
```

### Query Parameter Filtering

```yaml
/users:
  get:
    parameters:
      - name: status
        in: query
        schema:
          type: string
          enum: [active, inactive]
      - name: limit
        in: query
        schema:
          type: integer
          default: 10
    responses:
      '200':
        content:
          application/json:
            example: "{{#if (eq request.query.status 'active')}}active_users{{else}}all_users{{/if}}"
```

## Response Scenarios

MockForge supports multiple response scenarios for testing different conditions.

### Success Responses

```yaml
responses:
  '200':
    description: Success
    content:
      application/json:
        example:
          status: "success"
          data: { ... }
```

### Error Responses

```yaml
responses:
  '400':
    description: Bad Request
    content:
      application/json:
        example:
          error: "INVALID_INPUT"
          message: "The provided input is invalid"
  '404':
    description: Not Found
    content:
      application/json:
        example:
          error: "NOT_FOUND"
          message: "Resource not found"
  '500':
    description: Internal Server Error
    content:
      application/json:
        example:
          error: "INTERNAL_ERROR"
          message: "An unexpected error occurred"
```

### Conditional Responses

Use templates to return different responses based on request data:

```yaml
responses:
  '200':
    content:
      application/json:
        example: |
          {{#if (eq request.query.format 'detailed')}}
          {
            "id": "{{uuid}}",
            "name": "Detailed User",
            "email": "user@example.com",
            "profile": {
              "bio": "Detailed user profile",
              "preferences": { ... }
            }
          }
          {{else}}
          {
            "id": "{{uuid}}",
            "name": "Basic User",
            "email": "user@example.com"
          }
          {{/if}}
```

## Advanced Features

### Response Latency Simulation

```bash
# Add random latency (100-500ms)
MOCKFORGE_LATENCY_ENABLED=true \
MOCKFORGE_LATENCY_MIN_MS=100 \
MOCKFORGE_LATENCY_MAX_MS=500 \
mockforge serve --spec api-spec.json
```

### Failure Injection

```bash
# Enable random failures (10% chance)
MOCKFORGE_FAILURES_ENABLED=true \
MOCKFORGE_FAILURE_RATE=0.1 \
mockforge serve --spec api-spec.json
```

### Request/Response Recording

```bash
# Record all HTTP interactions
MOCKFORGE_RECORD_ENABLED=true \
mockforge serve --spec api-spec.json
```

### Response Replay

```bash
# Replay recorded responses
MOCKFORGE_REPLAY_ENABLED=true \
mockforge serve --spec api-spec.json
```

## Testing Your Mocks

### Manual Testing with curl

```bash
# Test GET endpoint
curl http://localhost:3000/users

# Test POST endpoint
curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Test User", "email": "test@example.com"}'

# Test path parameters
curl http://localhost:3000/users/123

# Test query parameters
curl "http://localhost:3000/users?limit=5&status=active"

# Test error scenarios
curl http://localhost:3000/users/999  # Should return 404
```

### Automated Testing

```bash
#!/bin/bash
# test-api.sh

BASE_URL="http://localhost:3000"

echo "Testing User API..."

# Test user creation
USER_RESPONSE=$(curl -s -X POST $BASE_URL/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Test User", "email": "test@example.com"}')

echo "Created user: $USER_RESPONSE"

# Extract user ID (assuming response contains id)
USER_ID=$(echo $USER_RESPONSE | jq -r '.id')

# Test user retrieval
RETRIEVED_USER=$(curl -s $BASE_URL/users/$USER_ID)
echo "Retrieved user: $RETRIEVED_USER"

# Test user listing
USER_LIST=$(curl -s $BASE_URL/users)
echo "User list: $USER_LIST"

echo "API tests completed!"
```

## Best Practices

### OpenAPI Specification Tips

1. **Use descriptive operation IDs** for better organization
2. **Include examples** in your OpenAPI spec for consistent responses
3. **Define reusable components** for common schemas
4. **Use appropriate HTTP status codes** for different scenarios
5. **Document all parameters** clearly

### Template Usage Guidelines

1. **Enable templates only when needed** for security
2. **Use meaningful template variables** for maintainability
3. **Test template expansion** thoroughly
4. **Avoid complex logic in templates** - keep it simple

### Response Design Principles

1. **Match real API behavior** as closely as possible
2. **Include appropriate error responses** for testing
3. **Use consistent data formats** across endpoints
4. **Consider pagination** for list endpoints
5. **Include metadata** like timestamps and request IDs

### Performance Considerations

1. **Use static responses** when dynamic data isn't needed
2. **Limit template complexity** to maintain response times
3. **Configure appropriate timeouts** for your use case
4. **Monitor memory usage** with large response payloads

## Troubleshooting

### Common Issues

**Templates not expanding**: Ensure `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true`

**OpenAPI spec not loading**: Check file path and JSON/YAML syntax

**Wrong response returned**: Verify request matching rules and parameter handling

**Performance issues**: Reduce template complexity or use static responses

**Port conflicts**: Change default ports with `--http-port` option

## Advanced Behavior and Simulation

MockForge supports advanced behavior simulation features for realistic API testing:

### Record & Playback

Automatically record API interactions and convert them to replayable fixtures:

```bash
# Record requests while proxying
mockforge serve --spec api-spec.json --proxy --record

# Convert recordings to stub mappings
mockforge recorder convert --input recordings.db --output fixtures/
```

### Stateful Behavior

Simulate stateful APIs where responses change based on previous requests:

```yaml
core:
  stateful:
    enabled: true
    state_machines:
      - name: "order_workflow"
        resource_id_extract:
          type: "path_param"
          param: "order_id"
        transitions:
          - method: "POST"
            path_pattern: "/api/orders"
            from_state: "initial"
            to_state: "pending"
```

### Per-Route Fault Injection

Configure fault injection on specific routes:

```yaml
core:
  routes:
    - path: "/api/payments/process"
      method: "POST"
      fault_injection:
        enabled: true
        probability: 0.05
        fault_types:
          - type: "http_error"
            status_code: 503
```

### Per-Route Latency

Simulate network conditions per route:

```yaml
core:
  routes:
    - path: "/api/search"
      method: "GET"
      latency:
        enabled: true
        distribution: "normal"
        mean_ms: 500.0
        std_dev_ms: 100.0
```

### Conditional Proxying

Proxy requests conditionally based on request attributes:

```yaml
core:
  proxy:
    rules:
      - pattern: "/api/admin/*"
        upstream_url: "https://admin-api.example.com"
        condition: "$.user.role == 'admin'"
```

For detailed documentation on these features, see [Advanced Behavior and Simulation](../../../docs/ADVANCED_BEHAVIOR_SIMULATION.md).

For more advanced HTTP mocking features, see the following guides:
- [OpenAPI Integration](http-mocking/openapi.md) - Advanced OpenAPI features
- [Custom Responses](http-mocking/custom-responses.md) - Complex response scenarios
- [Dynamic Data](http-mocking/dynamic-data.md) - Advanced templating techniques
