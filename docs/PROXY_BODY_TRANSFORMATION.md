# Proxy Body Transformation Guide

This guide covers the body transformation features in MockForge's browser proxy mode, allowing you to inspect and modify request/response bodies using JSONPath-based transformations.

## Overview

The body transformation feature enables you to:

- **Inspect intercepted requests and responses** from browsers and mobile apps
- **Modify request bodies** before they reach the upstream server
- **Modify response bodies** before they reach the client
- **Use JSONPath expressions** to target specific fields in JSON payloads
- **Apply template expansion** for dynamic values (UUIDs, faker data, etc.)
- **Manage transformation rules** via REST API or UI

## Quick Start

### 1. Start the Proxy with Body Transformation

```bash
mockforge proxy --port 8081 --admin --admin-port 9080
```

### 2. Access the Proxy Inspector UI

Navigate to the Proxy Inspector in the admin UI:
- Open `http://127.0.0.1:9080` in your browser
- Navigate to the "Proxy Inspector" tab
- Or access directly via the `proxy-inspector` route

### 3. Create Your First Transformation Rule

#### Using the UI:

1. Click "Create Rule" in the Proxy Inspector
2. Enter a URL pattern (e.g., `/api/users/*`)
3. Select rule type: "Request" or "Response"
4. Add body transformations:
   - **JSONPath**: `$.userId` (targets the `userId` field)
   - **Replacement**: `{{uuid}}` (replaces with a generated UUID)
   - **Operation**: Replace, Add, or Remove
5. Click "Create Rule"

#### Using the REST API:

```bash
curl -X POST http://127.0.0.1:9080/__mockforge/api/proxy/rules \
  -H "Content-Type: application/json" \
  -d '{
    "pattern": "/api/users/*",
    "type": "request",
    "body_transforms": [
      {
        "path": "$.userId",
        "replace": "{{uuid}}",
        "operation": "replace"
      }
    ],
    "enabled": true
  }'
```

## JSONPath Syntax

JSONPath expressions start with `$` and use dot notation to navigate JSON structures:

### Basic Examples

```json
{
  "userId": 123,
  "email": "user@example.com",
  "profile": {
    "name": "John Doe",
    "age": 30
  },
  "tags": ["admin", "user"]
}
```

| JSONPath | Targets |
|----------|---------|
| `$.userId` | The `userId` field (value: `123`) |
| `$.email` | The `email` field |
| `$.profile.name` | The nested `name` field (value: `"John Doe"`) |
| `$.profile.age` | The nested `age` field |
| `$.tags[0]` | First element of the `tags` array |
| `$.tags[1]` | Second element of the `tags` array |

### Supported Operations

#### Replace
Replaces the value at the specified JSONPath:

```json
// Original
{ "userId": 123 }

// Transform: $.userId → {{uuid}}
// Result
{ "userId": "550e8400-e29b-41d4-a716-446655440000" }
```

#### Add
Adds a new field at the specified JSONPath (only for objects):

```json
// Original
{ "userId": 123 }

// Transform: $.sessionId → {{uuid}} (operation: add)
// Result
{
  "userId": 123,
  "sessionId": "550e8400-e29b-41d4-a716-446655440000"
}
```

#### Remove
Removes the field at the specified JSONPath:

```json
// Original
{
  "userId": 123,
  "internalId": 456
}

// Transform: $.internalId (operation: remove)
// Result
{ "userId": 123 }
```

## Template Expansion

Replacement values support template expansion for dynamic data generation:

### Available Templates

| Template | Description | Example Output |
|----------|-------------|----------------|
| `{{uuid}}` | UUID v4 | `550e8400-e29b-41d4-a716-446655440000` |
| `{{uuid.short}}` | Short UUID (base62) | `2n9MvK7x` |
| `{{faker.name}}` | Random name | `John Doe` |
| `{{faker.email}}` | Random email | `john.doe@example.com` |
| `{{faker.phone}}` | Random phone | `+1-555-123-4567` |
| `{{timestamp}}` | Current Unix timestamp | `1704067200` |
| `{{timestamp.iso}}` | ISO 8601 timestamp | `2024-01-01T00:00:00Z` |
| `{{random.int}}` | Random integer (0-1000) | `742` |
| `{{random.float}}` | Random float (0-1) | `0.742` |

### Template Examples

#### Replace User ID with UUID

```json
{
  "pattern": "/api/users/*",
  "type": "request",
  "body_transforms": [
    {
      "path": "$.userId",
      "replace": "{{uuid}}",
      "operation": "replace"
    }
  ]
}
```

#### Add Session Tracking

```json
{
  "pattern": "/api/auth/login",
  "type": "response",
  "status_codes": [200],
  "body_transforms": [
    {
      "path": "$.sessionId",
      "replace": "{{uuid}}",
      "operation": "add"
    },
    {
      "path": "$.loginTime",
      "replace": "{{timestamp.iso}}",
      "operation": "add"
    }
  ]
}
```

#### Sanitize Sensitive Data

```json
{
  "pattern": "/api/users/*",
  "type": "response",
  "body_transforms": [
    {
      "path": "$.email",
      "replace": "{{faker.email}}",
      "operation": "replace"
    },
    {
      "path": "$.phone",
      "replace": "{{faker.phone}}",
      "operation": "replace"
    },
    {
      "path": "$.ssn",
      "replace": "",
      "operation": "remove"
    }
  ]
}
```

## Request vs Response Rules

### Request Rules

Request rules modify the request body **before** it reaches the upstream server:

```json
{
  "pattern": "/api/users",
  "type": "request",
  "body_transforms": [
    {
      "path": "$.userId",
      "replace": "{{uuid}}",
      "operation": "replace"
    }
  ]
}
```

**Use cases:**
- Inject test user IDs
- Add tracking headers/fields
- Modify request parameters for testing
- Sanitize sensitive data before logging

### Response Rules

Response rules modify the response body **after** it comes from the upstream server but **before** it reaches the client:

```json
{
  "pattern": "/api/users/*",
  "type": "response",
  "status_codes": [200, 201],
  "body_transforms": [
    {
      "path": "$.email",
      "replace": "{{faker.email}}",
      "operation": "replace"
    }
  ]
}
```

**Use cases:**
- Mask sensitive data in responses
- Inject test data
- Modify response structure for frontend testing
- Add debugging information

### Status Code Filtering

Response rules can filter by status codes:

```json
{
  "pattern": "/api/users/*",
  "type": "response",
  "status_codes": [404, 500],
  "body_transforms": [
    {
      "path": "$.error",
      "replace": "Test error message",
      "operation": "replace"
    }
  ]
}
```

This rule only applies to 404 and 500 responses.

## REST API Reference

### List All Rules

```bash
GET /__mockforge/api/proxy/rules
```

Response:
```json
{
  "rules": [
    {
      "id": 0,
      "pattern": "/api/users/*",
      "type": "request",
      "status_codes": [],
      "body_transforms": [
        {
          "path": "$.userId",
          "replace": "{{uuid}}",
          "operation": "replace"
        }
      ],
      "enabled": true
    }
  ]
}
```

### Get Specific Rule

```bash
GET /__mockforge/api/proxy/rules/{id}
```

### Create Rule

```bash
POST /__mockforge/api/proxy/rules
Content-Type: application/json

{
  "pattern": "/api/users/*",
  "type": "request",
  "status_codes": [],
  "body_transforms": [
    {
      "path": "$.userId",
      "replace": "{{uuid}}",
      "operation": "replace"
    }
  ],
  "enabled": true
}
```

### Update Rule

```bash
PUT /__mockforge/api/proxy/rules/{id}
Content-Type: application/json

{
  "pattern": "/api/users/*",
  "type": "request",
  "body_transforms": [
    {
      "path": "$.userId",
      "replace": "{{uuid}}",
      "operation": "replace"
    }
  ],
  "enabled": false
}
```

### Delete Rule

```bash
DELETE /__mockforge/api/proxy/rules/{id}
```

### Inspect Intercepted Traffic

```bash
GET /__mockforge/api/proxy/inspect?limit=50
```

Response:
```json
{
  "requests": [
    {
      "id": "req-123",
      "timestamp": "2024-01-01T00:00:00Z",
      "method": "POST",
      "url": "/api/users",
      "headers": { "Content-Type": "application/json" },
      "body": "{\"userId\":123}"
    }
  ],
  "responses": [
    {
      "id": "res-456",
      "timestamp": "2024-01-01T00:00:01Z",
      "status_code": 200,
      "headers": { "Content-Type": "application/json" },
      "body": "{\"id\":123,\"name\":\"John\"}"
    }
  ],
  "limit": 50
}
```

## Common Use Cases

### 1. Frontend Testing with Mock Data

Replace real API responses with test data:

```json
{
  "pattern": "/api/users/*",
  "type": "response",
  "status_codes": [200],
  "body_transforms": [
    {
      "path": "$.id",
      "replace": "{{random.int}}",
      "operation": "replace"
    },
    {
      "path": "$.email",
      "replace": "{{faker.email}}",
      "operation": "replace"
    }
  ]
}
```

### 2. Injecting Test User IDs

```json
{
  "pattern": "/api/auth/*",
  "type": "request",
  "body_transforms": [
    {
      "path": "$.userId",
      "replace": "test-user-{{uuid.short}}",
      "operation": "replace"
    }
  ]
}
```

### 3. Masking Sensitive Data

```json
{
  "pattern": "/api/*",
  "type": "response",
  "body_transforms": [
    {
      "path": "$.ssn",
      "replace": "",
      "operation": "remove"
    },
    {
      "path": "$.creditCard",
      "replace": "",
      "operation": "remove"
    },
    {
      "path": "$.email",
      "replace": "***REDACTED***",
      "operation": "replace"
    }
  ]
}
```

### 4. Adding Timestamps to Responses

```json
{
  "pattern": "/api/events",
  "type": "response",
  "body_transforms": [
    {
      "path": "$.timestamp",
      "replace": "{{timestamp.iso}}",
      "operation": "add"
    }
  ]
}
```

### 5. Testing Error Scenarios

Simulate error responses:

```json
{
  "pattern": "/api/payment",
  "type": "response",
  "status_codes": [200],
  "body_transforms": [
    {
      "path": "$.status",
      "replace": "failed",
      "operation": "replace"
    },
    {
      "path": "$.error",
      "replace": "Insufficient funds",
      "operation": "add"
    }
  ]
}
```

## Configuration File

You can also define transformation rules in a configuration file:

```yaml
proxy:
  enabled: true
  port: 8081

  # Request body transformations
  request_replacements:
    - pattern: "/api/users/*"
      enabled: true
      body_transforms:
        - path: "$.userId"
          replace: "{{uuid}}"
          operation: replace
        - path: "$.sessionId"
          replace: "{{uuid}}"
          operation: add

  # Response body transformations
  response_replacements:
    - pattern: "/api/users/*"
      status_codes: [200, 201]
      enabled: true
      body_transforms:
        - path: "$.email"
          replace: "{{faker.email}}"
          operation: replace
        - path: "$.internalId"
          replace: ""
          operation: remove
```

## Best Practices

1. **Use specific patterns** - Narrow patterns reduce unintended transformations
2. **Test transformations** - Verify transformations work as expected before enabling
3. **Enable/disable rules** - Use the `enabled` flag to toggle rules without deleting
4. **Status code filtering** - Use status code filters for response rules to target specific scenarios
5. **Template expansion** - Leverage templates for dynamic, realistic test data
6. **Monitor impact** - Use the Proxy Inspector UI to monitor intercepted traffic
7. **Document rules** - Keep track of why each rule exists and what it does

## Troubleshooting

### Transformations Not Applying

1. **Check rule is enabled** - Verify `enabled: true` in the rule
2. **Verify pattern matches** - Ensure the URL pattern matches the request path
3. **Check JSONPath syntax** - Validate JSONPath expressions are correct
4. **Verify body is JSON** - Transformations only work on JSON bodies
5. **Check status codes** - For response rules, ensure status code filters match

### JSONPath Errors

- **Empty JSONPath** - Ensure path starts with `$.`
- **Invalid array index** - Array indices must be numeric (e.g., `$.array[0]`)
- **Path not found** - The JSONPath must exist in the body (use "add" operation to create)

### Template Expansion Issues

- **Unknown template** - Check template syntax matches supported templates
- **Template not expanding** - Ensure template is wrapped in `{{}}`

## Examples

### Complete Example: User Registration Flow

```json
{
  "pattern": "/api/users/register",
  "type": "request",
  "body_transforms": [
    {
      "path": "$.userId",
      "replace": "{{uuid}}",
      "operation": "add"
    },
    {
      "path": "$.registrationTime",
      "replace": "{{timestamp.iso}}",
      "operation": "add"
    }
  ]
}
```

```json
{
  "pattern": "/api/users/register",
  "type": "response",
  "status_codes": [201],
  "body_transforms": [
    {
      "path": "$.user.email",
      "replace": "{{faker.email}}",
      "operation": "replace"
    },
    {
      "path": "$.user.phone",
      "replace": "{{faker.phone}}",
      "operation": "replace"
    }
  ]
}
```

This completes the Proxy Body Transformation documentation with comprehensive examples and use cases.
