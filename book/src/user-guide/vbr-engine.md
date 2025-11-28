# Virtual Backend Reality (VBR) Engine

The Virtual Backend Reality (VBR) Engine provides a virtual "database" layer that automatically generates CRUD operations from OpenAPI specifications. It enables stateful mocking with relationship management, data persistence, and realistic data generation.

## Overview

The VBR Engine transforms MockForge from a simple request/response mock server into a stateful backend simulator. Instead of returning static responses, VBR maintains a virtual database that supports:

- **Automatic CRUD operations** from OpenAPI specs
- **Relationship mapping** (1:N and N:N)
- **Data persistence** across server restarts
- **State snapshots** for point-in-time recovery
- **Realistic ID generation** with customizable patterns

## Quick Start

### From OpenAPI Specification

The easiest way to get started is to generate a VBR engine from an OpenAPI specification:

```bash
# Start server with VBR from OpenAPI spec
mockforge serve --spec api.yaml --vbr-enabled
```

Or in your configuration:

```yaml
vbr:
  enabled: true
  openapi_spec: "./api.yaml"
  backend: "sqlite"  # or "json", "memory"
  storage_path: "./vbr-data"
```

### Programmatic Usage

```rust
use mockforge_vbr::VbrEngine;

// Create engine from OpenAPI spec
let (engine, result) = VbrEngine::from_openapi_file(config, "./api-spec.yaml").await?;

// Or create manually
let mut engine = VbrEngine::new(config).await?;
```

## Features

### Automatic CRUD Generation

VBR automatically detects CRUD operations from your OpenAPI specification:

- **GET /users** → List all users
- **GET /users/{id}** → Get user by ID
- **POST /users** → Create new user
- **PUT /users/{id}** → Update user
- **DELETE /users/{id}** → Delete user

Primary keys are auto-detected (fields named `id`, `uuid`, etc.), and foreign keys are inferred from field names ending in `_id`.

### Relationship Mapping

#### One-to-Many (1:N)

VBR automatically detects foreign key relationships:

```yaml
# OpenAPI spec
components:
  schemas:
    User:
      properties:
        id: { type: integer }
        name: { type: string }
    
    Post:
      properties:
        id: { type: integer }
        user_id: { type: integer }  # Foreign key detected
        title: { type: string }
```

This creates a relationship where one User can have many Posts. Access related resources:

```bash
# Get all posts for a user
GET /vbr-api/users/1/posts
```

#### Many-to-Many (N:N)

Define many-to-many relationships explicitly:

```rust
use mockforge_vbr::ManyToManyDefinition;

let m2m = ManyToManyDefinition::new("User".to_string(), "Role".to_string());
schema.with_many_to_many(m2m);
```

This creates a junction table automatically (e.g., `user_role`) and enables:

```bash
# Get all roles for a user
GET /vbr-api/users/1/roles

# Get all users with a role
GET /vbr-api/roles/1/users
```

### Data Seeding

Seed your virtual database with initial data:

#### From File

```bash
# Seed from JSON file
mockforge vbr seed --file seed-data.json

# Seed from YAML file
mockforge vbr seed --file seed-data.yaml
```

**Seed file format:**

```json
{
  "users": [
    {"id": 1, "name": "Alice", "email": "alice@example.com"},
    {"id": 2, "name": "Bob", "email": "bob@example.com"}
  ],
  "posts": [
    {"id": 1, "user_id": 1, "title": "First Post"},
    {"id": 2, "user_id": 1, "title": "Second Post"}
  ]
}
```

#### Programmatic Seeding

```rust
// Seed a single entity
engine.seed_entity("users", vec![
    json!({"name": "Alice", "email": "alice@example.com"}),
    json!({"name": "Bob", "email": "bob@example.com"}),
]).await?;

// Seed all entities from file
engine.seed_from_file("./seed-data.json").await?;

// Clear entity data
engine.clear_entity("users").await?;

// Clear all data
engine.reset().await?;
```

### ID Generation

VBR supports multiple ID generation strategies:

#### Pattern-Based IDs

```rust
.with_auto_generation("id", AutoGenerationRule::Pattern("USR-{increment:06}".to_string()))
```

**Template variables:**
- `{increment}` or `{increment:06}` - Auto-incrementing with optional padding
- `{timestamp}` - Unix timestamp
- `{random}` or `{random:8}` - Random alphanumeric (default length 8)
- `{uuid}` - UUID v4

#### Realistic IDs (Stripe-style)

```rust
.with_auto_generation("id", AutoGenerationRule::Realistic {
    prefix: "cus".to_string(),
    length: 14
})
```

Generates IDs like: `cus_abc123def456`

### State Snapshots

Create point-in-time snapshots of your virtual database:

#### Create Snapshot

```bash
# Via CLI
mockforge vbr snapshot create --name initial --description "Initial state"

# Via API
curl -X POST http://localhost:3000/vbr-api/snapshots \
  -H "Content-Type: application/json" \
  -d '{"name": "initial", "description": "Initial state"}'
```

#### Restore Snapshot

```bash
# Via CLI
mockforge vbr snapshot restore --name initial

# Via API
curl -X POST http://localhost:3000/vbr-api/snapshots/initial/restore
```

#### List Snapshots

```bash
# Via CLI
mockforge vbr snapshot list

# Via API
curl http://localhost:3000/vbr-api/snapshots
```

#### Delete Snapshot

```bash
# Via CLI
mockforge vbr snapshot delete --name initial

# Via API
curl -X DELETE http://localhost:3000/vbr-api/snapshots/initial
```

### Time-Based Expiry

Configure records to expire after a certain time:

```yaml
vbr:
  entities:
    - name: sessions
      ttl_seconds: 3600  # Expire after 1 hour
      aging_enabled: true
```

Records older than the TTL are automatically removed.

## Storage Backends

VBR supports multiple storage backends:

### SQLite (Recommended)

Persistent storage with full SQL support:

```yaml
vbr:
  backend: "sqlite"
  storage_path: "./vbr-data.db"
```

**Advantages:**
- Full SQL query support
- ACID transactions
- Efficient for large datasets
- Easy to inspect with SQL tools

### JSON

File-based storage for simple use cases:

```yaml
vbr:
  backend: "json"
  storage_path: "./vbr-data.json"
```

**Advantages:**
- Human-readable
- Easy to version control
- Simple backup/restore

### In-Memory

Fast, non-persistent storage:

```yaml
vbr:
  backend: "memory"
```

**Advantages:**
- Fastest performance
- No disk I/O
- Perfect for testing

**Note:** Data is lost on server restart.

## API Endpoints

VBR automatically creates REST API endpoints for all entities:

### Entity Operations

```http
# List all entities
GET /vbr-api/{entity}

# Get entity by ID
GET /vbr-api/{entity}/{id}

# Create entity
POST /vbr-api/{entity}
Content-Type: application/json

{
  "name": "Alice",
  "email": "alice@example.com"
}

# Update entity
PUT /vbr-api/{entity}/{id}
Content-Type: application/json

{
  "name": "Alice Updated"
}

# Delete entity
DELETE /vbr-api/{entity}/{id}
```

### Relationship Operations

```http
# Get related entities (1:N)
GET /vbr-api/{entity}/{id}/{relationship}

# Get related entities (N:N)
GET /vbr-api/{entity}/{id}/{relationship}
```

### Snapshot Operations

```http
# Create snapshot
POST /vbr-api/snapshots
Content-Type: application/json

{
  "name": "snapshot1",
  "description": "Optional description"
}

# List snapshots
GET /vbr-api/snapshots

# Get snapshot metadata
GET /vbr-api/snapshots/{name}

# Restore snapshot
POST /vbr-api/snapshots/{name}/restore

# Delete snapshot
DELETE /vbr-api/snapshots/{name}
```

### Database Management

```http
# Reset entire database
POST /vbr-api/reset

# Reset specific entity
POST /vbr-api/reset/{entity}
```

## Configuration

### Full Configuration Example

```yaml
vbr:
  enabled: true
  
  # OpenAPI spec for auto-generation
  openapi_spec: "./api.yaml"
  
  # Storage backend
  backend: "sqlite"  # sqlite, json, memory
  storage_path: "./vbr-data"
  
  # Entity configuration
  entities:
    - name: users
      primary_key: "id"
      auto_generation:
        id: "pattern:USR-{increment:06}"
      ttl_seconds: null  # No expiry
      aging_enabled: false
    
    - name: sessions
      primary_key: "id"
      ttl_seconds: 3600  # Expire after 1 hour
      aging_enabled: true
  
  # Relationships
  relationships:
    - type: "one_to_many"
      from: "users"
      to: "posts"
      foreign_key: "user_id"
    
    - type: "many_to_many"
      from: "users"
      to: "roles"
      junction_table: "user_role"
  
  # Snapshot configuration
  snapshots:
    enabled: true
    directory: "./snapshots"
    max_snapshots: 10
```

## Use Cases

### Development Environment

Create a realistic development environment without a real database:

```yaml
vbr:
  enabled: true
  backend: "sqlite"
  openapi_spec: "./api.yaml"
```

### Integration Testing

Use VBR for integration tests with deterministic data:

```rust
// Setup
let engine = VbrEngine::from_openapi_file(config, "./api.yaml").await?;
engine.seed_from_file("./test-data.json").await?;

// Run tests
// ...

// Cleanup
engine.reset().await?;
```

### Demo Environments

Create snapshots for consistent demo environments:

```bash
# Setup demo data
mockforge vbr seed --file demo-data.json

# Create snapshot
mockforge vbr snapshot create --name demo

# Later, restore for consistent demos
mockforge vbr snapshot restore --name demo
```

## Best Practices

1. **Use SQLite for Production**: SQLite provides the best balance of performance and features
2. **Seed Initial Data**: Use seed files for consistent starting states
3. **Create Snapshots**: Save important states for quick restoration
4. **Configure TTL**: Use time-based expiry for session-like data
5. **Version Control Seed Files**: Keep seed data in version control
6. **Use Realistic IDs**: Pattern-based IDs make data look more realistic

## Troubleshooting

### Primary Key Not Detected

If VBR doesn't detect your primary key, specify it explicitly:

```yaml
vbr:
  entities:
    - name: users
      primary_key: "user_id"  # Explicit primary key
```

### Foreign Key Not Detected

If foreign key relationships aren't detected, define them explicitly:

```yaml
vbr:
  relationships:
    - type: "one_to_many"
      from: "users"
      to: "posts"
      foreign_key: "author_id"  # Custom foreign key name
```

### Snapshot Restore Fails

Ensure the snapshot directory exists and has write permissions:

```bash
mkdir -p ./snapshots
chmod 755 ./snapshots
```

## Related Documentation

- [Temporal Simulation](temporal-simulation.md) - Time-based data mutations
- [Scenario State Machines](scenario-state-machines.md) - State machine integration
- [Configuration Guide](../configuration/files.md) - Complete configuration reference

