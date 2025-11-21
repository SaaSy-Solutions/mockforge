# Cloud Environments & Governance

**Pillars:** [Cloud]

[Cloud] - Multi-tenant workspaces, governance, and team collaboration

## Overview

MockForge supports multi-environment mock workspaces with promotion workflows, environment-scoped RBAC, and comprehensive governance features. This enables teams to treat mocks as shared infrastructure with proper controls and workflows.

## Features

### 1. Multi-Environment Mock Workspaces

Each workspace can define multiple environments (dev, test, prod) with per-environment overrides for:
- **Reality Level**: Different reality levels per environment (e.g., light simulation in dev, high realism in prod)
- **Chaos Profiles**: Environment-specific chaos configurations
- **Drift Budgets**: Stricter budgets in prod, more lenient in dev

#### Creating Environments

Environments are automatically created when a workspace is initialized:

```rust
let workspace = Workspace::new("my-workspace".to_string());
// Automatically creates dev, test, and prod environments
```

#### Configuring Environments

```rust
workspace.set_mock_environment_config(
    MockEnvironmentName::Prod,
    Some(reality_config),
    Some(chaos_config),
    Some(drift_budget_config),
)?;
```

#### Switching Active Environment

```rust
workspace.set_active_mock_environment(MockEnvironmentName::Prod)?;
```

### 2. Promotion Workflow

Promote scenarios, personas, and configuration changes between environments with full audit trail and optional GitOps integration.

#### Promoting a Scenario

```rust
let request = PromotionRequest {
    entity_type: PromotionEntityType::Scenario,
    entity_id: "scenario-123".to_string(),
    entity_version: Some("v1.0.0".to_string()),
    workspace_id: workspace.id.clone(),
    from_environment: MockEnvironmentName::Dev,
    to_environment: MockEnvironmentName::Prod,
    requires_approval: true,
    comments: Some("Ready for production".to_string()),
    metadata: HashMap::new(),
};

let promotion_id = promotion_service.record_promotion(
    &request,
    user_id,
    PromotionStatus::Pending,
    Some(workspace_config_json),
).await?;
```

#### Promotion Status

Promotions can have the following statuses:
- **Pending**: Awaiting approval
- **Approved**: Approved but not yet completed
- **Rejected**: Promotion was rejected
- **Completed**: Successfully promoted
- **Failed**: Promotion failed

#### GitOps Integration

When GitOps is enabled, promotions automatically create Pull Requests:

```rust
let gitops_config = PromotionGitOpsConfig::new(
    true, // enabled
    PRProvider::GitHub,
    "myorg".to_string(),
    "mockforge-configs".to_string(),
    Some(token),
    "main".to_string(),
    Some("workspaces/{workspace_id}/config.yaml".to_string()),
);

let promotion_service = PromotionService::with_gitops(db, gitops_config);
```

The PR will include:
- Promotion details (entity type, ID, version)
- Environment transition (from → to)
- Comments and metadata
- Updated workspace configuration

### 3. RBAC Enhancements

Environment-scoped permissions allow fine-grained control over who can modify settings in specific environments.

#### Example: Platform-Only Prod Changes

```rust
let policy = EnvironmentPermissionPolicy::new(
    MockEnvironmentName::Prod,
    Permission::ScenarioModifyRealityDefaults,
    vec!["admin".to_string(), "platform".to_string()],
);

checker.add_policy(policy);
```

This ensures only Platform team members can change reality level defaults in production.

#### Permission Checking

```rust
if check_environment_permission(
    &checker,
    &user_role,
    Permission::ManageSettings,
    Some(MockEnvironmentName::Prod),
) {
    // User can modify settings in prod
}
```

#### Common Permission Patterns

- **Dev/Test**: Editors can modify most settings
- **Prod**: Only admins and platform team can modify reality defaults
- **Chaos Rules**: QA team can modify in test, restricted in prod
- **Drift Budgets**: Platform team controls prod budgets

### 4. Organization-Level Templates

Org admins can define templates for new workspaces, including:
- Standard security/drift/chaos defaults
- Recommended blueprints
- Environment configurations

#### Template Structure

```json
{
  "environments": {
    "dev": {
      "reality_level": "light_simulation",
      "chaos_config": {
        "enabled": true,
        "error_rate": 0.1
      },
      "drift_budget_config": {
        "enabled": true,
        "default_budget": {
          "max_breaking_changes": 5
        }
      }
    },
    "test": {
      "reality_level": "moderate_realism",
      "chaos_config": {
        "enabled": true,
        "error_rate": 0.05
      }
    },
    "prod": {
      "reality_level": "high_realism",
      "chaos_config": {
        "enabled": false
      },
      "drift_budget_config": {
        "enabled": true,
        "default_budget": {
          "max_breaking_changes": 0
        }
      }
    }
  },
  "default_reality_level": "moderate_realism",
  "security_baseline": {
    "default_validation_mode": "warn",
    "rbac_defaults": {
      "admin": ["*"],
      "editor": ["MockCreate", "MockUpdate", "MockRead"],
      "viewer": ["MockRead"]
    },
    "environment_permissions": {
      "prod": {
        "ManageSettings": ["admin", "platform"],
        "ScenarioModifyRealityDefaults": ["platform"]
      }
    }
  }
}
```

#### Applying Templates

```rust
let result = apply_template_to_workspace(
    &mut workspace,
    &template.blueprint_config,
    &template.security_baseline,
)?;
```

### 4.1.1 Pillar Tagging for Scenarios

Scenarios can be tagged with pillar tags to indicate which MockForge pillars they utilize. Pillar tags are formatted as `[PillarName]` and can be combined, e.g., `[Cloud][Contracts][Reality]`.

#### Pillar Tag Format

Pillar tags use bracket notation:
- `[Cloud]` - Cloud pillar features (registry, orgs, governance)
- `[Contracts]` - Contracts pillar features (validation, drift detection)
- `[Reality]` - Reality pillar features (personas, chaos, realism)
- `[DevX]` - DevX pillar features (SDKs, generators, playgrounds)
- `[AI]` - AI pillar features (LLM generation, AI diff)

Multiple pillar tags can be combined in a single tag string:
- `[Cloud][Contracts][Reality]` - Scenario uses all three pillars

#### High-Impact Pillar Combinations

Certain pillar tag combinations automatically trigger approval requirements for promotions:

**Default High-Impact Pattern:**
- `[Cloud][Contracts][Reality]` - Requires approval for all promotions (dev→test and test→prod)

This combination indicates a scenario that:
- Uses shared infrastructure (Cloud)
- Has contract validation requirements (Contracts)
- Requires realistic behavior (Reality)

Such scenarios are considered high-impact because they affect multiple critical systems.

#### Example: Tagging a Scenario

```json
{
  "id": "user-checkout-flow",
  "name": "User Checkout Flow",
  "tags": [
    "[Cloud][Contracts][Reality]",
    "checkout",
    "payment"
  ]
}
```

When promoting this scenario:
- The pillar tags `[Cloud][Contracts][Reality]` are automatically detected
- The approval workflow recognizes this as a high-impact combination
- Approval is required before promotion can proceed

#### Customizing Approval Rules

You can customize which pillar combinations require approval:

```rust
use mockforge_core::workspace::scenario_promotion::ApprovalRules;
use mockforge_core::pillars::Pillar;

let mut rules = ApprovalRules::default();

// Add custom high-impact pillar pattern
rules.high_impact_pillar_patterns.push(vec![
    Pillar::Cloud,
    Pillar::Contracts,
]);

// Require approval for any scenario with Contracts pillar
rules.require_approval_pillars.push(Pillar::Contracts);
```

#### API Usage

When creating a promotion, include scenario tags:

```bash
POST /api/v2/promotions
{
  "entity_type": "scenario",
  "entity_id": "user-checkout-flow",
  "workspace_id": "workspace-123",
  "from_environment": "dev",
  "to_environment": "test",
  "scenario_tags": [
    "[Cloud][Contracts][Reality]",
    "checkout",
    "payment"
  ]
}
```

The system will:
1. Parse pillar tags from `scenario_tags`
2. Check against approval rules
3. Set `requires_approval` and `approval_required_reason` automatically
4. Store tags in promotion metadata for audit trail

### 5. Pillar Usage Analytics

Track and report on the usage of MockForge's core pillars (Reality, Contracts, DevX, Cloud, AI) at both workspace and organization levels.

#### Metrics Tracked

**Reality Pillar:**
- Requests using blended reality vs pure mock vs live
- Persona usage statistics
- Chaos rule activations

**Contracts Pillar:**
- % of endpoints with validation enforce vs warn vs disabled
- Drift budget utilization
- Contract compliance rates

**DevX Pillar:**
- SDK generation counts
- Client library usage
- Code snippet generations

**Cloud Pillar:**
- Shared scenario usage
- Template applications
- Collaboration metrics

**AI Pillar:**
- AI-generated mock counts
- Contract diff analyses
- Auto-fix applications

#### API Endpoints

```bash
# Get workspace pillar metrics
GET /api/v2/analytics/pillars/workspace/{workspace_id}?duration=3600

# Get org pillar metrics
GET /api/v2/analytics/pillars/org/{org_id}?duration=3600

# Get detailed Reality pillar metrics
GET /api/v2/analytics/pillars/workspace/{workspace_id}/reality

# Get detailed Contracts pillar metrics
GET /api/v2/analytics/pillars/workspace/{workspace_id}/contracts

# Get detailed AI pillar metrics
GET /api/v2/analytics/pillars/workspace/{workspace_id}/ai
```

## Governance Guide for Larger Organizations

### Recommended Setup

1. **Environment Strategy**
   - **Dev**: Light simulation, high chaos tolerance, lenient drift budgets
   - **Test**: Moderate realism, controlled chaos, standard drift budgets
   - **Prod**: High realism, minimal chaos, strict drift budgets

2. **RBAC Strategy**
   - **Platform Team**: Full control over prod reality defaults and drift budgets
   - **QA Team**: Can modify chaos rules in test/prod
   - **Developers**: Can create and modify scenarios in dev/test
   - **Viewers**: Read-only access across all environments

3. **Promotion Workflow**
   - Require approval for dev → test and test → prod
   - Enable GitOps for all prod promotions
   - Maintain full audit trail

4. **Template Strategy**
   - Create org-wide templates with security baselines
   - Include recommended blueprints for common patterns
   - Set environment-specific defaults

5. **Monitoring**
   - Track pillar usage across environments
   - Monitor promotion success rates
   - Alert on drift budget violations in prod

### Example: Platform Team Controls Prod

```rust
// Only platform team can change reality defaults in prod
let prod_reality_policy = EnvironmentPermissionPolicy::new(
    MockEnvironmentName::Prod,
    Permission::ScenarioModifyRealityDefaults,
    vec!["platform".to_string()],
);

// QA can modify chaos rules in test and prod
let test_chaos_policy = EnvironmentPermissionPolicy::new(
    MockEnvironmentName::Test,
    Permission::ScenarioModifyChaosRules,
    vec!["qa".to_string(), "editor".to_string()],
);

let prod_chaos_policy = EnvironmentPermissionPolicy::new(
    MockEnvironmentName::Prod,
    Permission::ScenarioModifyChaosRules,
    vec!["qa".to_string()],
);
```

### Example: Promotion Workflow

```rust
// Developer promotes scenario from dev to test
let dev_to_test = PromotionRequest {
    entity_type: PromotionEntityType::Scenario,
    entity_id: "user-checkout-flow".to_string(),
    workspace_id: workspace.id.clone(),
    from_environment: MockEnvironmentName::Dev,
    to_environment: MockEnvironmentName::Test,
    requires_approval: true,
    comments: Some("Ready for QA testing".to_string()),
    metadata: HashMap::new(),
};

// QA approves and promotes to prod
promotion_service.update_promotion_status(
    promotion_id,
    PromotionStatus::Approved,
    Some(qa_user_id),
).await?;

// Promotion to prod creates GitOps PR
let test_to_prod = PromotionRequest {
    entity_type: PromotionEntityType::Scenario,
    entity_id: "user-checkout-flow".to_string(),
    workspace_id: workspace.id.clone(),
    from_environment: MockEnvironmentName::Test,
    to_environment: MockEnvironmentName::Prod,
    requires_approval: true,
    comments: Some("QA approved, ready for production".to_string()),
    metadata: HashMap::new(),
};
```

## Database Schema

### Promotion History

```sql
CREATE TABLE promotion_history (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_version TEXT,
    from_environment TEXT NOT NULL,
    to_environment TEXT NOT NULL,
    promoted_by TEXT NOT NULL,
    approved_by TEXT,
    status TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    comments TEXT,
    pr_url TEXT,
    metadata JSONB
);
```

### Environment Permission Policies

```sql
CREATE TABLE environment_permission_policies (
    id TEXT PRIMARY KEY,
    org_id TEXT,
    workspace_id TEXT,
    environment TEXT NOT NULL,
    permission TEXT NOT NULL,
    allowed_roles TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

## API Reference

### Environment Management

- `GET /__mockforge/workspaces/{workspace_id}/environments` - List all environments
- `GET /__mockforge/workspaces/{workspace_id}/environments/{env_name}` - Get environment config
- `PUT /__mockforge/workspaces/{workspace_id}/environments/{env_name}` - Update environment config
- `POST /__mockforge/workspaces/{workspace_id}/environments/active` - Set active environment

### Promotion Management

- `POST /api/v2/promotions` - Create promotion request
- `GET /api/v2/promotions/{promotion_id}` - Get promotion details
- `PUT /api/v2/promotions/{promotion_id}/status` - Update promotion status
- `GET /api/v2/promotions/workspace/{workspace_id}` - List workspace promotions
- `GET /api/v2/promotions/pending` - List pending promotions

### Pillar Analytics

- `GET /api/v2/analytics/pillars/workspace/{workspace_id}` - Workspace pillar metrics
- `GET /api/v2/analytics/pillars/org/{org_id}` - Organization pillar metrics
- `GET /api/v2/analytics/pillars/workspace/{workspace_id}/reality` - Reality pillar details
- `GET /api/v2/analytics/pillars/workspace/{workspace_id}/contracts` - Contracts pillar details
- `GET /api/v2/analytics/pillars/workspace/{workspace_id}/ai` - AI pillar details

## Best Practices

1. **Start with Dev**: Always develop and test scenarios in dev first
2. **Use Promotion Workflow**: Never directly modify prod - use promotions
3. **Enable GitOps**: Use GitOps for all prod changes to maintain audit trail
4. **Set Environment Policies**: Define clear RBAC policies per environment
5. **Monitor Pillar Usage**: Track pillar metrics to understand adoption
6. **Use Templates**: Leverage org templates for consistency
7. **Review Promotions**: Regularly review promotion history for compliance

## Migration Guide

### Upgrading Existing Workspaces

Existing workspaces automatically get default environments (dev, test, prod) when loaded:

```rust
// Automatically called during workspace load
workspace.initialize_default_mock_environments();
```

All existing configurations are preserved in the default (dev) environment.

### Enabling GitOps

1. Configure PR provider (GitHub or GitLab)
2. Set authentication token
3. Configure repository and base branch
4. Enable GitOps in promotion service

```rust
let gitops = PromotionGitOpsConfig::new(
    true,
    PRProvider::GitHub,
    "myorg".to_string(),
    "configs".to_string(),
    Some(env::var("GITHUB_TOKEN")?),
    "main".to_string(),
    None,
);
```

## Troubleshooting

### Promotion Fails

- Check user permissions for target environment
- Verify entity exists in source environment
- Check GitOps configuration if PR creation fails
- Review promotion history for error details

### Environment Permission Denied

- Verify user role is in allowed_roles for the policy
- Check environment-specific policy exists
- Review base permissions (environment policies are additive)

### Analytics Not Available

- Ensure analytics database is configured
- Check `MOCKFORGE_ANALYTICS_DB_PATH` environment variable
- Verify database migrations have run

## See Also

- [Workspace Management](./workspace-management.md)
- [RBAC Guide](./rbac-guide.md)
- [Analytics Documentation](./analytics.md)

