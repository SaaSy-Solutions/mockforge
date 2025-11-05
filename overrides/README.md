# MockForge Override Configuration for Apiary Pro Integration

This directory contains override configurations that customize MockForge's response generation for specific API integrations.

## apiary-pro-fixes.yaml

This override file standardizes response formats for Apiary Pro frontend integration. It addresses the following fixes:

### 1. Enhanced Route Optimization Response
- **Endpoint**: `POST /api/ai/route-optimizer/optimize`
- **Fix**: Standardizes response structure to include `route` object with stops array and `optimization_metrics`

### 2. Printable Labels Response
- **Endpoint**: `POST /api/assets/print-labels`
- **Fix**: Adds `download_url`, `label_urls`, and `format` fields to response

### 3. Invoice Export Response
- **Endpoint**: `GET /api/contractor/invoices/{id}/export`
- **Fix**: Adds `download_url`, `format`, and `generated_at` fields to response

### 4. Provenance Pack Response
- **Endpoint**: `POST /api/provenance/export`
- **Fix**: Adds `download_url`, `export_url`, `format`, and `generated_at` fields to response

### 5. Pagination Format (Examples)
- Provides examples for standardizing pagination format on list endpoints
- Should be applied selectively to specific endpoints as needed

### 6. Error Response Format (Examples)
- Provides examples for standardizing error response format
- Should be applied selectively to specific endpoints as needed

## Usage

To enable these overrides, set the `MOCKFORGE_HTTP_OVERRIDES_GLOB` environment variable:

```bash
export MOCKFORGE_HTTP_OVERRIDES_GLOB="overrides/apiary-pro-fixes.yaml"
```

Or add to your `mockforge.yaml` configuration file:

```yaml
http:
  overrides_glob: "overrides/apiary-pro-fixes.yaml"
```

## File Serving

The download URLs in the override file point to `http://localhost:3000/mock-files/...`. To make these URLs functional:

1. Implement a file generation service that creates mock files (PDF, CSV, JSON) based on route/request context
2. Add a route handler to serve files from the `./mock-files/` directory
3. Store generated files with unique identifiers (UUID + timestamp)

This file serving functionality can be implemented as a separate feature or plugin.
