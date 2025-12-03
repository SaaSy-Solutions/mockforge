# MockForge Publishing Guide

## Summary

This guide covers publishing MockForge crates to crates.io, including verification steps to ensure there are no issues with circular dependencies, SQLx compile-time query checking, or other publish blockers.

## Prerequisites

1. **Crates.io Account**: You need a crates.io account
2. **API Token**: Get your token from https://crates.io/me

## Pre-Publishing Verification Checklist

Before publishing, verify the following to ensure a smooth publishing experience:

### ✅ 1. SQLx Compile-Time Query Checking

**Issue**: `mockforge-collab` uses SQLx compile-time macros (`sqlx::query!` and `sqlx::query_as!`) that require query metadata.

**Verification Steps**:
```bash
# Run the verification script
cd crates/mockforge-collab
./verify-publish.sh
```

**Expected Results**:
- ✅ `.sqlx` directory exists with 51+ query cache files
- ✅ `Cargo.toml` includes `.sqlx/**/*` in `include` field
- ✅ Package includes all `.sqlx` query cache files

**Status**: ✅ Verified - All checks pass. The `.sqlx` directory is properly included in the package.

**Configuration**:
- `.cargo/config.toml` has been updated to remove absolute path dependencies
- Published crates will use `.sqlx` offline mode automatically via `build.rs`
- No database connection required for users installing from crates.io

### ✅ 2. Other SQLx-Using Crates

**Crates checked**: `mockforge-federation`, `mockforge-pipelines`, `mockforge-analytics`, `mockforge-vbr`, `mockforge-recorder`, `mockforge-registry-server`

**Status**: ✅ Verified - None of these crates use compile-time macros. They all use runtime queries (`sqlx::query` and `sqlx::query_as` without `!`), which don't require special handling.

### ✅ 3. Circular Dependencies

**Status**: ✅ Verified - No circular dependencies detected. The architecture follows a clean layered structure with `mockforge-core` as the foundation.

**Verification**: Documented in `docs/1.0_RELEASE_READINESS.md` and `ARCHITECTURE.md`

### ✅ 4. Path Dependencies

**Issue**: All crates use `path = "../..."` dependencies that must be converted to version dependencies before publishing.

**Solution**: The `scripts/publish-crates.sh` script automatically converts path dependencies to version dependencies before publishing.

**Verification**: The script handles conversion for all internal mockforge crates.

### ✅ 5. Package Manifest Verification

**Critical Check**: `mockforge-collab` must include `.sqlx/**/*` in its `include` field.

**Status**: ✅ Verified - `Cargo.toml` includes:
```toml
include = ["src/**/*", "migrations/**/*", ".sqlx/**/*", ".cargo/**/*", "build.rs", "Cargo.toml", "README.md", "LICENSE-*"]
```

**Package Contents Verification**:
```bash
# Verify .sqlx files are included
cargo package --list -p mockforge-collab | grep "\.sqlx" | wc -l
# Should show 51+ files
```

## Publishing Steps

### Step 1: Set Your Crates.io Token

```bash
export CRATES_IO_TOKEN='your_token_here'
```

### Step 2: Run Pre-Publishing Verification

```bash
# Verify SQLx setup for mockforge-collab
cd crates/mockforge-collab
./verify-publish.sh
cd ../..

# Verify package contents
cargo package --list -p mockforge-collab | grep "\.sqlx" | wc -l
```

### Step 3: Dry Run (Highly Recommended)

```bash
./scripts/publish-crates.sh --dry-run
```

This will:
- Convert path dependencies to version dependencies
- Verify package structure
- Test publishing without actually uploading to crates.io

### Step 4: Publish to Crates.io

```bash
./scripts/publish-crates.sh
```

The script will:
- Convert path dependencies to version dependencies automatically
- Publish crates in correct dependency order
- Skip crates already published
- Wait 30 seconds between publishes
- Handle all workspace crates

## Known Issues and Solutions

### SQLx Compile-Time Query Checking

**Issue**: `mockforge-collab` requires SQLx query metadata for compilation.

**Solution**:
- ✅ `.sqlx` directory is included in the published package
- ✅ `build.rs` automatically enables `SQLX_OFFLINE=true` when `.sqlx` exists
- ✅ Users installing from crates.io don't need a database connection

**For Local Development**:
- If you need to regenerate query cache: `cargo sqlx prepare --database-url <url>`
- The `.cargo/config.toml` has been updated to not interfere with published crates

### Path Dependencies

**Issue**: Workspace crates use path dependencies that won't work on crates.io.

**Solution**: The publishing script automatically converts all path dependencies to version dependencies before publishing.

### Version Conflicts with Already-Published Crates

**Root Cause**:
- **Current Status**: All crates on crates.io are at `0.3.4`
- **Workspace Version**: Should be `0.3.5` for the next release
- **Previous Issue**: When workspace was at `0.3.3`, the script converted dependencies to `"0.3.3"`, which Cargo interprets as `>=0.3.3, <0.4.0`
- This caused Cargo to resolve to the already-published `0.3.4` instead of the workspace version
- The published `0.3.4` may have different dependencies than the workspace version, causing conflicts

**Example Error**:
```
error[E0308]: mismatched types
expected type `opentelemetry::context::Context` (from opentelemetry@0.22.0)
found type `opentelemetry::context::Context` (from opentelemetry@0.21.0)
```

**Solutions**:

1. **Keep Workspace Version Ahead of Published Versions** (✅ Current Solution):
   - Workspace is now at `0.3.5` (next release)
   - Published crates are at `0.3.4` (current)
   - When publishing, dependencies will be converted to `"0.3.5"` which won't conflict with `0.3.4`
   - This ensures clean version resolution

2. **Publish All Crates Together**:
   - Publish all crates in the same batch so they all resolve to the same versions
   - The script already handles this with phase-based publishing

3. **Verify Before Publishing**:
   - Run `cargo tree -p <crate-name>` after dependency conversion to check for conflicts
   - Check for multiple versions of the same dependency: `cargo tree -i <dependency-name>`
   - Ensure workspace version matches what you intend to publish

## Post-Publishing Verification

After publishing, verify that:
1. All crates are accessible on crates.io
2. Dependencies resolve correctly: `cargo tree -p <crate-name>`
3. Users can install without issues: `cargo install <crate-name>` (for binary crates)

---

**Ready to publish?** Run `./scripts/publish-crates.sh --dry-run` to get started!
