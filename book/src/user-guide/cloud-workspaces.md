# Cloud Workspaces (Collaboration)

Cloud Workspaces is the clearest current path for team-oriented MockForge usage. It combines shared workspace records, cloud sync, and collaborator management so teams can keep mocks aligned across local development and shared environments.

## Overview

Cloud Workspaces provides:

- **Cloud authentication** via `mockforge cloud login`
- **Shared workspace records** with cloud workspace create/list/info flows
- **Directory linking** from a local project to a cloud workspace
- **Cloud sync** with status, history, and pending-change checks
- **Team membership management** through invite/list/remove commands

## Quick Start

### Authenticate and Set Org Context

```bash
# Authenticate with MockForge Cloud
mockforge cloud login

# Inspect available organizations
mockforge org list

# Set the active organization context
mockforge org use my-org
```

### Create and Inspect a Cloud Workspace

```bash
# Create a shared cloud workspace
mockforge cloud workspace create my-workspace --name "My Workspace"

# List cloud workspaces
mockforge cloud workspace list

# Inspect a workspace
mockforge cloud workspace info my-workspace
```

### Link a Local Project and Start Sync

```bash
# Link the current project directory to the cloud workspace
mockforge cloud workspace link . my-workspace

# Start bidirectional sync and watch for changes
mockforge cloud sync start --workspace my-workspace --watch
```

## Core Commands

### Authentication and Identity

- `mockforge cloud login`
- `mockforge cloud whoami`
- `mockforge cloud logout`

### Cloud Workspace Management

- `mockforge cloud workspace list`
- `mockforge cloud workspace create <workspace-id> --name "..."`
- `mockforge cloud workspace info <workspace-id>`
- `mockforge cloud workspace link <local-path> <cloud-workspace-id>`
- `mockforge cloud workspace unlink <local-path>`

### Team Membership

```bash
# Invite a collaborator
mockforge cloud team invite teammate@example.com \
  --workspace my-workspace \
  --role editor

# List collaborators
mockforge cloud team members --workspace my-workspace

# Remove a collaborator
mockforge cloud team remove teammate@example.com --workspace my-workspace
```

### Sync Operations

```bash
# Show sync status
mockforge cloud sync status --workspace my-workspace

# Show pending changes
mockforge cloud sync pending --workspace my-workspace

# Show sync history
mockforge cloud sync history --workspace my-workspace
```

## Notes on Readiness

- The cloud workspace and sync commands above are present in the CLI source and are the safest public workflow to document.
- Other collaboration surfaces exist in the codebase, but some older docs examples referenced command shapes that are not part of the current public CLI.
- If you are documenting a buyer-facing hosted workflow, prefer the verified `cloud`, `org`, and `workspace` flows above over speculative collaboration examples.

## Local Admin Workspaces vs Cloud Workspaces

MockForge also has local admin-oriented workspace commands such as:

- `mockforge workspace list`
- `mockforge workspace create <workspace-id> --name "..."`
- `mockforge workspace info <workspace-id>`

Those are useful for local or self-hosted admin flows, but they are distinct from the cloud collaboration commands documented above.

## Related Topics

- [Directory Synchronization](sync.md) - File-based sync outside cloud workflows
- [MockOps Pipelines](cloud/mockops-pipelines.md) - Advanced automation surface
- [Federation](cloud/federation.md) - Multi-workspace federation
- [Analytics Dashboard](cloud/analytics-dashboard.md) - Usage analytics
