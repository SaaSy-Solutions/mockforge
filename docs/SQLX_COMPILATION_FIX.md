# SQLx Compilation Fix

## Issue

The `mockforge-collab` crate uses `sqlx::query!` and `sqlx::query_as!` macros which require compile-time query checking. This requires either:
1. A database connection during compilation (DATABASE_URL)
2. Prepared query cache (SQLX_OFFLINE=true with prepared queries)

## Current Status

- SQLX_OFFLINE is temporarily disabled in `.cargo/config.toml`
- Type errors in `backup.rs` and `merge.rs` need to be fixed
- sqlx returns `Option<&str>` for TEXT fields, but code expects `Option<String>`

## Solution

To compile, set `DATABASE_URL` environment variable:

```bash
export DATABASE_URL="sqlite:///tmp/test-mockforge.db"
cargo check --package mockforge-collab
```

## Long-term Fix

1. Convert all `sqlx::query!` and `sqlx::query_as!` to runtime queries (`sqlx::query` and `sqlx::query_as`)
2. Or prepare queries with `cargo sqlx prepare` and re-enable SQLX_OFFLINE
3. Or use PostgreSQL which has better type support

## Files Needing Fixes

- `crates/mockforge-collab/src/backup.rs` - Type conversions for TEXT fields
- `crates/mockforge-collab/src/merge.rs` - Type conversions for TEXT fields
- `crates/mockforge-collab/src/user.rs` - Type conversions
- `crates/mockforge-collab/src/workspace.rs` - Type conversions
- `crates/mockforge-collab/src/history.rs` - Type conversions
- `crates/mockforge-collab/src/access_review_provider.rs` - Type conversions
