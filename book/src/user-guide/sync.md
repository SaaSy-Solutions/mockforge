# Directory Synchronization

MockForge's sync daemon enables automatic synchronization between workspace files and MockForge's internal storage, allowing you to work with your mock API definitions as files and keep them in version control.

## Overview

The sync daemon monitors a directory for `.yaml` and `.yml` files and automatically imports them into MockForge workspaces. This enables:

- **File-based workflows**: Edit workspace files with your favorite text editor
- **Version control**: Keep workspace definitions in Git
- **Team collaboration**: Share workspaces via Git repositories
- **Automated workflows**: CI/CD integration and automated deployment
- **Real-time feedback**: See exactly what's being synced as it happens

## How It Works

The sync daemon provides bidirectional synchronization:

1. **Monitors Directory**: Watches for file changes in the specified workspace directory
2. **Detects Changes**: Identifies created, modified, and deleted `.yaml`/`.yml` files
3. **Imports Automatically**: Parses and imports valid MockRequest files into workspaces
4. **Provides Feedback**: Shows clear, real-time output of all sync operations

### What Gets Synced

- **File Types**: Only `.yaml` and `.yml` files
- **File Format**: Files must be valid MockRequest YAML
- **Subdirectories**: Monitors all subdirectories recursively
- **Exclusions**: Skips hidden files (starting with `.`)

## Getting Started

### Starting the Sync Daemon

Use the CLI to start the sync daemon:

```bash
# Basic usage
mockforge sync --workspace-dir ./my-workspace

# Short form
mockforge sync -w ./my-workspace

# With custom configuration
mockforge sync --workspace-dir ./workspace --config sync-config.yaml
```

### What You'll See

When you start the sync daemon:

```
🔄 Starting MockForge Sync Daemon...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📁 Workspace directory: ./my-workspace

ℹ️  What the sync daemon does:
   • Monitors the workspace directory for .yaml/.yml file changes
   • Automatically imports new or modified request files
   • Syncs changes bidirectionally between files and workspace
   • Skips hidden files (starting with .)

🔍 Monitoring for file changes...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ Sync daemon started successfully!
💡 Press Ctrl+C to stop
```

### Real-time Feedback

As files change, you'll see detailed output:

```
🔄 Detected 1 file change in workspace 'default'
  ➕ Created: new-endpoint.yaml
     ✅ Successfully imported

🔄 Detected 2 file changes in workspace 'default'
  📝 Modified: user-api.yaml
     ✅ Successfully updated
  🗑️  Deleted: old-endpoint.yaml
     ℹ️  Auto-deletion from workspace is disabled
```

## Directory Organization

You can organize your workspace files however you like. The sync daemon monitors all subdirectories recursively:

```
my-workspace/
├── api-v1/
│   ├── users.yaml
│   ├── products.yaml
│   └── orders.yaml
├── api-v2/
│   ├── users.yaml
│   └── graphql.yaml
├── internal/
│   └── admin.yaml
└── shared/
    └── auth.yaml
```

All `.yaml` and `.yml` files will be monitored and imported automatically.

## File Format

Each file should contain a valid MockRequest in YAML format:

```yaml
id: "get-users"
name: "Get Users"
method: "GET"
path: "/api/users"
headers:
  Content-Type: "application/json"
response_status: 200
response_body: |
  [
    {"id": 1, "name": "Alice"},
    {"id": 2, "name": "Bob"}
  ]
```

## Usage Examples

### Git Integration

Keep your workspaces in version control:

```bash
# 1. Create a Git repository for your workspaces
mkdir api-mocks
cd api-mocks
git init

# 2. Start the sync daemon
mockforge sync --workspace-dir .

# 3. Create or edit workspace files
vim user-endpoints.yaml

# 4. Commit and push changes
git add .
git commit -m "Add user endpoints"
git push origin main

# 5. Team members can pull changes
# The sync daemon will automatically import updates
```

### Development Workflow

Use the sync daemon during active development:

```bash
# Terminal 1: Start sync daemon
mockforge sync --workspace-dir ./workspaces

# Terminal 2: Edit files
vim ./workspaces/new-feature.yaml

# Changes are automatically imported
# You'll see real-time feedback in Terminal 1
```

### CI/CD Integration

Automate workspace deployment:

```bash
#!/bin/bash
# deploy-mocks.sh

# Pull latest workspace definitions from Git
git pull origin main

# Start sync daemon in background
mockforge sync --workspace-dir ./workspaces &
SYNC_PID=$!

# Wait for initial sync
sleep 5

# Start MockForge server
mockforge serve --config mockforge.yaml

# Cleanup on exit
trap "kill $SYNC_PID" EXIT
```

## Best Practices

### 1. Use Version Control

Keep workspace files in Git for team collaboration:

```bash
# Create a .gitignore to exclude temporary files
echo ".DS_Store" >> .gitignore
echo "*.swp" >> .gitignore
echo "*.tmp" >> .gitignore

# Commit workspace definitions
git add *.yaml
git commit -m "Add workspace definitions"
```

### 2. Organize Files Logically

Structure your workspace files for clarity:

```
workspaces/
├── production/         # Production endpoints
│   ├── users-api.yaml
│   └── orders-api.yaml
├── staging/           # Staging endpoints
│   └── beta-features.yaml
└── development/       # Development/experimental
    └── new-feature.yaml
```

### 3. Use Descriptive Filenames

Name files based on what they contain:

```
✅ Good:
   - user-authentication.yaml
   - product-catalog-api.yaml
   - payment-processing.yaml

❌ Bad:
   - endpoint1.yaml
   - test.yaml
   - temp.yaml
```

### 4. Keep Sync Daemon Running

Run the sync daemon continuously during development:

```bash
# Use a terminal multiplexer like tmux
tmux new -s mockforge-sync
mockforge sync --workspace-dir ./workspaces

# Detach with Ctrl+B then D
# Reattach with: tmux attach -t mockforge-sync
```

### 5. Monitor Sync Output

Pay attention to the sync daemon's output:

- ✅ **Green checkmarks**: Files imported successfully
- ⚠️ **Warning icons**: Import failed, check file format
- 🔄 **Change notifications**: Shows what's being synced
- ❌ **Error messages**: Indicate issues that need fixing

### 6. Handle Errors Promptly

When you see errors, fix them immediately:

```
❌ Detected error:
  📝 Modified: broken-endpoint.yaml
     ⚠️  Failed to import: File is not a recognized format

Action: Check the file syntax and fix YAML formatting
```

## Troubleshooting

### Files Not Being Imported

**Check file extension:**
```bash
# Only .yaml and .yml files are monitored
ls -la workspaces/
# Ensure files end with .yaml or .yml
```

**Verify file format:**
```bash
# Files must be valid MockRequest YAML
cat workspaces/my-file.yaml
# Check for proper YAML syntax and required fields
```

**Check for hidden files:**
```bash
# Hidden files (starting with .) are ignored
# Rename: .hidden.yaml → visible.yaml
mv .hidden.yaml visible.yaml
```

### Permission Errors

```bash
# Ensure MockForge can read the directory
chmod 755 workspaces/
chmod 644 workspaces/*.yaml

# Check ownership
ls -la workspaces/
```

### Changes Not Detected

**Verify sync daemon is running:**
```bash
# Check if the process is still active
ps aux | grep "mockforge sync"
```

**Check filesystem notifications:**
```bash
# Some network filesystems don't support notifications
# Try editing locally instead of over NFS/SMB
```

**Restart sync daemon:**
```bash
# Stop with Ctrl+C, then restart
mockforge sync --workspace-dir ./workspaces
```

### YAML Syntax Errors

When files fail to import due to syntax errors:

```bash
# Use a YAML validator
yamllint workspaces/problematic-file.yaml

# Common issues:
# - Incorrect indentation
# - Missing quotes around special characters
# - Invalid escape sequences
```

### Debug Logging

Enable detailed logging to see what's happening:

```bash
# Enable debug logs for sync watcher
RUST_LOG=mockforge_core::sync_watcher=debug mockforge sync --workspace-dir ./workspaces

# Enable trace-level logs for maximum detail
RUST_LOG=mockforge_core::sync_watcher=trace mockforge sync --workspace-dir ./workspaces

# Log to a file
RUST_LOG=mockforge_core::sync_watcher=debug mockforge sync --workspace-dir ./workspaces 2>&1 | tee sync.log
```

### Getting Help

If you're still having issues:

1. Check the sync daemon output for error messages
2. Enable debug logging to see detailed information
3. Verify file format matches MockRequest YAML structure
4. Check file permissions and ownership
5. Try with a minimal test file to isolate the issue

Example minimal test file:

```yaml
# test-endpoint.yaml
id: "test"
name: "Test Endpoint"
method: "GET"
path: "/test"
response_status: 200
response_body: '{"status": "ok"}'
```

Save this file in your workspace directory and verify it gets imported successfully.