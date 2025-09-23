# Directory Synchronization

MockForge supports bidirectional synchronization between workspaces and external directories, allowing you to keep your mock API definitions in sync with your version control system or shared directories.

## Overview

Directory sync enables automatic synchronization of workspace changes with a specified directory on the filesystem. This is useful for:

- Version controlling your mock definitions
- Sharing workspaces across team members
- Maintaining consistency between development environments
- Automated deployment pipelines

## Sync Modes

### Bidirectional Sync
Changes in either the workspace or the directory are automatically synchronized. This is the default and recommended mode for most use cases.

### One-Way Sync (Workspace → Directory)
Changes in the workspace are automatically synced to the directory, but directory changes are ignored. Useful for read-only directories or when you want to maintain control from the UI.

### Manual Sync
Synchronization only occurs when manually triggered. No automatic monitoring or syncing.

## Setting Up Sync

### During Workspace Creation

When creating a new workspace, you can enable sync immediately:

1. Click "Create Workspace"
2. Fill in the workspace name and description
3. Check "Enable directory sync"
4. Enter the absolute path to the sync directory
5. Click "Create Workspace"

### For Existing Workspaces

To enable sync on an existing workspace:

1. Open the workspace
2. Click the "Settings" button in the workspace header
3. Configure the sync parameters:
   - **Target Directory**: Absolute path to sync with
   - **Sync Direction**: Choose bidirectional, one-way, or manual
   - **Real-time Monitoring**: Enable for automatic sync on file changes
   - **Directory Structure**: Choose how files are organized
   - **Filename Pattern**: Customize the naming pattern for synced files

### Opening from Directory

If you already have a directory with workspace files, you can open it directly:

1. Click "Open from Directory"
2. Enter the absolute path to the directory
3. MockForge will create a workspace and enable bidirectional sync

## Directory Structure Options

### Flat Structure
All workspaces are stored as individual YAML files in the root directory:
```
sync-directory/
├── workspace-1.yaml
├── workspace-2.yaml
└── workspace-3.yaml
```

### Nested Structure
Workspaces are organized in subdirectories:
```
sync-directory/
├── workspace-1/
│   ├── workspace.yaml
│   └── requests/
│       ├── endpoint-1.yaml
│       └── endpoint-2.yaml
└── workspace-2/
    ├── workspace.yaml
    └── requests/
        └── endpoint-3.yaml
```

## Environment Filtering

By default, MockForge excludes sensitive environments from sync to prevent accidental exposure of secrets:

- Environments marked as "not shared" are excluded
- Variables containing sensitive keywords (password, secret, key, token) are filtered out
- Only explicitly shared environments are included in synced files

## Sync Status and Controls

The sync status is displayed in the workspace header:

- **🟢 Synced**: Everything is up to date
- **🟡 Syncing**: Changes are being processed
- **🔴 Error**: Sync failed, check logs for details
- **⚪ Disabled**: Sync is disabled

Click the status indicator to:
- Manually trigger a sync
- View sync history
- Configure sync settings
- Disable sync temporarily

## Conflict Resolution

When bidirectional sync detects conflicting changes:

1. A confirmation dialog appears
2. You can choose to:
   - Apply workspace changes (overwrite directory)
   - Apply directory changes (overwrite workspace)
   - Merge changes manually
   - Skip this conflict

## CLI Integration

Use the CLI for headless sync operations:

```bash
# Start background sync daemon
mockforge sync start --directory /path/to/workspace

# Trigger manual sync
mockforge sync trigger --workspace-id <id>

# Check sync status
mockforge sync status --workspace-id <id>

# Stop sync
mockforge sync stop --workspace-id <id>
```

## Best Practices

### Version Control
- Always sync to a Git repository
- Use `.gitignore` to exclude sensitive files
- Commit sync changes regularly

### Team Collaboration
- Use shared directories for team workspaces
- Establish naming conventions for environments
- Document sync configurations in your team wiki

### Performance
- Enable real-time monitoring only when needed
- Use manual sync for large directories
- Monitor disk I/O for performance impact

### Security
- Never sync to directories containing secrets
- Use environment filtering to exclude sensitive data
- Regularly audit synced files for exposed credentials

## Troubleshooting

### Common Issues

**Sync fails with permission errors**
- Ensure MockForge has read/write access to the target directory
- Check file permissions and ownership

**Changes not syncing automatically**
- Verify real-time monitoring is enabled
- Check that the directory is accessible
- Restart the sync service

**Conflicting changes detected**
- Review the conflict resolution dialog
- Consider using one-way sync if conflicts are frequent
- Communicate with team members about concurrent changes

**Large directories cause performance issues**
- Switch to manual sync mode
- Use selective syncing with filename patterns
- Consider breaking large workspaces into smaller ones

### Logs and Debugging

Enable debug logging to troubleshoot sync issues:

```bash
RUST_LOG=mockforge_core::sync_watcher=debug mockforge
```

Check the application logs for detailed sync operation information.