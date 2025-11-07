# Mock → Real Migration Pipeline

## Overview

The Mock → Real Migration Pipeline enables MockForge to act as middleware, allowing gradual migration from mock responses to real backend APIs on a per-route basis. Routes can be toggled between mock and real backends at runtime without code changes.

## Features

- **Per-Route Migration**: Toggle individual routes between mock and real backends
- **Group-Based Migration**: Toggle entire groups of routes together
- **Migration Stages**: Support for mock → shadow → real progression
- **Runtime Control**: Change migration modes via Admin API without restarting
- **Shadow Mode**: Compare real and mock responses side-by-side
- **Configuration Persistence**: Save migration state to config files

## Migration Modes

### Mock Mode
- Route always returns mock response (from OpenAPI spec or mock generator)
- Proxy is ignored even if configured
- Useful for initial development and testing

### Shadow Mode
- Route proxies to real backend AND generates mock response
- Returns real response to client
- Logs both responses for comparison
- Useful for validating mock responses before full migration

### Real Mode
- Route always proxies to real backend
- Fails if proxy is unavailable (no fallback to mock)
- Useful for routes that have been fully migrated

### Auto Mode
- Uses existing priority chain (Replay → Fail → Proxy → Mock → Record)
- Default behavior for backward compatibility
- Migration features are opt-in

## Configuration

### Basic Setup

Enable migration features in your `config.yaml`:

```yaml
core:
  proxy:
    enabled: true
    migration_enabled: true  # Enable migration features
    target_url: "https://api.example.com"
    rules:
      - pattern: "/api/users/*"
        upstream_url: "https://api.example.com"
        migration_mode: "mock"  # Start with mock
```

### Group-Based Migration

Define migration groups to toggle multiple routes together:

```yaml
core:
  proxy:
    migration_enabled: true
    migration_groups:
      "api-v1": "mock"      # All v1 routes start with mock
      "api-v2": "shadow"    # All v2 routes in shadow mode
      "legacy": "real"      # Legacy routes already migrated
    rules:
      - pattern: "/api/v1/users/*"
        upstream_url: "https://api.example.com"
        migration_mode: "mock"
        migration_group: "api-v1"  # Belongs to api-v1 group
      - pattern: "/api/v2/orders/*"
        upstream_url: "https://api.example.com"
        migration_mode: "shadow"
        migration_group: "api-v2"
```

### Migration Stages Example

Gradual migration workflow:

```yaml
core:
  proxy:
    migration_enabled: true
    rules:
      # Stage 1: Start with mock
      - pattern: "/api/users/*"
        upstream_url: "https://api.example.com"
        migration_mode: "mock"

      # Stage 2: Move to shadow (compare real vs mock)
      - pattern: "/api/orders/*"
        upstream_url: "https://api.example.com"
        migration_mode: "shadow"

      # Stage 3: Fully migrated to real
      - pattern: "/api/payments/*"
        upstream_url: "https://api.example.com"
        migration_mode: "real"
```

## Admin API Endpoints

### List Migration Routes

```http
GET /__mockforge/migration/routes
```

Returns all routes with their migration status.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "pattern": "/api/users/*",
      "upstream_url": "https://api.example.com",
      "migration_mode": "mock",
      "route_mode": "mock",
      "migration_group": "api-v1",
      "enabled": true
    }
  ]
}
```

### Toggle Route Migration

```http
POST /__mockforge/migration/routes/{pattern}/toggle
```

Cycles route through stages: mock → shadow → real → mock

**Example:**
```bash
curl -X POST http://localhost:9080/__mockforge/migration/routes/api%2Fusers%2F%2A/toggle
```

### Set Route Migration Mode

```http
PUT /__mockforge/migration/routes/{pattern}
Content-Type: application/json

{
  "mode": "shadow"
}
```

Set a specific migration mode for a route.

**Example:**
```bash
curl -X PUT http://localhost:9080/__mockforge/migration/routes/api%2Fusers%2F%2A \
  -H "Content-Type: application/json" \
  -d '{"mode": "shadow"}'
```

### Toggle Group Migration

```http
POST /__mockforge/migration/groups/{group}/toggle
```

Cycles entire group through stages: mock → shadow → real → mock

**Example:**
```bash
curl -X POST http://localhost:9080/__mockforge/migration/groups/api-v1/toggle
```

### Set Group Migration Mode

```http
PUT /__mockforge/migration/groups/{group}
Content-Type: application/json

{
  "mode": "real"
}
```

Set migration mode for an entire group (affects all routes in the group).

**Example:**
```bash
curl -X PUT http://localhost:9080/__mockforge/migration/groups/api-v1 \
  -H "Content-Type: application/json" \
  -d '{"mode": "real"}'
```

### List Migration Groups

```http
GET /__mockforge/migration/groups
```

Returns all migration groups with their status.

**Response:**
```json
{
  "success": true,
  "data": {
    "api-v1": {
      "name": "api-v1",
      "migration_mode": "mock",
      "route_count": 5
    }
  }
}
```

### Get Migration Status

```http
GET /__mockforge/migration/status
```

Returns overall migration statistics.

**Response:**
```json
{
  "success": true,
  "data": {
    "total_routes": 10,
    "mock_routes": 5,
    "shadow_routes": 2,
    "real_routes": 3,
    "auto_routes": 0,
    "total_groups": 3,
    "migration_enabled": true
  }
}
```

## Migration Workflow

### Step 1: Start with Mock
All routes begin in mock mode, serving responses from OpenAPI specs or mock generators.

```yaml
migration_mode: "mock"
```

### Step 2: Enable Shadow Mode
Move routes to shadow mode to compare real vs mock responses.

```bash
curl -X PUT http://localhost:9080/__mockforge/migration/routes/api%2Fusers%2F%2A \
  -H "Content-Type: application/json" \
  -d '{"mode": "shadow"}'
```

Check logs for comparison results:
```
Shadow mode: comparing real and mock responses
Shadow mode: real and mock responses differ
```

### Step 3: Migrate to Real
Once validated, move routes to real mode.

```bash
curl -X PUT http://localhost:9080/__mockforge/migration/routes/api%2Fusers%2F%2A \
  -H "Content-Type: application/json" \
  -d '{"mode": "real"}'
```

### Group-Based Migration

Toggle entire groups at once:

```bash
# Move all api-v1 routes to shadow
curl -X PUT http://localhost:9080/__mockforge/migration/groups/api-v1 \
  -H "Content-Type: application/json" \
  -d '{"mode": "shadow"}'

# Move all api-v1 routes to real
curl -X PUT http://localhost:9080/__mockforge/migration/groups/api-v1 \
  -H "Content-Type: application/json" \
  -d '{"mode": "real"}'
```

## Priority Chain with Migration

The priority chain respects migration modes:

1. **Replay**: Check for recorded fixtures (always first)
2. **Fail**: Check for failure injection
3. **Migration Check**:
   - If `mock`: Skip proxy, continue to mock generator
   - If `shadow`: Proxy to real AND generate mock, return real, log comparison
   - If `real`: Force proxy (fail if unavailable)
   - If `auto`: Use existing priority chain
4. **Proxy**: Forward to real backend (if migration allows)
5. **Mock**: Generate mock response (if migration allows)
6. **Record**: Record request for future replay

## Troubleshooting

### Route Not Respecting Migration Mode

1. Check that `migration_enabled: true` is set in config
2. Verify the route pattern matches exactly
3. Check if a group override is affecting the route
4. Ensure the route is enabled

### Shadow Mode Not Comparing Responses

1. Verify mock generator is available
2. Check logs for shadow mode messages
3. Ensure both real and mock responses are being generated

### Group Override Not Working

1. Verify the route's `migration_group` matches the group name
2. Check that the group exists in `migration_groups`
3. Group overrides take precedence over route-specific modes

### Proxy Failing in Real Mode

In real mode, proxy failures are not allowed to fall back to mock. Ensure:
1. Proxy is enabled
2. Upstream URL is correct and reachable
3. Network connectivity is available
4. Consider using shadow mode first to validate

## Best Practices

1. **Start with Mock**: Begin all routes in mock mode
2. **Use Shadow for Validation**: Use shadow mode to compare responses before full migration
3. **Migrate Incrementally**: Move routes one at a time or by group
4. **Monitor Logs**: Watch for shadow mode comparison warnings
5. **Use Groups**: Organize routes into logical groups for easier management
6. **Test Thoroughly**: Validate each stage before moving to the next

## Example: Complete Migration Workflow

```yaml
# config.yaml - Initial state
core:
  proxy:
    migration_enabled: true
    migration_groups:
      "user-api": "mock"
    rules:
      - pattern: "/api/users/*"
        upstream_url: "https://api.example.com"
        migration_mode: "mock"
        migration_group: "user-api"
```

```bash
# Step 1: Check current status
curl http://localhost:9080/__mockforge/migration/status

# Step 2: Move to shadow mode
curl -X PUT http://localhost:9080/__mockforge/migration/groups/user-api \
  -H "Content-Type: application/json" \
  -d '{"mode": "shadow"}'

# Step 3: Monitor logs for comparison
# Check for "Shadow mode: comparing real and mock responses"

# Step 4: Once validated, move to real
curl -X PUT http://localhost:9080/__mockforge/migration/groups/user-api \
  -H "Content-Type: application/json" \
  -d '{"mode": "real"}'

# Step 5: Verify migration complete
curl http://localhost:9080/__mockforge/migration/status
```

## See Also

- [Proxy Configuration](PROXY_CONFIGURATION.md)
- [Admin API Documentation](../book/src/api/admin-ui-rest.md)
- [Configuration Guide](CONFIG.md)
