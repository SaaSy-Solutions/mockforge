# Conformance Testing Guide

MockForge includes an OpenAPI 3.0.0 conformance testing system that verifies whether an API server correctly implements its OpenAPI spec. It generates [k6](https://k6.io) scripts that exercise 47 features across 11 categories.

## Prerequisites

- [k6](https://k6.io/docs/get-started/installation/) must be installed and on your PATH
- An OpenAPI 3.0.0 spec (JSON or YAML)
- A running target server

## Quick Start

```bash
# Test MockForge mock server against your spec
mockforge bench --conformance \
  --spec my-api.json \
  --target http://localhost:3000

# Test your real API server
mockforge bench --conformance \
  --spec my-api.json \
  --target https://api.example.com \
  --conformance-header "Authorization: Bearer YOUR_TOKEN"
```

The test runs 1 virtual user for 1 iteration (functional test, not load test). Results are printed to the terminal and saved to `conformance-report.json`.

## How It Works

1. **Spec analysis**: MockForge parses your OpenAPI spec and detects which features each operation exercises (parameters, body types, response codes, security schemes, etc.)
2. **Script generation**: A k6 JavaScript script is generated with one check per feature
3. **Execution**: k6 runs the script against the target, sending real HTTP requests
4. **Reporting**: Results are aggregated by category and displayed in a table

## Test Categories

### Structural Categories

These verify that the server accepts requests correctly — routing, parameters, body parsing, authentication.

| Category | CLI Name | What It Tests |
|----------|----------|---------------|
| **HTTP Methods** | `http-methods` | Server routes GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS correctly |
| **Parameters** | `parameters` | Path params, query params, header params, cookie params are accepted |
| **Request Bodies** | `request-bodies` | JSON, form-urlencoded, and multipart bodies are accepted |
| **Constraints** | `constraints` | Required/optional fields, min/max, pattern, and enum constraints |
| **Security** | `security` | Bearer token, API key, and basic auth schemes work |
| **Content Types** | `content-types` | Content negotiation when multiple response types are declared |

### Behavioral Categories

These verify that the server returns correct responses — status codes, response schemas, data types.

| Category | CLI Name | What It Tests |
|----------|----------|---------------|
| **Response Codes** | `response-codes` | Server returns 200, 201, 204, 400, 404 for appropriate operations |
| **Schema Types** | `schema-types` | Response fields have correct types (string, integer, number, boolean, array, object) |
| **String Formats** | `string-formats` | String fields match declared formats (date, email, UUID, URI, IPv4, etc.) |
| **Composition** | `composition` | oneOf, anyOf, allOf schemas are handled correctly |
| **Response Validation** | `response-validation` | Full response body matches the declared JSON schema structure |

## Feature Details

### Parameters (7 checks)

| Check Name | Input Sent | Pass Condition |
|------------|------------|----------------|
| `param:path:string` | `GET /resource/{id}` with `id=test-value` | HTTP 2xx response |
| `param:path:integer` | `GET /resource/{id}` with `id=42` | HTTP 2xx response |
| `param:query:string` | `GET /resource?name=test-value` | HTTP 2xx response |
| `param:query:integer` | `GET /resource?limit=42` | HTTP 2xx response |
| `param:query:array` | `GET /resource?tags=a,b` | HTTP 2xx response |
| `param:header` | Request with custom header `X-Custom: test-value` | HTTP 2xx response |
| `param:cookie` | Request with `Cookie: name=test-value` | HTTP 2xx response |

### Request Bodies (3 checks)

| Check Name | Input Sent | Pass Condition |
|------------|------------|----------------|
| `body:json` | POST with `Content-Type: application/json` and generated JSON body | HTTP 2xx response |
| `body:form-urlencoded` | POST with `Content-Type: application/x-www-form-urlencoded` | HTTP 2xx response |
| `body:multipart` | POST with `Content-Type: multipart/form-data` | HTTP 2xx response |

The JSON body is auto-generated from the request body schema in your spec with placeholder values.

### Schema Types (6 checks)

| Check Name | What It Validates | Pass Condition |
|------------|-------------------|----------------|
| `schema:string` | Response contains string-typed fields | `typeof field === 'string'` |
| `schema:integer` | Response contains integer-typed fields | `Number.isInteger(field)` |
| `schema:number` | Response contains number-typed fields | `typeof field === 'number'` |
| `schema:boolean` | Response contains boolean-typed fields | `typeof field === 'boolean'` |
| `schema:array` | Response contains array-typed fields | `Array.isArray(field)` |
| `schema:object` | Response contains object-typed fields | `typeof field === 'object'` |

These are detected from the response schema in your OpenAPI spec.

### Composition (3 checks)

| Check Name | What It Validates | Pass Condition |
|------------|-------------------|----------------|
| `composition:oneOf` | Schema uses `oneOf` | HTTP 2xx + valid JSON response |
| `composition:anyOf` | Schema uses `anyOf` | HTTP 2xx + valid JSON response |
| `composition:allOf` | Schema uses `allOf` | HTTP 2xx + valid JSON response |

### String Formats (7 checks)

| Check Name | What It Validates | Pass Condition |
|------------|-------------------|----------------|
| `format:date` | String fields with `format: date` | Matches date pattern |
| `format:date-time` | String fields with `format: date-time` | Matches ISO 8601 |
| `format:email` | String fields with `format: email` | Contains `@` |
| `format:uuid` | String fields with `format: uuid` | Matches UUID pattern |
| `format:uri` | String fields with `format: uri` | Starts with `http` |
| `format:ipv4` | String fields with `format: ipv4` | Matches IPv4 pattern |
| `format:ipv6` | String fields with `format: ipv6` | Matches IPv6 pattern |

### Constraints (5 checks)

| Check Name | What It Validates | Pass Condition |
|------------|-------------------|----------------|
| `constraint:required` | Required fields declared in schema | HTTP 2xx when sending required fields |
| `constraint:optional` | Optional fields can be omitted | HTTP 2xx when omitting optional fields |
| `constraint:minmax` | `minLength`, `maxLength`, `minimum`, `maximum` | HTTP 2xx with valid values |
| `constraint:pattern` | `pattern` regex constraint | HTTP 2xx with matching value |
| `constraint:enum` | `enum` value restriction | HTTP 2xx with valid enum value |

### Response Codes (5 checks)

| Check Name | Input Sent | Pass Condition |
|------------|------------|----------------|
| `response:200` | Normal GET/POST to the endpoint | Response status === 200 |
| `response:201` | POST to endpoints declaring 201 | Response status === 201 |
| `response:204` | DELETE to endpoints declaring 204 | Response status === 204 |
| `response:400` | Request with `X-Mockforge-Response-Status: 400` header | Response status === 400 |
| `response:404` | Request with `X-Mockforge-Response-Status: 404` header | Response status === 404 |

**Note**: The `response:400` and `response:404` checks send a special `X-Mockforge-Response-Status` header that tells MockForge which status code to return. This only works against MockForge servers (not real API servers). For real servers, these checks validate that the endpoint returns the expected error codes for invalid inputs.

### HTTP Methods (7 checks)

| Check Name | Input Sent | Pass Condition |
|------------|------------|----------------|
| `method:GET` | GET request to an endpoint | HTTP 2xx response |
| `method:POST` | POST request to an endpoint | HTTP 2xx response |
| `method:PUT` | PUT request to an endpoint | HTTP 2xx response |
| `method:PATCH` | PATCH request to an endpoint | HTTP 2xx response |
| `method:DELETE` | DELETE request to an endpoint | HTTP 2xx response |
| `method:HEAD` | HEAD request to an endpoint | HTTP 2xx response |
| `method:OPTIONS` | OPTIONS request to an endpoint | HTTP 2xx response |

### Content Types (1 check)

| Check Name | What It Validates | Pass Condition |
|------------|-------------------|----------------|
| `content:negotiation` | Response supports multiple content types | HTTP 2xx response |

### Security (3 checks)

| Check Name | Input Sent | Pass Condition |
|------------|------------|----------------|
| `security:bearer` | `Authorization: Bearer <token>` header | HTTP 2xx (not 401/403) |
| `security:apikey` | API key in header/query/cookie per spec | HTTP 2xx (not 401/403) |
| `security:basic` | `Authorization: Basic <base64>` header | HTTP 2xx (not 401/403) |

Use `--conformance-api-key`, `--conformance-basic-auth`, or `--conformance-header` to provide real credentials.

### Response Validation (1 check)

| Check Name | What It Validates | Pass Condition |
|------------|-------------------|----------------|
| `response:schema:validation` | Full response body structure against OpenAPI schema | All declared fields present with correct types |

This performs deep structural validation of the response JSON against the schema declared in the spec. It checks field names, types, and nested structures.

## CLI Reference

```
mockforge bench --conformance [OPTIONS]
```

| Flag | Description | Default |
|------|-------------|---------|
| `--spec <FILE>` | OpenAPI spec to analyze | (required for spec-driven mode) |
| `--target <URL>` | Target server base URL | (required) |
| `--conformance-categories <LIST>` | Comma-separated categories to test | All categories |
| `--conformance-all-operations` | Test every endpoint, not just representatives | Off (one per feature) |
| `--conformance-api-key <KEY>` | API key for security scheme tests | None |
| `--conformance-basic-auth <USER:PASS>` | Basic auth credentials | None |
| `--conformance-header <HEADER>` | Custom header (repeatable) `"Name: Value"` | None |
| `--conformance-report <FILE>` | Output report path | `conformance-report.json` |
| `--conformance-report-format <FMT>` | `json` or `sarif` | `json` |

## Running Specific Categories

Use `--conformance-categories` to run only the categories you care about:

```bash
# Only structural checks
mockforge bench --conformance \
  --spec my-api.json \
  --target http://localhost:3000 \
  --conformance-categories "parameters,http-methods,request-bodies,constraints,security,content-types"

# Only behavioral checks
mockforge bench --conformance \
  --spec my-api.json \
  --target http://localhost:3000 \
  --conformance-categories "response-codes,schema-types,string-formats,composition,response-validation"

# Just one category
mockforge bench --conformance \
  --spec my-api.json \
  --target http://localhost:3000 \
  --conformance-categories "security"
```

Valid category names: `parameters`, `request-bodies`, `schema-types`, `composition`, `string-formats`, `constraints`, `response-codes`, `http-methods`, `content-types`, `security`, `response-validation`

## Testing All Operations

By default, the conformance test picks **one representative operation** per feature. For example, if your spec has 100 GET endpoints, only one is tested for the `method:GET` check.

Use `--conformance-all-operations` to test **every** endpoint:

```bash
mockforge bench --conformance \
  --spec my-api.json \
  --target http://localhost:3000 \
  --conformance-all-operations
```

In this mode, check names become path-qualified:
- Default: `method:GET` (1 check)
- All-operations: `method:GET:/users`, `method:GET:/orders`, `method:GET:/products` (N checks)

This makes it easy to identify exactly which endpoints fail.

## Authentication

When testing a real API that requires authentication:

```bash
# Bearer token
mockforge bench --conformance \
  --spec my-api.json \
  --target https://api.example.com \
  --conformance-header "Authorization: Bearer eyJhbGciOi..."

# API key in header
mockforge bench --conformance \
  --spec my-api.json \
  --target https://api.example.com \
  --conformance-api-key "your-api-key"

# Basic auth
mockforge bench --conformance \
  --spec my-api.json \
  --target https://api.example.com \
  --conformance-basic-auth "admin:password123"

# Multiple custom headers
mockforge bench --conformance \
  --spec my-api.json \
  --target https://api.example.com \
  --conformance-header "Authorization: Bearer token" \
  --conformance-header "X-Tenant-ID: my-tenant" \
  --conformance-header "Cookie: session=abc123"
```

## Reading the Report

### Terminal Output

The terminal shows a colored table:

```
OpenAPI 3.0.0 Conformance Report
----------------------------------------------------------------
Category              Passed   Failed    Total     Rate
----------------------------------------------------------------
Parameters                 7        0        7     100%
Request Bodies             2        1        3      67%
HTTP Methods               5        0        5     100%
Response Codes             3        2        5      60%
...
----------------------------------------------------------------
Total:                    30        5       35      86%

Failed Checks:
  body:multipart (0 passed, 1 failed)
  response:400 (0 passed, 1 failed)
  response:404 (1 passed, 1 failed)
```

- **Green (100%)**: Category fully passes
- **Yellow (>=80%)**: Mostly passing
- **Red (<80%)**: Needs attention

### JSON Report

The `conformance-report.json` file contains per-check results:

```json
{
  "checks": {
    "param:path:string": { "passes": 1, "fails": 0 },
    "method:GET": { "passes": 1, "fails": 0 },
    "response:200": { "passes": 1, "fails": 0 },
    "response:404": { "passes": 0, "fails": 1 },
    "body:json": { "passes": 1, "fails": 0 }
  }
}
```

In all-operations mode, check names include the path:
```json
{
  "checks": {
    "method:GET:/users": { "passes": 1, "fails": 0 },
    "method:GET:/orders": { "passes": 1, "fails": 0 },
    "response:200:/users": { "passes": 1, "fails": 0 },
    "response:200:/orders/{id}": { "passes": 0, "fails": 1 }
  }
}
```

### SARIF Report

For CI/CD integration with GitHub Code Scanning or VS Code SARIF Viewer:

```bash
mockforge bench --conformance \
  --spec my-api.json \
  --target http://localhost:3000 \
  --conformance-report-format sarif \
  --conformance-report conformance.sarif
```

## Understanding Common Failures

### `response:200` fails against a real server

The conformance test uses placeholder values for path parameters (e.g., `test-value` for strings, `42` for integers). If your API requires valid UUIDs or IDs that exist in the database, the test will get 404 or 400 instead of 200.

**Fix**: This is expected behavior for real servers with placeholder data. Focus on structural checks (Parameters, HTTP Methods) which verify routing works, not response correctness.

### `response:404` fails against MockForge

The `response:404` check sends `X-Mockforge-Response-Status: 404` to ask MockForge to return a 404. This only works when the OpenAPI spec declares a `404` response for that operation. If the spec doesn't declare one, MockForge returns its default response.

**Fix**: Add `404` responses to your OpenAPI spec for operations that should support it.

### `response:schema:validation` fails

The response body doesn't match the declared schema structure. This can happen when:
- The server returns an error response instead of the expected success response
- The mock generator doesn't fully replicate nested object structures
- The spec uses complex schemas (deeply nested objects, polymorphism)

**Fix**: Use `--conformance-all-operations` to identify which specific endpoints have schema mismatches.

### `security:bearer` or `security:apikey` fails

The test sends placeholder credentials. Without real credentials, a real server returns 401/403.

**Fix**: Provide real credentials via `--conformance-api-key`, `--conformance-basic-auth`, or `--conformance-header`.

## Inspecting the Generated k6 Script

The generated k6 script is saved to `output/k6-conformance.js`. You can inspect it to see exactly what requests are sent:

```bash
# Generate the script and inspect before running
mockforge bench --conformance \
  --spec my-api.json \
  --target http://localhost:3000

# View the generated script
cat output/k6-conformance.js
```

You can also run the generated script manually with k6 for debugging:

```bash
k6 run output/k6-conformance.js --verbose
```

## Example Workflow

```bash
# 1. Start MockForge with your spec
mockforge serve --spec my-api.json --http-port 3000 --admin --admin-port 9080

# 2. Run full conformance test against MockForge
mockforge bench --conformance \
  --spec my-api.json \
  --target http://localhost:3000

# 3. Run only structural checks
mockforge bench --conformance \
  --spec my-api.json \
  --target http://localhost:3000 \
  --conformance-categories "parameters,http-methods,request-bodies,constraints,security"

# 4. Drill into failures with all-operations mode
mockforge bench --conformance \
  --spec my-api.json \
  --target http://localhost:3000 \
  --conformance-all-operations \
  --conformance-categories "response-codes"

# 5. Test against your real API server with auth
mockforge bench --conformance \
  --spec my-api.json \
  --target https://api.example.com \
  --conformance-header "Authorization: Bearer YOUR_TOKEN" \
  --conformance-categories "parameters,http-methods"
```
