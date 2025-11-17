# SQLx Offline Mode

The `mockforge-collab` crate uses SQLx for database queries, which requires either:
1. A database connection during compilation (for compile-time query checking)
2. SQLx offline mode (using pre-generated query metadata)

## Using SQLx Offline Mode

To avoid database connection requirements during compilation, you can enable SQLx offline mode:

### Option 1: Environment Variable

```bash
export SQLX_OFFLINE=true
cargo check
```

### Option 2: Cargo Config

The `.cargo/config.toml` file has been configured to support SQLx offline mode. You can uncomment the `SQLX_OFFLINE` setting if needed.

### Option 3: Per-Command

```bash
SQLX_OFFLINE=true cargo build
SQLX_OFFLINE=true cargo test
```

## Updating Query Metadata

If you modify SQL queries in `mockforge-collab`, you'll need to regenerate the query metadata:

```bash
cd crates/mockforge-collab
cargo sqlx prepare --database-url postgresql://user:pass@localhost/dbname
```

This generates `.sqlx/` directory with query metadata that can be used for offline compilation.

## CI/CD

For CI/CD pipelines, ensure `SQLX_OFFLINE=true` is set in your build environment, or ensure the `.sqlx/` directory is committed to the repository.

## Note

The SQLx compilation errors you may see are from `mockforge-collab` and are pre-existing. They don't affect the voice workspace creation feature implementation.
