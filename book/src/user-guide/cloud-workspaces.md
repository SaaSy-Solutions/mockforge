# Cloud Workspaces (Collaboration)

Cloud Workspaces enables multi-user collaborative editing with real-time state synchronization, version control, and role-based permissions. Work together on mock configurations with Git-style versioning and conflict resolution.

## Overview

Cloud Workspaces provides:

- **User Authentication**: JWT-based authentication with secure sessions
- **Multi-User Editing**: Real-time collaborative editing with presence awareness
- **State Synchronization**: WebSocket-based real-time sync between clients
- **Version Control**: Git-style version control for mocks and data
- **Change Tracking**: Full history with rollback capabilities
- **Role-Based Permissions**: Owner, Editor, and Viewer roles

## Quick Start

### Create a Workspace

```bash
# Create a new workspace
mockforge workspace create --name "My Workspace" --description "Team workspace"

# Or via API
curl -X POST http://localhost:9080/api/workspaces \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "name": "My Workspace",
    "description": "Team workspace"
  }'
```

### Join a Workspace

```bash
# List available workspaces
mockforge workspace list

# Join a workspace (requires invitation)
mockforge workspace join <workspace-id>
```

### Start Collaborative Server

```bash
# Start server with collaboration enabled
mockforge serve --collab-enabled --collab-port 8080
```

## Features

### User Authentication

#### Register

```bash
# Register new user
mockforge auth register \
  --email "user@example.com" \
  --password "secure-password" \
  --name "User Name"
```

#### Login

```bash
# Login and get JWT token
mockforge auth login \
  --email "user@example.com" \
  --password "secure-password"
```

### Workspace Management

#### Create Workspace

```bash
mockforge workspace create \
  --name "Team Workspace" \
  --description "Shared workspace for team"
```

#### List Workspaces

```bash
# List your workspaces
mockforge workspace list

# List all workspaces (admin only)
mockforge workspace list --all
```

#### Get Workspace Details

```bash
mockforge workspace get <workspace-id>
```

### Member Management

#### Add Member

```bash
# Add member to workspace
mockforge workspace member add \
  --workspace <workspace-id> \
  --user <user-id> \
  --role editor
```

#### List Members

```bash
# List workspace members
mockforge workspace member list --workspace <workspace-id>
```

#### Change Role

```bash
# Change member role
mockforge workspace member role \
  --workspace <workspace-id> \
  --user <user-id> \
  --role viewer
```

#### Remove Member

```bash
# Remove member from workspace
mockforge workspace member remove \
  --workspace <workspace-id> \
  --user <user-id>
```

### Real-Time Synchronization

Workspaces use WebSocket for real-time synchronization:

#### WebSocket Connection

```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

// Subscribe to workspace
ws.send(JSON.stringify({
  type: 'subscribe',
  workspace_id: 'workspace-uuid'
}));

// Receive updates
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  if (data.type === 'change') {
    console.log('Change event:', data.event);
  }
};
```

#### Change Events

- `mock_created` - New mock added
- `mock_updated` - Mock modified
- `mock_deleted` - Mock removed
- `workspace_updated` - Workspace settings changed
- `member_added` - New team member
- `member_removed` - Member left
- `role_changed` - Member role updated
- `snapshot_created` - New snapshot
- `user_joined` - User connected
- `user_left` - User disconnected
- `cursor_moved` - Cursor position updated

### Version Control

#### Create Snapshot

```bash
# Create workspace snapshot
mockforge workspace snapshot create \
  --workspace <workspace-id> \
  --message "Initial state"
```

#### List Snapshots

```bash
# List workspace snapshots
mockforge workspace snapshot list --workspace <workspace-id>
```

#### Restore Snapshot

```bash
# Restore workspace to snapshot
mockforge workspace snapshot restore \
  --workspace <workspace-id> \
  --snapshot <snapshot-id>
```

### Conflict Resolution

When multiple users edit simultaneously, conflicts are resolved automatically:

- **Last Write Wins**: Default strategy for simple conflicts
- **Merge Strategy**: Intelligent merging for compatible changes
- **Manual Resolution**: Manual conflict resolution for complex cases

## API Endpoints

### Authentication

```http
POST /auth/register
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "secure-password",
  "name": "User Name"
}
```

```http
POST /auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "secure-password"
}
```

### Workspaces

```http
POST /workspaces
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "My Workspace",
  "description": "Team workspace"
}
```

```http
GET /workspaces
Authorization: Bearer <token>
```

```http
GET /workspaces/:id
Authorization: Bearer <token>
```

```http
PUT /workspaces/:id
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "Updated Name",
  "description": "Updated description"
}
```

```http
DELETE /workspaces/:id
Authorization: Bearer <token>
```

### Members

```http
POST /workspaces/:id/members
Authorization: Bearer <token>
Content-Type: application/json

{
  "user_id": "user-uuid",
  "role": "editor"
}
```

```http
GET /workspaces/:id/members
Authorization: Bearer <token>
```

```http
PUT /workspaces/:id/members/:user_id/role
Authorization: Bearer <token>
Content-Type: application/json

{
  "role": "viewer"
}
```

```http
DELETE /workspaces/:id/members/:user_id
Authorization: Bearer <token>
```

## Role-Based Permissions

### Owner

- Full access to workspace
- Can delete workspace
- Can manage all members
- Can change any member's role

### Editor

- Can create, update, and delete mocks
- Can view all workspace content
- Cannot delete workspace
- Cannot manage members

### Viewer

- Can view workspace content
- Cannot modify anything
- Read-only access

## Configuration

### Server Configuration

```yaml
collab:
  enabled: true
  port: 8080
  database:
    type: "sqlite"  # or "postgres"
    path: "./collab.db"  # For SQLite
    connection_string: "postgresql://..."  # For PostgreSQL
  jwt:
    secret: "${JWT_SECRET}"
    expiration_hours: 24
```

### Client Configuration

```yaml
collab:
  server_url: "http://localhost:8080"
  workspace_id: "workspace-uuid"
  auto_sync: true
  sync_interval_ms: 1000
```

## Use Cases

### Team Development

Multiple developers working on the same mock configuration:

1. Create shared workspace
2. Invite team members
3. Edit mocks collaboratively
4. View changes in real-time

### Staging Environment

Shared staging environment with controlled access:

1. Create workspace for staging
2. Add team members as editors
3. Add stakeholders as viewers
4. Track all changes with version control

### Client Demos

Share mock environments with clients:

1. Create workspace for client
2. Add client as viewer
3. Update mocks as needed
4. Client sees changes in real-time

## Best Practices

1. **Use Appropriate Roles**: Assign roles based on responsibilities
2. **Regular Snapshots**: Create snapshots before major changes
3. **Monitor Conflicts**: Watch for conflict warnings
4. **Version Control**: Use snapshots for important milestones
5. **Secure Secrets**: Never commit JWT secrets to version control

## Troubleshooting

### Connection Issues

- Verify WebSocket endpoint is accessible
- Check firewall settings
- Review server logs for errors

### Sync Conflicts

- Review conflict resolution strategy
- Use manual resolution for complex cases
- Create snapshots before major changes

### Permission Errors

- Verify user role has required permissions
- Check workspace membership
- Review JWT token expiration

## Related Documentation

- [VBR Engine](vbr-engine.md) - State management
- [Scenario Marketplace](scenario-marketplace.md) - Sharing scenarios
- [Configuration Guide](../configuration/files.md) - Complete configuration reference

