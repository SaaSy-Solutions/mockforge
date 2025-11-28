# MockOps Platform User Guide

Complete guide to using MockOps Pipelines and Federation features in MockForge.

## Table of Contents

1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Pipelines](#pipelines)
4. [Federation](#federation)
5. [Best Practices](#best-practices)
6. [Troubleshooting](#troubleshooting)

## Introduction

MockOps Platform provides two powerful features for managing mock environments at scale:

- **Pipelines**: Event-driven automation for mock lifecycle management
- **Federation**: Multi-workspace composition for distributed systems

### What are Pipelines?

Pipelines automate common tasks in your mock environment workflow. Think of them as "GitHub Actions for mocks" - they trigger automatically when events occur (like schema changes, scenario publications, or drift violations) and execute a series of steps.

### What is Federation?

Federation allows you to compose multiple isolated MockForge workspaces into a single "virtual system". This is essential for large organizations with microservices architectures where each service has its own workspace, but you need to test the entire system together.

## Getting Started

### Prerequisites

- MockForge Cloud account or self-hosted instance
- Access to workspace management
- Basic understanding of YAML configuration

### Quick Start: Your First Pipeline

1. **Navigate to Pipelines** in the MockForge admin UI
2. **Click "Create Pipeline"**
3. **Configure your pipeline:**

```yaml
name: my-first-pipeline
definition:
  enabled: true
  triggers:
    - event: schema.changed
  steps:
    - name: notify_me
      step_type: notify
      config:
        type: webhook
        webhook_url: "https://your-webhook-url.com"
        message: "Schema changed: {{event.spec_path}}"
```

4. **Save** and your pipeline is active!

## Pipelines

### Understanding Pipeline Structure

A pipeline consists of:

- **Triggers**: Events that activate the pipeline
- **Steps**: Actions to execute when triggered
- **Configuration**: Settings for each step

### Common Use Cases

#### 1. Auto-regenerate SDKs on Schema Changes

When your OpenAPI spec changes, automatically regenerate client SDKs:

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
        output_dir: "./generated-sdks"
        languages:
          - typescript
          - rust
```

#### 2. Auto-promote Scenarios to Test

When a scenario is published in dev, automatically promote it to test:

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
        channels:
          - "#devops"
        message: "Scenario {{event.scenario_name}} promoted to test"
```

#### 3. Create PR on Drift Violations

When drift exceeds threshold, automatically create a Git PR:

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

### Pipeline Events

Pipelines are triggered by events. Available events:

| Event Type | Description | Payload Fields |
|------------|------------|----------------|
| `schema.changed` | OpenAPI/Protobuf schema changed | `spec_path`, `schema_type`, `changes` |
| `scenario.published` | New scenario published | `scenario_id`, `scenario_name`, `version` |
| `drift.threshold_exceeded` | Drift threshold exceeded | `endpoint`, `drift_count`, `threshold` |
| `promotion.completed` | Entity promotion completed | `promotion_id`, `entity_type`, `from_environment`, `to_environment` |
| `workspace.created` | New workspace created | `workspace_id`, `workspace_name` |
| `persona.published` | New persona published | `persona_id`, `persona_name` |
| `config.changed` | Configuration changed | `config_path`, `changes` |

### Pipeline Steps

Available step types:

#### `regenerate_sdk`

Regenerates client SDKs from OpenAPI/Protobuf specs.

**Required Config:**
- `spec_path`: Path to OpenAPI/Protobuf spec file
- `languages`: Array of languages to generate (e.g., `["typescript", "rust"]`)

**Optional Config:**
- `output_dir`: Output directory (default: `./generated-sdks`)
- `port`: Server port for generated code
- `enable_cors`: Enable CORS (default: `true`)
- `default_delay_ms`: Default response delay
- `mock_data_strategy`: `random`, `examples`, `defaults`, or `examples_or_random`

#### `auto_promote`

Automatically promotes entities between environments.

**Required Config:**
- `entity_type`: `scenario`, `persona`, or `config`
- `entity_id`: ID of entity to promote (can use `{{event.entity_id}}`)
- `from_environment`: Source environment (`dev`, `test`, `prod`)
- `to_environment`: Target environment

**Optional Config:**
- `comments`: Promotion comments

#### `notify`

Sends notifications via Slack, email, or webhook.

**Required Config:**
- `type`: `slack`, `email`, or `webhook`
- `message`: Notification message (supports template variables)

**Slack-specific:**
- `slack_webhook_url`: Slack webhook URL
- `channels`: Array of channel names

**Email-specific:**
- `smtp`: SMTP configuration object
- `to`: Array of recipient email addresses
- `subject`: Email subject

**Webhook-specific:**
- `webhook_url`: Webhook URL
- `method`: HTTP method (`POST`, `PUT`, `PATCH`)
- `payload`: Additional payload data

#### `create_pr`

Creates a Git Pull Request.

**Required Config:**
- `repository`: Git repository URL
- `branch`: Target branch
- `title`: PR title
- `files`: Array of file changes

**File Object:**
- `path`: File path
- `content`: File content
- `operation`: `create`, `update`, or `delete`

### Template Variables

Pipeline steps support template variables in configuration. Available variables:

- `{{event.event_type}}` - Event type
- `{{event.workspace_id}}` - Workspace ID
- `{{event.spec_path}}` - Schema file path (for schema.changed events)
- `{{event.scenario_id}}` - Scenario ID (for scenario.published events)
- `{{event.entity_id}}` - Entity ID (for promotion events)
- `{{event.drift_count}}` - Drift count (for drift events)

Any field in the event payload can be accessed via `{{event.field_name}}`.

## Federation

### Understanding Federation

Federation allows you to:

- **Compose multiple workspaces** into one virtual system
- **Control reality level per service** independently
- **Run system-wide scenarios** that span multiple services
- **Test distributed systems** as a cohesive unit

### Creating a Federation

1. **Navigate to Federation** in the admin UI
2. **Click "Create Federation"**
3. **Define your services:**

```yaml
name: e-commerce-platform
description: Federated e-commerce system
services:
  - name: auth
    workspace_id: "workspace-auth-123"
    base_path: /auth
    reality_level: real  # Use real upstream

  - name: payments
    workspace_id: "workspace-payments-456"
    base_path: /payments
    reality_level: mock_v3  # Use mock v3

  - name: inventory
    workspace_id: "workspace-inventory-789"
    base_path: /inventory
    reality_level: blended  # Mix of mock and real

  - name: shipping
    workspace_id: "workspace-shipping-012"
    base_path: /shipping
    reality_level: chaos_driven  # Chaos testing mode
```

### Service Reality Levels

Each service in a federation can have its own reality level:

- **`real`**: Use real upstream (no mocking) - useful for services you want to test against real implementations
- **`mock_v3`**: Use mock with reality level 3 - high-fidelity mocks
- **`blended`**: Mix of mock and real data - useful for gradual migration
- **`chaos_driven`**: Chaos testing mode - inject failures and latency

### Routing Requests

When a request comes to the federation:

1. Federation router matches the request path to a service
2. Service-specific path is extracted (base_path stripped)
3. Request is routed to the appropriate workspace
4. Service's reality level is applied

**Example:**
- Request: `GET /auth/login`
- Matches service: `auth` (base_path: `/auth`)
- Service path: `/login`
- Routed to: `workspace-auth-123`
- Reality level: `real`

### Service Dependencies

Services can declare dependencies on other services:

```yaml
services:
  - name: payments
    dependencies:
      - auth  # Payments depends on auth service
```

Dependencies are used for:
- Validating federation configuration
- Determining service startup order
- System-wide scenario execution

### System-Wide Scenarios

Federations support scenarios that span multiple services:

```yaml
scenario:
  name: checkout-flow
  steps:
    - service: auth
      action: login
      user: test-user
    - service: payments
      action: process-payment
      amount: 100.00
    - service: inventory
      action: reserve-item
      item_id: "item-123"
    - service: shipping
      action: create-shipment
      address: "123 Main St"
```

## Best Practices

### Pipeline Best Practices

1. **Use descriptive names**: `auto-sdk-regeneration` is better than `pipeline-1`
2. **Enable/disable strategically**: Disable pipelines during maintenance
3. **Handle errors gracefully**: Use `continue_on_error: true` for non-critical steps
4. **Set timeouts**: Prevent steps from hanging indefinitely
5. **Test pipelines**: Use manual triggers to test before enabling
6. **Monitor executions**: Check execution logs regularly

### Federation Best Practices

1. **Map services clearly**: Use clear service names and base paths
2. **Start with real services**: Begin with `real` reality level, then mock incrementally
3. **Document dependencies**: Clearly document service dependencies
4. **Use consistent naming**: Follow naming conventions across services
5. **Test routing**: Verify requests route correctly before production use

### Security Considerations

1. **Protect webhook URLs**: Don't commit webhook URLs to version control
2. **Use environment variables**: Store sensitive config in environment variables
3. **Limit pipeline scope**: Use workspace/org filters to limit pipeline scope
4. **Review PRs carefully**: Auto-generated PRs should still be reviewed
5. **Monitor executions**: Set up alerts for failed pipeline executions

## Troubleshooting

### Pipeline Not Executing

**Check:**
1. Is the pipeline enabled?
2. Does the event match the trigger?
3. Are workspace/org filters correct?
4. Check execution logs for errors

### SDK Generation Failing

**Check:**
1. Is the spec path correct?
2. Is the spec file valid OpenAPI/Protobuf?
3. Are the output directories writable?
4. Check step execution logs for specific errors

### Federation Routing Issues

**Check:**
1. Are base paths correctly configured?
2. Do paths match exactly (case-sensitive)?
3. Are services in the correct order (longest path first)?
4. Check router logs for routing decisions

### Notification Not Received

**Check:**
1. Is the webhook URL correct?
2. Are Slack channels/email addresses valid?
3. Check network connectivity
4. Review notification step execution logs

## Advanced Topics

### Custom Step Types

You can extend pipelines with custom step types by implementing the `PipelineStepExecutor` trait.

### Federation Database Persistence

Federations are stored in the database. Use the `FederationDatabase` API for programmatic access.

### Event Filtering

Use filters in triggers to match specific events:

```yaml
triggers:
  - event: schema.changed
    filters:
      schema_type: openapi
      workspace_id: "specific-workspace-id"
```

### Pipeline Templates

Create reusable pipeline templates for common workflows:

```yaml
template: sdk-regeneration
variables:
  languages:
    - typescript
    - rust
steps:
  - step_type: regenerate_sdk
    config:
      languages: "{{variables.languages}}"
```

## Support

For additional help:

- Check the [API Documentation](./MOCKOPS_API_DOCUMENTATION.md)
- Review [Implementation Status](./MOCKOPS_FINAL_STATUS.md)
- Open an issue on GitHub
- Contact MockForge support
