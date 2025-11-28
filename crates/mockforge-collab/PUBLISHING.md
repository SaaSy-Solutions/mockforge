# Publishing mockforge-collab

## SQLx Query Cache

This crate uses SQLx with compile-time query checking. To ensure the published crate works correctly:

### ✅ Current Setup (Already Configured)

1. **`.sqlx` directory is included in the package**:
   - `Cargo.toml` includes `.sqlx/**/*` in the `include` field (line 14)
   - This ensures all 51+ query cache files are included in the published crate

2. **`.sqlx` is tracked in git**:
   - The `.gitignore` explicitly does NOT ignore `.sqlx` (see comment in `.gitignore`)
   - Query cache files are committed to the repository

3. **Build script handles both cases**:
   - If `.sqlx` exists: Automatically enables `SQLX_OFFLINE=true`
   - If `.sqlx` missing: Falls back to requiring a database connection (with helpful warning)

### Verification Before Publishing

Before publishing to crates.io, verify the `.sqlx` directory is included:

```bash
# Check that .sqlx files are in the package
cargo package --list | grep "\.sqlx" | wc -l
# Should show 51+ files

# Verify the package includes .sqlx
cargo package --list | grep "\.sqlx/query-"
# Should list all query cache files
```

### For Users Installing from crates.io

Users installing `mockforge-collab` from crates.io will:
- ✅ Get the `.sqlx` directory automatically (included in the package)
- ✅ Compile without needing a database connection
- ✅ Have offline mode enabled automatically by the build script

### For Users Building from Source

If building from source (e.g., from git), users can:

1. **Use the included `.sqlx`** (if present in the repo)
2. **Set `SQLX_OFFLINE=false`** to use a database connection
3. **Prepare queries**: Run `cargo sqlx prepare --database-url <url>`

### Updating Query Cache

When adding new SQL queries:

1. Run migrations: `sqlx migrate run --source migrations`
2. Prepare queries: `cargo sqlx prepare --database-url <url>`
3. Commit the new `.sqlx/*.json` files to git
4. Verify they're included: `cargo package --list | grep "\.sqlx"`

The build script will automatically detect and use the cached queries.
