# Role-Based Access Control (RBAC) Guide

MockForge provides **complete role-based access control (RBAC)** through the collaboration system, enabling fine-grained permission management for team workspaces.

## Overview

RBAC in MockForge allows you to:

- **Control Access**: Define who can view, edit, or manage mocks
- **Enforce Permissions**: Automatic permission checking on all operations
- **Team Collaboration**: Share workspaces with appropriate access levels
- **Audit Access**: Track all changes with user attribution

## User Roles

MockForge supports three built-in roles:

### Admin

**Full access** to all features:

- ✅ Create, edit, and delete workspaces
- ✅ Manage workspace members and invitations
- ✅ Create, edit, and delete mocks
- ✅ View and manage history
- ✅ Create and restore snapshots
- ✅ Manage workspace settings
- ✅ Manage integrations

**Use Case:** Team leads, project owners, administrators

### Editor

**Edit access** to mocks, but cannot manage workspace:

- ✅ Create, edit, and delete mocks
- ✅ View mocks and history
- ✅ Create snapshots
- ❌ Cannot manage workspace settings
- ❌ Cannot manage members
- ❌ Cannot restore snapshots

**Use Case:** Developers, QA engineers, mock creators

### Viewer

**Read-only access**:

- ✅ View workspaces and mocks
- ✅ View history and snapshots
- ❌ Cannot create or edit mocks
- ❌ Cannot manage workspace

**Use Case:** Stakeholders, product managers, external reviewers

## Permission System

### Available Permissions

MockForge defines **17 granular permissions**:

#### Workspace Permissions

- `WorkspaceCreate` - Create new workspaces
- `WorkspaceRead` - View workspace details
- `WorkspaceUpdate` - Modify workspace settings
- `WorkspaceDelete` - Delete workspaces
- `WorkspaceArchive` - Archive workspaces
- `WorkspaceManageMembers` - Add/remove workspace members

#### Mock/Route Permissions

- `MockCreate` - Create new mocks
- `MockRead` - View mocks
- `MockUpdate` - Modify existing mocks
- `MockDelete` - Delete mocks

#### Collaboration Permissions

- `InviteMembers` - Send workspace invitations
- `RemoveMembers` - Remove members from workspace
- `ChangeRoles` - Modify member roles

#### History Permissions

- `ViewHistory` - View commit history
- `CreateSnapshot` - Create named snapshots
- `RestoreSnapshot` - Restore from snapshots

#### Settings Permissions

- `ManageSettings` - Modify workspace settings
- `ManageIntegrations` - Configure integrations

### Permission Checking

Permissions are automatically checked on all operations:

```rust
use mockforge_collab::permissions::{Permission, PermissionChecker, UserRole};

// Check if user can create a mock
PermissionChecker::check(UserRole::Editor, Permission::MockCreate)?;

// Check if user can manage members
PermissionChecker::check(UserRole::Editor, Permission::WorkspaceManageMembers)?;
// Returns error: Editor role doesn't have WorkspaceManageMembers permission
```

## Configuration

### Enable RBAC

RBAC is enabled by default when using the collaboration system:

```yaml
# config.yaml
collaboration:
  enabled: true
  database_url: "sqlite://mockforge-collab.db"
  jwt_secret: "${JWT_SECRET}"

  # Role configuration
  roles:
    admin:
      permissions:
        - "scenarios:*"
        - "tenants:*"
        - "system:*"

    editor:
      permissions:
        - "scenarios:read"
        - "scenarios:execute"
        - "fixtures:*"

    viewer:
      permissions:
        - "scenarios:read"
        - "fixtures:read"
        - "metrics:read"
```

### Environment Variables

```bash
export MOCKFORGE_COLLAB_ENABLED=true
export MOCKFORGE_COLLAB_DATABASE_URL=sqlite://mockforge-collab.db
export MOCKFORGE_COLLAB_JWT_SECRET=your-secure-secret
```

## User Management

### Creating Users

```rust
use mockforge_collab::{CollabServer, models::User};

// Create a new user
let user = User::create(
    "username",
    "user@example.com",
    "password123",
    "Display Name"
).await?;
```

### Assigning Roles

When adding a user to a workspace, assign a role:

```rust
use mockforge_collab::{CollabServer, models::{UserRole, WorkspaceMember}};

// Add user to workspace with Editor role
let member = WorkspaceMember::create(
    workspace_id,
    user_id,
    UserRole::Editor
).await?;
```

### API Endpoints

#### Register User

```bash
POST /api/auth/register
Content-Type: application/json

{
  "username": "newuser",
  "email": "user@example.com",
  "password": "secure-password",
  "display_name": "New User"
}
```

#### Login

```bash
POST /api/auth/login
Content-Type: application/json

{
  "username": "newuser",
  "password": "secure-password"
}

# Response:
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 86400
}
```

#### Add Member to Workspace

```bash
POST /api/workspaces/{workspace_id}/members
Authorization: Bearer {token}
Content-Type: application/json

{
  "user_id": "user-uuid",
  "role": "editor"
}
```

## Authentication

### JWT Tokens

MockForge uses JWT (JSON Web Tokens) for authentication:

- **Algorithm**: HS256 or RS256
- **Expiration**: 24 hours (configurable)
- **Claims**: User ID, username, role

### Token Usage

Include the token in requests:

```bash
curl -H "Authorization: Bearer {token}" \
  http://localhost:8080/api/workspaces
```

### Token Refresh

Tokens can be refreshed before expiration:

```bash
POST /api/auth/refresh
Authorization: Bearer {token}

# Response:
{
  "access_token": "new-token",
  "expires_in": 86400
}
```

## Examples

### Basic Workflow

1. **Create Workspace** (Admin only)

```bash
POST /api/workspaces
Authorization: Bearer {admin-token}

{
  "name": "My Workspace",
  "description": "Team workspace"
}
```

2. **Invite Member** (Admin only)

```bash
POST /api/workspaces/{id}/invitations
Authorization: Bearer {admin-token}

{
  "email": "editor@example.com",
  "role": "editor"
}
```

3. **Create Mock** (Editor or Admin)

```bash
POST /api/workspaces/{id}/mocks
Authorization: Bearer {editor-token}

{
  "path": "/api/users",
  "method": "GET",
  "response": {
    "status": 200,
    "body": {"users": []}
  }
}
```

4. **View Workspace** (Any authenticated user)

```bash
GET /api/workspaces/{id}
Authorization: Bearer {viewer-token}
```

### Permission Enforcement

All operations automatically check permissions:

```rust
// This will fail if user doesn't have MockCreate permission
let mock = workspace.create_mock(mock_config, user_id).await?;
// PermissionChecker automatically validates permissions
```

## Integration with Admin UI

The Admin UI v2 includes complete RBAC support:

### UI Features

- **Role Selection**: Choose role when inviting users
- **Permission Indicators**: Visual indicators for user permissions
- **Access Control**: UI elements hidden based on permissions
- **User Management**: Add/remove members with role assignment

### Frontend Integration

```typescript
import { useAuth } from '@/hooks/useAuth';
import { Permission } from '@/types/permissions';

function MockEditor() {
  const { user, hasPermission } = useAuth();

  const canEdit = hasPermission(Permission.MockUpdate);

  return (
    <div>
      {canEdit ? (
        <EditButton />
      ) : (
        <ReadOnlyIndicator />
      )}
    </div>
  );
}
```

## Security Considerations

### Password Security

- **Argon2 Hashing**: Passwords hashed with Argon2 (OWASP recommended)
- **Salt Generation**: Automatic salt generation per password
- **No Plaintext Storage**: Passwords never stored in plaintext

### Token Security

- **Secure Secrets**: Use strong, random JWT secrets
- **Token Expiration**: Tokens expire after 24 hours
- **HTTPS Only**: Use HTTPS in production to protect tokens

### Best Practices

1. **Principle of Least Privilege**: Assign minimum required permissions
2. **Regular Audits**: Review user roles and permissions regularly
3. **Secure Secrets**: Store JWT secrets securely (environment variables, secrets manager)
4. **Token Rotation**: Implement token rotation for long-lived sessions
5. **Audit Logging**: Enable audit logging for all permission changes

## Troubleshooting

### Common Issues

#### "Permission Denied" Errors

**Error:**
```
Permission denied: Editor role does not have WorkspaceManageMembers permission
```

**Solution:**
- Check user role: `GET /api/users/me`
- Verify required permission for the operation
- Assign appropriate role or request admin to grant permission

#### Token Expired

**Error:**
```
Token expired: Please refresh your token
```

**Solution:**
- Refresh token: `POST /api/auth/refresh`
- Re-login: `POST /api/auth/login`
- Check token expiration time

#### Invalid Token

**Error:**
```
Invalid token: Token verification failed
```

**Solution:**
- Verify JWT secret matches server configuration
- Check token format (should be valid JWT)
- Ensure token hasn't been tampered with

## Related Documentation

- [Collaboration Guide](../crates/mockforge-collab/README.md) - Complete collaboration features
- [Security Guide](../book/src/user-guide/security.md) - General security features
- [Authentication](../docs/AUTHENTICATION.md) - Authentication details

## Support

For issues or questions:
- [GitHub Issues](https://github.com/SaaSy-Solutions/mockforge/issues)
- [GitHub Discussions](https://github.com/SaaSy-Solutions/mockforge/discussions)
- [Discord](https://discord.gg/2FxXqKpa)

---

**Last Updated:** 2025-01-27
