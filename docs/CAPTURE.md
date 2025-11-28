# Capture Scrubbing & Deterministic Replay

MockForge's capture system includes powerful features for scrubbing sensitive data and ensuring deterministic, diff-friendly recordings that are safe to commit to version control.

## Table of Contents

- [Overview](#overview)
- [Environment Variables](#environment-variables)
- [Scrubbing Rules](#scrubbing-rules)
- [Capture Filtering](#capture-filtering)
- [Deterministic Mode](#deterministic-mode)
- [Examples](#examples)
- [Best Practices](#best-practices)

## Overview

When recording API traffic, you often need to:
- **Redact sensitive data** (emails, API keys, passwords, IP addresses)
- **Normalize non-deterministic values** (timestamps, UUIDs, random IDs)
- **Filter captured requests** (only errors, specific paths, sample rates)
- **Create reproducible test fixtures** (deterministic output for version control)

MockForge's scrubbing and filtering features address all of these needs.

## Environment Variables

### `MOCKFORGE_CAPTURE_SCRUB`

JSON configuration for scrubbing rules. Defines what data should be redacted or normalized.

```bash
export MOCKFORGE_CAPTURE_SCRUB='[
  {"type": "email", "replacement": "user@example.com"},
  {"type": "uuid", "replacement": "00000000-0000-0000-0000-{{counter:012}}"},
  {"type": "regex", "pattern": "sk-[a-zA-Z0-9]{46}", "replacement": "sk-REDACTED", "target": "all"}
]'
```

### `MOCKFORGE_CAPTURE_FILTER`

JSON configuration for selective recording. Controls which requests are captured.

```bash
export MOCKFORGE_CAPTURE_FILTER='{
  "errors_only": true,
  "status_codes": [500, 502, 503, 504],
  "path_patterns": ["^/api/v1/.*"],
  "exclude_paths": ["/health", "/metrics"],
  "methods": ["POST", "PUT", "DELETE"],
  "sample_rate": 0.1
}'
```

### `MOCKFORGE_CAPTURE_DETERMINISTIC`

Enable deterministic mode for reproducible recordings.

```bash
export MOCKFORGE_CAPTURE_DETERMINISTIC=true
```

## Scrubbing Rules

### Built-in Rule Types

#### 1. Email Scrubbing

Redacts all email addresses.

```json
{
  "type": "email",
  "replacement": "user@example.com"
}
```

**Before:**
```json
{"user": "john.doe@company.com", "admin": "admin@secret.org"}
```

**After:**
```json
{"user": "user@example.com", "admin": "user@example.com"}
```

#### 2. UUID Scrubbing

Replaces UUIDs with deterministic counters.

```json
{
  "type": "uuid",
  "replacement": "00000000-0000-0000-0000-{{counter:012}}"
}
```

**Before:**
```json
{"id": "123e4567-e89b-12d3-a456-426614174000", "session": "987f6543-e21c-43d2-b456-426614174111"}
```

**After:**
```json
{"id": "00000000-0000-0000-0000-000000000000", "session": "00000000-0000-0000-0000-000000000001"}
```

The `{{counter}}` placeholder is replaced with an incrementing counter. Use `{{counter:012}}` for zero-padded 12-digit counters.

#### 3. IP Address Scrubbing

Redacts IPv4 addresses.

```json
{
  "type": "ipaddress",
  "replacement": "127.0.0.1"
}
```

**Before:**
```json
{"client_ip": "192.168.1.100", "server_ip": "10.0.0.5"}
```

**After:**
```json
{"client_ip": "127.0.0.1", "server_ip": "127.0.0.1"}
```

#### 4. Credit Card Scrubbing

Redacts credit card numbers.

```json
{
  "type": "creditcard",
  "replacement": "XXXX-XXXX-XXXX-XXXX"
}
```

#### 5. Regex Scrubbing

Use custom regular expressions for flexible pattern matching.

```json
{
  "type": "regex",
  "pattern": "sk-[a-zA-Z0-9]{46}",
  "replacement": "sk-REDACTED",
  "target": "all"
}
```

**Targets:**
- `"all"` - Scrub in both headers and body (default)
- `"headers"` - Only scrub in headers
- `"body"` - Only scrub in body

#### 6. JSON Field Scrubbing

Scrub specific JSON fields by path.

```json
{
  "type": "field",
  "field": "user.email",
  "replacement": "redacted@example.com",
  "target": "all"
}
```

**Before:**
```json
{"user": {"email": "secret@company.com", "name": "John", "ssn": "123-45-6789"}}
```

**After (with field="user.email"):**
```json
{"user": {"email": "redacted@example.com", "name": "John", "ssn": "123-45-6789"}}
```

Supports nested paths with dot notation (e.g., `"user.profile.email"`).

#### 7. Header Scrubbing

Scrub specific HTTP headers (case-insensitive).

```json
{
  "type": "header",
  "name": "Authorization",
  "replacement": "Bearer REDACTED"
}
```

**Before:**
```
Authorization: Bearer secret-token-12345
X-API-Key: super-secret-key
```

**After:**
```
Authorization: Bearer REDACTED
X-API-Key: super-secret-key
```

### Combining Multiple Rules

You can combine multiple scrubbing rules:

```bash
export MOCKFORGE_CAPTURE_SCRUB='[
  {"type": "email", "replacement": "user@example.com"},
  {"type": "uuid", "replacement": "00000000-0000-0000-0000-{{counter:012}}"},
  {"type": "ipaddress", "replacement": "127.0.0.1"},
  {"type": "header", "name": "Authorization", "replacement": "Bearer REDACTED"},
  {"type": "header", "name": "X-API-Key", "replacement": "REDACTED"},
  {"type": "field", "field": "user.ssn", "replacement": "XXX-XX-XXXX", "target": "body"},
  {"type": "regex", "pattern": "password\":\\s*\"[^\"]+\"", "replacement": "password\": \"REDACTED\"", "target": "body"}
]'
```

## Capture Filtering

### Filter Options

#### Status Code Filter

Only capture requests with specific status codes:

```json
{
  "status_codes": [500, 502, 503, 504]
}
```

This captures only 5xx server errors.

#### Errors Only

Capture only error responses (status >= 400):

```json
{
  "errors_only": true
}
```

#### Path Patterns

Use regex to match specific paths:

```json
{
  "path_patterns": ["^/api/v1/.*", "^/internal/.*"]
}
```

Only requests matching these patterns are captured.

#### Exclude Paths

Exclude specific paths from capture:

```json
{
  "exclude_paths": ["/health", "/metrics", "/readiness"]
}
```

Health check and monitoring endpoints are often excluded.

#### Method Filter

Only capture specific HTTP methods:

```json
{
  "methods": ["POST", "PUT", "DELETE", "PATCH"]
}
```

For example, only capture state-changing operations.

#### Sample Rate

Capture only a percentage of requests:

```json
{
  "sample_rate": 0.1
}
```

This captures 10% of requests (useful for high-traffic APIs). The value ranges from 0.0 (0%) to 1.0 (100%).

### Combining Filters

Filters can be combined with AND logic:

```bash
export MOCKFORGE_CAPTURE_FILTER='{
  "errors_only": true,
  "path_patterns": ["^/api/v1/.*"],
  "exclude_paths": ["/api/v1/health"],
  "methods": ["POST", "PUT", "DELETE"],
  "sample_rate": 0.5
}'
```

This configuration captures:
- Only error responses (>= 400)
- Only from /api/v1/* paths
- Excluding /api/v1/health
- Only POST, PUT, DELETE methods
- At a 50% sample rate

## Deterministic Mode

Enable deterministic mode to ensure recordings are reproducible:

```bash
export MOCKFORGE_CAPTURE_DETERMINISTIC=true
```

When enabled, deterministic mode:
- **Normalizes timestamps** to the start of day (00:00:00)
- **Uses deterministic UUID replacement** with incrementing counters
- **Ensures consistent ordering** of data

This makes recordings diff-friendly and suitable for version control.

### Deterministic Example

```bash
export MOCKFORGE_CAPTURE_SCRUB='[
  {"type": "uuid", "replacement": "00000000-0000-0000-0000-{{counter:012}}"}
]'
export MOCKFORGE_CAPTURE_DETERMINISTIC=true
```

**Request 1 at 2024-01-15 14:32:18:**
```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "timestamp": "2024-01-15T14:32:18Z"
}
```

**Recorded as:**
```json
{
  "id": "00000000-0000-0000-0000-000000000000",
  "timestamp": "2024-01-15T00:00:00Z"
}
```

## Examples

### Example 1: Production API Key Scrubbing

```bash
# Scrub API keys and emails
export MOCKFORGE_CAPTURE_SCRUB='[
  {"type": "regex", "pattern": "sk-[a-zA-Z0-9]{46}", "replacement": "sk-REDACTED"},
  {"type": "regex", "pattern": "Bearer [a-zA-Z0-9._-]+", "replacement": "Bearer REDACTED"},
  {"type": "email", "replacement": "user@example.com"},
  {"type": "header", "name": "X-API-Key", "replacement": "REDACTED"}
]'

# Start MockForge
mockforge serve --config config.yaml
```

### Example 2: Error-Only Recording

```bash
# Only record 5xx errors for debugging
export MOCKFORGE_CAPTURE_FILTER='{
  "status_codes": [500, 502, 503, 504]
}'

mockforge serve --config config.yaml
```

### Example 3: Test Fixture Generation

```bash
# Generate deterministic test fixtures
export MOCKFORGE_CAPTURE_SCRUB='[
  {"type": "uuid", "replacement": "00000000-0000-0000-0000-{{counter:012}}"},
  {"type": "email", "replacement": "test@example.com"},
  {"type": "field", "field": "user.id", "replacement": "USER_{{counter}}"}
]'
export MOCKFORGE_CAPTURE_DETERMINISTIC=true
export MOCKFORGE_CAPTURE_FILTER='{
  "path_patterns": ["^/api/v1/users.*"],
  "methods": ["POST", "PUT"]
}'

mockforge serve --config config.yaml
```

### Example 4: PII Redaction

```bash
# Comprehensive PII scrubbing
export MOCKFORGE_CAPTURE_SCRUB='[
  {"type": "email", "replacement": "user@example.com"},
  {"type": "creditcard", "replacement": "XXXX-XXXX-XXXX-XXXX"},
  {"type": "ipaddress", "replacement": "127.0.0.1"},
  {"type": "field", "field": "ssn", "replacement": "XXX-XX-XXXX"},
  {"type": "field", "field": "phone", "replacement": "555-0100"},
  {"type": "field", "field": "address", "replacement": "123 Main St"},
  {"type": "regex", "pattern": "\\b\\d{3}-\\d{2}-\\d{4}\\b", "replacement": "XXX-XX-XXXX"}
]'

mockforge serve --config config.yaml
```

### Example 5: High-Traffic Sampling

```bash
# Capture 1% of traffic, errors only
export MOCKFORGE_CAPTURE_FILTER='{
  "errors_only": true,
  "sample_rate": 0.01
}'

mockforge serve --config config.yaml
```

## Best Practices

### 1. Start with Built-in Rules

Use built-in rules (`email`, `uuid`, `ipaddress`, `creditcard`) when possible. They're well-tested and performant.

### 2. Test Scrubbing Rules

Always test your scrubbing rules on sample data before using in production:

```bash
# Test with a single request
curl -X POST http://localhost:3000/api/test \
  -H "Authorization: Bearer secret-token" \
  -d '{"email": "test@company.com", "id": "123e4567-e89b-12d3-a456-426614174000"}'

# Check the recorded data
curl http://localhost:3000/api/recorder/requests | jq .
```

### 3. Layer Scrubbing Rules

Apply multiple layers of scrubbing for defense in depth:

```json
[
  {"type": "header", "name": "Authorization", "replacement": "Bearer REDACTED"},
  {"type": "field", "field": "access_token", "replacement": "REDACTED"},
  {"type": "regex", "pattern": "Bearer [a-zA-Z0-9._-]+", "replacement": "Bearer REDACTED"}
]
```

### 4. Use Deterministic Mode for Tests

Always use deterministic mode when generating test fixtures:

```bash
export MOCKFORGE_CAPTURE_DETERMINISTIC=true
```

This ensures fixtures are reproducible and can be committed to git.

### 5. Filter Aggressively

Don't record everything. Use filters to reduce noise:

```json
{
  "exclude_paths": ["/health", "/metrics", "/readiness", "/liveness"],
  "sample_rate": 0.1
}
```

### 6. Validate Regex Patterns

Test regex patterns separately before adding them to scrub rules:

```bash
# Test in your shell first
echo '{"api_key": "sk-abcdefghijklmnopqrstuvwxyz0123456789ABCDEF"}' | \
  sed -E 's/sk-[a-zA-Z0-9]{46}/sk-REDACTED/g'
```

### 7. Document Your Rules

Add comments to your configuration files explaining why each rule exists:

```yaml
# config.yaml
recorder:
  scrub_rules:
    # Remove OAuth tokens to prevent credential leakage
    - type: regex
      pattern: "Bearer [a-zA-Z0-9._-]+"
      replacement: "Bearer REDACTED"

    # Normalize UUIDs for deterministic testing
    - type: uuid
      replacement: "00000000-0000-0000-0000-{{counter:012}}"
```

### 8. Monitor Scrubbing Performance

For high-volume APIs, monitor the performance impact of scrubbing:

```bash
# Check recorder stats
curl http://localhost:3000/api/recorder/stats
```

Complex regex patterns can impact performance. Optimize or simplify if needed.

### 9. Version Control Friendly

When generating fixtures for version control:

```bash
# Enable deterministic mode
export MOCKFORGE_CAPTURE_DETERMINISTIC=true

# Scrub all non-deterministic values
export MOCKFORGE_CAPTURE_SCRUB='[
  {"type": "uuid", "replacement": "00000000-0000-0000-0000-{{counter:012}}"},
  {"type": "email", "replacement": "user@example.com"},
  {"type": "ipaddress", "replacement": "127.0.0.1"}
]'

# Record, then commit
mockforge serve --config config.yaml
# ... run tests ...
git add fixtures/
git commit -m "Add test fixtures"
```

### 10. Security Audit

Regularly audit your scrubbing rules to ensure they cover all sensitive data:

```bash
# Review a sample of recordings
curl http://localhost:3000/api/recorder/requests?limit=10 | \
  jq '.[] | {path, headers, body}' | \
  grep -E "(password|secret|key|token|ssn|email)"
```

If you find sensitive data, add scrubbing rules immediately.

---

## Additional Resources

- [Recorder API Documentation](./RECORDER_API.md)
- [MockForge Configuration Guide](./CONFIGURATION.md)
- [Example Configurations](../examples/)

## Troubleshooting

### Scrubbing Not Working

**Check environment variables:**
```bash
echo $MOCKFORGE_CAPTURE_SCRUB
echo $MOCKFORGE_CAPTURE_DETERMINISTIC
```

**Verify JSON syntax:**
```bash
echo $MOCKFORGE_CAPTURE_SCRUB | jq .
```

### Regex Not Matching

**Test regex separately:**
```bash
echo "sk-abcdefghijklmnopqrstuvwxyz0123456789ABCDEF" | grep -E "sk-[a-zA-Z0-9]{46}"
```

**Check target location:**
Ensure `"target": "all"` or the appropriate target (`"headers"` or `"body"`).

### Filter Not Excluding Paths

**Verify regex pattern:**
```bash
echo "/health" | grep -E "/health"  # Should match
echo "/api/health" | grep -E "^/health$"  # Won't match (anchored)
```

Use appropriate anchors (`^` for start, `$` for end).

### Performance Issues

**Reduce regex complexity:**
Use simpler patterns or built-in rules when possible.

**Lower sample rate:**
```json
{"sample_rate": 0.01}
```

**Exclude more paths:**
```json
{"exclude_paths": ["/health", "/metrics", "/static/.*"]}
```
