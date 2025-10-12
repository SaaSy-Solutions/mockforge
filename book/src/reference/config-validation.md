# Configuration Validation Guide

MockForge provides configuration validation to help you catch errors before starting the server. This guide explains how to validate your configuration and troubleshoot common issues.

## Quick Start

### Initialize a New Configuration

```bash
# Create a new project with template configuration
mockforge init my-project

# Or initialize in current directory
mockforge init .
```

This creates:
- `mockforge.yaml` - Main configuration file
- `examples/` - Example OpenAPI spec and data files (unless `--no-examples` is used)

### Validate Configuration

```bash
# Validate the current directory's config
mockforge config validate

# Validate a specific config file
mockforge config validate --config ./my-config.yaml

# Auto-discover config in parent directories
mockforge config validate
```

## What Gets Validated

MockForge's `config validate` command currently performs these checks:

### 1. File Existence
- Checks if the config file exists
- Auto-discovers `mockforge.yaml` or `mockforge.yml` in current and parent directories

### 2. YAML Syntax
- Validates YAML syntax and structure
- Reports parsing errors with line numbers

### 3. Basic Structure
- Counts HTTP endpoints
- Counts request chains
- Warns about missing sections (HTTP, admin, WebSocket, gRPC)

### 4. Summary Report
```
✅ Configuration is valid

📊 Summary:
   Found 5 HTTP endpoints
   Found 2 chains

⚠️  Warnings:
   - No WebSocket configuration found
```

## Manual Validation Checklist

Since validation is currently basic, here's a manual checklist for comprehensive validation:

### Required Fields

#### HTTP Configuration
```yaml
http:
  port: 3000              # ✅ Required
  host: "0.0.0.0"        # ✅ Required
```

#### Admin Configuration
```yaml
admin:
  enabled: true           # ✅ Required if using admin UI
  port: 9080             # ✅ Required in standalone mode
```

### Common Mistakes

#### 1. Invalid Port Numbers
```yaml
# ❌ Wrong - port must be 1-65535
http:
  port: 70000

# ✅ Correct
http:
  port: 3000
```

#### 2. Invalid File Paths
```yaml
# ❌ Wrong - file doesn't exist
http:
  openapi_spec: "./nonexistent.json"

# ✅ Correct - verify file exists
http:
  openapi_spec: "./examples/openapi.json"
```

Test the path:
```bash
ls -la ./examples/openapi.json
```

#### 3. Invalid Validation Mode
```yaml
# ❌ Wrong - invalid mode
validation:
  mode: "strict"

# ✅ Correct - must be: off, warn, or enforce
validation:
  mode: "enforce"
```

#### 4. Invalid Latency Configuration
```yaml
# ❌ Wrong - base_ms is too high
core:
  default_latency:
    base_ms: 100000

# ✅ Correct - reasonable latency
core:
  default_latency:
    base_ms: 100
    jitter_ms: 50
```

#### 5. Missing Required Fields in Routes
```yaml
# ❌ Wrong - missing response status
http:
  routes:
    - path: /test
      method: GET
      response:
        body: "test"

# ✅ Correct - include status code
http:
  routes:
    - path: /test
      method: GET
      response:
        status: 200
        body: "test"
```

#### 6. Invalid Environment Variable Names
```bash
# ❌ Wrong - incorrect prefix
export MOCK_FORGE_HTTP_PORT=3000

# ✅ Correct - use MOCKFORGE_ prefix
export MOCKFORGE_HTTP_PORT=3000
```

#### 7. Conflicting Mount Path Configuration
```yaml
# ❌ Wrong - both standalone and embedded
admin:
  enabled: true
  port: 9080
  mount_path: "/admin"    # Conflicts with standalone mode

# ✅ Correct - choose one mode
admin:
  enabled: true
  mount_path: "/admin"    # Embedded under HTTP server
  # OR
  port: 9080              # Standalone mode (no mount_path)
```

#### 8. Advanced Validation Configuration
```yaml
# ✅ Complete validation configuration
validation:
  mode: enforce                    # off | warn | enforce
  aggregate_errors: true          # Combine multiple errors
  validate_responses: false       # Validate response payloads
  status_code: 400                # Error status code (400 or 422)
  skip_admin_validation: true     # Skip validation for admin routes

  # Per-route overrides
  overrides:
    "GET /health": "off"          # Disable validation for health checks
    "POST /api/users": "warn"     # Warning mode for user creation
    "/api/internal/**": "off"     # Disable for internal endpoints
```

## Validation Tools

### 1. YAML Syntax Validator

Use `yamllint` for syntax validation:

```bash
# Install yamllint
pip install yamllint

# Validate YAML syntax
yamllint mockforge.yaml
```

### 2. JSON Schema Validation (Future)

MockForge doesn't currently provide JSON Schema validation, but you can use the template as a reference:

```bash
# Copy the complete template
cp config.template.yaml mockforge.yaml

# Edit with your settings, keeping structure intact
```

### 3. Test Your Configuration

The best validation is starting the server:

```bash
# Try to start the server
mockforge serve --config mockforge.yaml

# Check for error messages in logs
```

## Troubleshooting

### Error: "Configuration file not found"

**Cause**: Config file doesn't exist or isn't in expected location

**Solution**:
```bash
# Check current directory
ls -la mockforge.yaml

# Create from template
mockforge init .

# Or specify path explicitly
mockforge serve --config /path/to/config.yaml
```

### Error: "Invalid YAML syntax"

**Cause**: YAML parsing error (usually indentation or quotes)

**Solution**:
```bash
# Use yamllint to find the exact error
yamllint mockforge.yaml

# Common fixes:
# - Fix indentation (use 2 spaces, not tabs)
# - Quote strings with special characters
# - Match opening/closing brackets and braces
```

### Warning: "No HTTP configuration found"

**Cause**: Missing `http:` section

**Solution**:
```yaml
# Add minimal HTTP config
http:
  port: 3000
  host: "0.0.0.0"
```

### Error: "Port already in use"

**Cause**: Another process is using the configured port

**Solution**:
```bash
# Find what's using the port
lsof -i :3000

# Kill the process or change the port
# Change port in config:
http:
  port: 3001  # Use different port
```

### OpenAPI Spec Not Loading

**Cause**: File path is incorrect or spec is invalid

**Solution**:
```bash
# Verify file exists
ls -la examples/openapi.json

# Validate OpenAPI spec at https://editor.swagger.io/
# Or use swagger-cli:
npm install -g @apidevtools/swagger-cli
swagger-cli validate examples/openapi.json
```

## Best Practices

### 1. Use Version Control

```bash
# Track your config in Git
git add mockforge.yaml
git commit -m "Add MockForge configuration"
```

### 2. Environment-Specific Configs

```bash
# Create configs for different environments
mockforge.dev.yaml      # Development
mockforge.test.yaml     # Testing
mockforge.prod.yaml     # Production

# Use with:
mockforge serve --config mockforge.dev.yaml
```

### 3. Document Custom Settings

```yaml
http:
  port: 3000

  # Custom validation override for legacy endpoint
  # TODO: Remove when v2 API is live
  validation_overrides:
    "POST /legacy/users": "off"
```

### 4. Start Simple, Add Complexity

```yaml
# Start with minimal config
http:
  port: 3000
  openapi_spec: "./api.json"

admin:
  enabled: true

# Add features incrementally:
# 1. Template expansion
# 2. Latency simulation
# 3. Failure injection
# 4. Custom plugins
```

### 5. Use the Complete Template

```bash
# Copy the complete annotated template
cp config.template.yaml mockforge.yaml

# Remove sections you don't need
# Keep comments for reference
```

## Complete Configuration Template

See the [complete annotated configuration template](https://github.com/SaaSy-Solutions/mockforge/blob/main/config.template.yaml) for all available options with documentation.

## Validation Roadmap

Future versions of MockForge will include:

- **JSON Schema Validation**: Full schema validation for all fields
- **Field Type Checking**: Validate types, ranges, and formats
- **Cross-Field Validation**: Check for conflicts between settings
- **External Resource Validation**: Verify files, URLs, and connections
- **Deprecation Warnings**: Warn about deprecated options
- **Migration Assistance**: Auto-migrate old configs to new formats

Track progress: [MockForge Issue #XXX](https://github.com/SaaSy-Solutions/mockforge/issues)

## Getting Help

**Configuration not working as expected?**

1. Run `mockforge config validate` first
2. Check the [Configuration Schema Reference](config-schema.md)
3. Review [example configurations](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples)
4. Ask on [GitHub Discussions](https://github.com/SaaSy-Solutions/mockforge/discussions)
5. Report bugs at [GitHub Issues](https://github.com/SaaSy-Solutions/mockforge/issues)

---

**Pro Tip**: Keep a backup of your working configuration before making significant changes. Use `cp mockforge.yaml mockforge.yaml.backup` before editing.
