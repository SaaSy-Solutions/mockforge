# Drift Budgets & GitOps for API Sync + AI Contract Diff

## Overview

MockForge's drift budget system makes it the "drift nerve center" for contracts. Teams can define acceptable drift thresholds, receive alerts when budgets are exceeded, and automatically generate pull requests to update contracts and fixtures.

## Concepts

### Drift Budget

A drift budget defines acceptable levels of contract changes:

- **Breaking changes**: Changes that will break existing clients (e.g., removing fields, changing types)
- **Non-breaking changes**: Additive changes that don't break clients (e.g., adding optional fields)
- **Percentage-based budgets**: Track field churn as a percentage over time (e.g., "max 10% field churn/month")

### Budget Hierarchy

Budgets are evaluated in priority order (most specific wins):

1. **Per-workspace budgets** - Apply to all endpoints in a workspace
2. **Per-service/tag budgets** - Apply to endpoints with specific OpenAPI tags or service names
3. **Per-endpoint budgets** - Apply to specific `{method} {endpoint}` combinations
4. **Default budget** - Fallback for all endpoints

### Drift Classification

Changes are classified into three categories:

- **Non-breaking**: Additive changes, documentation-only, unexpected fields (additive)
- **Potentially breaking**: Medium severity changes, format mismatches, constraint violations (requires review)
- **Definitely breaking**: Critical/High severity, missing required fields, type changes, removals

### Incidents

When a drift budget is exceeded, an incident is created with:

- Summary of affected endpoints
- Type of drift (breaking change vs threshold exceeded)
- Before/after contract samples
- Link to sync cycle or contract diff analysis
- Severity level

## Configuration

### YAML Configuration

```yaml
drift_budget:
  enabled: true
  
  # Default budget applied to all endpoints
  default_budget:
    max_breaking_changes: 0
    max_non_breaking_changes: 10
    severity_threshold: "high"
    enabled: true
    # Optional: percentage-based budget
    # max_field_churn_percent: 10.0
    # time_window_days: 30
  
  # Per-workspace budgets
  per_workspace_budgets:
    "workspace-1":
      max_breaking_changes: 0
      max_non_breaking_changes: 5
      enabled: true
  
  # Per-service budgets (by OpenAPI tag or service name)
  per_service_budgets:
    "user-service":
      max_breaking_changes: 0
      max_non_breaking_changes: 15
      max_field_churn_percent: 5.0
      time_window_days: 30
      enabled: true
  
  # Per-tag budgets (OpenAPI tags)
  per_tag_budgets:
    "users":
      max_breaking_changes: 0
      max_non_breaking_changes: 8
      enabled: true
  
  # Per-endpoint budgets
  per_endpoint_budgets:
    "POST /api/users":
      max_breaking_changes: 0
      max_non_breaking_changes: 3
      enabled: true
  
  # Breaking change detection rules
  breaking_change_rules:
    - type: "severity"
      severity: "high"
      include_higher: true
      enabled: true
    - type: "mismatch_type"
      mismatch_type: "missing_required_field"
      enabled: true
  
  incident_retention_days: 90
```

### API Configuration

You can also configure budgets via the API:

```bash
# Create workspace budget
curl -X POST http://localhost:3000/api/v1/drift/budgets/workspace \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_id": "workspace-1",
    "max_breaking_changes": 0,
    "max_non_breaking_changes": 5,
    "enabled": true
  }'

# Create service budget
curl -X POST http://localhost:3000/api/v1/drift/budgets/service \
  -H "Content-Type: application/json" \
  -d '{
    "service_name": "user-service",
    "max_breaking_changes": 0,
    "max_non_breaking_changes": 15,
    "max_field_churn_percent": 5.0,
    "time_window_days": 30,
    "enabled": true
  }'

# Create endpoint budget
curl -X POST http://localhost:3000/api/v1/drift/budgets \
  -H "Content-Type: application/json" \
  -d '{
    "endpoint": "/api/users",
    "method": "POST",
    "max_breaking_changes": 0,
    "max_non_breaking_changes": 3,
    "enabled": true
  }'
```

## Integration with API Sync

When API sync detects changes, drift budgets are automatically evaluated:

```rust
use mockforge_recorder::{SyncService, SyncDriftEvaluator};

// Create drift evaluator
let drift_evaluator = SyncDriftEvaluator::new(
    drift_engine,
    incident_manager,
    database,
);

// Sync with drift evaluation
let (changes, updated, pr_result) = sync_service
    .sync_with_gitops_and_drift(
        Some(&gitops_handler),
        Some(&drift_evaluator),
    )
    .await?;

// Incidents are automatically created if budgets are exceeded
```

### Sync Cycle Integration

Each sync cycle generates a unique ID that links incidents to the sync operation:

- `sync_cycle_id`: Links incidents to the sync cycle that detected the changes
- `before_sample`: Contract state before sync
- `after_sample`: Contract state after sync with detected differences

## Integration with AI Contract Diff

When contract diff analysis detects mismatches, drift budgets are evaluated:

```rust
use mockforge_core::ai_contract_diff::ContractDiffAnalyzer;
use mockforge_core::contract_drift::DriftBudgetEngine;

// Analyze request against contract
let diff_result = analyzer.analyze(&request, &spec).await?;

// Evaluate against drift budget
let drift_result = drift_engine.evaluate_with_context(
    &diff_result,
    &path,
    &method,
    workspace_id,
    service_name,
    tags,
);

// Create incident if budget exceeded
if drift_result.should_create_incident {
    incident_manager
        .create_incident_with_samples(
            path,
            method,
            incident_type,
            severity,
            details,
            budget_id,
            workspace_id,
            None, // sync_cycle_id
            Some(contract_diff_id),
            before_sample,
            after_sample,
        )
        .await;
}
```

## Webhook Notifications

### Slack Integration

Configure Slack webhooks to receive rich notifications:

```yaml
incidents:
  webhooks:
    - url: "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
      enabled: true
      events: ["incident.created"]
      headers:
        Content-Type: "application/json"
```

**Example Slack Notification:**

```json
{
  "blocks": [
    {
      "type": "header",
      "text": {
        "type": "plain_text",
        "text": "🚨 Drift Incident: POST /api/users"
      }
    },
    {
      "type": "section",
      "fields": [
        {
          "title": "Type",
          "value": "BreakingChange",
          "short": true
        },
        {
          "title": "Severity",
          "value": "Critical",
          "short": true
        },
        {
          "title": "Breaking Changes",
          "value": "2",
          "short": true
        }
      ]
    }
  ]
}
```

### Jira Integration

Configure Jira webhooks to automatically create issues:

```yaml
incidents:
  webhooks:
    - url: "https://your-domain.atlassian.net/rest/api/2/issue"
      enabled: true
      events: ["incident.created"]
      headers:
        Content-Type: "application/json"
        Authorization: "Basic ..."
        X-Jira-Project: "PROJ"
```

**Example Jira Issue:**

```json
{
  "fields": {
    "project": {"key": "PROJ"},
    "summary": "Drift Incident: POST /api/users - BreakingChange",
    "description": "Drift incident detected on endpoint `POST /api/users`\n\n*Type:* BreakingChange\n*Severity:* Critical\n*Breaking Changes:* 2",
    "issuetype": {"name": "Bug"},
    "priority": {"name": "Highest"},
    "labels": ["drift-incident", "severity-critical", "type-breaking_change"]
  }
}
```

## GitOps PR Flow

When drift budgets are exceeded, MockForge can automatically generate pull requests:

### Configuration

```yaml
drift_budget:
  gitops:
    enabled: true
    update_openapi_specs: true
    update_fixtures: true
    regenerate_clients: false
    run_tests: false
    openapi_spec_dir: "specs"
    fixtures_dir: "fixtures"
    branch_prefix: "mockforge/drift-fix"
```

### Automatic PR Generation

PRs are automatically created when:

1. Sync detects changes that exceed drift budget
2. Contract diff detects mismatches that exceed drift budget
3. Manual trigger via API

### PR Contents

Each PR includes:

- **OpenAPI spec updates**: Corrections applied to contract specifications
- **Fixture updates**: Updated mock response data
- **Incident summary**: Details of all incidents included in the PR
- **Change summary**: Breaking vs non-breaking changes breakdown

### Example PR

**Title:** `Fix drift: POST /api/users - BreakingChange`

**Body:**
```markdown
## Drift Budget Violation Fix

This PR was automatically generated by MockForge to fix drift budget violations.

### Summary

- **Total incidents**: 1
- **Breaking changes**: 1
- **Threshold exceeded**: 0

### Affected Endpoints

- `POST /api/users` - BreakingChange (Critical)

### Changes Made

- Updated OpenAPI specifications with corrections
- Updated fixture files with new response data

### Incident Details

#### POST /api/users

- **Incident ID**: `abc123...`
- **Type**: BreakingChange
- **Severity**: Critical
- **Breaking Changes**: 2
```

### Manual PR Generation

Generate a PR manually from incidents:

```bash
curl -X POST http://localhost:3000/api/v1/drift/gitops/generate-pr \
  -H "Content-Type: application/json" \
  -d '{
    "incident_ids": ["incident-1", "incident-2"],
    "workspace_id": "workspace-1"
  }'
```

Or generate from all open incidents:

```bash
curl -X POST http://localhost:3000/api/v1/drift/gitops/generate-pr \
  -H "Content-Type: application/json" \
  -d '{
    "status": "open",
    "workspace_id": "workspace-1"
  }'
```

## API Endpoints

### Budget Management

- `GET /api/v1/drift/budgets` - List all budgets
- `GET /api/v1/drift/budgets/lookup` - Get budget for endpoint/workspace/service
- `POST /api/v1/drift/budgets` - Create endpoint budget
- `POST /api/v1/drift/budgets/workspace` - Create workspace budget
- `POST /api/v1/drift/budgets/service` - Create service budget
- `GET /api/v1/drift/budgets/{id}` - Get specific budget

### Incident Management

- `GET /api/v1/drift/incidents` - List incidents (with filters)
- `GET /api/v1/drift/incidents/{id}` - Get specific incident
- `PATCH /api/v1/drift/incidents/{id}` - Update incident
- `POST /api/v1/drift/incidents/{id}/resolve` - Resolve incident
- `GET /api/v1/drift/incidents/stats` - Get incident statistics

### GitOps

- `POST /api/v1/drift/gitops/generate-pr` - Generate PR from incidents

### Metrics

- `GET /api/v1/drift/metrics` - Get drift metrics over time

## Example Workflow

### 1. Configure Budgets

```yaml
drift_budget:
  default_budget:
    max_breaking_changes: 0
    max_non_breaking_changes: 10
  
  per_service_budgets:
    "user-service":
      max_breaking_changes: 0
      max_non_breaking_changes: 5
```

### 2. Enable Sync with Drift Tracking

```rust
let drift_evaluator = SyncDriftEvaluator::new(
    drift_engine,
    incident_manager,
    database,
);

sync_service
    .sync_with_gitops_and_drift(
        Some(&gitops_handler),
        Some(&drift_evaluator),
    )
    .await?;
```

### 3. Receive Alerts

When sync detects changes exceeding the budget:

1. **Incident created** with before/after samples
2. **Slack notification** sent (if configured)
3. **Jira issue created** (if configured)
4. **PR generated** (if GitOps enabled)

### 4. Review and Merge

1. Review the generated PR
2. Verify OpenAPI spec updates
3. Check fixture changes
4. Merge PR to update contracts

## Best Practices

1. **Start with strict budgets**: Begin with `max_breaking_changes: 0` and gradually relax as needed
2. **Use percentage budgets for large APIs**: Track field churn percentage for APIs with many endpoints
3. **Configure workspace budgets**: Set different budgets for different environments (dev, staging, prod)
4. **Enable GitOps for production**: Automatically generate PRs for production incidents
5. **Link to external tickets**: Connect incidents to Jira/Linear tickets for tracking
6. **Review incidents regularly**: Use metrics endpoint to track drift trends over time

## Troubleshooting

### Incidents not being created

- Check that `drift_budget.enabled: true` in config
- Verify budget is not disabled for the endpoint
- Check incident manager logs for errors

### PRs not being generated

- Verify GitOps handler is configured in state
- Check PR generator credentials (GitHub/GitLab token)
- Ensure incidents have before/after samples

### Webhooks not firing

- Verify webhook URL is correct
- Check webhook is enabled
- Verify event subscription matches incident type
- Check webhook logs for errors

