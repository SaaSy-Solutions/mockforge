# MockForge Configuration Guide

This guide covers the unified configuration system in MockForge, including profiles, file formats, and configuration precedence.

## Table of Contents

- [Quick Start](#quick-start)
- [Configuration File Formats](#configuration-file-formats)
- [Configuration Profiles](#configuration-profiles)
- [Configuration Precedence](#configuration-precedence)
- [File Discovery](#file-discovery)
- [Environment Variables](#environment-variables)
- [CLI Flags](#cli-flags)
- [Profile Examples](#profile-examples)
- [Best Practices](#best-practices)

## Quick Start

### Basic Usage

Create a `mockforge.yaml` file:

```yaml
http:
  port: 3000

logging:
  level: "info"

admin:
  enabled: true
```

Run MockForge:

```bash
mockforge serve
```

### With Profiles

Create a config file with profiles:

```yaml
# Base configuration
http:
  port: 3000

# Profiles
profiles:
  dev:
    logging:
      level: "debug"
    admin:
      enabled: true

  ci:
    logging:
      level: "warn"
    admin:
      enabled: false
```

Use a specific profile:

```bash
mockforge serve --profile dev
# or
mockforge serve --profile ci
```

## Configuration File Formats

MockForge supports multiple configuration file formats:

### 1. YAML (Recommended)

**Files:** `mockforge.yaml`, `mockforge.yml`, `.mockforge.yaml`, `.mockforge.yml`

```yaml
http:
  port: 3000
  host: "0.0.0.0"

logging:
  level: "info"
```

**Pros:**
- Human-readable
- Widely adopted
- Good for version control
- Supports comments

### 2. TypeScript

**Files:** `mockforge.config.ts`

```typescript
const config = {
  http: {
    port: 3000,
    host: "0.0.0.0",
  },

  logging: {
    level: "info",
  },

  profiles: {
    dev: {
      logging: { level: "debug" }
    }
  }
};

config;
```

**Pros:**
- Type-safe with IDE autocomplete
- Programmatic configuration
- Can use JavaScript logic (conditionals, functions, etc.)

### 3. JavaScript

**Files:** `mockforge.config.js`

```javascript
const config = {
  http: {
    port: 3000,
    host: "0.0.0.0",
  },

  logging: {
    level: "info",
  }
};

config;
```

### 4. JSON

**Files:** `mockforge.json`

```json
{
  "http": {
    "port": 3000,
    "host": "0.0.0.0"
  },

  "logging": {
    "level": "info"
  }
}
```

## Configuration Profiles

Profiles allow you to define environment-specific configurations (dev, ci, demo, prod) in a single file.

### Defining Profiles

```yaml
# Base configuration (shared across all profiles)
http:
  port: 3000
  host: "0.0.0.0"

logging:
  level: "info"

# Named profiles
profiles:
  dev:
    logging:
      level: "debug"
    admin:
      enabled: true
      port: 9080
    observability:
      recorder:
        enabled: true
        database_path: "./dev-recordings.db"

  ci:
    logging:
      level: "warn"
      json_format: true
    admin:
      enabled: false
    observability:
      prometheus:
        enabled: true
        port: 9091

  demo:
    admin:
      enabled: true
      mount_path: "/admin"
    core:
      latency_enabled: true
      default_latency:
        base_ms: 100
        jitter_ms: 50

  prod:
    logging:
      level: "error"
      json_format: true
    admin:
      enabled: true
      auth_required: true
    observability:
      opentelemetry:
        enabled: true
        environment: "production"
```

### Using Profiles

```bash
# Use dev profile
mockforge serve --profile dev

# Use ci profile
mockforge serve --profile ci

# Specify config file and profile
mockforge serve --config ./mockforge.yaml --profile prod

# No profile (uses base config only)
mockforge serve
```

### How Profiles Work

1. **Base configuration** is loaded first
2. **Profile configuration** overrides matching fields
3. Fields not specified in the profile remain unchanged
4. Profile merging is **shallow** - nested objects are replaced entirely

**Example:**

```yaml
# Base
http:
  port: 3000
  host: "0.0.0.0"
  cors:
    enabled: true

# Profile
profiles:
  prod:
    http:
      port: 8080
      # cors is NOT inherited - the entire http config is replaced
```

To preserve nested fields, specify them in the profile:

```yaml
profiles:
  prod:
    http:
      port: 8080
      host: "0.0.0.0"
      cors:
        enabled: true
```

## Configuration Precedence

Configuration is merged from multiple sources with the following precedence (highest to lowest):

1. **CLI arguments** (highest priority)
2. **Environment variables**
3. **Profile configuration**
4. **Base configuration file**
5. **Default values** (lowest priority)

### Example

Given this config file:

```yaml
http:
  port: 3000

profiles:
  dev:
    http:
      port: 4000
```

And these environment variables:

```bash
export MOCKFORGE_HTTP_PORT=5000
```

Running:

```bash
mockforge serve --profile dev --http-port 6000
```

**Result:** HTTP server runs on port **6000** (CLI wins)

Without `--http-port`:

```bash
mockforge serve --profile dev
```

**Result:** Port **5000** (env var wins over profile)

Without env var:

```bash
unset MOCKFORGE_HTTP_PORT
mockforge serve --profile dev
```

**Result:** Port **4000** (profile wins over base config)

## File Discovery

MockForge automatically discovers configuration files in the following order:

1. **Explicit path:** `--config ./path/to/config.yaml`
2. **Auto-discovery** (searches current directory and 5 parent directories):
   - `mockforge.config.ts`
   - `mockforge.config.js`
   - `mockforge.yaml`
   - `mockforge.yml`
   - `.mockforge.yaml`
   - `.mockforge.yml`

### Directory Structure Example

```
my-project/
├── mockforge.config.ts     ← Found first
├── mockforge.yaml          ← Ignored if .ts exists
└── api/
    └── tests/
        └── current-dir/    ← Searches up to 5 levels
```

## Environment Variables

All configuration options can be overridden using environment variables.

### Common Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `MOCKFORGE_HTTP_PORT` | HTTP server port | `3000` |
| `MOCKFORGE_HTTP_HOST` | HTTP server host | `0.0.0.0` |
| `MOCKFORGE_WS_PORT` | WebSocket server port | `3001` |
| `MOCKFORGE_GRPC_PORT` | gRPC server port | `50051` |
| `MOCKFORGE_ADMIN_PORT` | Admin UI port | `9080` |
| `MOCKFORGE_ADMIN_ENABLED` | Enable admin UI | `true` |
| `MOCKFORGE_LOG_LEVEL` | Log level | `debug` |
| `MOCKFORGE_TRAFFIC_SHAPING_ENABLED` | Enable traffic shaping | `true` |

### Feature Flags

| Variable | Description | Example |
|----------|-------------|---------|
| `MOCKFORGE_LATENCY_ENABLED` | Enable latency injection | `true` |
| `MOCKFORGE_FAILURES_ENABLED` | Enable failure injection | `true` |
| `MOCKFORGE_OVERRIDES_ENABLED` | Enable request/response overrides | `true` |

### AI/RAG Configuration

| Variable | Description | Example |
|----------|-------------|---------|
| `MOCKFORGE_RAG_PROVIDER` | LLM provider | `openai` |
| `MOCKFORGE_RAG_MODEL` | Model name | `gpt-4` |
| `MOCKFORGE_RAG_API_KEY` | API key | `sk-...` |
| `MOCKFORGE_RAG_TEMPERATURE` | Temperature | `0.7` |

See [config.example.yaml](./config.example.yaml) for the complete list of environment variables.

## CLI Flags

CLI flags have the highest precedence and override all other configuration sources.

### Common Flags

```bash
# Server ports
--http-port <PORT>          HTTP server port
--ws-port <PORT>            WebSocket port
--grpc-port <PORT>          gRPC port
--admin-port <PORT>         Admin UI port

# Configuration
--config <PATH>             Config file path
--profile <PROFILE>         Configuration profile (dev, ci, demo, prod)

# Observability
--admin                     Enable admin UI
--metrics                   Enable Prometheus metrics
--metrics-port <PORT>       Metrics endpoint port
--tracing                   Enable OpenTelemetry tracing

# Recorder
--recorder                  Enable API Flight Recorder
--recorder-db <PATH>        Database file path
--recorder-max-requests N   Max recorded requests

# Chaos Engineering
--chaos                     Enable chaos engineering
--chaos-scenario <NAME>     Predefined scenario
--chaos-latency-ms <MS>     Fixed latency
--chaos-http-errors <CODES> HTTP error codes (comma-separated)

# Validation
--dry-run                   Validate config without starting servers
```

### Examples

```bash
# Basic server with custom port
mockforge serve --http-port 8080

# Dev environment with admin UI and recorder
mockforge serve --profile dev --admin --recorder

# CI with specific config
mockforge serve --config ./ci.yaml --profile ci --dry-run

# Production with all observability
mockforge serve --profile prod --admin --metrics --tracing --recorder
```

## Profile Examples

### Development Profile

Focus: Easy debugging, all features enabled

```yaml
profiles:
  dev:
    logging:
      level: "debug"
      json_format: false

    admin:
      enabled: true
      api_enabled: true

    observability:
      prometheus:
        enabled: true
      recorder:
        enabled: true
        database_path: "./dev-recordings.db"
        max_requests: 1000
        retention_days: 3

    core:
      latency_enabled: false
      failures_enabled: false
```

**Usage:**
```bash
mockforge serve --profile dev
```

### CI/Testing Profile

Focus: Fast execution, structured logging

```yaml
profiles:
  ci:
    http:
      port: 8080

    logging:
      level: "warn"
      json_format: true

    admin:
      enabled: false

    observability:
      prometheus:
        enabled: true
        port: 9091
      recorder:
        enabled: false

    core:
      latency_enabled: false
      failures_enabled: false
```

**Usage:**
```bash
mockforge serve --profile ci
```

### Demo Profile

Focus: Showcase features, realistic delays

```yaml
profiles:
  demo:
    logging:
      level: "info"

    admin:
      enabled: true
      mount_path: "/admin"

    observability:
      recorder:
        enabled: true
        api_enabled: true
      chaos:
        enabled: true
        latency:
          enabled: true
          fixed_delay_ms: 150
          probability: 0.5

    core:
      latency_enabled: true
      default_latency:
        base_ms: 100
        jitter_ms: 50
        distribution: "normal"
```

**Usage:**
```bash
mockforge serve --profile demo
```

### Production Profile

Focus: Performance, security, observability

```yaml
profiles:
  prod:
    logging:
      level: "warn"
      json_format: true
      file_path: "/var/log/mockforge/mockforge.log"

    admin:
      enabled: true
      auth_required: true
      username: "admin"

    observability:
      prometheus:
        enabled: true
      opentelemetry:
        enabled: true
        service_name: "mockforge-prod"
        environment: "production"
        sampling_rate: 0.1
      recorder:
        enabled: true
        database_path: "/var/lib/mockforge/recordings.db"
        max_requests: 100000
        retention_days: 30

    core:
      latency_enabled: false
      failures_enabled: false
```

**Usage:**
```bash
# Set sensitive values via env vars
export MOCKFORGE_ADMIN_PASSWORD="secure-password"
export MOCKFORGE_RAG_API_KEY="sk-..."

mockforge serve --profile prod
```

## Best Practices

### 1. Use Profiles for Environments

Define environment-specific settings in profiles rather than maintaining separate config files:

**Good:**
```yaml
# mockforge.yaml (single file)
profiles:
  dev: { ... }
  staging: { ... }
  prod: { ... }
```

**Avoid:**
```
mockforge.dev.yaml
mockforge.staging.yaml
mockforge.prod.yaml
```

### 2. Keep Secrets in Environment Variables

Never commit secrets to config files:

**Good:**
```yaml
admin:
  auth_required: true
  username: "admin"
  # password loaded from MOCKFORGE_ADMIN_PASSWORD
```

```bash
export MOCKFORGE_ADMIN_PASSWORD="secret"
```

**Avoid:**
```yaml
admin:
  password: "hardcoded-secret"  # Never do this!
```

### 3. Use TypeScript for Complex Configurations

For large, complex configurations, TypeScript provides type safety:

```typescript
interface Profile {
  logging: { level: string };
  admin: { enabled: boolean };
}

const isDev = process.env.NODE_ENV === 'development';

const config = {
  profiles: {
    dev: {
      logging: { level: isDev ? "debug" : "info" }
    } as Profile
  }
};

config;
```

### 4. Validate Configs in CI

Use `--dry-run` to validate configs without starting servers:

```bash
# In CI pipeline
mockforge serve --config ./mockforge.yaml --profile ci --dry-run
```

### 5. Document Custom Profiles

Add comments explaining the purpose of each profile:

```yaml
profiles:
  # Development: Verbose logging, all features enabled
  dev:
    logging: { level: "debug" }

  # Load Testing: High limits, minimal logging
  load:
    logging: { level: "error" }
    observability:
      recorder:
        max_requests: 1000000
```

### 6. Use Consistent Naming

Follow consistent naming conventions for profiles:

- `dev` - Local development
- `ci` - Continuous Integration
- `staging` - Staging environment
- `prod` - Production
- `test` - Integration testing
- `demo` - Product demonstrations

### 7. Layer Profiles for Reusability

Create base profiles and extend them:

```yaml
profiles:
  # Base observability profile
  _observability_base:
    observability:
      prometheus: { enabled: true }
      recorder: { enabled: true }

  # Extend base profiles
  dev:
    # Copy relevant fields from _observability_base
    observability:
      prometheus: { enabled: true }
      recorder:
        enabled: true
        max_requests: 1000
```

## Migration from Environment Variables

If you're currently using environment variables exclusively, here's how to migrate:

### Before (env vars only)

```bash
export MOCKFORGE_HTTP_PORT=3000
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_LOG_LEVEL=debug
mockforge serve
```

### After (unified config)

**Option 1:** Config file
```yaml
# mockforge.yaml
http:
  port: 3000

admin:
  enabled: true

logging:
  level: "debug"
```

```bash
mockforge serve
```

**Option 2:** Profiles
```yaml
# mockforge.yaml
profiles:
  dev:
    http: { port: 3000 }
    admin: { enabled: true }
    logging: { level: "debug" }
```

```bash
mockforge serve --profile dev
```

**Option 3:** Hybrid (recommended)
```yaml
# mockforge.yaml (base config)
http:
  port: 3000

admin:
  enabled: true

logging:
  level: "info"
```

```bash
# Override for specific needs
export MOCKFORGE_LOG_LEVEL=debug
mockforge serve
```

## Troubleshooting

### Profile Not Found

```
Error: Profile 'prod' not found in configuration.
Available profiles: dev, ci, demo
```

**Solution:** Check the profile name in your config file matches the `--profile` flag.

### Config File Not Discovered

```
No configuration file found. Expected one of: mockforge.config.ts, mockforge.yaml, ...
```

**Solution:**
1. Create a config file in the current directory
2. Use `--config` to specify an explicit path
3. Check file naming (must be exact match)

### TypeScript Config Fails to Load

```
Failed to evaluate JS config: SyntaxError: ...
```

**Solution:**
- Ensure the config file returns a value (use `config;` at the end)
- Check for TypeScript syntax errors
- Use simpler type annotations (advanced TS features may not work)

### Env Var Not Applied

**Issue:** Environment variable seems to be ignored

**Solution:** Check precedence order:
1. Is a CLI flag set? (overrides env vars)
2. Is the env var name correct? (see list above)
3. Is the value in the correct format? (boolean: `true`/`false`, numbers: `3000`)

## See Also

- [config.example.yaml](./config.example.yaml) - Comprehensive config example
- [examples/mockforge.config.yaml](./examples/mockforge.config.yaml) - YAML with profiles
- [examples/mockforge.config.ts](./examples/mockforge.config.ts) - TypeScript example
- [OpenAPI Specification Guide](./docs/openapi.md)
- [Admin UI Guide](./docs/admin-ui.md)

## Contributing

Found a configuration issue or have suggestions? Please [open an issue](https://github.com/SaaSy-Solutions/mockforge/issues) or submit a PR!
