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

Let's set up MockForge Cloud for team collaboration:

```bash
# Install MockForge CLI
cargo install mockforge-cli

# Login to MockForge Cloud
mockforge cloud login

# Create or join an organization
mockforge cloud org create my-org
# Or join an existing org
mockforge cloud org join my-org
```

### Option 1: Create a Shared Workspace

```bash
# Create a workspace in your organization
mockforge cloud workspace create my-workspace --org my-org

# Share the workspace with your team
mockforge cloud workspace share my-workspace --user teammate@example.com

# Sync your local config to the cloud
mockforge cloud sync --workspace my-workspace
```

### Option 2: Use Marketplace Scenarios

```bash
# Browse available scenarios
mockforge marketplace list

# Search for specific scenarios
mockforge marketplace search "user management"

# Install a scenario
mockforge marketplace install ecommerce-api-scenario

# Use the installed scenario
mockforge serve --scenario ecommerce-api-scenario
```

### Option 3: Publish Your Own Scenario

```bash
# Create a scenario from your config
mockforge scenario create my-scenario --from-config mockforge.yaml

# Publish to marketplace
mockforge marketplace publish my-scenario --org my-org

# Share with specific organizations
mockforge marketplace share my-scenario --org partner-org
```

## Key Cloud Features

### 1. Organization Management

- **Multi-tenant workspaces** - Isolate workspaces by organization
- **Team collaboration** - Share workspaces and scenarios
- **Access controls** - RBAC for fine-grained permissions
- **Audit logging** - Track all changes and access

### 2. Scenario Marketplace

- **Discover scenarios** - Browse pre-built mock scenarios
- **Publish scenarios** - Share your scenarios with the community
- **Version control** - Track scenario versions and updates
- **Ratings and reviews** - Community feedback on scenarios

### 3. Registry Server

- **Centralized registry** - Single source of truth for mocks
- **Distribution** - Easy sharing across teams
- **Versioning** - Manage mock versions centrally
- **Governance** - Control what gets published

### 4. Cloud Workspaces

- **Synchronization** - Sync local and cloud workspaces
- **Backup and restore** - Automatic backups of workspace configs
- **Collaboration** - Real-time collaboration on workspace configs
- **History** - Track changes over time

### 5. Governance & Access Control

- **Role-based access** - Fine-grained permissions
- **Organization policies** - Enforce standards across teams
- **Audit trails** - Complete audit logging
- **Compliance** - Meet regulatory requirements

## Example: Team Workflow

```bash
# 1. Team lead creates organization
mockforge cloud org create acme-corp

# 2. Team lead creates workspace
mockforge cloud workspace create api-mocks --org acme-corp

# 3. Team members join organization
mockforge cloud org join acme-corp

# 4. Team members clone workspace
mockforge cloud workspace clone api-mocks

# 5. Team members work locally
# ... make changes to mockforge.yaml ...

# 6. Team members sync changes
mockforge cloud sync --workspace api-mocks

# 7. Team lead reviews and approves
mockforge cloud workspace review api-mocks

# 8. Changes are deployed
mockforge cloud workspace deploy api-mocks
```

## Marketplace Examples

### E-commerce API Scenario

```bash
# Install e-commerce scenario
mockforge marketplace install ecommerce-api

# Use in your tests
mockforge serve --scenario ecommerce-api
```

### Payment Gateway Scenario

```bash
# Search for payment scenarios
mockforge marketplace search "payment"

# Install payment gateway scenario
mockforge marketplace install stripe-mock

# Use in integration tests
mockforge serve --scenario stripe-mock --port 3000
```

## Next Steps

1. **Set up your organization**: Create or join an organization
2. **Create workspaces**: Set up shared workspaces for your teams
3. **Explore marketplace**: Discover and use pre-built scenarios
4. **Publish scenarios**: Share your scenarios with the community
5. **Configure governance**: Set up access controls and policies

## Cross-Pillar Exploration

Once you've mastered Cloud, explore these complementary pillars:

- **Add realism** → Explore [Reality](reality-first.md) features
- **Add validation** → Explore [Contracts](contracts-first.md) features
- **Improve workflow** → Explore [DevX](devx-first.md) features
- **Enhance with AI** → Explore [AI](ai-first.md) features

## Resources

- [Cloud Documentation](../docs/cloud/GETTING_STARTED.md)
- [Marketplace Guide](../docs/SCENARIOS_MARKETPLACE.md)
- [Organization Management](../docs/cloud/API_REFERENCE.md)
- [RBAC Guide](../docs/RBAC_GUIDE.md)

