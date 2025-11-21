# SQLx Offline Mode

The `mockforge-collab` crate uses SQLx for database queries, which requires either:
1. A database connection during compilation (for compile-time query checking)
2. SQLx offline mode (using pre-generated query metadata)

## Automatic Offline Mode

The `build.rs` script automatically enables SQLx offline mode if a `.sqlx/` directory with query cache files exists. This means:
- If `.sqlx/` exists with cached queries, offline mode is enabled automatically
- If `.sqlx/` doesn't exist, SQLx will require a database connection during compilation
- You can override this behavior by explicitly setting `SQLX_OFFLINE=true` or `SQLX_OFFLINE=false`

## Manual Control

### Option 1: Environment Variable

```bash
export SQLX_OFFLINE=true
cargo check
```

### Option 2: Per-Command

```bash
SQLX_OFFLINE=true cargo build
SQLX_OFFLINE=true cargo test
```

### Option 3: Disable Offline Mode

If you want to use a database connection instead of cached queries:

```bash
SQLX_OFFLINE=false DATABASE_URL="sqlite:/path/to/db" cargo build
```

## Regenerating Query Cache

If you modify SQL queries in `mockforge-collab`, you **must** regenerate the query metadata:

### Using SQLite (Recommended for Development)

```bash
cd crates/mockforge-collab

# Create a temporary database
sqlx database create --database-url "sqlite:/tmp/mockforge-sqlx-prepare.db"

# Run migrations
sqlx migrate run --database-url "sqlite:/tmp/mockforge-sqlx-prepare.db"

# Generate query cache
cargo sqlx prepare --database-url "sqlite:/tmp/mockforge-sqlx-prepare.db"

# Clean up (optional)
rm /tmp/mockforge-sqlx-prepare.db
```

### Using PostgreSQL

```bash
cd crates/mockforge-collab
cargo sqlx prepare --database-url "postgresql://user:pass@localhost/dbname"
```

This updates the `.sqlx/` directory with query metadata for offline compilation.

## Maintenance

### When to Regenerate

You should regenerate the query cache when:
- Adding new SQL queries using `sqlx::query!` or `sqlx::query_as!` macros
- Modifying existing SQL queries
- Changing database schema (migrations)
- After pulling changes that include new/modified queries

### Keeping Cache Up-to-Date

The `.sqlx/` directory should be committed to the repository to enable offline builds for all developers. When you modify queries:

1. Regenerate the cache using `cargo sqlx prepare`
2. Commit the updated `.sqlx/` directory
3. Other developers can build without a database connection

## Troubleshooting

### Error: "SQLX_OFFLINE=true but there is no cached data for this query"

**Cause**: The query cache is missing or out of date.

**Solution**: Regenerate the query cache:
```bash
cd crates/mockforge-collab
cargo sqlx prepare --database-url "sqlite:/tmp/prepare.db"
```

### Error: "Failed to connect to database"

**Cause**: You're trying to use offline mode but the cache is missing, or you're trying to prepare queries without a database.

**Solution**: 
- If you want offline mode: Ensure `.sqlx/` directory exists and is up-to-date
- If you want to prepare queries: Set up a database and provide `DATABASE_URL`

### Build Script Warnings

The build script may show warnings:
- `"SQLx offline mode enabled (found N cached queries)"` - This is informational, offline mode is working
- `".sqlx directory exists but contains no query cache files"` - Run `cargo sqlx prepare` to generate cache
- `"No .sqlx directory found"` - Either generate cache or set `SQLX_OFFLINE=false` with a database

## CI/CD

For CI/CD pipelines:

1. **Option A (Recommended)**: Commit `.sqlx/` directory to repository
   - No database needed in CI
   - Fast builds
   - Ensure cache is kept up-to-date

2. **Option B**: Use database connection in CI
   ```yaml
   - name: Build
     env:
       SQLX_OFFLINE: false
       DATABASE_URL: "sqlite:/tmp/test.db"
     run: cargo build
   ```

3. **Option C**: Generate cache in CI
   ```yaml
   - name: Prepare SQLx cache
     run: |
       cd crates/mockforge-collab
       sqlx database create --database-url "$DATABASE_URL"
       sqlx migrate run --database-url "$DATABASE_URL"
       cargo sqlx prepare --database-url "$DATABASE_URL"
   ```

## Makefile Target

A convenience target is available in the root `Makefile`:

```bash
make sqlx-prepare
```

This will automatically set up a temporary database, run migrations, and regenerate the query cache.
