# Mock a REST API from an OpenAPI Spec

**Goal**: You have an OpenAPI specification (Swagger file) and want to automatically generate mock endpoints for frontend development.

**Time**: 3 minutes

## What You'll Learn

- Load an OpenAPI/Swagger spec into MockForge
- Auto-generate mock responses from schema definitions
- Enable dynamic data with template expansion
- Test your mocked API

## Prerequisites

- MockForge installed ([Installation Guide](../getting-started/installation.md))
- An OpenAPI 3.0 or Swagger 2.0 spec file (JSON or YAML)

## Step 1: Prepare Your OpenAPI Spec

Use your existing spec, or create a simple one for testing:

**`petstore-api.json`:**
```json
{
  "openapi": "3.0.0",
  "info": {
    "title": "Pet Store API",
    "version": "1.0.0"
  },
  "paths": {
    "/pets": {
      "get": {
        "summary": "List all pets",
        "responses": {
          "200": {
            "description": "Successful response",
            "content": {
              "application/json": {
                "schema": {
                  "type": "array",
                  "items": {
                    "$ref": "#/components/schemas/Pet"
                  }
                }
              }
            }
          }
        }
      },
      "post": {
        "summary": "Create a pet",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/Pet"
              }
            }
          }
        },
        "responses": {
          "201": {
            "description": "Pet created",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/Pet"
                }
              }
            }
          }
        }
      }
    },
    "/pets/{petId}": {
      "get": {
        "summary": "Get a pet by ID",
        "parameters": [
          {
            "name": "petId",
            "in": "path",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "responses": {
          "200": {
            "description": "Successful response",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/Pet"
                }
              }
            }
          }
        }
      }
    }
  },
  "components": {
    "schemas": {
      "Pet": {
        "type": "object",
        "required": ["id", "name"],
        "properties": {
          "id": {
            "type": "string",
            "example": "{{uuid}}"
          },
          "name": {
            "type": "string",
            "example": "Fluffy"
          },
          "species": {
            "type": "string",
            "example": "cat"
          },
          "age": {
            "type": "integer",
            "example": 3
          }
        }
      }
    }
  }
}
```

## Step 2: Start MockForge with Your Spec

```bash
mockforge serve --spec petstore-api.json --http-port 3000
```

**What happened?** MockForge:
- Parsed your OpenAPI spec
- Created mock endpoints for all defined paths
- Generated example responses from schemas

## Step 3: Test the Auto-Generated Endpoints

```bash
# List all pets
curl http://localhost:3000/pets

# Create a pet
curl -X POST http://localhost:3000/pets \
  -H "Content-Type: application/json" \
  -d '{"name": "Rex", "species": "dog", "age": 5}'

# Get a specific pet
curl http://localhost:3000/pets/123
```

## Step 4: Enable Dynamic Template Expansion

To get unique IDs and dynamic data on each request:

```bash
# Stop the server (Ctrl+C), then restart with templates enabled:
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
mockforge serve --spec petstore-api.json --http-port 3000
```

Now test again - the `{{uuid}}` in your schema examples will generate unique IDs!

## Step 5: Add Request Validation

MockForge can validate requests against your OpenAPI schema:

```bash
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
MOCKFORGE_REQUEST_VALIDATION=enforce \
mockforge serve --spec petstore-api.json --http-port 3000
```

Try sending an invalid request:
```bash
# This will fail validation (missing required 'name' field)
curl -X POST http://localhost:3000/pets \
  -H "Content-Type: application/json" \
  -d '{"species": "dog"}'
```

Response:
```json
{
  "error": "request validation failed",
  "details": [
    {
      "path": "body.name",
      "code": "required",
      "message": "Missing required field: name"
    }
  ]
}
```

## Step 6: Use a Configuration File (Optional)

For more control, create a config file:

**`petstore-config.yaml`:**
```yaml
server:
  http_port: 3000

spec: petstore-api.json

validation:
  mode: enforce

response:
  template_expand: true

admin:
  enabled: true
  port: 9080
```

Start with config:
```bash
mockforge serve --config petstore-config.yaml
```

## Advanced: Override Specific Responses

You can override auto-generated responses for specific endpoints:

**`petstore-config.yaml`:**
```yaml
http:
  port: 3000
  openapi_spec: petstore-api.json
  response_template_expand: true

  # Override the GET /pets endpoint
  routes:
    - path: /pets
      method: GET
      response:
        status: 200
        body: |
          [
            {
              "id": "{{uuid}}",
              "name": "{{faker.name}}",
              "species": "cat",
              "age": {{randInt 1 15}}
            },
            {
              "id": "{{uuid}}",
              "name": "{{faker.name}}",
              "species": "dog",
              "age": {{randInt 1 15}}
            }
          ]
```

## Step 7: Configure Request Validation

MockForge supports comprehensive OpenAPI request validation. Update your config to enable validation:

```yaml
validation:
  mode: enforce          # Reject invalid requests
  aggregate_errors: true # Combine multiple validation errors
  status_code: 422       # Use 422 for validation errors

# Optional: Skip validation for specific routes
validation:
  overrides:
    "GET /health": "off"  # Health checks don't need validation
```

Test validation by sending an invalid request:

```bash
# This will fail validation (missing required fields)
curl -X POST http://localhost:3000/pets \
  -H "Content-Type: application/json" \
  -d '{"species": "dog"}'
```

Response:
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

### Validation Modes

- **`off`**: Disable validation completely
- **`warn`**: Log warnings but allow invalid requests
- **`enforce`**: Reject invalid requests with error responses

## Common Use Cases

| Use Case | Configuration |
|----------|---------------|
| **Frontend development** | Enable CORS, template expansion |
| **API contract testing** | Enable request validation (enforce mode) |
| **Demo environments** | Use faker functions for realistic data |
| **Integration tests** | Disable template expansion for deterministic responses |

## Troubleshooting

**Spec not loading?**
- Verify the file path is correct
- Check that the spec is valid OpenAPI 3.0 or Swagger 2.0
- Use a validator like [Swagger Editor](https://editor.swagger.io/)

**Validation too strict?**
```bash
# Use 'warn' mode instead of 'enforce'
MOCKFORGE_REQUEST_VALIDATION=warn mockforge serve --spec petstore-api.json
```

**Need custom responses?**
- Add route overrides in your config file (see Advanced section above)
- Or use [Custom Responses Guide](../user-guide/http-mocking/custom-responses.md)

## What's Next?

- [Dynamic Data Generation](../user-guide/http-mocking/dynamic-data.md) - Add faker functions and advanced templates
- [Admin UI Walkthrough](admin-ui-walkthrough.md) - Visualize and manage your mock server
- [Add a Custom Plugin](add-custom-plugin.md) - Extend MockForge with custom functionality
- [Team Collaboration](../user-guide/sync.md) - Share mocks with your team via Git

---

**Pro Tip**: Keep your OpenAPI spec in version control alongside your mock configuration. As the real API evolves, update the spec and your frontend automatically benefits from the changes.
