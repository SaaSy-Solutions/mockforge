# MockForge Test Fixtures Library

This directory contains reusable test fixtures for common API testing scenarios.

## Directory Structure

```
fixtures/
├── http/               # HTTP endpoint fixtures
│   ├── get/           # GET request fixtures
│   ├── post/          # POST request fixtures
│   └── common/        # Shared HTTP responses
├── grpc/              # gRPC service fixtures
├── mqtt/              # MQTT message fixtures
├── amqp/              # AMQP/RabbitMQ fixtures
├── ws/                # WebSocket fixtures
├── errors/            # Common error responses
├── auth/              # Authentication fixtures
├── pagination/        # Paginated response patterns
└── scenarios/         # Multi-step test scenarios
```

## Usage

### Load Fixtures in Configuration

```yaml
# mockforge.yaml
fixtures:
  directory: "./fixtures"

endpoints:
  - path: "/api/users"
    method: GET
    fixture: "http/get/_api_users/sample.json"
```

### Load via CLI

```bash
# Load specific fixture
mockforge fixture load ./fixtures/http/get/_api_users/sample.json

# Load fixture set
mockforge fixture load-set ./fixtures/scenarios/user-journey.yaml
```

### Load in Tests

```typescript
import { loadFixture } from '@mockforge/sdk';

const users = await loadFixture('http/get/_api_users/sample.json');
```

## Common Patterns

### REST API Responses

See `http/common/` for reusable response patterns.

### Error Responses

See `errors/` for standard error response formats.

### Authentication

See `auth/` for JWT, OAuth, and API key fixtures.

## Creating Custom Fixtures

1. Create a JSON or YAML file in the appropriate directory
2. Follow the naming convention: `{endpoint_path}/{scenario}.json`
3. Include response metadata and body

```json
{
  "response": {
    "status": 200,
    "headers": {
      "Content-Type": "application/json"
    },
    "body": {
      "data": []
    }
  }
}
```

## See Also

- [Configuration Specification](../docs/CONFIGURATION_SPEC.md)
- [Testing Guide](../docs/TESTING_GUIDE.md)
