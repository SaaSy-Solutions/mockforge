# MockForge Workspace Sync

This document describes how to use MockForge's workspace synchronization feature to mirror workspace data to external directories for integration with Git repositories or cloud storage services like Dropbox.

## Overview

The `mockforge workspace sync` command allows you to export your workspace data to external directories with various organizational structures and sync strategies. This is perfect for:

- **Git Integration**: Version control your API configurations
- **Cloud Backup**: Sync to Dropbox, Google Drive, or other cloud storage
- **Team Collaboration**: Share workspace configurations across team members
- **Backup & Recovery**: Maintain external copies of your workspace data

## Basic Usage

```bash
# Sync all workspaces to a directory
mockforge workspace sync --target-dir /path/to/sync

# Sync to current directory's git-sync folder
mockforge workspace sync --target-dir ./git-sync

# Preview what would be synced (dry run)
mockforge workspace sync --target-dir ./preview --dry-run
```

## Directory Structures

### Flat Structure (`--structure flat`)
All workspaces are exported as individual YAML files in the target directory:
```
target-dir/
├── workspace-1.yaml
├── workspace-2.yaml
└── .mockforge-meta.json
```

### Nested Structure (`--structure nested`) - *Default*
Each workspace gets its own directory with organized files:
```
target-dir/
├── workspace-1/
│   ├── workspace.yaml
│   └── requests/
│       ├── get-users.yaml
│       ├── create-user.yaml
│       └── update-profile.yaml
├── workspace-2/
│   ├── workspace.yaml
│   └── requests/
└── .mockforge-meta.json
```

### Grouped Structure (`--structure grouped`)
Workspaces and requests are organized by type:
```
target-dir/
├── workspaces/
│   ├── workspace-1.yaml
│   └── workspace-2.yaml
├── requests/
│   ├── workspace-1/
│   │   ├── get-users.yaml
│   │   └── create-user.yaml
│   └── workspace-2/
└── .mockforge-meta.json
```

## Sync Strategies

### Full Sync (`--strategy full`) - *Default*
Syncs all available workspaces completely.

### Incremental Sync (`--strategy incremental`)
Only syncs workspaces that have been modified since the last sync (planned feature).

### Selective Sync (`--strategy selective`)
Only syncs specific workspaces:
```bash
mockforge workspace sync --target-dir ./sync \
  --strategy selective \
  --workspaces "workspace-1,workspace-2,user-api"
```

## Configuration Options

### Custom Filename Patterns
Use placeholders to customize file naming:
```bash
# Use workspace name
mockforge workspace sync --target-dir ./sync --filename-pattern "{name}"

# Include timestamp
mockforge workspace sync --target-dir ./sync --filename-pattern "{name}_{timestamp}"

# Use workspace ID
mockforge workspace sync --target-dir ./sync --filename-pattern "{id}"
```

### Exclude Patterns
Skip workspaces matching a regex pattern:
```bash
# Exclude test workspaces
mockforge workspace sync --target-dir ./sync --exclude ".*test.*"

# Exclude by ID pattern
mockforge workspace sync --target-dir ./sync --exclude "^temp-.*"
```

### Force Overwrite
```bash
# Overwrite existing files
mockforge workspace sync --target-dir ./sync --force
```

### Include Metadata
```bash
# Include Git-friendly metadata files
mockforge workspace sync --target-dir ./sync --include-meta
```

## Makefile Targets

The project includes convenient Makefile targets for common sync scenarios:

```bash
# Sync to a git repository directory
make sync-git

# Sync to Dropbox
make sync-dropbox

# Sync specific workspaces
make sync-selective

# Preview sync without executing
make sync-dry-run
```

## Examples

### Git Repository Integration

1. Create a dedicated git repository for your API configurations:
```bash
mkdir api-configs
cd api-configs
git init
```

2. Sync your MockForge workspaces:
```bash
mockforge workspace sync --target-dir . --structure nested --include-meta
```

3. Commit and push:
```bash
git add .
git commit -m "Sync MockForge workspaces"
git push origin main
```

### Dropbox Backup

```bash
# Sync to Dropbox with grouped structure
mockforge workspace sync \
  --target-dir ~/Dropbox/MockForge-Backup \
  --structure grouped \
  --force \
  --include-meta
```

### Selective Team Sharing

```bash
# Share only production workspaces
mockforge workspace sync \
  --target-dir ./team-share \
  --strategy selective \
  --workspaces "prod-api,prod-db,shared-auth" \
  --structure nested
```

## Output Format

### Workspace Export Format
Each workspace is exported as clean YAML with Git-friendly structure:

```yaml
metadata:
  id: "workspace-123"
  name: "User API"
  description: "User management endpoints"
  exported_at: "2025-09-18T10:30:00Z"
  request_count: 15
  folder_count: 3

config:
  auth:
    auth_type: "bearer"
    params:
      token: "${AUTH_TOKEN}"
  base_url: "https://api.example.com"
  variables:
    version: "v1"

requests:
  get-users:
    id: "req-123"
    name: "Get Users"
    method: "GET"
    path: "/users"
    folder_path: "users"
    headers:
      Accept: "application/json"
    query_params: {}
    response_status: 200
    response_body: null
```

### Metadata File
When `--include-meta` is used, a `.mockforge-meta.json` file is created:

```json
{
  "workspace_id": "workspace-123",
  "workspace_name": "User API",
  "description": "User management endpoints",
  "exported_at": "2025-09-18T10:30:00Z",
  "structure": "Nested",
  "version": "1.0",
  "source": "mockforge"
}
```

## Best Practices

1. **Use Nested Structure for Git**: Provides better organization and diff visibility
2. **Include Metadata**: Helps track export information and source
3. **Use Selective Sync**: For team sharing or partial backups
4. **Regular Sync**: Set up automated sync for important workspaces
5. **Version Control**: Commit exported files to track configuration changes
6. **Exclude Sensitive Data**: Use exclude patterns for test/development workspaces

## Troubleshooting

### Common Issues

**Permission Denied**: Ensure write access to the target directory
**Path Not Found**: The target directory will be created automatically
**Invalid Workspace IDs**: Use `mockforge workspace list` to see available workspaces
**Regex Errors**: Test your exclude patterns with a regex tester

### Recovery

If you need to restore workspaces from synced files, you can:
1. Use the existing import functionality
2. Manually recreate workspaces from the YAML files
3. Restore from Git history if using version control

## Future Enhancements

- **Automated Sync**: Scheduled synchronization
- **Change Detection**: Only sync modified workspaces
- **Conflict Resolution**: Handle merge conflicts in synced files
- **Encryption**: Secure sensitive configuration data
- **Webhooks**: Trigger sync on workspace changes
