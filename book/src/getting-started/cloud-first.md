# Cloud-First Onboarding

**Pillars:** [Cloud]

[Cloud] - Registry, orgs, governance, monetization, marketplace

## Start Here If...

You're a **team or organization** that needs collaboration, sharing, and governance. You want to share mock scenarios across teams, manage workspaces, and leverage the marketplace for pre-built scenarios.

Perfect for:
- Teams needing to share mock scenarios
- Organizations requiring workspace management
- Teams wanting to discover and use marketplace scenarios
- Organizations needing governance and access controls

## Quick Start: 5 Minutes

Use this path when you need a shared workspace, cloud sync, and basic team collaboration around mocks.

```bash
# Install MockForge CLI
cargo install mockforge-cli

# Login to MockForge Cloud
mockforge cloud login

# See which organizations are available to your account
mockforge org list

# Set the active organization context
mockforge org use my-org
```

### Option 1: Create a Shared Workspace

```bash
# Create a cloud workspace
mockforge cloud workspace create my-workspace --name "Shared API Mocks"

# Inspect workspace details
mockforge cloud workspace info my-workspace

# Invite a teammate
mockforge cloud team invite teammate@example.com --workspace my-workspace --role editor

# Link a local directory to the cloud workspace
mockforge cloud workspace link . my-workspace

# Start sync
mockforge cloud sync start --workspace my-workspace --watch
```

### Option 2: Manage Team Access and Activity

```bash
# List team members
mockforge cloud team members --workspace my-workspace

# Review recent activity
mockforge cloud activity --workspace my-workspace
```

### Option 3: Check Sync Health

```bash
# Current sync status
mockforge cloud sync status --workspace my-workspace

# Pending changes
mockforge cloud sync pending --workspace my-workspace

# Sync history
mockforge cloud sync history --workspace my-workspace
```

## Key Cloud Features

### 1. Organization Management

- **Organization context** - Switch between org scopes with `mockforge org use`
- **Shared workspaces** - Keep team mocks under a common cloud workspace
- **Basic access control flows** - Invite collaborators and manage roles
- **Activity visibility** - Review workspace-level team activity

### 2. Cloud Workspaces

- **Synchronization** - Sync local and cloud workspaces
- **Linking** - Connect a local project directory to a cloud workspace
- **History** - Inspect sync history and pending changes
- **Team collaboration** - Invite team members to a shared workspace

### 3. Advanced Cloud Surfaces

- **Federation** - Compose multi-workspace systems for larger environments
- **Analytics** - Query usage and coverage signals across workspaces
- **Pipelines** - Event-driven automation crate and integration surface
- **Marketplace and governance** - Present in the broader platform, but verify current deployment and command surface before standardizing team workflows around them

## Example: Team Workflow

```bash
# 1. Authenticate and set org context
mockforge cloud login
mockforge org use acme-corp

# 2. Create a shared cloud workspace
mockforge cloud workspace create api-mocks --name "API Mocks"

# 3. Link the local repo to that workspace
mockforge cloud workspace link . api-mocks

# 4. Invite teammates
mockforge cloud team invite teammate@example.com --workspace api-mocks --role editor

# 5. Start sync and keep changes flowing
mockforge cloud sync start --workspace api-mocks --watch
```

## Next Steps

1. **Set up your organization context**: Run `mockforge org list` and `mockforge org use`
2. **Create a shared workspace**: Use `mockforge cloud workspace create`
3. **Connect local work**: Use `mockforge cloud workspace link` and `mockforge cloud sync start`
4. **Invite collaborators**: Use `mockforge cloud team invite`
5. **Explore deeper cloud features carefully**: Federation, analytics, and pipelines exist in the docs and codebase, but should be validated against your deployed environment before they become standard operating workflow

## Cross-Pillar Exploration

Once you've mastered Cloud, explore these complementary pillars:

- **Add realism** → Explore [Reality](reality-first.md) features
- **Add validation** → Explore [Contracts](contracts-first.md) features
- **Improve workflow** → Explore [DevX](devx-first.md) features
- **Enhance with AI** → Explore [AI](ai-first.md) features

## Resources

- [Cloud Workspaces](../user-guide/cloud-workspaces.md)
- [Multi-Workspace Federation](../user-guide/cloud/federation.md)
- [Analytics Dashboard](../user-guide/cloud/analytics-dashboard.md)
- [MockOps Pipelines](../user-guide/cloud/mockops-pipelines.md)
