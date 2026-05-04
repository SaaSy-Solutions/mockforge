# Cloud Sync for Local Development - Complete Implementation Guide

Comprehensive guide for implementing cloud sync in MockForge, enabling seamless synchronization between local development environments and cloud workspaces for team collaboration.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [CLI Integration](#cli-integration)
- [File Watching & Auto-Sync](#file-watching--auto-sync)
- [Real-Time Collaboration](#real-time-collaboration)
- [Conflict Resolution](#conflict-resolution)
- [Team Collaboration Features](#team-collaboration-features)
- [API Reference](#api-reference)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

---

## Overview

Cloud sync enables developers to:

- **Sync Local Workspaces**: Automatically sync local mock configurations with cloud workspaces
- **Team Collaboration**: Work together on the same mocks in real-time
- **Version Control**: Track changes with Git-style history
- **Conflict Resolution**: Handle concurrent edits intelligently
- **Offline Support**: Work offline and sync when connected

### Key Features

✅ **Bidirectional Sync**: Local ↔ Cloud synchronization
✅ **Real-Time Updates**: WebSocket-based live collaboration
✅ **File Watching**: Automatic sync on file changes
✅ **Conflict Resolution**: Multiple merge strategies
✅ **Version History**: Git-style commit history
✅ **Offline Mode**: Queue changes when offline
✅ **Selective Sync**: Sync specific workspaces only

---

## Architecture

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                    Local Development                         │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐      ┌──────────────┐      ┌──────────┐  │
│  │   File       │      │   Sync       │      │  Collab  │  │
│  │   Watcher    │─────▶│   Service    │─────▶│  Client  │  │
│  │              │      │              │      │          │  │
│  └──────────────┘      └──────────────┘      └────┬─────┘  │
│                                                     │        │
│  ┌──────────────┐      ┌──────────────┐           │        │
│  │   Workspace  │      │   Conflict   │           │        │
│  │   Manager    │─────▶│   Resolver   │           │        │
│  │              │      │              │           │        │
│  └──────────────┘      └──────────────┘           │        │
│                                                     │        │
└─────────────────────────────────────────────────────┼────────┘
                                                      │
                                                      │ WebSocket
                                                      │ HTTP/REST
                                                      │
┌─────────────────────────────────────────────────────▼────────┐
│                    Cloud Service                              │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐      ┌──────────────┐      ┌──────────┐  │
│  │   Collab     │      │   Sync       │      │  History │  │
│  │   Server     │─────▶│   Engine     │─────▶│  Tracker │  │
│  │              │      │              │      │          │  │
│  └──────────────┘      └──────────────┘      └──────────┘  │
│         │                      │                             │
│         │                      │                             │
│  ┌──────▼──────┐      ┌───────▼──────┐                      │
│  │   Event     │      │   Database   │                      │
│  │   Bus       │      │   (Postgres) │                      │
│  │             │      │              │                      │
│  └─────────────┘      └──────────────┘                      │
│                                                               │
└───────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **Local File Change** → File watcher detects change
2. **Sync Service** → Processes change and queues sync
3. **Collab Client** → Sends change to cloud via WebSocket
4. **Sync Engine** → Processes change and broadcasts to team
5. **Database** → Stores change in version history
6. **Other Clients** → Receive real-time updates

---

## CLI Integration

### Cloud Login

Authenticate with MockForge Cloud:

```bash
# Login with email/password
mockforge cloud login

# Login with API token
mockforge cloud login --token <api-token>

# Login with OAuth (GitHub/Google)
mockforge cloud login --provider github

# Check login status
mockforge cloud whoami
```

### Sync Commands

**Start Cloud Sync:**

```bash
# Sync specific workspace
mockforge cloud sync --workspace my-workspace

# Sync all workspaces
mockforge cloud sync --all

# Sync with specific project
mockforge cloud sync --project my-project --workspace my-workspace

# Sync in watch mode (auto-sync on file changes)
mockforge cloud sync --workspace my-workspace --watch

# Sync with conflict resolution strategy
mockforge cloud sync --workspace my-workspace --strategy theirs
```

**Sync Direction:**

```bash
# Upload local to cloud (default)
mockforge cloud sync --workspace my-workspace --direction up

# Download cloud to local
mockforge cloud sync --workspace my-workspace --direction down

# Bidirectional sync
mockforge cloud sync --workspace my-workspace --direction both
```

**Sync Status:**

```bash
# Check sync status
mockforge cloud sync status

# View sync history
mockforge cloud sync history --workspace my-workspace

# View pending changes
mockforge cloud sync pending
```

### Workspace Management

```bash
# List cloud workspaces
mockforge cloud workspace list

# Create cloud workspace
mockforge cloud workspace create my-workspace --name "My Workspace"

# Link local workspace to cloud
mockforge cloud workspace link local-workspace cloud-workspace-id

# Unlink workspace
mockforge cloud workspace unlink local-workspace

# View workspace info
mockforge cloud workspace info cloud-workspace-id
```

### Team Collaboration

```bash
# List team members
mockforge cloud team members --workspace my-workspace

# Invite team member
mockforge cloud team invite user@example.com --workspace my-workspace --role editor

# Remove team member
mockforge cloud team remove user@example.com --workspace my-workspace

# View workspace activity
mockforge cloud activity --workspace my-workspace
```

---

## File Watching & Auto-Sync

### Configuration

Create `.mockforge/sync.yaml`:

```yaml
# Cloud sync configuration
sync:
  enabled: true
  provider: cloud
  service_url: https://api.mockforge.dev
  api_key: ${MOCKFORGE_API_KEY}

  # Workspace mappings
  workspaces:
    - local: ./workspaces/local-api
      cloud: workspace-abc123
      watch: true
      auto_sync: true
      sync_direction: bidirectional

    - local: ./workspaces/prod-api
      cloud: workspace-def456
      watch: false
      auto_sync: false
      sync_direction: local_to_remote

  # File watching
  watch:
    enabled: true
    debounce_ms: 500
    ignore_patterns:
      - "**/.git/**"
      - "**/node_modules/**"
      - "**/*.tmp"
      - "**/.mockforge/**"

  # Conflict resolution
  conflict:
    default_strategy: merge
    auto_resolve: true
    backup_on_conflict: true

  # Sync behavior
  behavior:
    retry_attempts: 3
    retry_delay_ms: 1000
    batch_size: 10
    max_queue_size: 1000
```

### Implementation

```rust
// File watcher with cloud sync integration
pub struct CloudSyncWatcher {
    sync_service: Arc<SyncService>,
    collab_client: Arc<CollabClient>,
    config: SyncConfig,
    file_watcher: notify::RecommendedWatcher,
}

impl CloudSyncWatcher {
    pub async fn new(config: SyncConfig) -> Result<Self> {
        // Initialize sync service
        let sync_service = Arc::new(SyncService::new(&config)?);

        // Initialize collaboration client
        let client_config = ClientConfig {
            server_url: config.service_url.clone(),
            auth_token: config.api_key.clone(),
            ..Default::default()
        };
        let collab_client = Arc::new(CollabClient::connect(client_config).await?);

        // Initialize file watcher
        let (tx, mut rx) = mpsc::channel(100);
        let file_watcher = notify::recommended_watcher(move |event| {
            if let Ok(event) = event {
                let _ = tx.try_send(event);
            }
        })?;

        let watcher = Self {
            sync_service,
            collab_client,
            config,
            file_watcher,
        };

        // Start processing file events
        watcher.start_event_processor(rx).await;

        Ok(watcher)
    }

    async fn start_event_processor(&self, mut rx: mpsc::Receiver<notify::Event>) {
        let sync_service = self.sync_service.clone();
        let collab_client = self.collab_client.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut debouncer = Debouncer::new(Duration::from_millis(config.watch.debounce_ms));

            while let Some(event) = rx.recv().await {
                // Filter ignored paths
                if Self::should_ignore(&event.paths, &config.watch.ignore_patterns) {
                    continue;
                }

                // Debounce events
                debouncer.add_event(event);

                if let Some(debounced_events) = debouncer.flush().await {
                    // Process file changes
                    for event in debounced_events {
                        Self::handle_file_change(
                            &sync_service,
                            &collab_client,
                            &config,
                            event,
                        ).await;
                    }
                }
            }
        });
    }

    async fn handle_file_change(
        sync_service: &SyncService,
        collab_client: &CollabClient,
        config: &SyncConfig,
        event: notify::Event,
    ) {
        for path in event.paths {
            // Find workspace mapping
            if let Some(workspace_mapping) = config.find_workspace_mapping(&path) {
                // Load workspace from file
                if let Ok(workspace) = sync_service.load_workspace_from_file(&path).await {
                    // Sync to cloud
                    match workspace_mapping.sync_direction {
                        SyncDirection::LocalToRemote | SyncDirection::Bidirectional => {
                            if let Err(e) = collab_client.push_workspace_update(
                                &workspace_mapping.cloud_id,
                                &workspace,
                            ).await {
                                eprintln!("Failed to sync to cloud: {}", e);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
```

---

## Real-Time Collaboration

### WebSocket Integration

The collaboration client provides real-time updates:

```rust
// Subscribe to workspace updates
let client = CollabClient::connect(client_config).await?;

// Subscribe to workspace
client.subscribe_to_workspace("workspace-abc123").await?;

// Register callback for updates
client.on_workspace_update(Box::new(|event: ChangeEvent| {
    match event {
        ChangeEvent::MockCreated { mock_id, mock } => {
            println!("New mock created: {}", mock_id);
            // Update local workspace
        }
        ChangeEvent::MockUpdated { mock_id, changes } => {
            println!("Mock updated: {}", mock_id);
            // Apply changes to local workspace
        }
        ChangeEvent::MockDeleted { mock_id } => {
            println!("Mock deleted: {}", mock_id);
            // Remove from local workspace
        }
        ChangeEvent::UserJoined { user_id, username } => {
            println!("{} joined the workspace", username);
        }
        ChangeEvent::UserLeft { user_id, username } => {
            println!("{} left the workspace", username);
        }
    }
})).await;

// Push local changes
client.push_workspace_update("workspace-abc123", &workspace).await?;
```

### Presence Awareness

Track who's working in the workspace:

```rust
// Get active users
let active_users = client.get_active_users("workspace-abc123").await?;
for user in active_users {
    println!("{} is active (last seen: {})", user.username, user.last_seen);
}

// Set user presence
client.set_presence("workspace-abc123", Presence::Active).await?;
```

---

## Conflict Resolution

### Conflict Detection

Conflicts are detected when:
- Local and remote changes modify the same resource
- Changes occur while offline
- Multiple users edit simultaneously

### Resolution Strategies

**1. Merge (Default):**

```rust
// Automatic three-way merge
let merged = conflict_resolver.merge(
    &base_version,
    &local_changes,
    &remote_changes,
)?;
```

**2. Local Wins:**

```bash
mockforge cloud sync --workspace my-workspace --strategy local
```

**3. Remote Wins:**

```bash
mockforge cloud sync --workspace my-workspace --strategy remote
```

**4. Manual Resolution:**

```bash
# View conflicts
mockforge cloud sync conflicts --workspace my-workspace

# Resolve conflict interactively
mockforge cloud sync resolve --workspace my-workspace --conflict conflict-id
```

### Implementation

```rust
pub struct ConflictResolver {
    merge_service: Arc<MergeService>,
}

impl ConflictResolver {
    pub async fn resolve_conflict(
        &self,
        conflict: Conflict,
        strategy: ConflictStrategy,
    ) -> Result<ResolvedConflict> {
        match strategy {
            ConflictStrategy::Merge => {
                // Three-way merge
                let merged = self.merge_service.three_way_merge(
                    &conflict.base,
                    &conflict.local,
                    &conflict.remote,
                ).await?;

                Ok(ResolvedConflict {
                    resolution: merged,
                    strategy: ConflictStrategy::Merge,
                })
            }
            ConflictStrategy::Local => {
                Ok(ResolvedConflict {
                    resolution: conflict.local,
                    strategy: ConflictStrategy::Local,
                })
            }
            ConflictStrategy::Remote => {
                Ok(ResolvedConflict {
                    resolution: conflict.remote,
                    strategy: ConflictStrategy::Remote,
                })
            }
            ConflictStrategy::Manual => {
                // Save conflict for manual resolution
                self.save_conflict_for_manual_resolution(conflict).await?;
                Err(ConflictError::RequiresManualResolution)
            }
        }
    }
}
```

---

## Team Collaboration Features

### Workspace Sharing

**Invite Team Members:**

```bash
# Invite by email
mockforge cloud team invite user@example.com \
  --workspace my-workspace \
  --role editor

# Invite with custom permissions
mockforge cloud team invite user@example.com \
  --workspace my-workspace \
  --role editor \
  --permissions "create,update,delete"
```

**Manage Roles:**

- **Admin**: Full access including workspace management
- **Editor**: Can create, edit, and delete mocks
- **Viewer**: Read-only access

### Activity Feed

View workspace activity:

```bash
# View recent activity
mockforge cloud activity --workspace my-workspace

# Filter by user
mockforge cloud activity --workspace my-workspace --user user@example.com

# Filter by action
mockforge cloud activity --workspace my-workspace --action created
```

### Comments & Discussions

```bash
# Add comment to mock
mockforge cloud comment add \
  --workspace my-workspace \
  --mock mock-id \
  --message "This looks good!"

# List comments
mockforge cloud comment list \
  --workspace my-workspace \
  --mock mock-id

# Resolve comment
mockforge cloud comment resolve \
  --workspace my-workspace \
  --comment comment-id
```

---

## API Reference

### REST API

**Sync Workspace:**

```http
POST /api/v1/sync/workspaces/{workspace_id}
Authorization: Bearer <token>
Content-Type: application/json

{
  "direction": "bidirectional",
  "strategy": "merge"
}
```

**Get Sync Status:**

```http
GET /api/v1/sync/workspaces/{workspace_id}/status
Authorization: Bearer <token>
```

**Get Conflicts:**

```http
GET /api/v1/sync/workspaces/{workspace_id}/conflicts
Authorization: Bearer <token>
```

**Resolve Conflict:**

```http
POST /api/v1/sync/conflicts/{conflict_id}/resolve
Authorization: Bearer <token>
Content-Type: application/json

{
  "strategy": "merge",
  "resolution": { ... }
}
```

### WebSocket API

**Subscribe to Workspace:**

```json
{
  "type": "subscribe",
  "workspace_id": "workspace-abc123"
}
```

**Push Update:**

```json
{
  "type": "change",
  "workspace_id": "workspace-abc123",
  "event": {
    "type": "mock_updated",
    "mock_id": "mock-123",
    "changes": { ... }
  }
}
```

**Receive Update:**

```json
{
  "type": "change",
  "workspace_id": "workspace-abc123",
  "event": {
    "type": "mock_created",
    "mock_id": "mock-456",
    "mock": { ... }
  },
  "user": {
    "id": "user-789",
    "username": "alice"
  }
}
```

---

## Best Practices

### 1. Workspace Organization

- **One workspace per API**: Keep related mocks together
- **Use naming conventions**: Consistent naming helps team collaboration
- **Document changes**: Add comments explaining significant changes

### 2. Sync Strategy

- **Use bidirectional sync**: For active collaboration
- **Use local-to-remote**: For one-way publishing
- **Use remote-to-local**: For pulling team changes

### 3. Conflict Prevention

- **Communicate changes**: Notify team before major changes
- **Use branches**: For experimental changes (future feature)
- **Regular syncs**: Sync frequently to minimize conflicts

### 4. Performance

- **Selective sync**: Only sync workspaces you're actively using
- **Batch changes**: Group related changes together
- **Offline queue**: Queue changes when offline, sync when connected

### 5. Security

- **Use API tokens**: Store tokens securely (environment variables)
- **Review permissions**: Regularly audit team member access
- **Encrypt sensitive data**: Use encryption for sensitive mock data

---

## Troubleshooting

### Common Issues

**1. Sync Fails with Authentication Error**

```bash
# Re-authenticate
mockforge cloud login

# Check token validity
mockforge cloud whoami
```

**2. Conflicts Not Resolving**

```bash
# View conflicts
mockforge cloud sync conflicts --workspace my-workspace

# Resolve manually
mockforge cloud sync resolve --workspace my-workspace --interactive
```

**3. File Changes Not Syncing**

```bash
# Check watch configuration
cat .mockforge/sync.yaml

# Restart sync daemon
mockforge cloud sync --workspace my-workspace --watch --restart
```

**4. WebSocket Connection Issues**

```bash
# Check connection status
mockforge cloud status

# Test connection
mockforge cloud ping

# View connection logs
mockforge cloud logs --workspace my-workspace
```

### Debug Mode

Enable verbose logging:

```bash
# Enable debug mode
MOCKFORGE_LOG=debug mockforge cloud sync --workspace my-workspace

# Save logs to file
mockforge cloud sync --workspace my-workspace --log-file sync.log
```

---

## Implementation Checklist

- [x] Cloud sync infrastructure (`workspace/sync.rs`)
- [x] Collaboration client/server (`mockforge-collab`)
- [x] File watching (`sync` command)
- [x] WebSocket real-time sync
- [x] Conflict resolution
- [ ] CLI cloud commands (in progress)
- [ ] Offline queue support
- [ ] Batch sync optimization
- [ ] Comments & discussions
- [ ] Activity feed

---

## Next Steps

1. **Implement CLI Commands**: Add `mockforge cloud` command suite
2. **Offline Support**: Queue changes when offline, sync when connected
3. **Batch Optimization**: Group multiple changes into single sync
4. **Comments System**: Add commenting and discussion features
5. **Activity Feed**: Implement activity tracking and feed

---

**Last Updated**: 2024-01-01
**Version**: 1.0
