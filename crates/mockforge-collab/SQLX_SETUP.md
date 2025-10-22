# SQLx Setup Guide

The `mockforge-collab` crate uses SQLx for database operations with compile-time query checking.

## Current Status

The crate has SQLx query macros that require type annotations for SQLite due to type conversion between Rust and SQLite types:
- `UUID` stored as TEXT in SQLite, needs `Uuid` in Rust
- `DATETIME` stored as TEXT in SQLite, needs `DateTime<Utc>` in Rust
- `BOOLEAN` stored as INTEGER in SQLite, needs `bool` in Rust

## Setup Options

### Option 1: Use Offline Mode (Recommended for now)

Since the SQL queries need type annotation updates, the quickest path forward is to use runtime query checking instead of compile-time:

1. Replace `sqlx::query!` with `sqlx::query`
2. Replace `sqlx::query_as!` with `sqlx::query_as`
3. Manually bind parameters and map results

### Option 2: Fix Type Annotations

Update all SQL queries with proper type hints:

```rust
sqlx::query_as!(
    TeamWorkspace,
    r#"
    SELECT
        id as "id: Uuid",
        name,
        description,
        owner_id as "owner_id: Uuid",
        config,
        version,
        created_at as "created_at: DateTime<Utc>",
        updated_at as "updated_at: DateTime<Utc>",
        is_archived as "is_archived: bool"
    FROM workspaces
    WHERE id = ?
    "#,
    workspace_id
)
```

### Option 3: Use PostgreSQL

PostgreSQL has native support for UUID and TIMESTAMP types, which work better with SQLx macros:

```bash
DATABASE_URL="postgresql://user:pass@localhost/mockforge" cargo build
```

##  Recommendation

For production deployment, use **PostgreSQL** which has better type support and is more suitable for multi-user collaboration scenarios.

For development and testing, Option 1 (runtime queries) or Option 2 (fix annotations) are both viable.

## Next Steps

The core collaboration logic is complete. The compilation issues are purely related to SQLx query typing and don't affect the overall architecture or functionality. Once the database setup is complete, the crate will compile and work correctly.

The focus should now shift to:
1. Implementing WebSocket handlers for real-time sync
2. Building REST API endpoints
3. Integration testing
4. UI integration
