# MockOps Pipelines

**Pillars:** [Cloud]

MockOps Pipelines provide GitHub Actions-like automation for mock lifecycle management. Think GitHub Actions + mocks—event-driven automation that makes MockForge orchestration for mock environments.

## Overview

MockOps Pipelines enable:

- **Schema Change → Auto-Regenerate SDK**: When OpenAPI changes, automatically regenerate client SDKs
- **Scenario Published → Auto-Promote to Test → Notify Teams**: Automatically promote scenarios and notify teams
- **Drift Threshold Exceeded → Auto-Generate Git PR**: Create PRs with fixes when drift exceeds thresholds

## Pipeline Structure

Pipelines are defined in YAML and triggered by events:

```yaml
name: schema-change-pipeline
definition:
  enabled: true
  triggers:
    - event: schema.changed
      filters:
        workspace_id: "workspace-123"
        schema_type: ["openapi", "protobuf"]
  steps:
    - name: regenerate-sdks
      step_type: regenerate_sdk
      config:
        languages: ["typescript", "python", "rust"]
        workspace_id: "{{workspace_id}}"
    - name: notify-teams
      step_type: notify
      config:
        type: slack
        channels: ["#api-team", "#frontend-team"]
        message: "SDKs regenerated for {{workspace_id}}"
```

## Available Events

### Schema Changed

Triggered when OpenAPI/Protobuf schemas are modified:

```yaml
triggers:
  - event: schema.changed
    filters:
      schema_type: ["openapi", "protobuf"]
      workspace_id: "workspace-123"
```

**Payload:**
- `spec_path`: Path to schema file
- `schema_type`: "openapi" or "protobuf"
- `changes`: List of changes (added/removed/modified endpoints)

### Scenario Published

Triggered when a new scenario is published:

```yaml
triggers:
  - event: scenario.published
    filters:
      workspace_id: "workspace-123"
```

**Payload:**
- `scenario_id`: ID of published scenario
- `scenario_name`: Name of scenario
- `version`: Scenario version
- `workspace_id`: Workspace ID

### Drift Threshold Exceeded

Triggered when drift budget threshold is exceeded:

```yaml
triggers:
  - event: drift.threshold_exceeded
```

**Payload:**
- `endpoint`: Affected endpoint
- `drift_count`: Number of drift incidents
- `threshold`: Threshold that was exceeded
- `drift_data`: Detailed drift information

### Promotion Completed

Triggered when entity promotion completes:

```yaml
triggers:
  - event: promotion.completed
```

**Payload:**
- `promotion_id`: Promotion ID
- `entity_type`: "scenario", "persona", or "config"
- `from_environment`: Source environment
- `to_environment`: Target environment

## Available Steps

### Regenerate SDK

Regenerates client SDKs from OpenAPI/Protobuf specs:

```yaml
steps:
  - name: regenerate-sdks
    step_type: regenerate_sdk
    config:
      spec_path: "{{event.spec_path}}"
      languages: ["typescript", "python", "rust"]
      output_dir: "./generated-sdks"
```

**Config Options:**
- `spec_path`: Path to OpenAPI/Protobuf spec
- `languages`: Array of languages to generate
- `output_dir`: Output directory (default: `./generated-sdks`)

### Auto-Promote

Automatically promotes entities between environments:

```yaml
steps:
  - name: auto-promote
    step_type: auto_promote
    config:
      entity_type: scenario
      entity_id: "{{event.scenario_id}}"
      from_environment: dev
      to_environment: test
```

**Config Options:**
- `entity_type`: "scenario", "persona", or "config"
- `entity_id`: ID of entity to promote
- `from_environment`: Source environment
- `to_environment`: Target environment
- `comments`: Optional promotion comments

### Notify

Sends notifications via Slack, email, or webhook:

```yaml
steps:
  - name: notify-teams
    step_type: notify
    config:
      type: slack
      slack_webhook_url: "https://hooks.slack.com/services/..."
      channels: ["#api-team", "#frontend-team"]
      message: "SDKs regenerated for {{workspace_id}}"
```

**Config Options:**
- `type`: "slack", "email", or "webhook"
- `message`: Notification message (supports template variables)
- `slack_webhook_url`: Slack webhook URL (for Slack type)
- `channels`: Array of channel names (for Slack type)
- `email_to`: Array of email addresses (for email type)
- `webhook_url`: Webhook URL (for webhook type)

### Create PR

Creates Git pull requests:

```yaml
steps:
  - name: create-pr
    step_type: create_pr
    config:
      repository: "https://github.com/org/repo"
      branch: "main"
      title: "Drift Violation: {{event.endpoint}}"
      body: "Drift count: {{event.drift_count}}, Threshold: {{event.threshold}}"
      files:
        - path: "drift-report.json"
          content: "{{event.drift_data}}"
          operation: create
```

**Config Options:**
- `repository`: Git repository URL
- `branch`: Target branch
- `title`: PR title
- `body`: PR body (supports template variables)
- `files`: Array of files to include in PR

## Common Use Cases

### 1. Auto-Regenerate SDKs on Schema Changes

```yaml
name: auto-sdk-regeneration
definition:
  enabled: true
  triggers:
    - event: schema.changed
      filters:
        schema_type: openapi
  steps:
    - name: regenerate_sdk
      step_type: regenerate_sdk
      config:
        spec_path: "{{event.spec_path}}"
        languages: ["typescript", "rust", "python"]
        output_dir: "./generated-sdks"
```

### 2. Auto-Promote Scenarios to Test

```yaml
name: auto-promote-to-test
definition:
  enabled: true
  triggers:
    - event: scenario.published
      filters:
        workspace_id: "your-workspace-id"
  steps:
    - name: auto_promote
      step_type: auto_promote
      config:
        entity_type: scenario
        entity_id: "{{event.scenario_id}}"
        from_environment: dev
        to_environment: test
    - name: notify_team
      step_type: notify
      config:
        type: slack
        slack_webhook_url: "https://hooks.slack.com/services/..."
        channels: ["#devops"]
        message: "Scenario {{event.scenario_name}} promoted to test"
```

### 3. Create PR on Drift Violations

```yaml
name: drift-gitops-pr
definition:
  enabled: true
  triggers:
    - event: drift.threshold_exceeded
  steps:
    - name: create_pr
      step_type: create_pr
      config:
        repository: "https://github.com/org/repo"
        branch: "main"
        title: "Drift Violation: {{event.endpoint}}"
        body: "Drift count: {{event.drift_count}}, Threshold: {{event.threshold}}"
        files:
          - path: "drift-report.json"
            content: "{{event.drift_data}}"
            operation: create
```

## Template Variables

Pipeline steps support template variables:

- `{{event.field}}`: Access event payload fields
- `{{workspace_id}}`: Current workspace ID
- `{{workspace_name}}`: Current workspace name
- `{{timestamp}}`: Current timestamp

## Pipeline Management

### Create Pipeline

```bash
# Create pipeline from YAML
mockforge pipelines create pipeline.yaml

# Or via API
POST /api/v1/pipelines
{
  "name": "my-pipeline",
  "definition": {...}
}
```

### List Pipelines

```bash
# List all pipelines
mockforge pipelines list

# List pipelines for workspace
mockforge pipelines list --workspace workspace-123
```

### Enable/Disable Pipeline

```bash
# Enable pipeline
mockforge pipelines enable <pipeline-id>

# Disable pipeline
mockforge pipelines disable <pipeline-id>
```

### View Pipeline Executions

```bash
# View pipeline executions
mockforge pipelines executions <pipeline-id>

# View execution details
mockforge pipelines execution <execution-id>
```

## Best Practices

1. **Start Simple**: Begin with one-step pipelines
2. **Test Incrementally**: Test each step individually
3. **Use Filters**: Filter events to avoid unnecessary executions
4. **Monitor Executions**: Review execution logs regularly
5. **Version Control**: Keep pipeline definitions in Git

## Related Documentation

- [Federation](federation.md) - Multi-workspace federation
- [Analytics Dashboard](analytics-dashboard.md) - Usage analytics
- [Cloud Workspaces](cloud-workspaces.md) - Workspace management

