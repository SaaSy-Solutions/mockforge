# Multi-Tenant Workspaces - Design Document

## Overview

Multi-tenant workspaces enable a single MockForge instance to host multiple isolated mock environments (workspaces) with namespace separation. This allows:
- One MockForge deployment serving multiple projects
- Isolated mock configurations per workspace/project
- Self-hosted users managing multiple projects in one service
- Foundation for future SaaS offerings

## Design Goals

1. **Isolation**: Each workspace operates independently with its own routes, mocks, and configuration
2. **Flexibility**: Support both path-based and (optionally) port-based routing
3. **Simplicity**: Minimal configuration overhead for single-workspace deployments
4. **Performance**: Efficient routing without significant overhead
5. **Backward Compatibility**: Existing single-workspace setups continue to work

## Architecture

### 1. Routing Strategy

#### Path-Based Routing (Primary)

Workspaces are accessed via URL path prefixes:

```
http://localhost:3000/workspace/{workspace_id}/api/users
http://localhost:3000/w/{workspace_id}/api/users  (short form)
```

- **Default workspace**: Requests without workspace prefix route to a default workspace
- **Workspace ID formats**:
  - Full UUID: `550e8400-e29b-41d4-a716-446655440000`
  - Short ID: Configurable short names (e.g., `project-a`, `dev`, `staging`)

#### Port-Based Routing (Optional Future Enhancement)

Each workspace can optionally bind to its own port:

```
http://localhost:3001  -> workspace-1
http://localhost:3002  -> workspace-2
```

### 2. Configuration

#### Server Configuration (`ServerConfig`)

```yaml
# mockforge.yaml
multi_tenant:
  enabled: true
  routing_strategy: path  # or "port" or "both"
  workspace_prefix: "/workspace"  # or "/w" for short form
  default_workspace: "default"
  max_workspaces: 100  # optional limit

  # Optional workspace-specific ports (for port-based routing)
  workspace_ports:
    workspace-1: 3001
    workspace-2: 3002

workspaces:
  default:
    name: "Default Workspace"
    base_url: null
    enabled: true

  project-a:
    name: "Project A Development"
    base_url: "https://api.project-a.com"
    enabled: true
    auth:
      api_key:
        keys: ["key-for-project-a"]

  project-b:
    name: "Project B Staging"
    base_url: "https://staging.project-b.com"
    enabled: true
```

### 3. Middleware Architecture

#### Workspace Extraction Middleware

```rust
pub struct WorkspaceMiddleware {
    registry: Arc<RwLock<MultiTenantWorkspaceRegistry>>,
    config: MultiTenantConfig,
}

impl WorkspaceMiddleware {
    // Extracts workspace ID from request path
    fn extract_workspace_id(&self, path: &str) -> Option<String>

    // Routes request to appropriate workspace
    async fn route_to_workspace(&self, workspace_id: &str, req: Request) -> Response
}
```

Request Flow:
1. Request arrives: `GET /workspace/project-a/api/users`
2. Middleware extracts: `workspace_id = "project-a"`
3. Strips prefix: `/api/users`
4. Routes to workspace-specific route registry
5. Returns response

### 4. Multi-Tenant Workspace Registry

```rust
pub struct MultiTenantWorkspaceRegistry {
    /// Workspaces indexed by ID
    workspaces: HashMap<EntityId, TenantWorkspace>,
    /// Default workspace ID
    default_workspace_id: EntityId,
    /// Configuration
    config: MultiTenantConfig,
}

pub struct TenantWorkspace {
    /// Workspace metadata
    workspace: Workspace,
    /// Workspace-specific route registry
    route_registry: Arc<RwLock<RouteRegistry>>,
    /// Workspace-specific request logger
    request_logger: Arc<CentralizedRequestLogger>,
    /// Workspace-specific metrics
    metrics: WorkspaceMetrics,
    /// Last access timestamp (for cleanup)
    last_accessed: DateTime<Utc>,
}
```

### 5. Workspace-Specific Features

Each workspace maintains its own:
- **Route Registry**: Independent routing table
- **Request Logger**: Isolated request logs
- **Metrics**: Per-workspace Prometheus metrics with labels
- **Environment Variables**: Workspace-specific environments
- **Authentication**: Optional per-workspace auth configuration

### 6. Admin UI Updates

#### Workspace Selector

```
┌─────────────────────────────────────┐
│ MockForge Admin                      │
│                                      │
│ Workspace: [Default ▼]              │
│   - Default                          │
│   - Project A Development            │
│   - Project B Staging                │
│   - + New Workspace                  │
└─────────────────────────────────────┘
```

#### Multi-Workspace Dashboard

- View all workspaces
- Quick stats per workspace (request count, active routes, etc.)
- Switch between workspaces
- Create/edit/delete workspaces

## Implementation Plan

### Phase 1: Core Multi-Tenant Infrastructure

1. **Configuration Layer**
   - Add `MultiTenantConfig` to `ServerConfig`
   - Add workspace configuration schema
   - Environment variable overrides

2. **Multi-Tenant Registry**
   - Implement `MultiTenantWorkspaceRegistry`
   - Implement `TenantWorkspace` wrapper
   - Workspace lifecycle management (create, update, delete)

3. **Routing Middleware**
   - Implement `WorkspaceMiddleware`
   - Path extraction and workspace resolution
   - Request path rewriting (strip workspace prefix)

### Phase 2: Integration

4. **HTTP Server Integration**
   - Integrate workspace middleware into HTTP server
   - Update route registration to support multi-tenant registry
   - Workspace-aware request handling

5. **Request Logging & Metrics**
   - Per-workspace request logging
   - Prometheus metrics with `workspace_id` label
   - Workspace-specific log filtering

### Phase 3: CLI & Admin

6. **CLI Commands**
   ```bash
   mockforge workspace list
   mockforge workspace create <name> [--config workspace.yaml]
   mockforge workspace delete <id>
   mockforge workspace info <id>
   ```

7. **Admin UI**
   - Workspace selector component
   - Multi-workspace dashboard
   - Workspace management UI

### Phase 4: Advanced Features

8. **Workspace Templates**
   - Predefined workspace configurations
   - Quick workspace creation from templates

9. **Workspace Import/Export**
   - Export workspace configuration
   - Import workspace from file/URL

10. **Resource Limits**
    - Per-workspace request rate limiting
    - Memory limits per workspace
    - Auto-cleanup for inactive workspaces

## API Design

### Workspace Management API

#### List Workspaces
```
GET /__mockforge/api/workspaces
Response:
{
  "workspaces": [
    {
      "id": "default",
      "name": "Default Workspace",
      "enabled": true,
      "stats": {
        "total_requests": 1234,
        "active_routes": 56,
        "last_request_at": "2025-10-09T10:30:00Z"
      }
    }
  ]
}
```

#### Create Workspace
```
POST /__mockforge/api/workspaces
Body:
{
  "name": "New Project",
  "description": "Development workspace for new project",
  "config": {
    "base_url": "https://api.example.com",
    "auth": {...}
  }
}
```

#### Get Workspace
```
GET /__mockforge/api/workspaces/{id}
```

#### Update Workspace
```
PUT /__mockforge/api/workspaces/{id}
```

#### Delete Workspace
```
DELETE /__mockforge/api/workspaces/{id}
```

## Migration Path

### Existing Single-Workspace Deployments

1. **Default Behavior**: Multi-tenant disabled by default
2. **Automatic Migration**: First startup migrates existing workspace to "default" workspace
3. **Opt-in**: Enable via configuration `multi_tenant.enabled = true`

### Migration Steps

```bash
# 1. Backup existing configuration
mockforge backup --output ./backup

# 2. Enable multi-tenant mode
mockforge config set multi_tenant.enabled true

# 3. Verify migration
mockforge workspace list
```

## Security Considerations

1. **Workspace Isolation**: Ensure workspaces cannot access each other's data
2. **Authentication**: Optional per-workspace authentication
3. **Authorization**: Admin API requires authentication in multi-tenant mode
4. **Rate Limiting**: Per-workspace rate limits to prevent abuse
5. **Resource Limits**: Configurable memory/CPU limits per workspace

## Performance Considerations

1. **Route Lookup**: O(1) workspace lookup via HashMap
2. **Path Parsing**: Minimal overhead for extracting workspace ID
3. **Memory**: Each workspace maintains its own route registry (isolation trade-off)
4. **Lazy Loading**: Workspaces loaded on-demand, not all at startup

## Examples

### Example 1: Development Teams

```yaml
# mockforge.yaml
multi_tenant:
  enabled: true
  workspace_prefix: "/w"

workspaces:
  frontend:
    name: "Frontend Team"
    base_url: "https://api.backend.dev"

  backend:
    name: "Backend Team"
    base_url: null  # Pure mocks

  qa:
    name: "QA Environment"
    base_url: "https://api.staging.com"
```

Frontend team:
```bash
curl http://localhost:3000/w/frontend/api/users
```

Backend team:
```bash
curl http://localhost:3000/w/backend/api/users
```

### Example 2: SaaS Multi-Tenant

```yaml
multi_tenant:
  enabled: true
  workspace_prefix: "/workspace"
  max_workspaces: 1000
  auth:
    require_auth: true
    api_key:
      header_name: "X-Workspace-Key"
```

Client A:
```bash
curl -H "X-Workspace-Key: key-for-client-a" \
  http://mockforge.example.com/workspace/client-a/api/orders
```

Client B:
```bash
curl -H "X-Workspace-Key: key-for-client-b" \
  http://mockforge.example.com/workspace/client-b/api/orders
```

## Future Enhancements

1. **Workspace Sharing**: Allow workspaces to be shared between users
2. **Workspace Versioning**: Version control for workspace configurations
3. **Cross-Workspace References**: Reference mocks from other workspaces
4. **Workspace Analytics**: Advanced analytics per workspace
5. **Workspace Billing**: Track usage for billing (SaaS offering)
6. **Workspace Collaboration**: Real-time collaborative editing
7. **Workspace Templates Marketplace**: Community-shared templates

## Testing Strategy

1. **Unit Tests**: Multi-tenant registry, middleware, routing
2. **Integration Tests**: Full request flow through workspace middleware
3. **Load Tests**: Performance with 100+ workspaces
4. **Security Tests**: Workspace isolation verification
5. **Migration Tests**: Single to multi-tenant migration

## Documentation Requirements

1. **User Guide**: Setting up multi-tenant workspaces
2. **Configuration Reference**: All multi-tenant configuration options
3. **API Reference**: Workspace management API
4. **Migration Guide**: Migrating from single to multi-tenant
5. **Examples**: Real-world use cases and configurations

## Open Questions

1. Should we support dynamic workspace creation via API without restart?
2. How to handle workspace name conflicts?
3. Should default workspace be required or optional?
4. What's the strategy for workspace cleanup (max age, inactivity)?
5. Should we support hierarchical workspaces (parent/child)?

## Success Metrics

1. **Adoption**: % of deployments using multi-tenant mode
2. **Performance**: < 5ms overhead for workspace routing
3. **Reliability**: 99.9% uptime with multi-tenant enabled
4. **Scalability**: Support 1000+ workspaces without degradation
5. **Usability**: < 5 minutes to set up first multi-tenant deployment
