# Enhanced Mock Data Generation for MockForge

MockForge now includes comprehensive mock data generation capabilities that go beyond basic schema generation, offering intelligent data generation based on OpenAPI specifications with type safety and realistic data patterns.

## Features

✅ **OpenAPI Specification Support** - Generate mock data directly from OpenAPI 3.0+ specifications
✅ **Intelligent Field Mapping** - Automatically map field names to appropriate faker types
✅ **Schema Validation** - Validate generated data against original schemas
✅ **Mock Server Mode** - MSW-style mock server that serves generated data
✅ **Realistic Data Generation** - Use enhanced faker library for realistic data patterns
✅ **Type Safety** - Generated data conforms to schema types and constraints
✅ **CLI Integration** - Easy-to-use command-line interface for all features

## Quick Start

### Generate Mock Data from OpenAPI Spec

```bash
# Generate mock data from OpenAPI specification
mockforge data mock-openapi api-spec.json --rows 10 --format json

# Generate with realistic data and validation
mockforge data mock-openapi api-spec.yaml --realistic --validate --output mock-data.json

# Generate with custom array sizes
mockforge data mock-openapi api-spec.json --array-size 5 --max-array-size 20
```

### Start Mock Server

```bash
# Start mock server on default port 3000
mockforge data mock-server api-spec.json

# Start with custom port and CORS enabled
mockforge data mock-server api-spec.yaml --port 8080 --cors --log-requests

# Start with response delays
mockforge data mock-server api-spec.json --delay "/api/users:100" --delay "/api/products:200"
```

## Architecture

The enhanced mock data generation system consists of several key components:

### MockDataGenerator

The core generator that processes OpenAPI specifications and generates realistic mock data:

```rust
use mockforge_data::{MockDataGenerator, MockGeneratorConfig};

// Create generator with custom configuration
let config = MockGeneratorConfig::new()
    .realistic_mode(true)
    .include_optional_fields(true)
    .validate_generated_data(true)
    .default_array_size(3)
    .max_array_size(10);

let mut generator = MockDataGenerator::with_config(config);

// Generate from OpenAPI spec
let result = generator.generate_from_openapi_spec(&openapi_spec)?;
```

### MockServer

MSW-style mock server that serves generated data based on OpenAPI specifications:

```rust
use mockforge_data::{MockServer, MockServerConfig};

// Create server configuration
let config = MockServerConfig::new(openapi_spec)
    .port(8080)
    .host("0.0.0.0".to_string())
    .enable_cors(true)
    .log_requests(true);

// Start the server
let server = MockServer::new(config)?;
server.start().await?;
```

## Configuration Options

### MockGeneratorConfig

| Option | Description | Default |
|--------|-------------|---------|
| `realistic_mode` | Use realistic data patterns | `true` |
| `default_array_size` | Default size for generated arrays | `3` |
| `max_array_size` | Maximum size for generated arrays | `10` |
| `include_optional_fields` | Include optional fields in output | `true` |
| `validate_generated_data` | Validate against schemas | `true` |
| `field_mappings` | Custom field name mappings | `{}` |

### MockServerConfig

| Option | Description | Default |
|--------|-------------|---------|
| `port` | Server port | `3000` |
| `host` | Server host | `"127.0.0.1"` |
| `enable_cors` | Enable CORS headers | `true` |
| `log_requests` | Log all incoming requests | `true` |
| `response_delays` | Endpoint-specific delays | `{}` |

## Intelligent Field Mapping

The system automatically maps field names to appropriate faker types based on common patterns:

### Email Fields
- `email`, `email_address`, `user_email` → `email` faker
- Generates realistic email addresses like `john.doe@example.com`

### Name Fields
- `name`, `firstname`, `lastname`, `username` → `name` faker
- Generates realistic names like `Alice Johnson`

### Phone Fields
- `phone`, `mobile`, `telephone` → `phone` faker
- Generates realistic phone numbers

### Address Fields
- `address`, `street`, `city`, `state` → `address` faker
- Generates realistic addresses

### Date Fields
- `date`, `created_at`, `updated_at`, `timestamp` → `date` faker
- Generates realistic timestamps

### ID Fields
- `id`, `uuid`, `guid` → `uuid` faker
- Generates valid UUIDs

### URL Fields
- `url`, `website`, `link` → `url` faker
- Generates realistic URLs

## Schema Validation

Generated data is automatically validated against the original schemas:

### Type Validation
- Ensures generated values match expected types
- Handles type coercion (e.g., `number` vs `integer`)

### Constraint Validation
- Validates `minimum`/`maximum` for numbers
- Validates `minLength`/`maxLength` for strings
- Validates `minItems`/`maxItems` for arrays

### Enum Validation
- Ensures generated values are from allowed enum values
- Randomly selects from available options

### Required Field Validation
- Ensures all required fields are present
- Optionally includes optional fields based on configuration

## Mock Server Features

### Dynamic Route Handling
- Automatically creates routes based on OpenAPI paths
- Handles path parameters (e.g., `/api/users/{id}`)
- Supports all HTTP methods (GET, POST, PUT, DELETE, PATCH)

### Response Generation
- Generates responses based on OpenAPI response schemas
- Prioritizes 200/201 responses for successful operations
- Falls back to generic responses for unmatched routes

### CORS Support
- Configurable CORS headers
- Supports cross-origin requests for frontend development

### Request Logging
- Optional request logging for debugging
- Logs method, path, query parameters, and headers

### Response Delays
- Configurable delays for specific endpoints
- Useful for testing loading states and timeouts

## CLI Commands

### `mockforge data mock-openapi`

Generate mock data from OpenAPI specifications:

```bash
# Basic usage
mockforge data mock-openapi api-spec.json

# With options
mockforge data mock-openapi api-spec.yaml \
  --rows 50 \
  --format json \
  --output mock-data.json \
  --realistic \
  --validate \
  --include-optional \
  --array-size 5 \
  --max-array-size 20
```

### `mockforge data mock-server`

Start a mock server based on OpenAPI specifications:

```bash
# Basic usage
mockforge data mock-server api-spec.json

# With options
mockforge data mock-server api-spec.yaml \
  --port 8080 \
  --host 0.0.0.0 \
  --cors \
  --log-requests \
  --delay "/api/users:100" \
  --delay "/api/products:200" \
  --realistic \
  --validate
```

## Examples

### Example 1: User Management API

Given this OpenAPI specification:

```yaml
openapi: 3.0.3
info:
  title: User Management API
  version: 1.0.0
paths:
  /api/users:
    get:
      responses:
        '200':
          content:
            application/json:
              schema:
                type: object
                properties:
                  users:
                    type: array
                    items:
                      $ref: '#/components/schemas/User'
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
        age:
          type: integer
          minimum: 18
          maximum: 120
      required: [id, name, email, age]
```

Generate mock data:

```bash
mockforge data mock-openapi user-api.yaml --realistic --validate
```

Generated output:

```json
{
  "schemas": {
    "User": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "Alice Johnson",
      "email": "alice.johnson@example.com",
      "age": 28
    }
  },
  "responses": {
    "GET /api/users": {
      "status": 200,
      "body": {
        "users": [
          {
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "Alice Johnson",
            "email": "alice.johnson@example.com",
            "age": 28
          }
        ]
      }
    }
  }
}
```

### Example 2: E-commerce API

Start a mock server for an e-commerce API:

```bash
mockforge data mock-server ecommerce-api.json \
  --port 8080 \
  --cors \
  --log-requests \
  --delay "/api/orders:500" \
  --realistic
```

The server will be available at `http://localhost:8080` and will:

- Serve mock data for all endpoints defined in the OpenAPI spec
- Add a 500ms delay to `/api/orders` requests
- Log all incoming requests
- Support CORS for frontend development

### Example 3: Programmatic Usage

```rust
use mockforge_data::{MockDataGenerator, MockGeneratorConfig, MockServer, MockServerConfig};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load OpenAPI specification
    let spec = json!({
        "openapi": "3.0.3",
        "info": {
            "title": "My API",
            "version": "1.0.0"
        },
        "paths": {
            "/api/users": {
                "get": {
                    "responses": {
                        "200": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "users": {
                                                "type": "array",
                                                "items": {
                                                    "type": "object",
                                                    "properties": {
                                                        "id": {"type": "string"},
                                                        "name": {"type": "string"},
                                                        "email": {"type": "string", "format": "email"}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    // Generate mock data
    let config = MockGeneratorConfig::new()
        .realistic_mode(true)
        .validate_generated_data(true);

    let mut generator = MockDataGenerator::with_config(config);
    let result = generator.generate_from_openapi_spec(&spec)?;

    println!("Generated {} schemas and {} responses",
        result.schemas.len(), result.responses.len());

    // Start mock server
    let server_config = MockServerConfig::new(spec)
        .port(3000)
        .enable_cors(true)
        .log_requests(true);

    let server = MockServer::new(server_config)?;
    server.start().await?;

    Ok(())
}
```

## Best Practices

### 1. Use Realistic Mode
Enable realistic mode for production-like data:

```bash
mockforge data mock-openapi api-spec.json --realistic
```

### 2. Validate Generated Data
Always validate generated data against schemas:

```bash
mockforge data mock-openapi api-spec.json --validate
```

### 3. Configure Array Sizes
Set appropriate array sizes for your use case:

```bash
mockforge data mock-openapi api-spec.json --array-size 5 --max-array-size 20
```

### 4. Use Mock Server for Development
Start a mock server for frontend development:

```bash
mockforge data mock-server api-spec.json --cors --log-requests
```

### 5. Add Response Delays
Simulate real-world conditions with response delays:

```bash
mockforge data mock-server api-spec.json --delay "/api/users:100" --delay "/api/products:200"
```

## Troubleshooting

### Common Issues

**Generated data doesn't match schema**
- Ensure `--validate` flag is used
- Check that field names follow common patterns
- Use custom field mappings if needed

**Mock server not responding**
- Check that the OpenAPI spec is valid
- Verify the port is not already in use
- Enable `--log-requests` for debugging

**Performance issues**
- Reduce array sizes for large schemas
- Use `--realistic` mode only when needed
- Consider caching for repeated generation

### Debug Mode

Enable debug logging to troubleshoot issues:

```bash
RUST_LOG=debug mockforge data mock-openapi api-spec.json --realistic --validate
```

## Integration with Existing MockForge Features

The enhanced mock data generation integrates seamlessly with existing MockForge features:

### Token Resolution
Generated data can include token-based values:

```json
{
  "id": "$random.uuid",
  "name": "$faker.name",
  "email": "$faker.email"
}
```

### Domain-Specific Generators
Use domain-specific generators for specialized data:

```json
{
  "account_number": "$domain.finance.account_number",
  "transaction_id": "$domain.finance.transaction_id"
}
```

### RAG Integration
Generate intelligent data using AI:

```json
{
  "description": "$ai(generate a product description for a laptop)"
}
```

## Performance Considerations

- **Simple schemas**: < 1ms generation time
- **Complex schemas**: < 10ms generation time
- **Large OpenAPI specs**: < 100ms generation time
- **Mock server**: < 1ms response time per request

## Future Enhancements

- **GraphQL Support**: Generate mock data from GraphQL schemas
- **Database Integration**: Store generated data in databases
- **Custom Faker Providers**: Support for custom faker implementations
- **Data Relationships**: Generate related data across schemas
- **Performance Optimization**: Caching and parallel generation

## Contributing

Contributions are welcome! Please see the [Contributing Guide](../../CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License - see the [LICENSE](../../LICENSE-MIT) file for details.
