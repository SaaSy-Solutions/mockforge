# Schema-Driven Mock Generation

MockForge now supports importing API specifications (OpenAPI/AsyncAPI) and automatically generating comprehensive mock endpoints with coverage reporting.

## Features

### ✅ Supported Specification Formats

- **OpenAPI 3.x** (JSON and YAML)
- **AsyncAPI 2.x/3.x** (JSON and YAML)
- Auto-detection of specification type
- URL and local file support

### ✅ Mock Generation Capabilities

- Automatic endpoint/channel generation from specifications
- Schema-based mock data generation
- Example data extraction from specs
- Path/query parameter handling
- Request body and response generation
- Realistic data with proper typing (dates, emails, UUIDs, etc.)

### ✅ Coverage Reporting

- Total endpoints mocked (100% coverage for all imports)
- Breakdown by HTTP method or protocol
- Endpoints with request bodies
- Endpoints with mock responses
- Detailed route/channel listings

## CLI Usage

### Import OpenAPI Specification

```bash
# Import from local file (YAML)
mockforge import openapi ./specs/petstore.yaml --verbose

# Import from local file (JSON)
mockforge import openapi ./specs/api.json

# Import from URL
mockforge import openapi https://api.example.com/openapi.json

# Save generated mocks to file
mockforge import openapi ./specs/api.yaml --output mocks.json

# Specify base URL override
mockforge import openapi ./specs/api.yaml --base-url https://api.production.com
```

### Import AsyncAPI Specification

```bash
# Import AsyncAPI spec
mockforge import asyncapi ./specs/events.yaml --verbose

# Filter by protocol
mockforge import asyncapi ./specs/mqtt-api.yaml --protocol mqtt

# Save generated channels
mockforge import asyncapi ./specs/kafka.yaml --output channels.json
```

### Generate Coverage Report

```bash
# Auto-detect spec type and show coverage
mockforge import coverage ./specs/api.yaml

# Specify spec type explicitly
mockforge import coverage ./specs/api.json --spec-type openapi
```

### Example Output

#### OpenAPI Import
```
📋 Importing OpenAPI Specification...

📂 Loading specification from file: ./specs/petstore.yaml
📖 Specification Info:
  Title: Pet Store API
  Version: 1.0.0
  Description: A sample Pet Store API
  OpenAPI Version: 3.0.3

🌐 Servers:
  • https://petstore.example.com/api/v1
  • https://dev.petstore.example.com/api/v1

✨ Generated Routes:
  Total Routes: 6

  By Method:
    GET: 3
    POST: 1
    PUT: 1
    DELETE: 1

📋 Route Details:
  1: GET /pets → 200
  2: POST /pets → 201 (with request body)
  3: GET /pets/search → 200
  4: GET /pets/{petId} → 200
  5: PUT /pets/{petId} → 200 (with request body)
  6: DELETE /pets/{petId} → 204

✅ Saved 6 routes to mocks.json
```

#### Coverage Report
```
📊 Generating Coverage Report...

📂 Loading specification from file: ./specs/petstore.yaml
📊 Coverage Statistics:

  Total Endpoints: 6
  Endpoints with Mock Responses: 6 (100%)
  Endpoints with Request Bodies: 2 (33%)

  Coverage by HTTP Method:
    GET: 3 (50%)
    POST: 1 (17%)
    PUT: 1 (17%)
    DELETE: 1 (17%)

✅ Overall Coverage: 100%
```

## HTTP API Usage

MockForge provides RESTful API endpoints for programmatic spec import and management.

### Import Specification

**POST /specs**

```bash
curl -X POST http://localhost:3000/specs \
  -H "Content-Type: application/json" \
  -d '{
    "spec_content": "{\"openapi\":\"3.0.0\",\"info\":{\"title\":\"Test API\",\"version\":\"1.0.0\"},\"paths\":{\"/users\":{\"get\":{\"responses\":{\"200\":{\"description\":\"Success\"}}}}}}",
    "spec_type": "openapi",
    "name": "Test API",
    "base_url": null,
    "auto_generate_mocks": true
  }'
```

**Response:**
```json
{
  "spec_id": "spec-1729614000123",
  "spec_type": "openapi",
  "routes_generated": 1,
  "warnings": [],
  "coverage": {
    "total_endpoints": 1,
    "mocked_endpoints": 1,
    "coverage_percentage": 100,
    "by_method": {
      "GET": 1
    }
  }
}
```

### Upload Specification File

**POST /specs/upload**

```bash
curl -X POST http://localhost:3000/specs/upload \
  -F "file=@./specs/petstore.yaml" \
  -F "name=Pet Store API" \
  -F "base_url=https://api.example.com"
```

### List Imported Specifications

**GET /specs**

```bash
# List all specs
curl http://localhost:3000/specs

# Filter by type
curl http://localhost:3000/specs?spec_type=openapi

# Paginate
curl http://localhost:3000/specs?limit=10&offset=0
```

**Response:**
```json
[
  {
    "id": "spec-1729614000123",
    "name": "Pet Store API",
    "spec_type": "openapi",
    "version": "1.0.0",
    "description": "A sample Pet Store API",
    "servers": [
      "https://petstore.example.com/api/v1"
    ],
    "uploaded_at": "2025-10-22T18:00:00Z",
    "route_count": 6
  }
]
```

### Get Specification Details

**GET /specs/{id}**

```bash
curl http://localhost:3000/specs/spec-1729614000123
```

### Get Generated Routes

**GET /specs/{id}/routes**

```bash
curl http://localhost:3000/specs/spec-1729614000123/routes
```

**Response:**
```json
[
  {
    "method": "GET",
    "path": "/pets",
    "headers": {},
    "body": null,
    "response": {
      "status": 200,
      "headers": {
        "Content-Type": "application/json"
      },
      "body": {
        "data": [
          {
            "id": 1,
            "name": "Fluffy",
            "species": "cat"
          }
        ]
      }
    }
  }
]
```

### Get Coverage Statistics

**GET /specs/{id}/coverage**

```bash
curl http://localhost:3000/specs/spec-1729614000123/coverage
```

### Delete Specification

**DELETE /specs/{id}**

```bash
curl -X DELETE http://localhost:3000/specs/spec-1729614000123
```

## Integration with MockForge Server

To enable the Spec Import API in your MockForge server:

```rust
use mockforge_http::management::management_router_with_spec_import;
use mockforge_http::management::ManagementState;

let state = ManagementState::new(None, None, 3000);
let router = management_router_with_spec_import(state);

// Start server
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
axum::serve(listener, router).await?;
```

The Spec Import API will be available at the `/specs` endpoints.

## Example Specifications

### OpenAPI Example

See [examples/specs/petstore-openapi.yaml](../examples/specs/petstore-openapi.yaml) for a complete Pet Store API example with:
- Full CRUD operations
- Path and query parameters
- Request/response schemas
- Example data
- Error responses

### AsyncAPI Example

See [examples/specs/iot-asyncapi.yaml](../examples/specs/iot-asyncapi.yaml) for an IoT sensor network with:
- MQTT protocol channels
- Temperature, humidity, and motion sensors
- Control commands and alerts
- Message schemas with examples

## Mock Data Generation

MockForge generates realistic mock data from JSON schemas:

### Supported Types

- **Strings**: Format-aware (email, uuid, uri, date, date-time, hostname, ipv4, ipv6)
- **Numbers/Integers**: With min/max constraints
- **Booleans**: Random true/false
- **Arrays**: With minItems/maxItems
- **Objects**: Nested structures
- **Enums**: Random selection from allowed values

### Example Schema → Mock Data

**Schema:**
```json
{
  "type": "object",
  "properties": {
    "id": { "type": "integer" },
    "email": { "type": "string", "format": "email" },
    "createdAt": { "type": "string", "format": "date-time" },
    "status": { "type": "string", "enum": ["active", "inactive"] }
  }
}
```

**Generated Mock:**
```json
{
  "id": 42,
  "email": "user@example.com",
  "createdAt": "2025-10-22T12:00:00Z",
  "status": "active"
}
```

## Best Practices

### 1. Include Examples in Your Specs

OpenAPI and AsyncAPI support `example` fields. When provided, MockForge uses these for more realistic mocks:

```yaml
paths:
  /users:
    get:
      responses:
        '200':
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/User'
              examples:
                users:
                  value:
                    - id: 1
                      name: "Alice"
                    - id: 2
                      name: "Bob"
```

### 2. Use Descriptive Schemas

Provide detailed schemas with:
- Format specifications
- Min/max constraints
- Pattern restrictions
- Required fields
- Descriptions

### 3. Organize Specs by Domain

Keep separate specs for different API domains:
```
specs/
├── user-api.yaml       # User management
├── order-api.yaml      # Order processing
├── notification.yaml   # Async notifications (AsyncAPI)
└── analytics.yaml      # Analytics events (AsyncAPI)
```

### 4. Version Your Specifications

Use semantic versioning and maintain specs in version control alongside your code.

### 5. Leverage Coverage Reports

Run coverage reports regularly to ensure all endpoints are documented and mockable:

```bash
mockforge import coverage ./specs/*.yaml
```

## Advanced Usage

### Custom Base URLs

Override server URLs defined in specs:

```bash
mockforge import openapi ./specs/api.yaml \
  --base-url https://staging.api.example.com
```

### Batch Processing

Process multiple specs:

```bash
for spec in specs/*.yaml; do
  mockforge import openapi "$spec" \
    --output "mocks/$(basename $spec .yaml).json"
done
```

### CI/CD Integration

Add spec validation to your pipeline:

```yaml
# .github/workflows/validate-specs.yml
name: Validate API Specs

on: [push]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install MockForge
        run: cargo install mockforge-cli
      - name: Import and validate specs
        run: |
          mockforge import coverage ./specs/api.yaml
          mockforge import openapi ./specs/api.yaml --output /tmp/mocks.json
```

## Troubleshooting

### Spec fails to import

- Verify JSON/YAML syntax is valid
- Ensure spec conforms to OpenAPI 3.x or AsyncAPI 2.x/3.x
- Check for required fields (title, version, paths/channels)

### Generated mocks don't match expectations

- Add explicit `example` fields in your spec
- Verify schema types and formats
- Check that $ref references are valid

### Coverage is less than 100%

MockForge generates mocks for all endpoints in valid specs. Less than 100% coverage typically means:
- Spec has validation errors
- Some operations are missing responses
- Reference resolution failed

Run with `--verbose` to see detailed warnings.

## Limitations

- **Reference Resolution**: Complex `$ref` chains may not fully resolve (coming soon)
- **Swagger 2.0**: Currently only OpenAPI 3.x is supported (Swagger support planned)
- **AI Enhancement**: Integration with AI mock data generation (planned)
- **Custom Templates**: User-defined mock data templates (planned)

## Changelog

### Version 0.1.3 (Current)
- ✅ OpenAPI 3.x import with full CRUD support
- ✅ AsyncAPI 2.x/3.x import for event-driven APIs
- ✅ CLI commands with coverage reporting
- ✅ HTTP API for programmatic spec management
- ✅ Schema-based mock data generation
- ✅ YAML and JSON format support
- ✅ URL and local file import
- ✅ Comprehensive integration tests

## Support

- **Documentation**: https://docs.mockforge.dev
- **Issues**: https://github.com/SaaSy-Solutions/mockforge/issues
- **Examples**: [examples/specs/](../examples/specs/)

---

**Next Steps**: Try importing your API specifications and see 100% mock coverage in action!

```bash
mockforge import openapi ./your-api-spec.yaml --verbose
```
