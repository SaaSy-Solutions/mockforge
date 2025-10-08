# AI-Powered Test Generation from Recordings

MockForge can automatically generate test cases from recorded API interactions, making it easy to create comprehensive test suites from real API traffic.

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Quick Start](#quick-start)
- [CLI Usage](#cli-usage)
- [API Usage](#api-usage)
- [Test Formats](#test-formats)
- [AI-Powered Descriptions](#ai-powered-descriptions)
- [Advanced Configuration](#advanced-configuration)
- [Examples](#examples)
- [Best Practices](#best-practices)

## Overview

The test generation feature analyzes recorded API requests and responses to automatically create:

- **Executable test code** in multiple languages and frameworks
- **Assertions** for status codes, response bodies, headers, and timing
- **AI-generated test descriptions** that explain what each test validates
- **Organized test suites** grouped by endpoint

This dramatically reduces the time required to create comprehensive test coverage and ensures tests match real production traffic patterns.

## Features

### Supported Test Formats

- **Rust** (`rust_reqwest`) - Tokio async tests using reqwest
- **HTTP Files** (`http_file`) - JetBrains HTTP Client format
- **cURL** (`curl`) - Shell scripts with curl commands
- **Postman** (`postman`) - Postman collection JSON
- **k6** (`k6`) - Load testing scripts
- **Python** (`python_pytest`) - pytest test functions
- **JavaScript** (`javascript_jest`) - Jest test cases
- **Go** (`go_test`) - Go testing package

### Key Capabilities

- ✅ **Automatic test generation** from recorded traffic
- ✅ **Multiple output formats** for different testing tools
- ✅ **Filtering** by protocol, method, path, status code
- ✅ **AI-powered test descriptions** using LLM
- ✅ **Validation assertions** for body, status, headers, timing
- ✅ **Grouped test suites** organized by endpoint
- ✅ **Base URL substitution** for different environments

## Quick Start

### 1. Record API Traffic

First, start MockForge with recording enabled:

```bash
mockforge serve --recorder --recorder-db ./recordings.db
```

Make API requests to your mock server:

```bash
curl http://localhost:3000/api/users
curl -X POST http://localhost:3000/api/users -d '{"name":"John"}'
```

### 2. Generate Tests

Generate Rust tests from recordings:

```bash
mockforge generate-tests \
  --database ./recordings.db \
  --format rust_reqwest \
  --output tests/integration_tests.rs
```

### 3. Run Generated Tests

```bash
cargo test
```

That's it! You now have executable tests based on real API traffic.

## CLI Usage

### Basic Command

```bash
mockforge generate-tests [OPTIONS]
```

### Common Options

```bash
# Generate Python tests
mockforge generate-tests \
  --database ./recordings.db \
  --format python_pytest \
  --output tests/test_api.py

# Generate HTTP files for manual testing
mockforge generate-tests \
  --database ./recordings.db \
  --format http_file \
  --output requests.http

# Generate k6 load tests
mockforge generate-tests \
  --database ./recordings.db \
  --format k6 \
  --output loadtest.js
```

### Filtering Options

Filter which recordings to generate tests from:

```bash
# Only HTTP GET requests
mockforge generate-tests \
  --protocol http \
  --method GET \
  --limit 20

# Only successful requests
mockforge generate-tests \
  --status-code 200 \
  --limit 50

# Specific endpoint patterns
mockforge generate-tests \
  --path "/api/users/*" \
  --limit 30
```

### AI-Powered Descriptions

Generate intelligent test descriptions using LLM:

```bash
# Using Ollama (free, local)
mockforge generate-tests \
  --database ./recordings.db \
  --ai-descriptions \
  --llm-provider ollama \
  --llm-model llama2

# Using OpenAI
mockforge generate-tests \
  --database ./recordings.db \
  --ai-descriptions \
  --llm-provider openai \
  --llm-model gpt-3.5-turbo \
  --llm-api-key $OPENAI_API_KEY
```

### Validation Options

Control what validations are included:

```bash
mockforge generate-tests \
  --validate-body true \
  --validate-status true \
  --validate-headers true \
  --validate-timing true \
  --max-duration-ms 1000
```

### Full Options Reference

| Option | Description | Default |
|--------|-------------|---------|
| `--database, -d` | Path to recordings database | `./mockforge-recordings.db` |
| `--format, -f` | Test format | `rust_reqwest` |
| `--output, -o` | Output file path | stdout |
| `--protocol` | Filter by protocol (http, grpc, websocket, graphql) | all |
| `--method` | Filter by HTTP method | all |
| `--path` | Filter by path pattern | all |
| `--status-code` | Filter by status code | all |
| `--limit, -l` | Maximum tests to generate | 50 |
| `--suite-name` | Test suite name | `generated_tests` |
| `--base-url` | Base URL for tests | `http://localhost:3000` |
| `--ai-descriptions` | Use AI for test descriptions | false |
| `--llm-provider` | LLM provider (ollama, openai) | `ollama` |
| `--llm-model` | LLM model name | `llama2` |
| `--llm-endpoint` | LLM API endpoint | provider default |
| `--llm-api-key` | LLM API key | from env |
| `--validate-body` | Include body validation | true |
| `--validate-status` | Include status validation | true |
| `--validate-headers` | Include header validation | false |
| `--validate-timing` | Include timing validation | false |
| `--max-duration-ms` | Max duration threshold | none |

## API Usage

You can also generate tests via the Management API:

### Endpoint

```
POST /api/recorder/generate-tests
```

### Request Body

```json
{
  "format": "rust_reqwest",
  "protocol": "Http",
  "method": "GET",
  "limit": 50,
  "suite_name": "api_tests",
  "base_url": "http://localhost:3000",
  "ai_descriptions": false,
  "validate_body": true,
  "validate_status": true,
  "validate_headers": false,
  "validate_timing": false
}
```

### Response

```json
{
  "success": true,
  "metadata": {
    "suite_name": "api_tests",
    "test_count": 15,
    "endpoint_count": 8,
    "protocols": ["Http"],
    "format": "RustReqwest",
    "generated_at": "2025-10-07T10:30:00Z"
  },
  "tests": [
    {
      "name": "test_get_api_users",
      "description": "Verify that the user listing endpoint returns a valid array of user objects",
      "endpoint": "/api/users",
      "method": "GET"
    }
  ],
  "test_file": "// Generated test file\n..."
}
```

### Example: Generate Tests via API

```bash
curl -X POST http://localhost:3000/api/recorder/generate-tests \
  -H "Content-Type: application/json" \
  -d '{
    "format": "python_pytest",
    "protocol": "Http",
    "limit": 20,
    "base_url": "https://api.example.com"
  }' | jq -r '.test_file' > tests/test_api.py
```

### Example: AI-Powered Descriptions via API

```bash
curl -X POST http://localhost:3000/api/recorder/generate-tests \
  -H "Content-Type: application/json" \
  -d '{
    "format": "javascript_jest",
    "limit": 10,
    "ai_descriptions": true,
    "llm_config": {
      "provider": "ollama",
      "api_endpoint": "http://localhost:11434/api/generate",
      "model": "llama2",
      "temperature": 0.3
    }
  }' | jq -r '.test_file' > tests/api.test.js
```

## Test Formats

### Rust (reqwest)

Generates async Tokio tests using the reqwest HTTP client:

```rust
#[tokio::test]
async fn test_get_api_users() {
    let client = reqwest::Client::new();
    let response = client.get("http://localhost:3000/api/users")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status().as_u16(), 200);
    let body = response.text().await.expect("Failed to read body");
    let json: serde_json::Value = serde_json::from_str(&body).expect("Invalid JSON");
    assert!(json.is_object() || json.is_array());
}
```

**Use case**: Integration tests, CI/CD pipelines

### HTTP Files

Generates JetBrains HTTP Client format files:

```http
### GET /api/users
GET http://localhost:3000/api/users
Content-Type: application/json

### POST /api/users
POST http://localhost:3000/api/users
Content-Type: application/json

{"name":"John Doe","email":"john@example.com"}
```

**Use case**: Manual API testing, documentation

### cURL

Generates executable shell scripts with curl commands:

```bash
# GET /api/users
curl -X GET 'http://localhost:3000/api/users' \
  -H 'Content-Type: application/json'

# POST /api/users
curl -X POST 'http://localhost:3000/api/users' \
  -H 'Content-Type: application/json' \
  -d '{"name":"John Doe"}'
```

**Use case**: Quick manual testing, shell scripts

### Postman Collection

Generates Postman collection JSON:

```json
{
  "info": {
    "name": "generated_tests",
    "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
  },
  "item": [
    {
      "name": "GET /api/users",
      "request": {
        "method": "GET",
        "url": "http://localhost:3000/api/users"
      }
    }
  ]
}
```

**Use case**: API documentation, team collaboration

### k6 Load Tests

Generates k6 performance test scripts:

```javascript
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  vus: 10,
  duration: '30s',
};

export default function() {
  // GET /api/users
  {
    const params = {
      headers: {
        'Content-Type': 'application/json',
      },
    };
    const res = http.get('http://localhost:3000/api/users', null, params);
    check(res, {
      'status is 200': (r) => r.status === 200,
    });
  }
  sleep(1);
}
```

**Use case**: Load testing, performance validation

### Python (pytest)

Generates pytest test functions:

```python
def test_get_api_users():
    headers = {
        'Content-Type': 'application/json',
    }
    response = requests.get('http://localhost:3000/api/users', headers=headers)
    assert response.status_code == 200
```

**Use case**: Python projects, API testing

### JavaScript (Jest)

Generates Jest test cases:

```javascript
describe('generated_tests', () => {
  test('GET /api/users', async () => {
    const options = {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    };
    const response = await fetch('http://localhost:3000/api/users', options);
    expect(response.status).toBe(200);
    const data = await response.json();
    expect(data).toBeDefined();
  });
});
```

**Use case**: JavaScript/TypeScript projects, Node.js testing

### Go Test

Generates Go testing package tests:

```go
func TestGetApiUsers(t *testing.T) {
    req, err := http.NewRequest("GET", "http://localhost:3000/api/users", nil)
    if err != nil {
        t.Fatal(err)
    }
    req.Header.Set("Content-Type", "application/json")
    client := &http.Client{}
    resp, err := client.Do(req)
    if err != nil {
        t.Fatal(err)
    }
    defer resp.Body.Close()
    if resp.StatusCode != 200 {
        t.Errorf("Expected status 200, got %d", resp.StatusCode)
    }
}
```

**Use case**: Go projects, integration testing

## AI-Powered Descriptions

When `--ai-descriptions` is enabled, MockForge uses LLM to generate meaningful test descriptions instead of generic ones.

### Without AI

```rust
#[tokio::test]
async fn test_post_api_orders() {
    // Test POST /api/orders
    ...
}
```

### With AI

```rust
#[tokio::test]
async fn test_post_api_orders() {
    // Verify that creating a new order returns a 201 status
    // and includes the order ID in the response body
    ...
}
```

### Supported LLM Providers

#### Ollama (Free, Local)

```bash
# Install Ollama
curl https://ollama.ai/install.sh | sh

# Pull a model
ollama pull llama2

# Generate tests with AI descriptions
mockforge generate-tests \
  --ai-descriptions \
  --llm-provider ollama \
  --llm-model llama2
```

#### OpenAI

```bash
export OPENAI_API_KEY=sk-...

mockforge generate-tests \
  --ai-descriptions \
  --llm-provider openai \
  --llm-model gpt-3.5-turbo \
  --llm-api-key $OPENAI_API_KEY
```

#### Anthropic Claude

```bash
mockforge generate-tests \
  --ai-descriptions \
  --llm-provider openai \
  --llm-model claude-3-sonnet \
  --llm-endpoint https://api.anthropic.com/v1/messages \
  --llm-api-key $ANTHROPIC_API_KEY
```

## Advanced Configuration

### Custom Base URLs

Generate tests for different environments:

```bash
# Development
mockforge generate-tests --base-url http://localhost:3000 -o dev_tests.rs

# Staging
mockforge generate-tests --base-url https://staging.api.com -o staging_tests.rs

# Production
mockforge generate-tests --base-url https://api.example.com -o prod_tests.rs
```

### Combining Filters

Create targeted test suites:

```bash
# Only test user-related endpoints
mockforge generate-tests \
  --path "/api/users/*" \
  --path "/api/auth/*" \
  --status-code 200 \
  --limit 100

# Only test error cases
mockforge generate-tests \
  --status-code 400 \
  --status-code 404 \
  --status-code 500 \
  --format python_pytest
```

### Performance Tests

Generate load tests with timing validation:

```bash
mockforge generate-tests \
  --format k6 \
  --validate-timing \
  --max-duration-ms 500 \
  --output loadtest.js
```

## Examples

### Example 1: Generate Rust Integration Tests

```bash
# Record production traffic
mockforge serve --recorder

# Let it run for a while...

# Generate comprehensive test suite
mockforge generate-tests \
  --database ./mockforge-recordings.db \
  --format rust_reqwest \
  --limit 100 \
  --suite-name integration_tests \
  --validate-body true \
  --validate-status true \
  --output tests/integration.rs

# Add to Cargo.toml
# [dev-dependencies]
# reqwest = { version = "0.11", features = ["json"] }
# tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
# serde_json = "1.0"

# Run tests
cargo test
```

### Example 2: Generate Postman Collection

```bash
# Generate from recent recordings
mockforge generate-tests \
  --database ./recordings.db \
  --format postman \
  --limit 50 \
  --base-url https://api.example.com \
  --output api_collection.json

# Import into Postman:
# File > Import > api_collection.json
```

### Example 3: Generate Python Tests with AI

```bash
# Generate AI-powered pytest tests
mockforge generate-tests \
  --database ./recordings.db \
  --format python_pytest \
  --ai-descriptions \
  --llm-provider ollama \
  --llm-model llama2 \
  --limit 30 \
  --output tests/test_api.py

# Install dependencies
# pip install pytest requests

# Run tests
# pytest tests/test_api.py
```

### Example 4: Generate Load Tests

```bash
# Generate k6 load test
mockforge generate-tests \
  --database ./recordings.db \
  --format k6 \
  --protocol http \
  --method GET \
  --status-code 200 \
  --limit 20 \
  --output loadtest.js

# Install k6
# brew install k6 (macOS)
# choco install k6 (Windows)

# Run load test
# k6 run loadtest.js
```

## Best Practices

### 1. Filter Wisely

Don't generate tests for everything - focus on important endpoints:

```bash
# Good: Specific, targeted
mockforge generate-tests --path "/api/orders/*" --limit 20

# Bad: Too broad, creates noise
mockforge generate-tests --limit 1000
```

### 2. Use AI for Documentation

AI descriptions help document what each test validates:

```bash
mockforge generate-tests \
  --ai-descriptions \
  --llm-provider ollama \
  --format rust_reqwest
```

### 3. Separate Test Types

Generate different test files for different purposes:

```bash
# Integration tests
mockforge generate-tests --format rust_reqwest --status-code 200 -o integration.rs

# Error handling tests
mockforge generate-tests --format rust_reqwest --status-code 400 -o errors.rs

# Load tests
mockforge generate-tests --format k6 --status-code 200 -o loadtest.js
```

### 4. Update Base URLs

Generate environment-specific tests:

```bash
mockforge generate-tests --base-url $API_URL --output tests.rs
```

### 5. Review Generated Tests

Always review generated tests before committing:

```bash
# Generate to stdout first
mockforge generate-tests --limit 10

# Review and adjust
# Then output to file
mockforge generate-tests --limit 10 -o tests.rs
```

### 6. Combine with CI/CD

Automatically generate and run tests in your pipeline:

```yaml
# .github/workflows/api-tests.yml
- name: Record API traffic
  run: mockforge serve --recorder &

- name: Run application tests
  run: npm test

- name: Generate integration tests
  run: |
    mockforge generate-tests \
      --database ./mockforge-recordings.db \
      --format rust_reqwest \
      --output tests/generated.rs

- name: Run generated tests
  run: cargo test
```

## Next Steps

- [API Flight Recorder](./API_FLIGHT_RECORDER.md) - Learn more about recording
- [Observability](./OBSERVABILITY.md) - Integrate with monitoring
- [AI-Driven Mocking](./AI_DRIVEN_MOCKING.md) - AI features overview
