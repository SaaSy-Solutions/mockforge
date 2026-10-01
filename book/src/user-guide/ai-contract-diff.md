# AI Contract Diff

AI Contract Diff automatically detects and analyzes differences between API contracts (OpenAPI specifications) and live requests. It provides contextual recommendations for mismatches and generates correction proposals to keep your contracts in sync with reality.

## Overview

AI Contract Diff helps you:

- **Detect Contract Drift**: Find discrepancies between your OpenAPI spec and actual API usage
- **Get AI-Powered Recommendations**: Understand why mismatches occur and how to fix them
- **Generate Correction Patches**: Automatically create JSON Patch files to update your specs
- **Integrate with CI/CD**: Automatically verify contracts in your pipeline
- **Visualize Mismatches**: Dashboard visualization of contract differences

## Quick Start

### Analyze a Request

```bash
# Export a capture from the running server (captures live only in its memory)
curl -s http://localhost:3000/__mockforge/api/contract-diff/captures/<capture-id> | jq .request > request.json

# Analyze the request against an OpenAPI spec
mockforge contract-diff analyze \
  --spec api.yaml \
  --request-path request.json
```

The CLI reads requests from a file. Captures are held only in the running
server's memory, so there is no `--capture-id` option; export a capture as
shown above, or analyze a capture in place on the server with
`POST /__mockforge/api/contract-diff/analyze/{id}`.

### Compare Two Specs

```bash
# Compare two OpenAPI specifications
mockforge contract-diff compare \
  --old-spec api-v1.yaml \
  --new-spec api-v2.yaml
```

### Generate Correction Patch

```bash
# Generate JSON Patch file for corrections
mockforge contract-diff generate-patch \
  --spec api.yaml \
  --request-path request.json \
  --output patch.json
```

## How It Works

### 1. Request Capture

MockForge automatically captures requests for contract analysis:

```yaml
# config.yaml
core:
  contract_diff:
    enabled: true
    auto_capture: true
    capture_all: false  # Only capture mismatches
```

### 2. Contract Analysis

When a request is captured, it's analyzed against your OpenAPI specification:

- **Path Matching**: Verify request path matches spec
- **Method Validation**: Check HTTP method is defined
- **Header Validation**: Compare request headers with spec
- **Query Parameter Validation**: Verify query params match
- **Body Validation**: Validate request body against schema

### 3. Mismatch Detection

The analyzer identifies several types of mismatches:

- **Missing Endpoint**: Request path not in spec
- **Invalid Method**: HTTP method not allowed
- **Missing Header**: Required header not present
- **Invalid Parameter**: Query param doesn't match spec
- **Schema Mismatch**: Request body doesn't match schema
- **Type Mismatch**: Value type doesn't match spec

### 4. AI Recommendations

AI-powered recommendations explain mismatches:

```json
{
  "mismatch": {
    "type": "missing_field",
    "field": "email",
    "location": "request.body"
  },
  "recommendation": {
    "message": "The 'email' field is required but missing from the request. Add it to the request body or mark it as optional in the schema.",
    "confidence": 0.95,
    "suggested_fix": "Add 'email' field to request body or update schema to make it optional"
  }
}
```

### 5. Correction Proposals

Generate JSON Patch files to fix mismatches:

```json
[
  {
    "op": "add",
    "path": "/paths/~1users/post/requestBody/content/application~1json/schema/required",
    "value": ["email"]
  }
]
```

## Configuration

### Basic Configuration

```yaml
core:
  contract_diff:
    enabled: true
    auto_capture: true
    capture_all: false
    spec_path: "./api.yaml"
```

### AI Provider Configuration

```yaml
core:
  contract_diff:
    ai_provider: "ollama"  # or "openai", "anthropic"
    ai_model: "llama3.2"
    ai_base_url: "http://localhost:11434"
    ai_api_key: "${AI_API_KEY}"  # For OpenAI/Anthropic
```

### Webhook Configuration

```yaml
core:
  contract_diff:
    webhooks:
      - url: "https://example.com/webhook"
        events: ["mismatch", "high_severity"]
        secret: "${WEBHOOK_SECRET}"
```

## CLI Commands

### Analyze Request

```bash
# Analyze a request from a file (see Quick Start for exporting a capture)
mockforge contract-diff analyze \
  --spec api.yaml \
  --request-path request.json

# With AI recommendations
mockforge contract-diff analyze \
  --spec api.yaml \
  --request-path request.json \
  --llm-provider ollama \
  --llm-model llama3.2
```

### Compare Specs

```bash
# Compare two OpenAPI specs
mockforge contract-diff compare \
  --old-spec api-v1.yaml \
  --new-spec api-v2.yaml

# Output to file
mockforge contract-diff compare \
  --old-spec api-v1.yaml \
  --new-spec api-v2.yaml \
  --output diff.json
```

### Generate Patch

```bash
# Generate correction patch
mockforge contract-diff generate-patch \
  --spec api.yaml \
  --request-path request.json \
  --output patch.json
```

`generate-patch` always writes the patch to the `--output` file; use `apply-patch` (below) to apply it.

### Apply Patch

```bash
# Apply patch to spec
mockforge contract-diff apply-patch \
  --spec api.yaml \
  --patch patch.json \
  --output api-updated.yaml
```

## API Endpoints

### Upload Request

```http
POST /__mockforge/contract-diff/upload
Content-Type: application/json

{
  "method": "POST",
  "path": "/users",
  "headers": {"Content-Type": "application/json"},
  "query_params": {},
  "body": {"name": "Alice", "email": "alice@example.com"}
}
```

### Get Captured Requests

```http
GET /__mockforge/contract-diff/captures?limit=10&offset=0
```

### Analyze Request

```http
POST /__mockforge/contract-diff/captures/{id}/analyze
Content-Type: application/json

{
  "spec_path": "./api.yaml"
}
```

### Generate Patch

```http
POST /__mockforge/contract-diff/captures/{id}/patch
Content-Type: application/json

{
  "spec_path": "./api.yaml"
}
```

### Get Statistics

```http
GET /__mockforge/contract-diff/statistics
```

## Dashboard

The Contract Diff dashboard provides:

- **Statistics Overview**: Total captures, analyzed requests, mismatch counts
- **Captured Requests List**: Browse and filter captured requests
- **Analysis Results**: View mismatches, recommendations, and confidence scores
- **Patch Generation**: Generate and download correction patches

Access via: **Admin UI → Contract Diff**

## CI/CD Integration

### GitHub Actions

```yaml
name: Contract Diff Analysis

on:
  pull_request:
    paths:
      - 'api.yaml'
      - '**/*.yaml'

jobs:
  contract-diff:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Analyze contracts
        run: |
          mockforge contract-diff analyze \
            --spec api.yaml \
            --request-path tests/contract/request.json
      
      - name: Generate patch
        run: |
          mockforge contract-diff generate-patch \
            --spec api.yaml \
            --request-path tests/contract/request.json \
            --output patch.json
      
      - name: Upload patch
        uses: actions/upload-artifact@v3
        with:
          name: contract-patch
          path: patch.json
```

### GitLab CI

```yaml
contract-diff:
  script:
    - mockforge contract-diff analyze --spec api.yaml --request-path tests/contract/request.json
    - mockforge contract-diff generate-patch --spec api.yaml --request-path tests/contract/request.json --output patch.json
  artifacts:
    paths:
      - patch.json
```

## Use Cases

### Contract Validation

Ensure your API spec matches actual usage:

```bash
# Analyze the most recent captured requests against the spec the server was started with
curl -X POST "http://localhost:3000/__mockforge/api/contract-diff/analyze?limit=50"
```

### Spec Maintenance

Keep specs up-to-date automatically:

```bash
# Generate a patch for a request's mismatches
mockforge contract-diff generate-patch \
  --spec api.yaml \
  --request-path request.json \
  --output patches/patch-1.json

# Review and apply patches
mockforge contract-diff apply-patch \
  --spec api.yaml \
  --patch patches/patch-1.json \
  --output api-updated.yaml
```

### API Versioning

Compare API versions:

```bash
# Compare v1 and v2
mockforge contract-diff compare \
  --old-spec api-v1.yaml \
  --new-spec api-v2.yaml \
  --output version-diff.json
```

## Best Practices

1. **Enable Auto-Capture**: Automatically capture requests for analysis
2. **Regular Analysis**: Run analysis regularly to catch drift early
3. **Review Recommendations**: Always review AI recommendations before applying
4. **Version Control Patches**: Commit patches to version control
5. **CI/CD Integration**: Automate contract validation in your pipeline

## Troubleshooting

### No Mismatches Detected

- Verify OpenAPI spec is valid
- Check that request path matches spec
- Ensure method is defined in spec

### AI Recommendations Not Available

- Check AI provider is configured
- Verify API key is set (for OpenAI/Anthropic)
- Ensure Ollama is running (for local provider)

### Patch Generation Fails

- Verify spec path is correct
- Check that mismatches exist
- Review patch generation logs

## Related Documentation

- [OpenAPI Integration](http-mocking/openapi.md) - Working with OpenAPI specs
- [Configuration Guide](../configuration/files.md) - Complete configuration reference
- [CI/CD Integration](../contributing/release.md) - Pipeline integration

