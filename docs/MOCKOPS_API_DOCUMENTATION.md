# MockOps Platform API Documentation

This document provides comprehensive API documentation for the MockOps Platform, including Pipelines and Federation endpoints.

## Base URL

All API endpoints are prefixed with `/api/v1` for pipelines and `/api/v1/federation` for federation endpoints.

## Authentication

Most endpoints require authentication. Include authentication headers as configured in your MockForge instance.

## Pipelines API

### Create Pipeline

**POST** `/api/v1/pipelines`

Create a new pipeline.

**Request Body:**
```json
{
  "name": "auto-sdk-regeneration",
  "definition": {
    "name": "auto-sdk-regeneration",
    "description": "Automatically regenerate SDKs on schema changes",
    "enabled": true,
    "triggers": [
      {
        "event": "schema.changed",
        "filters": {}
      }
    ],
    "steps": [
      {
        "name": "regenerate_sdk",
        "step_type": "regenerate_sdk",
        "config": {
          "spec_path": "/path/to/openapi.yaml",
          "output_dir": "./generated-sdks",
          "languages": ["typescript", "rust"]
        },
        "continue_on_error": false,
        "timeout": null
      }
    ]
  },
  "workspace_id": "550e8400-e29b-41d4-a716-446655440000",
  "org_id": "550e8400-e29b-41d4-a716-446655440001"
}
```

**Response:** `200 OK`
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440002",
  "name": "auto-sdk-regeneration",
  "definition": { ... },
  "workspace_id": "550e8400-e29b-41d4-a716-446655440000",
  "org_id": "550e8400-e29b-41d4-a716-446655440001",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

### List Pipelines

**GET** `/api/v1/pipelines`

List all pipelines, optionally filtered by workspace or organization.

**Query Parameters:**
- `workspace_id` (optional): Filter by workspace ID
- `org_id` (optional): Filter by organization ID
- `enabled` (optional): Filter by enabled status (true/false)

**Response:** `200 OK`
```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440002",
    "name": "auto-sdk-regeneration",
    "definition": { ... },
    "workspace_id": "550e8400-e29b-41d4-a716-446655440000",
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T10:30:00Z"
  }
]
```

### Get Pipeline

**GET** `/api/v1/pipelines/{id}`

Get a specific pipeline by ID.

**Response:** `200 OK`
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440002",
  "name": "auto-sdk-regeneration",
  "definition": { ... },
  "workspace_id": "550e8400-e29b-41d4-a716-446655440000",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

### Update Pipeline

**PATCH** `/api/v1/pipelines/{id}`

Update a pipeline.

**Request Body:**
```json
{
  "name": "updated-pipeline-name",
  "definition": { ... },
  "enabled": true
}
```

**Response:** `200 OK` (same as Get Pipeline)

### Delete Pipeline

**DELETE** `/api/v1/pipelines/{id}`

Delete a pipeline.

**Response:** `204 No Content`

### Trigger Pipeline

**POST** `/api/v1/pipelines/{id}/trigger`

Manually trigger a pipeline execution.

**Request Body (optional):**
```json
{
  "event": {
    "event_type": "schema.changed",
    "workspace_id": "550e8400-e29b-41d4-a716-446655440000",
    "payload": {
      "spec_path": "/path/to/spec.yaml",
      "schema_type": "openapi"
    }
  }
}
```

**Response:** `200 OK`
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440003",
  "pipeline_id": "550e8400-e29b-41d4-a716-446655440002",
  "trigger_event": { ... },
  "status": "started",
  "started_at": "2024-01-15T10:35:00Z",
  "completed_at": null,
  "error_message": null,
  "execution_log": []
}
```

### Get Pipeline Statistics

**GET** `/api/v1/pipelines/{id}/stats`

Get statistics for a pipeline.

**Response:** `200 OK`
```json
{
  "pipeline_id": "550e8400-e29b-41d4-a716-446655440002",
  "total_executions": 42,
  "successful_executions": 40,
  "failed_executions": 2,
  "last_execution_at": "2024-01-15T10:35:00Z",
  "average_duration_ms": 1250
}
```

### List Pipeline Executions

**GET** `/api/v1/pipelines/executions`

List pipeline executions.

**Query Parameters:**
- `pipeline_id` (optional): Filter by pipeline ID
- `status` (optional): Filter by status (started, running, completed, failed, cancelled)
- `limit` (optional): Limit number of results (default: 50)
- `offset` (optional): Offset for pagination (default: 0)

**Response:** `200 OK`
```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440003",
    "pipeline_id": "550e8400-e29b-41d4-a716-446655440002",
    "trigger_event": { ... },
    "status": "completed",
    "started_at": "2024-01-15T10:35:00Z",
    "completed_at": "2024-01-15T10:35:02Z",
    "error_message": null,
    "execution_log": [
      {
        "step_name": "regenerate_sdk",
        "step_type": "regenerate_sdk",
        "status": "completed",
        "started_at": "2024-01-15T10:35:00Z",
        "completed_at": "2024-01-15T10:35:02Z",
        "output": {
          "generated_files": ["mock_server.ts"],
          "status": "success"
        },
        "error_message": null
      }
    ]
  }
]
```

### Get Pipeline Execution

**GET** `/api/v1/pipelines/executions/{id}`

Get a specific pipeline execution by ID.

**Response:** `200 OK` (same structure as execution in list)

## Federation API

### Create Federation

**POST** `/api/v1/federation`

Create a new federation.

**Request Body:**
```json
{
  "name": "e-commerce-platform",
  "description": "Federated e-commerce system",
  "org_id": "550e8400-e29b-41d4-a716-446655440001",
  "services": [
    {
      "name": "auth",
      "workspace_id": "550e8400-e29b-41d4-a716-446655440010",
      "base_path": "/auth",
      "reality_level": "real",
      "config": {},
      "dependencies": []
    },
    {
      "name": "payments",
      "workspace_id": "550e8400-e29b-41d4-a716-446655440011",
      "base_path": "/payments",
      "reality_level": "mock_v3",
      "config": {},
      "dependencies": ["auth"]
    }
  ]
}
```

**Response:** `200 OK`
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440020",
  "name": "e-commerce-platform",
  "description": "Federated e-commerce system",
  "org_id": "550e8400-e29b-41d4-a716-446655440001",
  "services": [ ... ],
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

### List Federations

**GET** `/api/v1/federation`

List all federations for an organization.

**Query Parameters:**
- `org_id` (required): Organization ID

**Response:** `200 OK`
```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440020",
    "name": "e-commerce-platform",
    "description": "Federated e-commerce system",
    "org_id": "550e8400-e29b-41d4-a716-446655440001",
    "services": [ ... ],
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T10:30:00Z"
  }
]
```

### Get Federation

**GET** `/api/v1/federation/{id}`

Get a specific federation by ID.

**Response:** `200 OK` (same structure as Create Federation)

### Update Federation

**PATCH** `/api/v1/federation/{id}`

Update a federation.

**Request Body:**
```json
{
  "name": "updated-federation-name",
  "description": "Updated description",
  "services": [ ... ]
}
```

**Response:** `200 OK` (same as Get Federation)

### Delete Federation

**DELETE** `/api/v1/federation/{id}`

Delete a federation.

**Response:** `204 No Content`

### Route Request

**POST** `/api/v1/federation/{id}/route`

Route a request through the federation.

**Request Body:**
```json
{
  "path": "/auth/login",
  "method": "GET",
  "headers": {},
  "body": null
}
```

**Response:** `200 OK`
```json
{
  "workspace_id": "550e8400-e29b-41d4-a716-446655440010",
  "service": {
    "name": "auth",
    "workspace_id": "550e8400-e29b-41d4-a716-446655440010",
    "base_path": "/auth",
    "reality_level": "real"
  },
  "service_path": "/login"
}
```

## Pipeline Event Types

The following event types can trigger pipelines:

- `schema.changed` - OpenAPI/Protobuf schema changed
- `scenario.published` - New scenario published
- `drift.threshold_exceeded` - Drift threshold exceeded
- `promotion.completed` - Entity promotion completed
- `workspace.created` - New workspace created
- `persona.published` - New persona published
- `config.changed` - Configuration changed

## Pipeline Step Types

The following step types are available:

### `regenerate_sdk`

Regenerates client SDKs for specified languages.

**Configuration:**
```json
{
  "spec_path": "/path/to/openapi.yaml",
  "output_dir": "./generated-sdks",
  "languages": ["typescript", "rust", "javascript"],
  "port": 3000,
  "enable_cors": true,
  "default_delay_ms": 100,
  "mock_data_strategy": "examples_or_random"
}
```

### `auto_promote`

Automatically promotes entities between environments.

**Configuration:**
```json
{
  "entity_type": "scenario",
  "entity_id": "{{event.entity_id}}",
  "from_environment": "dev",
  "to_environment": "test",
  "comments": "Auto-promoted on schema change"
}
```

### `notify`

Sends notifications via Slack, email, or webhook.

**Configuration (Slack):**
```json
{
  "type": "slack",
  "slack_webhook_url": "https://hooks.slack.com/services/...",
  "channels": ["#devops", "#alerts"],
  "message": "Schema changed: {{event.spec_path}}"
}
```

**Configuration (Email):**
```json
{
  "type": "email",
  "smtp": {
    "host": "smtp.example.com",
    "port": 587,
    "username": "user@example.com",
    "password": "password",
    "from": "mockforge@example.com"
  },
  "to": ["team@example.com"],
  "subject": "Pipeline Notification",
  "message": "Schema changed"
}
```

**Configuration (Webhook):**
```json
{
  "type": "webhook",
  "webhook_url": "https://example.com/webhook",
  "method": "POST",
  "message": "Schema changed: {{event.spec_path}}",
  "payload": {
    "custom_field": "value"
  }
}
```

### `create_pr`

Creates a Git Pull Request.

**Configuration:**
```json
{
  "repository": "https://github.com/org/repo",
  "branch": "main",
  "title": "Auto-generated PR: Schema changes",
  "body": "This PR was auto-generated due to schema changes.",
  "files": [
    {
      "path": "generated-sdks/mock_server.ts",
      "content": "...",
      "operation": "create"
    }
  ]
}
```

## Service Reality Levels

Federation services support the following reality levels:

- `real` - Use real upstream (no mocking)
- `mock_v3` - Use mock with reality level 3
- `blended` - Mix of mock and real data
- `chaos_driven` - Chaos testing mode

## Error Responses

All endpoints may return the following error responses:

**400 Bad Request**
```json
{
  "error": "Invalid request",
  "message": "Missing required field: name"
}
```

**404 Not Found**
```json
{
  "error": "Not found",
  "message": "Pipeline not found"
}
```

**500 Internal Server Error**
```json
{
  "error": "Internal server error",
  "message": "Failed to execute pipeline step"
}
```

## Rate Limiting

API requests are rate-limited. Check response headers for rate limit information:

- `X-RateLimit-Limit`: Maximum requests per window
- `X-RateLimit-Remaining`: Remaining requests in current window
- `X-RateLimit-Reset`: Time when rate limit resets

## Pagination

List endpoints support pagination via `limit` and `offset` query parameters.

**Example:**
```
GET /api/v1/pipelines?limit=20&offset=40
```

## Webhooks

Pipeline executions can trigger webhooks. Configure webhooks in pipeline step configurations.

## Examples

### Example: Auto-regenerate SDKs on Schema Change

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
    - name: notify_team
      step_type: notify
      config:
        type: slack
        slack_webhook_url: "https://hooks.slack.com/services/..."
        channels:
          - "#devops"
        message: "SDKs regenerated for {{event.spec_path}}"
```

### Example: Auto-promote on Scenario Publication

```yaml
name: auto-promote-to-test
definition:
  enabled: true
  triggers:
    - event: scenario.published
      filters:
        workspace_id: "550e8400-e29b-41d4-a716-446655440000"
  steps:
    - name: auto_promote
      step_type: auto_promote
      config:
        entity_type: scenario
        entity_id: "{{event.scenario_id}}"
        from_environment: dev
        to_environment: test
    - name: notify
      step_type: notify
      config:
        type: webhook
        webhook_url: "https://example.com/webhook"
        message: "Scenario {{event.scenario_name}} promoted to test"
```

### Example: Federation Configuration

```yaml
name: e-commerce-platform
description: Federated e-commerce system
services:
  - name: auth
    workspace_id: "550e8400-e29b-41d4-a716-446655440010"
    base_path: /auth
    reality_level: real
    dependencies: []
  - name: payments
    workspace_id: "550e8400-e29b-41d4-a716-446655440011"
    base_path: /payments
    reality_level: mock_v3
    dependencies:
      - auth
  - name: inventory
    workspace_id: "550e8400-e29b-41d4-a716-446655440012"
    base_path: /inventory
    reality_level: blended
    dependencies:
      - payments
```
