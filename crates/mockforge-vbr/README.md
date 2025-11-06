# MockForge Virtual Backend Reality (VBR) Engine

The VBR engine creates stateful mock servers with persistent virtual databases, auto-generated CRUD APIs, relationship constraints, session management, and time-based data evolution.

## Overview

VBR acts like a mini real backend with:
- **Persistent virtual database** (SQLite, JSON, in-memory options)
- **CRUD APIs auto-generated** from entity schemas
- **Relationship modeling** and constraint enforcement
- **User session & auth emulation**
- **Time-based data evolution** (data aging, expiring sessions)

## Quick Start

### Installation

The VBR engine is included in MockForge. No additional installation needed.

### Basic Usage

```bash
# Create an entity
mockforge vbr create entity User --fields id:string,name:string,email:string

# Start a VBR server
mockforge vbr serve --port 3000 --storage sqlite --db-path ./data/vbr.db
```

### Programmatic Usage

```rust
use mockforge_vbr::{VbrEngine, VbrConfig, StorageBackend};
use mockforge_vbr::entities::{Entity, EntityRegistry};
use mockforge_vbr::schema::VbrSchemaDefinition;
use mockforge_data::schema::{FieldDefinition, SchemaDefinition};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create VBR engine with SQLite storage
    let config = VbrConfig::default()
        .with_storage_backend(StorageBackend::Sqlite {
            path: "./data/vbr.db".into(),
        });

    let engine = VbrEngine::new(config).await?;

    // Define a User entity
    let user_schema = VbrSchemaDefinition {
        base: SchemaDefinition {
            name: "User".to_string(),
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    description: None,
                    default: None,
                    constraints: HashMap::new(),
                    faker_template: None,
                },
                FieldDefinition {
                    name: "name".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    description: None,
                    default: None,
                    constraints: HashMap::new(),
                    faker_template: None,
                },
                FieldDefinition {
                    name: "email".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    description: None,
                    default: None,
                    constraints: HashMap::new(),
                    faker_template: None,
                },
            ],
            description: None,
            metadata: HashMap::new(),
            relationships: HashMap::new(),
        },
        primary_key: vec!["id".to_string()],
        foreign_keys: Vec::new(),
        unique_constraints: Vec::new(),
        indexes: Vec::new(),
        auto_generation: HashMap::new(),
    };

    let user_entity = Entity::new("User".to_string(), user_schema);
    engine.registry().register(user_entity)?;

    // Create HTTP router with VBR endpoints
    let context = mockforge_vbr::handlers::HandlerContext {
        database: engine.database_arc(),
        registry: engine.registry().clone(),
        session_manager: None,
    };

    let router = mockforge_vbr::integration::create_vbr_router_with_context(
        "/vbr-api",
        context,
    )?;

    // Start server...

    Ok(())
}
```

## Features

### 1. Virtual Database

VBR supports multiple storage backends:

- **SQLite** (default): Persistent, production-like database
- **JSON**: Human-readable file storage
- **Memory**: Fast, no persistence (for testing)

```rust
use mockforge_vbr::{VbrConfig, StorageBackend};

// SQLite backend
let config = VbrConfig::default()
    .with_storage_backend(StorageBackend::Sqlite {
        path: "./data/vbr.db".into(),
    });

// JSON backend
let config = VbrConfig::default()
    .with_storage_backend(StorageBackend::Json {
        path: "./data/vbr.json".into(),
    });

// In-memory backend
let config = VbrConfig::default()
    .with_storage_backend(StorageBackend::Memory);
```

### 2. Entity Definition

Define entities with schemas, relationships, and constraints:

```rust
use mockforge_vbr::schema::{VbrSchemaDefinition, ForeignKeyDefinition, CascadeAction};

let order_schema = VbrSchemaDefinition {
    base: SchemaDefinition {
        name: "Order".to_string(),
        fields: vec![
            // ... field definitions
        ],
        // ...
    },
    primary_key: vec!["id".to_string()],
    foreign_keys: vec![ForeignKeyDefinition {
        field: "user_id".to_string(),
        target_entity: "User".to_string(),
        target_field: "id".to_string(),
        on_delete: CascadeAction::Cascade,
        on_update: CascadeAction::Cascade,
    }],
    unique_constraints: Vec::new(),
    indexes: Vec::new(),
    auto_generation: HashMap::new(),
};
```

### 3. Auto-Generated CRUD APIs

Once entities are registered, CRUD endpoints are automatically generated:

- `GET /vbr-api/{entity}` - List all entities (with pagination, filtering, sorting)
- `GET /vbr-api/{entity}/{id}` - Get entity by ID
- `POST /vbr-api/{entity}` - Create new entity
- `PUT /vbr-api/{entity}/{id}` - Update entity (full replacement)
- `PATCH /vbr-api/{entity}/{id}` - Partial update entity
- `DELETE /vbr-api/{entity}/{id}` - Delete entity

**Example:**

```bash
# Create a user
curl -X POST http://localhost:3000/vbr-api/User \
  -H "Content-Type: application/json" \
  -d '{"id": "user1", "name": "John Doe", "email": "john@example.com"}'

# Get user by ID
curl http://localhost:3000/vbr-api/User/user1

# List all users with pagination
curl "http://localhost:3000/vbr-api/User?limit=10&offset=0&sort=name"

# Update user
curl -X PUT http://localhost:3000/vbr-api/User/user1 \
  -H "Content-Type: application/json" \
  -d '{"id": "user1", "name": "Jane Doe", "email": "jane@example.com"}'

# Delete user
curl -X DELETE http://localhost:3000/vbr-api/User/user1
```

### 4. Relationship Endpoints

Traverse relationships automatically:

- `GET /vbr-api/{entity}/{id}/{relationship}` - Get related entities

**Example:**

```bash
# Get all orders for a user (one-to-many)
curl http://localhost:3000/vbr-api/User/user1/Order

# Get the user for an order (many-to-one)
curl http://localhost:3000/vbr-api/Order/order1/User
```

### 5. Session Management

Enable session-scoped data for isolated per-session databases:

```rust
use mockforge_core::intelligent_behavior::session::{SessionManager, SessionTracking};
use mockforge_vbr::session::SessionDataManager;

let session_manager = Arc::new(SessionManager::new(
    SessionTracking::default(),
    3600, // 1 hour timeout
));

let session_data_manager = SessionDataManager::new(
    session_manager,
    StorageBackend::Memory,
);

// Each session gets its own isolated database
let session_db = session_data_manager.get_session_database("session-id").await?;
```

### 6. Data Aging

Automatically clean up expired data:

```rust
use mockforge_vbr::aging::{AgingManager, AgingRule, AgingAction};

let mut aging_manager = AgingManager::new();
aging_manager.add_rule(AgingRule {
    entity_name: "Session".to_string(),
    expiration_field: "expires_at".to_string(),
    expiration_duration: 3600, // 1 hour
    action: AgingAction::Delete,
});

// Run cleanup (typically done in background)
let cleaned = aging_manager.cleanup_expired(&database, &registry).await?;
```

### 7. Authentication

Virtual user management and JWT token generation:

```rust
use mockforge_vbr::auth::VbrAuthService;

let auth_service = VbrAuthService::new(
    "your-secret-key".to_string(),
    86400, // 24 hour token expiration
);

// Create a user
let user = auth_service.create_default_user(
    "john".to_string(),
    "password123".to_string(),
    "john@example.com".to_string(),
).await?;

// Authenticate
let user = auth_service.authenticate("john", "password123").await?;

// Generate JWT token
let token = auth_service.generate_token(&user)?;

// Validate token
let user = auth_service.validate_token(&token)?;
```

## CLI Commands

### Create Entity

```bash
# Create entity from fields
mockforge vbr create entity User \
  --fields id:string,name:string,email:string,age:number

# Create entity from schema file
mockforge vbr create entity User --schema user-schema.json

# Save to file
mockforge vbr create entity User \
  --fields id:string,name:string \
  --output user-entity.json
```

### Serve VBR API

```bash
# Start with in-memory storage
mockforge vbr serve --port 3000

# Start with SQLite
mockforge vbr serve --port 3000 \
  --storage sqlite \
  --db-path ./data/vbr.db

# Start with JSON storage
mockforge vbr serve --port 3000 \
  --storage json \
  --db-path ./data/vbr.json

# Enable session-scoped data
mockforge vbr serve --port 3000 --session-scoped
```

### Manage Entities

```bash
# List all entities
mockforge vbr manage entities list

# Show entity details
mockforge vbr manage entities show User
```

## Integration with mockforge-http

To integrate VBR routes into the main MockForge HTTP server:

```rust
use mockforge_vbr::integration::integrate_vbr_routes;
use axum::Router;

let app = Router::new(); // Your existing router

let context = mockforge_vbr::handlers::HandlerContext {
    database: engine.database_arc(),
    registry: engine.registry().clone(),
    session_manager: None,
};

let app = integrate_vbr_routes(app, "/vbr-api", context)?;
```

## Configuration

VBR can be configured via `VbrConfig`:

```rust
use mockforge_vbr::{VbrConfig, StorageBackend};

let config = VbrConfig::default()
    .with_storage_backend(StorageBackend::Sqlite {
        path: "./data/vbr.db".into(),
    })
    .with_session_timeout(7200) // 2 hours
    .with_session_scoped_data(true)
    .with_data_aging(true)
    .with_cleanup_interval(3600); // 1 hour
```

## Examples

See the `tests/tests/vbr_integration.rs` file for comprehensive examples of:
- Creating entities
- CRUD operations
- Relationship traversal
- Pagination and filtering
- Error handling

## Architecture

### Components

1. **VirtualDatabase**: Abstraction over storage backends
2. **EntityRegistry**: Manages entity definitions
3. **MigrationManager**: Generates SQL schemas from entities
4. **ConstraintValidator**: Enforces foreign keys and constraints
5. **HandlerContext**: Shared state for HTTP handlers
6. **SessionDataManager**: Per-session database isolation
7. **AgingManager**: Time-based data cleanup
8. **VbrAuthService**: User authentication and JWT tokens

### Request Flow

```
HTTP Request → Handler → HandlerContext → VirtualDatabase → Storage Backend
                                      ↓
                                 EntityRegistry (validate schema)
                                      ↓
                              ConstraintValidator (check constraints)
```

## Best Practices

1. **Use SQLite for production-like testing**: Provides realistic database behavior
2. **Use Memory for fast tests**: No I/O overhead, perfect for unit tests
3. **Define relationships explicitly**: Foreign keys enable relationship traversal
4. **Enable data aging for realistic behavior**: Simulates data lifecycle
5. **Use session-scoped data for multi-user scenarios**: Each session gets isolated data

## Limitations

- Currently supports SQLite, JSON, and in-memory storage only
- JWT authentication requires the `jwt` feature flag (fallback available)
- Full archive functionality requires manual archive table creation
- Complex queries (JOINs across multiple tables) not yet supported

## Contributing

VBR is part of the MockForge project. See the main [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## License

Licensed under MIT OR Apache-2.0, same as MockForge.
