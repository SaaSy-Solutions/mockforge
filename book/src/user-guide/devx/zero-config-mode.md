# Zero-Config Mode (Runtime Daemon)

**Pillars:** [DevX]

Zero-Config Mode provides the "invisible mock server" experience. When you hit an endpoint that doesn't exist, MockForge automatically creates a mock, generates types, creates client stubs, and sets up scenarios—all without manual configuration.

## Overview

The Runtime Daemon is a background process that:
- **Detects** when you hit an endpoint that doesn't exist (404)
- **Automatically creates** a mock endpoint
- **Generates types** (TypeScript, JSON Schema)
- **Generates client stubs** (React, Vue, etc.)
- **Updates OpenAPI schema**
- **Creates example responses**
- **Sets up scenarios**

This is "mock server in your shadow"—an AI-assisted backend-on-demand.

## Quick Start

### Enable Zero-Config Mode

```bash
# Start MockForge with runtime daemon
mockforge serve --runtime-daemon

# Or via environment variable
MOCKFORGE_RUNTIME_DAEMON_ENABLED=true mockforge serve
```

### Configuration

```yaml
# mockforge.yaml
runtime_daemon:
  enabled: true
  auto_create_on_404: true
  ai_generation: true  # Use AI to generate intelligent responses
  generate_types: true  # Generate TypeScript/JSON Schema
  generate_client_stubs: true  # Generate client code
  update_openapi: true  # Update OpenAPI schema
  create_scenario: true  # Create scenarios automatically
  exclude_patterns:
    - "/health"
    - "/metrics"
    - "/__mockforge/*"
```

## How It Works

### 1. Detection

When a request hits a non-existent endpoint (404), the daemon detects it:

```
GET /api/users/123 → 404 Not Found
→ Runtime Daemon detects 404
→ Analyzes request (method, path, headers, body)
```

### 2. Auto-Generation

The daemon automatically:

1. **Creates Mock Endpoint**
   - Analyzes request to infer response structure
   - Uses AI to generate intelligent response
   - Sets appropriate status code

2. **Generates Types** (if enabled)
   - TypeScript types
   - JSON Schema
   - Saved to `generated/types/`

3. **Generates Client Stubs** (if enabled)
   - React hooks
   - Vue composables
   - Angular services
   - Saved to `generated/clients/`

4. **Updates OpenAPI Schema** (if enabled)
   - Adds endpoint to OpenAPI spec
   - Infers request/response schemas
   - Updates `openapi.json`

5. **Creates Scenario** (if enabled)
   - Basic scenario for the endpoint
   - Includes example request/response
   - Saved to `scenarios/`

### 3. Response

The next request to the same endpoint gets the auto-generated mock:

```
GET /api/users/123 → 200 OK
{
  "id": "123",
  "name": "John Doe",
  "email": "john@example.com"
}
```

## Configuration Options

### Auto-Create on 404

```yaml
runtime_daemon:
  auto_create_on_404: true  # Create mocks automatically
```

### AI Generation

```yaml
runtime_daemon:
  ai_generation: true  # Use AI for intelligent responses
```

When enabled, AI generates realistic responses based on:
- Endpoint path patterns
- Request body structure
- Domain context
- Existing mocks

### Type Generation

```yaml
runtime_daemon:
  generate_types: true
  types_output_dir: "./generated/types"
```

Generates:
- TypeScript types
- JSON Schema
- Go types
- Rust types

### Client Stub Generation

```yaml
runtime_daemon:
  generate_client_stubs: true
  clients_output_dir: "./generated/clients"
  client_frameworks:
    - react
    - vue
    - angular
```

### OpenAPI Updates

```yaml
runtime_daemon:
  update_openapi: true
  openapi_path: "./openapi.json"
```

Automatically updates OpenAPI spec with new endpoints.

### Scenario Creation

```yaml
runtime_daemon:
  create_scenario: true
  scenarios_output_dir: "./scenarios"
```

Creates basic scenarios for auto-generated endpoints.

## Example Workflow

### 1. Start Development

```bash
# Start MockForge with runtime daemon
mockforge serve --runtime-daemon
```

### 2. Make Request

```bash
# Frontend makes request to non-existent endpoint
curl http://localhost:3000/api/products/123
# → 404 Not Found
```

### 3. Auto-Generation

The daemon automatically:
- Creates mock for `/api/products/{id}`
- Generates TypeScript types
- Creates React hook
- Updates OpenAPI spec
- Creates scenario

### 4. Use Generated Code

```typescript
// Generated React hook
import { useProduct } from './generated/clients/react';

function ProductPage({ id }) {
  const { data, loading } = useProduct(id);
  // ...
}
```

## Excluding Patterns

Exclude certain paths from auto-generation:

```yaml
runtime_daemon:
  exclude_patterns:
    - "/health"
    - "/metrics"
    - "/__mockforge/*"
    - "/api/internal/*"
```

## Workspace Integration

The daemon saves generated artifacts to your workspace:

```
workspace/
├── mocks/
│   └── auto-generated/
│       └── api-products-{id}.yaml
├── generated/
│   ├── types/
│   │   └── Product.ts
│   └── clients/
│       └── react/
│           └── useProduct.ts
├── scenarios/
│   └── auto-products-{id}.yaml
└── openapi.json  # Updated automatically
```

## AI-Powered Generation

When AI generation is enabled, the daemon uses AI to create intelligent responses:

### Request Analysis

```bash
POST /api/orders
{
  "product_id": "123",
  "quantity": 2
}
```

### AI-Generated Response

```json
{
  "id": "order-456",
  "product_id": "123",
  "quantity": 2,
  "status": "pending",
  "total": 99.98,
  "created_at": "2025-01-27T10:00:00Z"
}
```

The AI infers:
- Order structure from request
- Realistic IDs and timestamps
- Calculated fields (total)
- Appropriate status values

## Best Practices

1. **Start with AI Generation**: Let AI create intelligent responses
2. **Review Generated Mocks**: Check auto-generated mocks for accuracy
3. **Refine Over Time**: Update mocks as you learn more about the API
4. **Version Control**: Commit generated artifacts to Git
5. **Exclude Internal Endpoints**: Don't auto-generate for internal APIs

## Troubleshooting

### Mocks Not Created

- Check `auto_create_on_404` is enabled
- Verify endpoint isn't in `exclude_patterns`
- Check daemon is running: `mockforge serve --runtime-daemon`

### Generated Code Issues

- Review generated types for accuracy
- Update OpenAPI spec manually if needed
- Regenerate: `mockforge generate --from-openapi openapi.json`

### AI Generation Quality

- Provide more context in existing mocks
- Use domain-specific reality profiles
- Adjust AI model/temperature settings

## Related Documentation

- [ForgeConnect SDK](forgeconnect-sdk.md) - Browser integration
- [DevTools Integration](devtools-integration.md) - Browser DevTools
- [Scenario Marketplace](scenario-marketplace.md) - Sharing scenarios

