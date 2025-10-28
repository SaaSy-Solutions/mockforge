# HTTP Scenario Switching

MockForge supports scenario-based response switching, allowing you to define multiple response examples and switch between them at runtime. This is useful for testing different application states, error conditions, and edge cases without modifying your OpenAPI specification.

## Overview

Scenarios are defined using the standard OpenAPI `examples` field (plural) in your response definitions. You can then switch between these scenarios using either:

1. **`X-Mockforge-Scenario` HTTP Header** - Per-request scenario selection
2. **`MOCKFORGE_HTTP_SCENARIO` Environment Variable** - Global scenario selection

## Defining Scenarios

Scenarios are defined in your OpenAPI specification using the `examples` field under response content types.

### Basic Example

```yaml
openapi: 3.0.3
info:
  title: User API
  version: 1.0.0

paths:
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
          description: Successful response
          content:
            application/json:
              examples:
                # Happy path scenario
                happy:
                  summary: User is active
                  value:
                    id: "123"
                    name: "John Doe"
                    status: "active"
                    email: "john@example.com"

                # Error scenario
                errors:
                  summary: User is suspended
                  value:
                    id: "123"
                    name: "Suspended User"
                    status: "suspended"
                    reason: "Terms of service violation"

                # Edge case scenario
                edge:
                  summary: New user with minimal data
                  value:
                    id: "123"
                    name: "New User"
                    status: "pending_verification"
                    email: "new@example.com"
```

## Switching Scenarios

### Method 1: HTTP Header (Recommended)

Use the `X-Mockforge-Scenario` header to select a scenario per request:

```bash
# Request with "happy" scenario
curl -H "X-Mockforge-Scenario: happy" http://localhost:8080/users/123

# Request with "errors" scenario
curl -H "X-Mockforge-Scenario: errors" http://localhost:8080/users/123

# Request with "edge" scenario
curl -H "X-Mockforge-Scenario: edge" http://localhost:8080/users/123
```

**Benefits:**
- Per-request control
- Can be used in automated tests
- Allows testing different scenarios in the same test run
- Header takes precedence over environment variable

### Method 2: Environment Variable

Use the `MOCKFORGE_HTTP_SCENARIO` environment variable to set a global scenario:

```bash
# Set global scenario to "errors"
export MOCKFORGE_HTTP_SCENARIO=errors
mockforge serve examples/scenario-switching-demo.yaml

# Or inline:
MOCKFORGE_HTTP_SCENARIO=happy mockforge serve examples/scenario-switching-demo.yaml
```

**Benefits:**
- Simple configuration
- Works for all requests
- Good for testing a specific scenario across your entire application

**Note:** The HTTP header takes precedence over the environment variable if both are set.

## Scenario Precedence

MockForge uses the following order when selecting which example to return:

1. **Explicit scenario match**: If the requested scenario name exists, use it
2. **Fallback to first example**: If the scenario doesn't exist or no scenario is specified, use the first example defined
3. **Schema-based generation**: If no examples are defined, generate from the schema

## Example Usage Scenarios

### Testing Happy Path

```yaml
examples:
  happy:
    value:
      orderId: "ord_123"
      status: "confirmed"
      total: 99.99
      items: [...]
```

```bash
curl -H "X-Mockforge-Scenario: happy" http://localhost:8080/orders
```

### Testing Error Conditions

```yaml
examples:
  errors:
    value:
      orderId: "ord_123"
      status: "payment_failed"
      errorCode: "INSUFFICIENT_FUNDS"
      errorMessage: "Payment card has insufficient funds"
```

```bash
curl -H "X-Mockforge-Scenario: errors" http://localhost:8080/orders
```

### Testing Edge Cases

```yaml
examples:
  edge:
    value:
      orderId: "ord_123"
      status: "partially_available"
      total: 49.99
      warning: "Some items are unavailable"
```

```bash
curl -H "X-Mockforge-Scenario: edge" http://localhost:8080/orders
```

## Integration with Testing Frameworks

### JavaScript/TypeScript (Vitest, Jest)

```typescript
import { test, expect } from 'vitest';

test('should return active user in happy scenario', async () => {
  const response = await fetch('http://localhost:8080/users/123', {
    headers: {
      'X-Mockforge-Scenario': 'happy'
    }
  });

  const data = await response.json();
  expect(data.status).toBe('active');
});

test('should return suspended user in errors scenario', async () => {
  const response = await fetch('http://localhost:8080/users/123', {
    headers: {
      'X-Mockforge-Scenario': 'errors'
    }
  });

  const data = await response.json();
  expect(data.status).toBe('suspended');
});
```

### Python (pytest, requests)

```python
import pytest
import requests

BASE_URL = "http://localhost:8080"

def test_happy_scenario():
    response = requests.get(
        f"{BASE_URL}/users/123",
        headers={"X-Mockforge-Scenario": "happy"}
    )
    assert response.json()["status"] == "active"

def test_error_scenario():
    response = requests.get(
        f"{BASE_URL}/users/123",
        headers={"X-Mockforge-Scenario": "errors"}
    )
    assert response.json()["status"] == "suspended"
```

### Rust (reqwest)

```rust
#[tokio::test]
async fn test_happy_scenario() {
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8080/users/123")
        .header("X-Mockforge-Scenario", "happy")
        .send()
        .await
        .unwrap();

    let data: serde_json::Value = response.json().await.unwrap();
    assert_eq!(data["status"], "active");
}
```

## Advanced Patterns

### Scenario Naming Conventions

We recommend using these standard scenario names for consistency:

- **`happy`**: Normal, successful operation with all data present
- **`errors`**: Error conditions, failures, or problematic states
- **`edge`**: Edge cases, boundary conditions, minimal data, unusual but valid states

However, you can use any names that make sense for your use case:

```yaml
examples:
  new_user:
    value: { ... }
  premium_user:
    value: { ... }
  trial_expired:
    value: { ... }
  rate_limited:
    value: { ... }
```

### Multiple Status Codes with Scenarios

You can define scenarios for different status codes:

```yaml
responses:
  '200':
    content:
      application/json:
        examples:
          happy:
            value: { status: "success" }
          errors:
            value: { status: "degraded" }

  '404':
    content:
      application/json:
        example:
          error: "Resource not found"
```

Note: The scenario selection only works with the response that matches the status code defined in your spec.

### Combining with Other MockForge Features

Scenarios work seamlessly with other MockForge features:

#### With Template Expansion

```yaml
examples:
  happy:
    value:
      orderId: "{{uuid}}"
      createdAt: "{{now}}"
      status: "confirmed"
```

Enable template expansion:
```bash
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true mockforge serve spec.yaml
```

#### With Latency Injection

```yaml
x-mockforge-latency:
  mode: fixed
  duration_ms: 500
```

The latency applies regardless of which scenario is selected.

#### With Request Validation

Request validation runs before scenario selection, so invalid requests will be rejected before any scenario is evaluated.

## Troubleshooting

### Scenario Not Found

If you request a scenario that doesn't exist:

```bash
curl -H "X-Mockforge-Scenario: nonexistent" http://localhost:8080/users/123
```

MockForge will fall back to the first defined example and log a warning:

```
WARN Scenario 'nonexistent' not found in examples, falling back to first example
```

### No Examples Defined

If no examples are defined, MockForge will generate a response based on the schema:

```yaml
responses:
  '200':
    content:
      application/json:
        schema:
          type: object
          properties:
            id:
              type: string
            name:
              type: string
```

The scenario header is ignored in this case, and schema-based generation is used.

### Single `example` vs Multiple `examples`

If you use the singular `example` field instead of `examples`, scenarios won't work:

```yaml
# This will NOT work with scenarios
content:
  application/json:
    example:
      id: "123"
      name: "John"
```

Always use the plural `examples` field for scenario support:

```yaml
# This WILL work with scenarios
content:
  application/json:
    examples:
      happy:
        value:
          id: "123"
          name: "John"
```

## Best Practices

1. **Use Consistent Naming**: Stick to standard scenario names (`happy`, `errors`, `edge`) for better team communication

2. **Document Scenarios**: Use the `summary` and `description` fields to explain each scenario:
   ```yaml
   examples:
     happy:
       summary: Normal successful operation
       description: User is active with all required fields populated
       value: { ... }
   ```

3. **Cover Common Cases**: At minimum, define:
   - One happy path scenario
   - One error scenario
   - One edge case scenario

4. **Keep Responses Realistic**: Make sure your scenario responses match what your real API would return

5. **Test Scenario Switching**: Include scenario switching in your integration tests to ensure examples stay in sync

6. **Version Control**: Keep your OpenAPI specs with scenarios in version control alongside your application code

## Complete Example

See [examples/scenario-switching-demo.yaml](../examples/scenario-switching-demo.yaml) for a complete working example with multiple endpoints and scenarios.

## API Reference

### HTTP Header

- **Name**: `X-Mockforge-Scenario`
- **Type**: String
- **Example**: `happy`, `errors`, `edge`
- **Precedence**: Highest (overrides environment variable)

### Environment Variable

- **Name**: `MOCKFORGE_HTTP_SCENARIO`
- **Type**: String
- **Example**: `happy`, `errors`, `edge`
- **Precedence**: Lower (header takes precedence)

### OpenAPI Extensions

Scenarios use the standard OpenAPI `examples` field - no custom extensions required:

```yaml
content:
  application/json:
    examples:
      <scenario-name>:
        summary: <optional summary>
        description: <optional description>
        value: <response body>
```

## Related Features

- [Request Validation](./VALIDATION.md) - Validate requests before scenario selection
- [Response Templating](./TEMPLATING.md) - Use dynamic templates in scenario responses
- [Latency Injection](./LATENCY.md) - Add realistic delays to scenario responses
- [Failure Injection](./FAILURE_INJECTION.md) - Simulate failures alongside scenarios
