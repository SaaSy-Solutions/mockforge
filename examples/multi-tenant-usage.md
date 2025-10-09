# Multi-Tenant Workspaces - Usage Guide

## Overview

Multi-tenant workspaces enable a single MockForge instance to host multiple isolated mock environments, each with its own configuration, routes, and data. This is ideal for:

- **Team collaboration**: Different teams (frontend, backend, QA) can have isolated mock environments
- **Environment separation**: Separate development, staging, and production mocks
- **Multi-project setups**: One MockForge instance serving multiple projects
- **SaaS deployments**: Serve multiple clients from one installation

## Quick Start

### 1. Enable Multi-Tenant Mode

Create a `mockforge.yaml` configuration file:

```yaml
multi_tenant:
  enabled: true
  routing_strategy: path
  workspace_prefix: "/workspace"
  default_workspace: "default"
```

### 2. Start MockForge

```bash
mockforge serve --config mockforge.yaml
```

### 3. Create Workspaces

Using the CLI:

```bash
# Create a workspace for frontend team
mockforge workspace create frontend-dev \
  --name "Frontend Development" \
  --description "Mocks for frontend team"

# Create a workspace for backend team
mockforge workspace create backend-staging \
  --name "Backend Staging" \
  --description "Backend integration testing"
```

Using the API:

```bash
curl -X POST http://localhost:9080/__mockforge/api/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "id": "qa-testing",
    "name": "QA Testing Environment",
    "description": "Isolated environment for QA team",
    "enabled": true
  }'
```

### 4. Access Workspace-Specific Mocks

```bash
# Default workspace (no prefix)
curl http://localhost:3000/api/users

# Frontend development workspace
curl http://localhost:3000/workspace/frontend-dev/api/users

# Backend staging workspace
curl http://localhost:3000/workspace/backend-staging/api/orders

# QA testing workspace
curl http://localhost:3000/workspace/qa-testing/api/products
```

## Routing Strategies

### Path-Based Routing (Default)

Workspaces are accessed via URL path prefixes:

```
http://localhost:3000/workspace/{workspace-id}/{path}
```

**Advantages:**
- Single port for all workspaces
- Easy to configure
- Works well with reverse proxies

**Example:**
```bash
curl http://localhost:3000/workspace/project-a/api/users
curl http://localhost:3000/workspace/project-b/api/orders
```

### Port-Based Routing

Each workspace gets its own port:

```yaml
multi_tenant:
  enabled: true
  routing_strategy: port
  workspace_ports:
    project-a: 3001
    project-b: 3002
```

**Advantages:**
- Clear separation
- No path prefix required
- Easier for some clients

**Example:**
```bash
curl http://localhost:3001/api/users   # project-a
curl http://localhost:3002/api/orders  # project-b
```

### Both Strategies

Support both path and port-based routing:

```yaml
multi_tenant:
  enabled: true
  routing_strategy: both
  workspace_prefix: "/workspace"
  workspace_ports:
    project-a: 3001
```

**Example:**
```bash
# Both work for project-a:
curl http://localhost:3001/api/users
curl http://localhost:3000/workspace/project-a/api/users
```

## Workspace Management

### CLI Commands

```bash
# List all workspaces
mockforge workspace list

# Create a workspace
mockforge workspace create <id> \
  --name "Workspace Name" \
  --description "Description" \
  --base-url "https://api.example.com"

# Get workspace info
mockforge workspace info <id>

# Delete a workspace
mockforge workspace delete <id>

# Enable/disable a workspace
mockforge workspace enable <id>
mockforge workspace disable <id>
```

### API Endpoints

#### List Workspaces

```bash
GET /__mockforge/api/workspaces

Response:
{
  "workspaces": [
    {
      "id": "frontend-dev",
      "name": "Frontend Development",
      "enabled": true,
      "stats": {
        "total_requests": 1234,
        "active_routes": 56,
        "last_request_at": "2025-10-09T10:30:00Z",
        "avg_response_time_ms": 45.2
      }
    }
  ]
}
```

#### Create Workspace

```bash
POST /__mockforge/api/workspaces

Request:
{
  "id": "new-project",
  "name": "New Project",
  "description": "Mocks for new project",
  "enabled": true,
  "config": {
    "base_url": "https://api.example.com",
    "auth": {
      "api_key": {
        "keys": ["key-123"]
      }
    }
  }
}
```

#### Get Workspace

```bash
GET /__mockforge/api/workspaces/{id}
```

#### Update Workspace

```bash
PUT /__mockforge/api/workspaces/{id}

Request:
{
  "name": "Updated Name",
  "enabled": true
}
```

#### Delete Workspace

```bash
DELETE /__mockforge/api/workspaces/{id}
```

## Use Cases

### Example 1: Team Collaboration

**Scenario:** Frontend and backend teams need isolated mock environments

**Configuration:**

```yaml
multi_tenant:
  enabled: true
  workspace_prefix: "/w"
```

**Setup:**

```bash
# Frontend team workspace
mockforge workspace create frontend \
  --name "Frontend Team" \
  --description "Frontend development mocks"

# Backend team workspace
mockforge workspace create backend \
  --name "Backend Team" \
  --description "Backend integration mocks"
```

**Usage:**

```bash
# Frontend team
curl http://localhost:3000/w/frontend/api/users
curl http://localhost:3000/w/frontend/api/products

# Backend team
curl http://localhost:3000/w/backend/api/orders
curl http://localhost:3000/w/backend/api/inventory
```

### Example 2: Environment Separation

**Scenario:** Separate mocks for development, staging, and production testing

**Setup:**

```bash
mockforge workspace create dev --name "Development"
mockforge workspace create staging --name "Staging"
mockforge workspace create prod-test --name "Production Testing"
```

**Usage:**

```bash
# Development environment
curl http://localhost:3000/workspace/dev/api/users

# Staging environment
curl http://localhost:3000/workspace/staging/api/users

# Production testing
curl http://localhost:3000/workspace/prod-test/api/users
```

### Example 3: Multi-Project Setup

**Scenario:** One MockForge instance serving multiple projects

**Configuration:**

```yaml
multi_tenant:
  enabled: true
  max_workspaces: 50
  workspace_prefix: "/workspace"
```

**Setup:**

```bash
mockforge workspace create ecommerce --name "E-commerce Platform"
mockforge workspace create analytics --name "Analytics Dashboard"
mockforge workspace create notifications --name "Notification Service"
```

**Usage:**

```bash
curl http://localhost:3000/workspace/ecommerce/api/products
curl http://localhost:3000/workspace/analytics/api/stats
curl http://localhost:3000/workspace/notifications/api/messages
```

## Workspace Configuration

Each workspace can have its own configuration:

```json
{
  "base_url": "https://api.backend.com",
  "auth": {
    "api_key": {
      "header_name": "X-API-Key",
      "keys": ["workspace-specific-key"]
    }
  },
  "default_headers": {
    "X-Workspace": "frontend-dev"
  },
  "environments": [
    {
      "name": "Local",
      "variables": {
        "API_URL": "http://localhost:8080",
        "ENV": "local"
      }
    }
  ]
}
```

## Metrics and Monitoring

### Workspace-Specific Metrics

MockForge tracks metrics per workspace:

- **Total requests**: Number of requests handled
- **Active routes**: Number of configured routes
- **Average response time**: Mean response time in milliseconds
- **Last request timestamp**: When the workspace was last accessed

### Prometheus Metrics

When Prometheus is enabled, workspace metrics are labeled:

```
mockforge_requests_total{workspace="frontend-dev"} 1234
mockforge_response_time_ms{workspace="frontend-dev"} 45.2
mockforge_active_routes{workspace="frontend-dev"} 56
```

## Best Practices

### 1. Workspace Naming

- Use descriptive IDs: `frontend-dev`, `backend-staging`, `qa-testing`
- Avoid special characters (use hyphens instead of underscores)
- Keep IDs short for cleaner URLs

### 2. Workspace Organization

- **By team**: `frontend`, `backend`, `qa`, `devops`
- **By environment**: `dev`, `staging`, `prod-test`
- **By project**: `project-a`, `project-b`, `project-c`
- **By feature**: `feature-auth`, `feature-payments`, `feature-search`

### 3. Resource Limits

Set appropriate limits to prevent resource exhaustion:

```yaml
multi_tenant:
  max_workspaces: 100
```

### 4. Security

- Use workspace-specific authentication when needed
- Don't share sensitive data across workspaces
- Implement access controls via API keys or auth config

### 5. Monitoring

- Monitor workspace usage and performance
- Set up alerts for inactive workspaces
- Track workspace-specific request patterns

## Troubleshooting

### Workspace Not Found

```bash
# Check if workspace exists
curl http://localhost:9080/__mockforge/api/workspaces/my-workspace

# List all workspaces
curl http://localhost:9080/__mockforge/api/workspaces
```

### Path Not Resolving

Ensure you're using the correct path prefix:

```bash
# Correct:
curl http://localhost:3000/workspace/my-workspace/api/users

# Incorrect:
curl http://localhost:3000/my-workspace/api/users
```

### Workspace Disabled

Check if the workspace is enabled:

```bash
# Enable workspace
mockforge workspace enable my-workspace

# Or via API
curl -X PUT http://localhost:9080/__mockforge/api/workspaces/my-workspace \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'
```

## Migration from Single Workspace

Existing single-workspace deployments can be migrated:

1. **Enable multi-tenant mode** in configuration
2. **Existing mocks** automatically become part of the default workspace
3. **No code changes required** - default workspace accessible without prefix
4. **Gradually add** new workspaces as needed

## Next Steps

- See [Multi-Tenant Architecture](../docs/multi-tenant-workspaces.md) for detailed design
- Explore [Workspace API Reference](#) for complete API documentation
- Check [Admin UI Guide](#) for workspace management via UI
