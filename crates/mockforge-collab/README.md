# MockForge Collaboration

Cloud collaboration features for MockForge enabling teams to work together on mock environments in real-time.

## Features

### Team Workspaces
- **Shared Environments**: Create collaborative workspaces where team members can work together
- **Role-Based Access Control**: Three roles with different permission levels:
  - **Admin**: Full access including workspace management, member management, and all editing capabilities
  - **Editor**: Can create, edit, and delete mocks; cannot manage workspace settings or members
  - **Viewer**: Read-only access to view mocks and history

### Real-Time Synchronization
- **WebSocket-Based Sync**: Changes are broadcast instantly to all connected users
- **Presence Awareness**: See who else is currently working in the workspace
- **Cursor Tracking**: View where other users are editing in real-time
- **Automatic Conflict Resolution**: Intelligent merge strategies for concurrent edits

### Version Control
- **Git-Style History**: Every change creates a commit with full workspace snapshot
- **Named Snapshots**: Create tagged versions (like git tags) for important milestones
- **Time Travel**: Restore workspace to any previous commit or snapshot
- **Diff Viewing**: Compare changes between any two versions

### Self-Hosted Option
- **Run Your Own Server**: Deploy on your infrastructure for complete control
- **SQLite or PostgreSQL**: Choose your database backend
- **Simple Configuration**: Environment variables or config files
- **Docker Support**: Easy deployment with Docker/Kubernetes

### Compilation Notes

The crate uses SQLx with offline mode support. When installing from crates.io, the `.sqlx` query cache is included, so compilation works without a database connection. If you're building from source and encounter SQLx errors, you can either:

1. **Use the included query cache** (default): The published crate includes all cached queries
2. **Set `SQLX_OFFLINE=false`**: Compile with a database connection
3. **Prepare queries yourself**: Run `cargo sqlx prepare --database-url <your-database-url>`

## Quick Start

### Starting a Collaboration Server

```rust
use mockforge_collab::{CollabServer, CollabConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = CollabConfig {
        database_url: "sqlite://mockforge-collab.db".to_string(),
        bind_address: "127.0.0.1:8080".to_string(),
        jwt_secret: "your-secure-secret".to_string(),
        ..Default::default()
    };

    let server = CollabServer::new(config).await?;
    server.run("127.0.0.1:8080").await?;

    Ok(())
}
```

### Connecting as a Client

```rust
use mockforge_collab::{CollabClient, ClientConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ClientConfig {
        server_url: "ws://localhost:8080".to_string(),
        auth_token: "your-jwt-token".to_string(),
    };

    let client = CollabClient::connect(config).await?;
    client.subscribe_to_workspace("workspace-id").await?;

    // Client now receives real-time updates

    Ok(())
}
```

## Architecture

### Components

1. **Authentication Service**: JWT-based authentication with Argon2 password hashing
2. **Workspace Service**: CRUD operations for workspaces with permission checks
3. **Event Bus**: Broadcast channel for real-time change notifications
4. **Sync Engine**: Manages active connections and synchronization state
5. **History Tracker**: Version control with commits and snapshots
6. **Conflict Resolver**: Three-way merge with multiple strategies

### Database Schema

- **users**: User accounts with credentials and profile info
- **workspaces**: Team workspace definitions
- **workspace_members**: User-workspace relationships with roles
- **workspace_invitations**: Pending invitations to join workspaces
- **commits**: Version history with full snapshots
- **snapshots**: Named versions for easy restoration

## Configuration

### Environment Variables

```bash
MOCKFORGE_JWT_SECRET=your-secret-key
MOCKFORGE_DATABASE_URL=sqlite://mockforge-collab.db
MOCKFORGE_BIND_ADDRESS=0.0.0.0:8080
```

### Configuration File

```toml
[collab]
jwt_secret = "your-secret-key"
database_url = "postgresql://user:pass@localhost/mockforge"
bind_address = "0.0.0.0:8080"
max_connections_per_workspace = 100
auto_commit = true
```

## Security

- **Password Hashing**: Argon2 for secure password storage
- **JWT Tokens**: Industry-standard authentication
- **Role-Based Access**: Granular permission control
- **Database Encryption**: Optional encryption at rest
- **Audit Logging**: Track all changes with author information

## Deployment

### Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --package mockforge-collab

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/mockforge-collab /usr/local/bin/
CMD ["mockforge-collab"]
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mockforge-collab
spec:
  replicas: 3
  selector:
    matchLabels:
      app: mockforge-collab
  template:
    metadata:
      labels:
        app: mockforge-collab
    spec:
      containers:
      - name: mockforge-collab
        image: mockforge/collab:latest
        env:
        - name: MOCKFORGE_DATABASE_URL
          value: "postgresql://..."
```

## API Examples

### Create a Workspace

```rust
let workspace = workspace_service
    .create_workspace(
        "My Team Workspace".to_string(),
        Some("Shared mocks for our API".to_string()),
        user_id,
    )
    .await?;
```

### Add a Member

```rust
let member = workspace_service
    .add_member(workspace_id, admin_id, new_user_id, UserRole::Editor)
    .await?;
```

### Create a Snapshot

```rust
let snapshot = history
    .create_snapshot(
        workspace_id,
        "v1.0.0".to_string(),
        Some("Production release".to_string()),
        user_id,
    )
    .await?;
```

### Restore from Snapshot

```rust
let state = history
    .restore_snapshot(workspace_id, "v1.0.0")
    .await?;
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../../LICENSE-MIT))

at your option.
