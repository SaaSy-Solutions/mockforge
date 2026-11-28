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
# Add random latency via the chaos engine
mockforge serve --spec api-spec.json \
  --chaos --chaos-latency-range "100-500"
```

For full control (jitter, probability, fixed delay, per-route), use
`observability.chaos.latency.*` in YAML. See
[Chaos Engineering](./chaos-engineering.md).

### Failure Injection

```bash
# 10% of requests get 500/503
mockforge serve --spec api-spec.json \
  --chaos --chaos-http-errors "500,503" --chaos-http-error-probability 0.1
```

### Request/Response Recording & Replay

Recording, replay, and proxy modes are configured via CLI flags or YAML —
see `mockforge serve --help` for `--record`, `--replay`, `--proxy-target`.

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

**Templates not expanding**: ensure `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true` is set in your environment (it's off by default to keep responses literal)

**OpenAPI spec not loading**: Check file path and JSON/YAML syntax

**Wrong response returned**: Verify request matching rules and parameter handling

**Performance issues**: Reduce template complexity or use static responses

**Port conflicts**: Change default ports with `--http-port` option

## Advanced Behavior and Simulation

MockForge supports advanced behavior simulation features for realistic API testing:

### Record & Playback

Automatically record API interactions and convert them to replayable fixtures:

```bash
# Record the requests the server handles
mockforge serve --spec api-spec.json --recorder --recorder-db ./mockforge-recordings.db

# Convert recordings to stub mappings
mockforge recorder convert --input ./mockforge-recordings.db --output fixtures/
```

#### Scrub sensitive data while recording

Nothing is scrubbed unless you say how. Set `MOCKFORGE_CAPTURE_SCRUB` to a
JSON object with a `rules` list before recording traffic that carries
personal data:

```bash
export MOCKFORGE_CAPTURE_SCRUB='{
  "rules": [
    {"type": "header", "name": "authorization", "replacement": "Bearer REDACTED"},
    {"type": "field", "field": "email", "replacement": "user@example.com"},
    {"type": "uuid", "replacement": "00000000-0000-0000-0000-{{counter:012}}"},
    {"type": "email", "replacement": "user{{counter}}@example.com"},
    {"type": "ip_address", "replacement": "127.0.0.1"},
    {"type": "credit_card", "replacement": "XXXX-XXXX-XXXX-XXXX"},
    {"type": "key"},
    {"type": "regex", "pattern": "sk-[A-Za-z0-9]{20,}", "replacement": "sk-REDACTED"}
  ]
}'
```

| Rule | Replaces |
| --- | --- |
| `header` | One header, by name, wherever it appears. |
| `field` | One JSON field, by a path such as `user.email`, and a header of the same name. |
| `regex` | Every match of a pattern. |
| `uuid`, `email`, `ip_address`, `credit_card` | Every value of that shape. |
| `key` | Every key of a known shape, with a made-up key of the same shape. |

`{{counter}}` in the replacement of a built-in rule is the value's
pseudonym. The same UUID, address, IP or card number is replaced the same way
wherever it appears in a recording, and two different values never share one,
so a converted fixture still tells one user from another. A replacement
without it is used for every match. `counter_seed` sets the first number.

##### Keep keys out of a recording

The `key` rule takes no replacement. A key is known by the shape of its
value, not by the name of the header or of the field that holds it, so the
rule needs no list of names to keep up to date. It finds a key wherever the
request or the answer holds one:

- as the whole of a header's value, or after a scheme, as in `Bearer <key>`;
- as a parameter of the query string;
- as a string of a JSON body, or as the name of a field that is itself a key.

Each key is replaced with a made-up key of the same shape. The same key gives
the same made-up key in every exchange of a recording and in the next
recording, so a converted fixture and a pack begun from the recording still
agree, and nothing of the key is kept beyond what its shape fixes. The shapes
are those of the table `mockforge_contracts::keys`, the one the exchange log
and `range new` use: the keys of about a hundred kinds, from AWS, Anthropic,
GitHub, Google, OpenAI, Slack, Stripe and the others.

```bash
export MOCKFORGE_CAPTURE_SCRUB='{"rules": [{"type": "key"}]}'
```

A shape says what a key looks like and not that a value is one. A value of a
known shape that is not a key is replaced too, in the same shape, so a
recording made with the rule is still fit to replay. Only text bodies are
scrubbed. A body recorded in another encoding is stored as it arrived.

If `MOCKFORGE_CAPTURE_SCRUB` cannot be read, the recorder keeps the method,
the path and the status and redacts everything else, so a mistake in the
configuration never records raw data. The recorded body then says that
`MOCKFORGE_CAPTURE_SCRUB` could not be read.

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
