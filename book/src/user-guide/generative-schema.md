# Generative Schema Mode

Generative Schema Mode enables you to generate complete API ecosystems from JSON payloads. Simply provide example JSON data, and MockForge automatically creates routes, schemas, and entity relationships for a fully functional mock API.

## Overview

Generative Schema Mode transforms example JSON payloads into:

- **Complete OpenAPI specifications** with all endpoints
- **Automatic CRUD routes** for each entity
- **Entity relationship inference** from data structure
- **One-click environment creation** ready for deployment
- **Preview and edit** generated schemas before deployment

## Quick Start

### Generate from JSON File

```bash
# Generate API ecosystem from JSON payloads
mockforge generate --from-json examples.json --output ./generated-api

# Or from multiple files
mockforge generate --from-json file1.json file2.json --output ./generated-api
```

### Generate from JSON Payloads

```bash
# Generate from inline JSON
mockforge generate --from-json '{"users": [{"id": 1, "name": "Alice"}]}' --output ./api
```

### One-Click Environment Creation

```bash
# Generate and start server in one command
mockforge generate --from-json data.json --serve --port 3000
```

## How It Works

### 1. Entity Inference

MockForge analyzes JSON payloads to infer entity structures:

**Input JSON:**
```json
{
  "users": [
    {"id": 1, "name": "Alice", "email": "alice@example.com"},
    {"id": 2, "name": "Bob", "email": "bob@example.com"}
  ],
  "posts": [
    {"id": 1, "user_id": 1, "title": "First Post", "content": "..."},
    {"id": 2, "user_id": 1, "title": "Second Post", "content": "..."}
  ]
}
```

**Inferred Entities:**
- `User` entity with fields: `id`, `name`, `email`
- `Post` entity with fields: `id`, `user_id`, `title`, `content`
- Relationship: `User` has many `Post` (via `user_id`)

### 2. Route Generation

Automatically generates CRUD routes for each entity:

**Generated Routes:**
- `GET /users` - List all users
- `GET /users/{id}` - Get user by ID
- `POST /users` - Create user
- `PUT /users/{id}` - Update user
- `DELETE /users/{id}` - Delete user

Same routes generated for `posts`.

### 3. Schema Building

Creates complete OpenAPI 3.0 specification:

```yaml
openapi: 3.0.0
info:
  title: Generated API
  version: 1.0.0
paths:
  /users:
    get:
      summary: List users
      responses:
        '200':
          description: List of users
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/User'
components:
  schemas:
    User:
      type: object
      properties:
        id:
          type: integer
        name:
          type: string
        email:
          type: string
          format: email
```

## Configuration

### Generation Options

```yaml
generative_schema:
  enabled: true
  
  # API metadata
  title: "My Generated API"
  version: "1.0.0"
  
  # Naming rules
  naming_rules:
    entity_case: "PascalCase"  # PascalCase, camelCase, snake_case
    route_case: "kebab-case"   # kebab-case, snake_case, camelCase
    pluralization: "standard"  # standard, none, custom
    
  # Generation options
  generate_crud: true
  infer_relationships: true
  merge_schemas: true
```

### Naming Rules

Customize how entities and routes are named:

```yaml
naming_rules:
  # Entity naming
  entity_case: "PascalCase"  # User, OrderItem
  entity_suffix: ""           # Optional suffix
  
  # Route naming
  route_case: "kebab-case"    # /api/users, /api/order-items
  route_prefix: "/api"        # Route prefix
  
  # Pluralization
  pluralization: "standard"   # users, orders
  custom_plurals:
    person: "people"
    child: "children"
```

## CLI Commands

### Generate from JSON

```bash
# Basic generation
mockforge generate --from-json data.json

# With output directory
mockforge generate --from-json data.json --output ./generated

# With options
mockforge generate \
  --from-json data.json \
  --title "My API" \
  --version "1.0.0" \
  --output ./generated
```

### Preview Before Generation

```bash
# Preview generated schema without creating files
mockforge generate --from-json data.json --preview
```

### Generate and Serve

```bash
# Generate and start server
mockforge generate --from-json data.json --serve --port 3000
```

## Programmatic Usage

### Generate Ecosystem

```rust
use mockforge_core::generative_schema::{
    EcosystemGenerator, GenerationOptions, NamingRules
};
use serde_json::json;

// Example payloads
let payloads = vec![
    json!({
        "users": [
            {"id": 1, "name": "Alice", "email": "alice@example.com"}
        ]
    })
];

// Generation options
let options = GenerationOptions {
    title: Some("My API".to_string()),
    version: Some("1.0.0".to_string()),
    naming_rules: NamingRules::default(),
    generate_crud: true,
    output_dir: Some("./generated".into()),
};

// Generate ecosystem
let result = EcosystemGenerator::generate_from_json(payloads, options).await?;

// Access generated spec
let spec = result.spec;
let entities = result.entities;
let routes = result.routes;
```

## Entity Relationship Inference

MockForge automatically detects relationships from JSON structure:

### One-to-Many (1:N)

Detected from foreign key patterns:

```json
{
  "users": [{"id": 1, "name": "Alice"}],
  "posts": [{"id": 1, "user_id": 1, "title": "Post"}]
}
```

**Detected Relationship:**
- `User` has many `Post` (via `user_id`)

### Many-to-Many (N:N)

Detected from junction patterns:

```json
{
  "users": [{"id": 1, "name": "Alice"}],
  "roles": [{"id": 1, "name": "admin"}],
  "user_roles": [
    {"user_id": 1, "role_id": 1}
  ]
}
```

**Detected Relationship:**
- `User` has many `Role` through `user_roles`

## Schema Merging

When generating from multiple JSON files, schemas are intelligently merged:

```bash
# Generate from multiple files
mockforge generate \
  --from-json users.json posts.json comments.json \
  --output ./generated
```

**Merging Strategy:**
- Common fields are preserved
- New fields are added
- Type conflicts are resolved (prefer more specific types)
- Relationships are merged

## Preview and Edit

Before deploying, preview and edit the generated schema:

### Preview Generated Schema

```bash
# Preview in terminal
mockforge generate --from-json data.json --preview

# Preview in browser (opens generated OpenAPI spec)
mockforge generate --from-json data.json --preview --open-browser
```

### Edit Before Deployment

```bash
# Generate and open in editor
mockforge generate --from-json data.json --output ./generated --edit

# Manually edit generated/openapi.yaml, then deploy
mockforge serve --spec ./generated/openapi.yaml
```

## Integration with VBR

Generated schemas can be automatically integrated with VBR:

```bash
# Generate with VBR integration
mockforge generate \
  --from-json data.json \
  --vbr-enabled \
  --output ./generated
```

This creates:
- VBR entity definitions
- Relationship mappings
- Seed data from JSON

## Use Cases

### Rapid Prototyping

Quickly create mock APIs from example data:

```bash
# Generate API from sample responses
mockforge generate --from-json sample-responses.json --serve
```

### API Design

Design APIs by example:

```bash
# Create API from design mockups
mockforge generate --from-json design-mockups.json --output ./api-design
```

### Testing Data Generation

Generate test APIs with realistic data:

```bash
# Generate API with test data
mockforge generate --from-json test-data.json --output ./test-api
```

## Best Practices

1. **Provide Complete Examples**: Include all fields you want in the generated schema
2. **Use Consistent Naming**: Consistent naming in JSON helps with entity inference
3. **Include Relationships**: Show relationships in JSON for automatic detection
4. **Preview Before Deploy**: Always preview generated schemas before deployment
5. **Version Control**: Commit generated schemas to version control

## Troubleshooting

### Entities Not Detected

- Ensure JSON has a clear structure (arrays of objects)
- Use consistent field names
- Include ID fields for relationship detection

### Routes Not Generated

- Check that `generate_crud` is enabled
- Verify entity names are valid
- Review naming rules configuration

### Relationships Not Inferred

- Use standard foreign key naming (`entity_id`)
- Include junction tables for many-to-many
- Provide complete relationship data in JSON

## Related Documentation

- [VBR Engine](vbr-engine.md) - State management for generated entities
- [OpenAPI Integration](http-mocking/openapi.md) - Working with generated OpenAPI specs
- [Configuration Guide](../configuration/files.md) - Complete configuration reference

