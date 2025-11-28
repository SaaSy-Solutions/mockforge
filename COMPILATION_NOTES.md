# Compilation Notes

## ✅ Resolved Issues

### X-Ray Request Context Storage
- **Status**: ✅ Implemented
- **Location**: `crates/mockforge-http/src/handlers/xray.rs`
- **Implementation**:
  - Added `RequestContextSnapshot` struct to store state snapshots
  - Added `request_contexts` field to `XRayState` for in-memory storage
  - Implemented `store_request_context()` function called by middleware
  - Updated `get_request_context()` to retrieve stored snapshots
  - Automatic cleanup: limits to 1000 requests per workspace, removes oldest 100 when limit exceeded

### mockforge-ui Dependency
- **Status**: ✅ Fixed
- **Change**: Updated `crates/mockforge-ui/Cargo.toml` to use local `mockforge-collab` path dependency
- **Before**: `mockforge-collab = "^0.3.0"` (from crates.io)
- **After**: `mockforge-collab = { version = "0.3.2", path = "../mockforge-collab" }`

## ⚠️ Known Issue: mockforge-cli Compilation

### Problem
`mockforge-cli` fails to compile due to SQLX query cache issues when `SQLX_OFFLINE=true` is set.

### Root Cause
When compiling `mockforge-cli`, it transitively depends on `mockforge-collab`:
- `mockforge-cli` → `mockforge-ui` → `mockforge-collab`
- Even though `mockforge-ui` now uses the local path, there may be other transitive dependencies pulling in the published `mockforge-collab@0.3.1` from crates.io
- The published version doesn't include the `.sqlx` query cache directory

### Solutions

#### Option 1: Wait for Published Version (Recommended)
When `mockforge-collab@0.3.2` is published to crates.io with the `.sqlx` directory included:
- The published crate will include all 51+ query cache files
- Users installing from crates.io will be able to compile without a database connection
- This is already configured in `Cargo.toml` (line 14: `include = [..., ".sqlx/**/*", ...]`)

#### Option 2: Use Database Connection During Compilation
Set `SQLX_OFFLINE=false` when compiling:
```bash
SQLX_OFFLINE=false cargo build -p mockforge-cli
```

#### Option 3: Prepare SQLX Queries Locally
If you have a database available:
```bash
# Set up temporary database
export DATABASE_URL="sqlite://test-sqlx.db"

# Run migrations
cargo sqlx migrate run --source crates/mockforge-collab/migrations

# Prepare queries
cargo sqlx prepare --database-url "$DATABASE_URL" --workspace

# Now compile
cargo build -p mockforge-cli
```

### Verification
To verify the `.sqlx` directory will be included in the published crate:
```bash
cd crates/mockforge-collab
./verify-publish.sh
```

This script checks:
- ✅ `.sqlx` directory exists with query cache files
- ✅ `Cargo.toml` includes `.sqlx/**/*`
- ✅ `.sqlx` is not ignored by `.gitignore`
- ✅ Package includes all `.sqlx` files

## Current Compilation Status

- ✅ `mockforge-core`: Compiles successfully
- ✅ `mockforge-http`: Compiles successfully (all handlers complete)
- ✅ `mockforge-schema`: Compiles successfully
- ✅ `mockforge-collab`: Compiles successfully (local version 0.3.2)
- ⚠️ `mockforge-cli`: Compiles with `SQLX_OFFLINE=false`, fails with `SQLX_OFFLINE=true` (expected until 0.3.2 is published)

## All TODOs Completed

- ✅ Request context storage/retrieval in `xray.rs`
- ✅ Database row mapping for ThreatAssessment
- ✅ Database row mapping for SemanticIncident
- ✅ Database row mapping for forecasts
- ✅ Database queries for contract health timeline
- ✅ Threat findings and remediation mapping
