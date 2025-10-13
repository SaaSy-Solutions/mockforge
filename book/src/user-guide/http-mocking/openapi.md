# OpenAPI Integration

MockForge provides advanced OpenAPI integration capabilities beyond basic spec loading and response generation. This guide covers sophisticated features for enterprise-grade API mocking.

## Advanced Request Validation

MockForge supports comprehensive request validation against OpenAPI schemas with multiple validation modes and granular control.

### Validation Modes

```bash
# Disable validation completely
MOCKFORGE_REQUEST_VALIDATION=off mockforge serve --spec api-spec.json

# Log warnings but allow invalid requests
MOCKFORGE_REQUEST_VALIDATION=warn mockforge serve --spec api-spec.json

# Reject invalid requests (default)
MOCKFORGE_REQUEST_VALIDATION=enforce mockforge serve --spec api-spec.json
```

### Response Validation

Enable validation of generated responses against OpenAPI schemas:

```bash
# Validate responses against schemas
MOCKFORGE_RESPONSE_VALIDATION=true mockforge serve --spec api-spec.json
```

### Custom Validation Status Codes

Configure HTTP status codes for validation failures:

```bash
# Use 422 Unprocessable Entity for validation errors
MOCKFORGE_VALIDATION_STATUS=422 mockforge serve --spec api-spec.json
```

### Validation Overrides

Skip validation for specific routes:

```yaml
validation:
  mode: enforce
  overrides:
    "GET /health": "off"
    "POST /webhooks/*": "warn"
```

### Aggregated Error Reporting

Control how validation errors are reported:

```bash
# Report all validation errors at once
MOCKFORGE_AGGREGATE_ERRORS=true mockforge serve --spec api-spec.json

# Stop at first validation error
MOCKFORGE_AGGREGATE_ERRORS=false mockforge serve --spec api-spec.json
```

## Security Scheme Validation

MockForge validates authentication and authorization requirements defined in your OpenAPI spec.

### Supported Security Schemes

- **HTTP Basic Authentication**: Validates `Authorization: Basic <credentials>` headers
- **Bearer Tokens**: Validates `Authorization: Bearer <token>` headers
- **API Keys**: Supports header and query parameter API keys
- **OAuth2**: Basic OAuth2 flow validation

### Security Validation Example

```yaml
openapi: 3.0.0
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
    apiKey:
      type: apiKey
      in: header
      name: X-API-Key

security:
  - bearerAuth: []
  - apiKey: []

paths:
  /protected:
    get:
      security:
        - bearerAuth: []
```

```bash
# Test with valid Bearer token
curl -H "Authorization: Bearer eyJ0eXAi..." http://localhost:3000/protected

# Test with API key
curl -H "X-API-Key: your-api-key" http://localhost:3000/protected
```

## Schema Resolution and References

MockForge fully supports OpenAPI schema references (`$ref`) for reusable components.

### Component References

```yaml
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
        profile:
          $ref: '#/components/schemas/UserProfile'

    UserProfile:
      type: object
      properties:
        bio:
          type: string
        avatar:
          type: string
          format: uri

  responses:
    UserResponse:
      description: User data
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/User'

paths:
  /users/{id}:
    get:
      responses:
        '200':
          $ref: '#/components/responses/UserResponse'
```

### Request Body References

```yaml
components:
  requestBodies:
    UserCreate:
      required: true
      content:
        application/json:
          schema:
            type: object
            required:
              - name
              - email
            properties:
              name:
                type: string
              email:
                type: string
                format: email

paths:
  /users:
    post:
      requestBody:
        $ref: '#/components/requestBodies/UserCreate'
```

## Multiple OpenAPI Specifications

MockForge can serve multiple OpenAPI specifications simultaneously with path-based routing.

### Configuration for Multiple Specs

```yaml
server:
  http_port: 3000

specs:
  - name: user-api
    path: /api/v1
    spec: user-api.json
  - name: admin-api
    path: /api/admin
    spec: admin-api.json
```

### Base Path Routing

```bash
# Routes to user-api.json endpoints
curl http://localhost:3000/api/v1/users

# Routes to admin-api.json endpoints
curl http://localhost:3000/api/admin/users
```

## Advanced Routing and Matching

MockForge provides sophisticated request matching beyond simple path/method combinations.

### Path Parameter Constraints

```yaml
paths:
  /users/{id}:
    get:
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
            pattern: '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
```

### Query Parameter Matching

```yaml
paths:
  /users:
    get:
      parameters:
        - name: status
          in: query
          schema:
            type: string
            enum: [active, inactive, pending]
        - name: limit
          in: query
          schema:
            type: integer
            minimum: 1
            maximum: 100
            default: 10
```

### Header-Based Routing

```yaml
paths:
  /api/v1/users:
    get:
      parameters:
        - name: X-API-Version
          in: header
          schema:
            type: string
            enum: [v1, v2]
```

## Template Expansion in Responses

Advanced template features for dynamic response generation.

### Advanced Template Functions

```yaml
responses:
  '200':
    content:
      application/json:
        example:
          id: "{{uuid}}"
          createdAt: "{{now}}"
          expiresAt: "{{now+1h}}"
          lastModified: "{{now-30m}}"
          randomValue: "{{randInt 1 100}}"
          randomFloat: "{{randFloat 0.0 5.0}}"
          userAgent: "{{request.header.User-Agent}}"
          apiVersion: "{{request.header.X-API-Version}}"
          userId: "{{request.path.id}}"
          searchQuery: "{{request.query.q}}"
```

### Conditional Templates

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
            "profile": {
              "bio": "User biography",
              "preferences": {}
            }
          }
          {{else}}
          {
            "id": "{{uuid}}",
            "name": "Basic User"
          }
          {{/if}}
```

### Template Security

Enable template expansion only when needed:

```bash
# Enable template expansion
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true mockforge serve --spec api-spec.json
```

## Performance Optimization

Strategies for handling large OpenAPI specifications efficiently.

### Lazy Loading

MockForge loads and parses OpenAPI specs on startup but generates routes lazily:

```bash
# Monitor startup performance
time mockforge serve --spec large-api.json
```

### Route Caching

Generated routes are cached in memory for optimal performance:

```bash
# Check memory usage with large specs
MOCKFORGE_LOG_LEVEL=debug mockforge serve --spec large-api.json
```

### Validation Performance

Disable expensive validations in high-throughput scenarios:

```bash
# Disable response validation for better performance
MOCKFORGE_RESPONSE_VALIDATION=false mockforge serve --spec api-spec.json
```

## Custom Validation Options

Fine-tune validation behavior for your specific needs.

### Validation Configuration

```yaml
validation:
  mode: enforce
  aggregate_errors: true
  validate_responses: false
  status_code: 422
  overrides:
    "GET /health": "off"
    "POST /webhooks/*": "warn"
  admin_skip_prefixes:
    - "/admin"
    - "/internal"
```

### Environment Variables

```bash
# Validation mode
MOCKFORGE_REQUEST_VALIDATION=enforce

# Error aggregation
MOCKFORGE_AGGREGATE_ERRORS=true

# Response validation
MOCKFORGE_RESPONSE_VALIDATION=false

# Custom status code
MOCKFORGE_VALIDATION_STATUS=422

# Template expansion
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true
```

## OpenAPI Extensions

MockForge supports OpenAPI extensions (`x-` prefixed properties) for custom behavior.

### Custom Extensions

```yaml
paths:
  /users:
    get:
      x-mockforge-delay: 1000  # Add 1 second delay
      x-mockforge-failure-rate: 0.1  # 10% failure rate
      responses:
        '200':
          x-mockforge-template: true  # Enable template expansion
```

### Vendor Extensions

```yaml
info:
  x-mockforge-config:
    enable_cors: true
    default_response_format: json

paths:
  /api/users:
    x-vendor-custom-behavior: enabled
```

## Troubleshooting

Common issues and solutions for advanced OpenAPI integration.

### Validation Errors

**Problem**: Requests are rejected with validation errors
```json
{
  "error": "request validation failed",
  "status": 422,
  "details": [
    {
      "path": "body.name",
      "code": "required",
      "message": "Missing required field: name"
    }
  ]
}
```

**Solutions**:
```bash
# Switch to warning mode
MOCKFORGE_REQUEST_VALIDATION=warn mockforge serve --spec api-spec.json

# Disable validation for specific routes
# Add to config.yaml:
validation:
  overrides:
    "POST /users": "off"
```

### Schema Reference Issues

**Problem**: `$ref` references not resolving correctly

**Solutions**:
- Ensure component names match exactly
- Check that referenced components exist
- Validate your OpenAPI spec with external tools

### Performance Issues

**Problem**: Slow startup or high memory usage with large specs

**Solutions**:
```bash
# Disable non-essential features
MOCKFORGE_RESPONSE_VALIDATION=false
MOCKFORGE_AGGREGATE_ERRORS=false

# Monitor with debug logging
MOCKFORGE_LOG_LEVEL=debug mockforge serve --spec api-spec.json
```

### Security Validation Failures

**Problem**: Authentication requests failing

**Solutions**:
- Verify security scheme definitions
- Check header formats (e.g., `Bearer ` prefix)
- Ensure global security requirements are met

### Template Expansion Issues

**Problem**: Templates not expanding in responses

**Solutions**:
```bash
# Enable template expansion
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true mockforge serve --spec api-spec.json

# Check template syntax
# Use {{variable}} format, not ${variable}
```

## Best Practices

### Specification Management

1. **Version Control**: Keep OpenAPI specs in version control alongside mock configurations
2. **Validation**: Use external validators to ensure spec correctness
3. **Documentation**: Include comprehensive examples and descriptions
4. **Modularity**: Use components and references for maintainable specs

### Performance Tuning

1. **Selective Validation**: Disable validation for high-traffic endpoints
2. **Template Usage**: Only enable templates when dynamic data is needed
3. **Caching**: Leverage MockForge's built-in route caching
4. **Monitoring**: Monitor memory usage and response times

### Security Considerations

1. **Validation Modes**: Use appropriate validation levels for different environments
2. **Template Security**: Be cautious with user-controlled template input
3. **Authentication**: Properly configure security schemes for protected endpoints
4. **Overrides**: Use validation overrides judiciously

For basic OpenAPI integration features, see the [HTTP Mocking guide](../http-mocking.md). For dynamic data generation, see the [Dynamic Data guide](dynamic-data.md).