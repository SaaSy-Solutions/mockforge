# MockForge API Response Fixes - Implementation Summary

## Overview
This document summarizes the implementation of fixes for MockForge's API responses to support Apiary Pro frontend integration.

## Changes Made

### 1. Multipart Form Data Support (Generic Implementation)

**Files Modified:**
- `crates/mockforge-core/src/ai_response.rs`
- `crates/mockforge-core/src/openapi_routes.rs`

**Changes:**
- Extended `RequestContext` struct to include:
  - `multipart_fields: HashMap<String, Value>` - Form field values
  - `multipart_files: HashMap<String, String>` - File upload paths
- Added helper methods: `with_multipart_fields()` and `with_multipart_files()`
- Extended template expansion to support `{{multipart.*}}` variables
- Created `extract_multipart_from_bytes()` function to parse multipart/form-data requests
- Integrated multipart detection and extraction into both route handlers:
  - `build_router()`
  - `build_router_with_injectors_and_overrides()`

**How It Works:**
1. Route handler checks `Content-Type` header for `multipart/form-data`
2. If multipart, calls `extract_multipart_from_bytes()` to parse the request body
3. Extracts form fields and file uploads
4. Stores uploaded files in `./tmp/mockforge-uploads/` directory
5. Makes multipart fields available via `RequestContext` for response generation
6. Creates JSON representation of multipart fields for validation

**Benefits:**
- Generic implementation works for any endpoint that receives multipart/form-data
- Files are stored with unique UUIDs to prevent conflicts
- Form fields are available in response templates via `{{multipart.fieldName}}`
- Works with OpenAPI spec validation

### 2. Override Configuration

**Files Created:**
- `overrides/apiary-pro-fixes.yaml` - Override rules for Apiary Pro endpoints
- `overrides/README.md` - Documentation for override configuration

**Override Rules:**
1. **Route Optimizer Response** - Standardizes `POST /api/ai/route-optimizer/optimize` response
2. **Print Labels Response** - Adds download URLs to `POST /api/assets/print-labels`
3. **Invoice Export Response** - Adds download URLs to `GET /api/contractor/invoices/{id}/export`
4. **Provenance Export Response** - Adds download URLs to `POST /api/provenance/export`
5. **Pagination Examples** - Provides examples for standardizing pagination (commented out)
6. **Error Format Examples** - Provides examples for standardizing errors (commented out)

**Usage:**
Set environment variable:
```bash
export MOCKFORGE_HTTP_OVERRIDES_GLOB="overrides/apiary-pro-fixes.yaml"
```

Or add to `mockforge.yaml`:
```yaml
http:
  overrides_glob: "overrides/apiary-pro-fixes.yaml"
```

### 3. File Generation Service

**Files Created:**
- `crates/mockforge-http/src/file_generator.rs` - File generation service

**Features:**
- Generates mock files in multiple formats: PDF, CSV, JSON, EPCIS XML
- Creates unique filenames with UUIDs and timestamps
- Organizes files by route ID in directory structure: `mock-files/{route_id}/{filename}`
- Tracks generation statistics (files generated, total bytes)

**File Types Supported:**
- **PDF**: Minimal PDF structure with document metadata
- **CSV**: Comma-separated values with headers
- **JSON**: Pretty-printed JSON with metadata
- **EPCIS XML**: EPCIS-compliant XML for provenance tracking

### 4. File Serving Route

**Files Created:**
- `crates/mockforge-http/src/file_server.rs` - File serving endpoint

**Features:**
- Serves files from `mock-files/` directory structure
- Path: `/mock-files/{route_id}/{filename}`
- Security: Prevents path traversal attacks
- Proper Content-Type and Content-Disposition headers
- Configurable base directory via `MOCKFORGE_MOCK_FILES_DIR` environment variable

**Integration:**
- Added to HTTP router in all router building functions
- Accessible at `http://localhost:3000/mock-files/{route_id}/{filename}`

## Implementation Details

### Multipart Parser
The multipart parser:
- Extracts boundary from `Content-Type` header
- Parses multipart body using byte-level operations (handles binary files)
- Identifies form fields vs. file uploads via `Content-Disposition` header
- Stores files with UUID-based unique names
- Handles UTF-8 and binary data correctly

### Download URLs
The override file includes download URLs in the format:
- `http://localhost:3000/mock-files/{type}/{uuid}.{ext}`

Files are generated on-demand when endpoints are called. The file generation service can be integrated into override handlers or response generation to create files when download URLs are requested.

## Testing Recommendations

1. **Multipart Form Data:**
   ```bash
   curl -X POST http://localhost:3000/api/contractor/proofs \
     -F "taskId=task123" \
     -F "latitude=40.7128" \
     -F "longitude=-74.0060" \
     -F "notes=Test proof" \
     -F "media_1=@test.jpg"
   ```

2. **Route Optimizer:**
   ```bash
   curl -X POST http://localhost:3000/api/ai/route-optimizer/optimize \
     -H "Content-Type: application/json" \
     -d '{"tasks": [...], "technicians": [...]}'
   ```

3. **Export Endpoints:**
   Verify that responses include `download_url`, `format`, and `generated_at` fields.

4. **File Serving:**
   ```bash
   # After generating a file, test serving it
   curl http://localhost:3000/mock-files/labels/some-uuid.pdf
   ```

## Next Steps

1. **File Generation Integration:** Integrate file generation into override handlers or response generation to automatically create files when download URLs are requested
2. **File Cleanup:** Add periodic cleanup of old generated files
3. **Selective Pagination:** Add pagination overrides for specific list endpoints as needed
4. **Selective Error Formatting:** Add error format overrides for specific endpoints as needed

## Compatibility

- ✅ Works with existing OpenAPI spec validation
- ✅ Compatible with existing override system
- ✅ Generic multipart support (not Apiary Pro-specific)
- ✅ Backward compatible (doesn't break existing functionality)

## Files Modified

1. `crates/mockforge-core/src/ai_response.rs` - Extended RequestContext
2. `crates/mockforge-core/src/openapi_routes.rs` - Added multipart handling
3. `crates/mockforge-http/src/file_generator.rs` - Created file generation service
4. `crates/mockforge-http/src/file_server.rs` - Created file serving endpoint
5. `crates/mockforge-http/src/lib.rs` - Integrated file serving router
6. `overrides/apiary-pro-fixes.yaml` - Created override configuration
7. `overrides/README.md` - Created documentation

## Notes

- The multipart parser is a basic implementation that handles common multipart/form-data scenarios
- For production use with complex multipart scenarios, consider using a dedicated multipart parsing library
- File cleanup for uploaded files is not implemented - consider adding periodic cleanup
- The override file uses template variables like `{{uuid}}` and `{{now}}` which are expanded by MockForge's templating system
- File generation service is available but not automatically triggered - files need to be generated on-demand or integrated into response handlers
