# Drift Budget & Contract Automation Setup Guide

This guide explains how to set up and configure drift budget tracking, incident management, PR generation, and webhook notifications.

## Database Setup

### 1. Enable Database Feature

The database feature is optional. To enable it, add the `database` feature when building:

```bash
cargo build --features database
```

Or add to your `Cargo.toml`:

```toml
[dependencies]
mockforge-http = { path = "../mockforge-http", features = ["database"] }
```

### 2. Configure Database Connection

Set the `DATABASE_URL` environment variable:

```bash
export DATABASE_URL="postgresql://user:password@localhost/mockforge"
```

Migrations will run automatically on application startup if the database is connected.

### 3. Manual Migration (Optional)

If you prefer to run migrations manually:

```bash
sqlx migrate run --database-url "$DATABASE_URL"
```

## PR Generation Configuration

### GitHub Setup

1. Create a GitHub Personal Access Token (PAT) with `repo` scope
2. Set environment variables:

```bash
export PR_PROVIDER="github"
export GITHUB_TOKEN="ghp_your_token_here"
export PR_REPO_OWNER="your-org"
export PR_REPO_NAME="your-repo"
export PR_BASE_BRANCH="main"  # optional, defaults to "main"
```

### GitLab Setup

1. Create a GitLab Personal Access Token with `api` scope
2. Set environment variables:

```bash
export PR_PROVIDER="gitlab"
export GITLAB_TOKEN="glpat-your-token-here"
export PR_REPO_OWNER="your-group"
export PR_REPO_NAME="your-project"
export PR_BASE_BRANCH="main"  # optional, defaults to "main"
```

### Configuration via Config File

You can also configure PR generation in your config file:

```yaml
pr_generation:
  enabled: true
  provider: github  # or gitlab
  owner: "your-org"
  repo: "your-repo"
  base_branch: "main"
  branch_prefix: "mockforge/contract-update"
  auto_merge: false
  reviewers: []
  labels:
    - "automated"
    - "contract-update"
```

Note: Tokens should be set via environment variables for security, not in config files.

## Webhook Testing

### Test Webhook Endpoint

Use the webhook testing endpoints to validate webhook delivery:

```bash
# Test sending a webhook
curl -X POST http://localhost:3000/api/v1/webhooks/test \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://your-webhook-endpoint.com/webhook",
    "event": "incident.created",
    "payload": {
      "incident": {
        "id": "test-123",
        "endpoint": "/api/users",
        "method": "GET"
      }
    }
  }'

# Receive webhooks (for testing)
curl -X POST http://localhost:3000/api/v1/webhooks/receive \
  -H "Content-Type: application/json" \
  -H "X-Webhook-Event: incident.created" \
  -d '{
    "event": "incident.created",
    "incident": {
      "id": "test-123"
    }
  }'

# Get received webhooks
curl http://localhost:3000/api/v1/webhooks/received

# Clear received webhooks
curl -X DELETE http://localhost:3000/api/v1/webhooks/received
```

### Configure Webhooks for Incidents

Webhooks can be configured in the incident management config:

```yaml
incidents:
  webhooks:
    - url: "https://your-slack-webhook.com/webhook"
      events:
        - "incident.created"
        - "incident.resolved"
      headers:
        Authorization: "Bearer your-token"
      hmac_secret: "your-secret"  # optional, for signature verification
      enabled: true
```

## Environment Variables Summary

| Variable | Description | Required |
|----------|-------------|----------|
| `DATABASE_URL` | PostgreSQL connection string | No (optional) |
| `PR_PROVIDER` | PR provider: `github` or `gitlab` | No |
| `GITHUB_TOKEN` | GitHub Personal Access Token | Yes (for GitHub) |
| `GITLAB_TOKEN` | GitLab Personal Access Token | Yes (for GitLab) |
| `PR_REPO_OWNER` | Repository owner/org | Yes (for PR generation) |
| `PR_REPO_NAME` | Repository name | Yes (for PR generation) |
| `PR_BASE_BRANCH` | Base branch for PRs | No (defaults to "main") |

## UI Components (Separate Task)

UI components for incident management are planned as a separate task. The backend APIs are ready:

- `GET /api/v1/drift/incidents` - List incidents
- `GET /api/v1/drift/incidents/{id}` - Get incident details
- `PATCH /api/v1/drift/incidents/{id}` - Update incident
- `POST /api/v1/drift/incidents/{id}/resolve` - Resolve incident
- `GET /api/v1/drift/incidents/stats` - Get statistics

These endpoints can be used to build UI components for:
- Incident dashboard
- Incident details view
- Incident filtering and search
- Statistics and metrics visualization
