# MockForge Cloud API Reference

Complete reference for all MockForge Cloud API endpoints.

**Base URL:** `https://api.mockforge.dev` (or your custom domain)

**API Version:** `v1`

---

## Table of Contents

- [Authentication](#authentication)
- [Rate Limiting](#rate-limiting)
- [Error Handling](#error-handling)
- [Endpoints](#endpoints)
  - [Health & Status](#health--status)
  - [Authentication](#authentication-endpoints)
  - [Organizations](#organizations)
  - [Billing & Subscriptions](#billing--subscriptions)
  - [Usage](#usage)
  - [API Tokens](#api-tokens)
  - [Hosted Mocks](#hosted-mocks)
  - [Marketplace](#marketplace)
  - [Settings](#settings)
  - [Security](#security)
  - [GDPR](#gdpr)
  - [Legal & Support](#legal--support)
  - [Admin](#admin)

---

## Authentication

MockForge Cloud API supports two authentication methods:

### 1. JWT Token (Web UI)

After logging in via the web UI, you receive a JWT token. Include it in the `Authorization` header:

```http
Authorization: Bearer <jwt_token>
```

### 2. API Token (CLI/Programmatic)

Generate an API token from Settings → API Tokens. Include it in the `Authorization` header:

```http
Authorization: Bearer mfx_<token>
```

**Note:** API tokens are organization-scoped and have specific scopes (read, write, admin).

---

## Rate Limiting

All endpoints are rate-limited based on your plan:

- **Free:** 60 requests/minute (global)
- **Pro:** Based on plan limits (250K requests/month)
- **Team:** Based on plan limits (1M requests/month)

Rate limit headers are included in all responses:

```http
X-RateLimit-Limit: 10000
X-RateLimit-Remaining: 9999
X-RateLimit-Reset: 1735689600
```

When rate limit is exceeded, you'll receive a `429 Too Many Requests` response.

---

## Error Handling

All errors follow this format:

```json
{
  "error": "Error message",
  "code": "ERROR_CODE",
  "details": {}
}
```

**HTTP Status Codes:**
- `200` - Success
- `201` - Created
- `400` - Bad Request
- `401` - Unauthorized
- `403` - Forbidden
- `404` - Not Found
- `429` - Rate Limit Exceeded
- `500` - Internal Server Error

---

## Endpoints

### Health & Status

#### Health Check

```http
GET /health
```

Basic health check endpoint.

**Response:**
```json
{
  "status": "ok",
  "version": "1.0.0"
}
```

#### Detailed Health Check

```http
GET /health/detailed
```

Comprehensive health check including database, Redis, and storage connectivity.

**Response:**
```json
{
  "status": "ok",
  "version": "1.0.0",
  "timestamp": "2025-01-20T12:00:00Z",
  "checks": {
    "database": "ok",
    "redis": "ok",
    "storage": "ok"
  }
}
```

#### Liveness Probe

```http
GET /health/live
```

Kubernetes liveness probe endpoint.

**Response:**
```json
{
  "status": "alive"
}
```

#### Readiness Probe

```http
GET /health/ready
```

Kubernetes readiness probe endpoint.

**Response:**
```json
{
  "status": "ready"
}
```

#### System Status

```http
GET /api/v1/status
```

Public status page endpoint.

**Response:**
```json
{
  "overall": "operational",
  "services": [
    {
      "name": "Database",
      "status": "operational",
      "message": null
    },
    {
      "name": "Redis",
      "status": "operational",
      "message": null
    },
    {
      "name": "Object Storage",
      "status": "operational",
      "message": null
    },
    {
      "name": "API Service",
      "status": "operational",
      "message": null
    }
  ],
  "timestamp": "2025-01-20T12:00:00Z"
}
```

---

### Authentication Endpoints

#### Register

```http
POST /api/v1/auth/register
```

Create a new user account.

**Request:**
```json
{
  "username": "johndoe",
  "email": "john@example.com",
  "password": "securepassword123"
}
```

**Response:**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "username": "johndoe"
}
```

**Validation:**
- Username: minimum 3 characters
- Password: minimum 8 characters
- Email: must be valid email format

#### Login

```http
POST /api/v1/auth/login
```

Authenticate and receive JWT token.

**Request:**
```json
{
  "email": "john@example.com",
  "password": "securepassword123"
}
```

**Response:**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "username": "johndoe"
}
```

**Rate Limiting:**
- Max 5 failed attempts per email in 15 minutes
- Max 10 failed attempts per IP in 15 minutes
- Account locked after exceeding limits

#### Refresh Token

```http
POST /api/v1/auth/token/refresh
```

Refresh an expired JWT token.

**Request:**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**Response:**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "username": "johndoe"
}
```

#### OAuth Authorize

```http
GET /api/v1/auth/oauth/:provider/authorize
```

Initiate OAuth flow (GitHub, Google).

**Parameters:**
- `provider`: `github` or `google`

**Response:** Redirects to OAuth provider

#### OAuth Callback

```http
GET /api/v1/auth/oauth/:provider/callback
```

OAuth callback handler (handled automatically by OAuth flow).

#### Verify Email

```http
GET /api/v1/auth/verify-email?token=<verification_token>
```

Verify email address using token from verification email.

**Response:**
```json
{
  "message": "Email verified successfully"
}
```

#### Resend Verification Email

```http
POST /api/v1/auth/resend-verification
```

**Authentication:** Required

**Response:**
```json
{
  "message": "Verification email sent"
}
```

---

### Organizations

#### List Organizations

```http
GET /api/v1/organizations
```

**Authentication:** Required

**Response:**
```json
[
  {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "name": "My Organization",
    "slug": "my-org",
    "plan": "pro",
    "owner_id": "123e4567-e89b-12d3-a456-426614174001",
    "created_at": "2025-01-01T00:00:00Z"
  }
]
```

#### Get Organization

```http
GET /api/v1/organizations/:org_id
```

**Authentication:** Required

**Response:**
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "name": "My Organization",
  "slug": "my-org",
  "plan": "pro",
  "owner_id": "123e4567-e89b-12d3-a456-426614174001",
  "created_at": "2025-01-01T00:00:00Z"
}
```

#### Get Organization Members

```http
GET /api/v1/organizations/:org_id/members
```

**Authentication:** Required

**Response:**
```json
[
  {
    "user_id": "123e4567-e89b-12d3-a456-426614174001",
    "username": "johndoe",
    "email": "john@example.com",
    "role": "admin",
    "joined_at": "2025-01-01T00:00:00Z"
  }
]
```

#### Get Audit Logs

```http
GET /api/v1/organizations/:org_id/audit-logs
```

**Authentication:** Required (Org Admin only)

**Query Parameters:**
- `limit` (optional): Number of logs to return (default: 50)
- `offset` (optional): Pagination offset
- `event_type` (optional): Filter by event type
- `user_id` (optional): Filter by user ID

**Response:**
```json
[
  {
    "id": "123e4567-e89b-12d3-a456-426614174002",
    "org_id": "123e4567-e89b-12d3-a456-426614174000",
    "user_id": "123e4567-e89b-12d3-a456-426614174001",
    "event_type": "billing_checkout",
    "description": "Checkout session created for pro plan",
    "metadata": {
      "plan": "pro",
      "session_id": "cs_..."
    },
    "ip_address": "192.168.1.1",
    "user_agent": "Mozilla/5.0...",
    "created_at": "2025-01-20T12:00:00Z"
  }
]
```

---

### Billing & Subscriptions

#### Get Subscription

```http
GET /api/v1/billing/subscription
```

**Authentication:** Required

**Response:**
```json
{
  "org_id": "123e4567-e89b-12d3-a456-426614174000",
  "plan": "pro",
  "status": "active",
  "current_period_end": "2025-02-20T12:00:00Z",
  "usage": {
    "requests": 50000,
    "requests_limit": 250000,
    "storage_bytes": 1073741824,
    "storage_limit_bytes": 21474836480,
    "ai_tokens_used": 10000,
    "ai_tokens_limit": 100000
  },
  "limits": {
    "max_projects": 10,
    "max_collaborators": 5,
    "requests_per_30d": 250000,
    "storage_gb": 20,
    "ai_tokens_per_month": 100000,
    "hosted_mocks": true
  }
}
```

#### Create Checkout Session

```http
POST /api/v1/billing/checkout
```

**Authentication:** Required

**Request:**
```json
{
  "plan": "pro",
  "success_url": "https://app.mockforge.dev/billing/success",
  "cancel_url": "https://app.mockforge.dev/billing/cancel"
}
```

**Response:**
```json
{
  "checkout_url": "https://checkout.stripe.com/c/pay/cs_...",
  "session_id": "cs_..."
}
```

**Note:** Redirect user to `checkout_url` to complete payment.

#### Stripe Webhook

```http
POST /api/v1/billing/webhook
```

**Authentication:** Not required (Stripe signature verified)

**Note:** This endpoint is called by Stripe. Do not call directly.

---

### Usage

#### Get Current Usage

```http
GET /api/v1/usage
```

**Authentication:** Required

**Response:**
```json
{
  "requests": 50000,
  "requests_limit": 250000,
  "storage_bytes": 1073741824,
  "storage_limit_bytes": 21474836480,
  "ai_tokens_used": 10000,
  "ai_tokens_limit": 100000,
  "period_start": "2025-01-01T00:00:00Z",
  "period_end": "2025-01-31T23:59:59Z"
}
```

#### Get Usage History

```http
GET /api/v1/usage/history
```

**Authentication:** Required

**Query Parameters:**
- `months` (optional): Number of months to retrieve (default: 6)

**Response:**
```json
[
  {
    "period_start": "2025-01-01T00:00:00Z",
    "period_end": "2025-01-31T23:59:59Z",
    "requests": 50000,
    "storage_bytes": 1073741824,
    "ai_tokens_used": 10000
  }
]
```

---

### API Tokens

#### Create Token

```http
POST /api/v1/tokens
```

**Authentication:** Required

**Request:**
```json
{
  "name": "CI/CD Token",
  "scopes": ["read", "write"],
  "expires_at": "2025-12-31T23:59:59Z"
}
```

**Response:**
```json
{
  "token": "mfx_abc123...",
  "id": "123e4567-e89b-12d3-a456-426614174003",
  "name": "CI/CD Token",
  "token_prefix": "mfx_abc",
  "scopes": ["read", "write"],
  "expires_at": "2025-12-31T23:59:59Z",
  "created_at": "2025-01-20T12:00:00Z"
}
```

**Note:** The full token is only shown once. Store it securely.

#### List Tokens

```http
GET /api/v1/tokens
```

**Authentication:** Required

**Response:**
```json
[
  {
    "id": "123e4567-e89b-12d3-a456-426614174003",
    "name": "CI/CD Token",
    "token_prefix": "mfx_abc",
    "scopes": ["read", "write"],
    "last_used_at": "2025-01-20T11:00:00Z",
    "expires_at": "2025-12-31T23:59:59Z",
    "created_at": "2025-01-01T00:00:00Z",
    "age_days": 19,
    "needs_rotation": false
  }
]
```

#### Delete Token

```http
DELETE /api/v1/tokens/:token_id
```

**Authentication:** Required

**Response:**
```json
{
  "success": true,
  "message": "API token deleted successfully"
}
```

#### Rotate Token

```http
POST /api/v1/tokens/:token_id/rotate
```

**Authentication:** Required

**Request:**
```json
{
  "new_name": "CI/CD Token (Rotated)",
  "delete_old": true
}
```

**Response:**
```json
{
  "new_token": "mfx_xyz789...",
  "new_token_id": "123e4567-e89b-12d3-a456-426614174004",
  "new_token_name": "CI/CD Token (Rotated)",
  "old_token_id": "123e4567-e89b-12d3-a456-426614174003",
  "old_token_deleted": true
}
```

#### Get Tokens Needing Rotation

```http
GET /api/v1/tokens/rotation-status
```

**Authentication:** Required

**Response:**
```json
[
  {
    "id": "123e4567-e89b-12d3-a456-426614174003",
    "name": "Old Token",
    "token_prefix": "mfx_abc",
    "created_at": "2024-01-01T00:00:00Z",
    "age_days": 384,
    "expires_at": null,
    "needs_rotation": true
  }
]
```

---

### Hosted Mocks

#### Create Deployment

```http
POST /api/v1/deployments
```

**Authentication:** Required

**Request:**
```json
{
  "name": "User API Mock",
  "slug": "user-api",
  "description": "Mock user API for testing",
  "project_id": "123e4567-e89b-12d3-a456-426614174005",
  "openapi_spec_url": "https://example.com/openapi.json",
  "config_json": {
    "cors_enabled": true,
    "response_template_expand": true
  }
}
```

**Response:**
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174006",
  "org_id": "123e4567-e89b-12d3-a456-426614174000",
  "name": "User API Mock",
  "slug": "user-api",
  "status": "deploying",
  "health_status": "unknown",
  "url": "https://my-org.mockforge.dev/user-api",
  "created_at": "2025-01-20T12:00:00Z"
}
```

#### List Deployments

```http
GET /api/v1/deployments
```

**Authentication:** Required

**Query Parameters:**
- `status` (optional): Filter by status (active, deploying, failed, stopped)
- `project_id` (optional): Filter by project ID

**Response:**
```json
[
  {
    "id": "123e4567-e89b-12d3-a456-426614174006",
    "name": "User API Mock",
    "slug": "user-api",
    "status": "active",
    "health_status": "healthy",
    "url": "https://my-org.mockforge.dev/user-api",
    "created_at": "2025-01-20T12:00:00Z"
  }
]
```

#### Get Deployment

```http
GET /api/v1/deployments/:deployment_id
```

**Authentication:** Required

**Response:**
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174006",
  "org_id": "123e4567-e89b-12d3-a456-426614174000",
  "name": "User API Mock",
  "slug": "user-api",
  "description": "Mock user API for testing",
  "status": "active",
  "health_status": "healthy",
  "url": "https://my-org.mockforge.dev/user-api",
  "config_json": {
    "cors_enabled": true
  },
  "created_at": "2025-01-20T12:00:00Z",
  "updated_at": "2025-01-20T12:05:00Z"
}
```

#### Update Deployment Status

```http
PUT /api/v1/deployments/:deployment_id/status
```

**Authentication:** Required

**Request:**
```json
{
  "status": "stopped"
}
```

**Response:**
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174006",
  "status": "stopped",
  "updated_at": "2025-01-20T12:10:00Z"
}
```

#### Delete Deployment

```http
DELETE /api/v1/deployments/:deployment_id
```

**Authentication:** Required

**Response:**
```json
{
  "success": true,
  "message": "Deployment deleted successfully"
}
```

#### Get Deployment Logs

```http
GET /api/v1/deployments/:deployment_id/logs
```

**Authentication:** Required

**Query Parameters:**
- `limit` (optional): Number of log entries (default: 100)
- `level` (optional): Filter by log level (info, warn, error)

**Response:**
```json
[
  {
    "id": "123e4567-e89b-12d3-a456-426614174007",
    "level": "info",
    "message": "Deployment started",
    "created_at": "2025-01-20T12:00:00Z"
  }
]
```

#### Get Deployment Metrics

```http
GET /api/v1/deployments/:deployment_id/metrics
```

**Authentication:** Required

**Query Parameters:**
- `start_time` (optional): Start time for metrics (ISO 8601)
- `end_time` (optional): End time for metrics (ISO 8601)

**Response:**
```json
{
  "requests_total": 1000,
  "requests_per_minute": 10,
  "average_response_time_ms": 50,
  "error_rate": 0.01,
  "status_codes": {
    "200": 990,
    "404": 10
  },
  "period_start": "2025-01-20T12:00:00Z",
  "period_end": "2025-01-20T13:00:00Z"
}
```

---

### Marketplace

#### Search Plugins

```http
POST /api/v1/plugins/search
```

**Request:**
```json
{
  "query": "http",
  "category": "protocol",
  "limit": 20,
  "offset": 0
}
```

**Response:**
```json
{
  "plugins": [
    {
      "name": "http-auth",
      "version": "1.0.0",
      "description": "HTTP authentication plugin",
      "category": "protocol",
      "downloads": 1000,
      "rating": 4.5
    }
  ],
  "total": 1
}
```

#### Get Plugin

```http
GET /api/v1/plugins/:name
```

**Response:**
```json
{
  "name": "http-auth",
  "description": "HTTP authentication plugin",
  "category": "protocol",
  "author": "mockforge",
  "latest_version": "1.0.0",
  "downloads_total": 1000,
  "rating_avg": 4.5,
  "rating_count": 100
}
```

#### Get Plugin Version

```http
GET /api/v1/plugins/:name/versions/:version
```

**Response:**
```json
{
  "name": "http-auth",
  "version": "1.0.0",
  "download_url": "https://storage.mockforge.dev/plugins/http-auth-1.0.0.wasm",
  "checksum": "sha256:abc123...",
  "published_at": "2025-01-01T00:00:00Z"
}
```

#### Publish Plugin

```http
POST /api/v1/plugins/publish
```

**Authentication:** Required

**Request:** (multipart/form-data)
- `name`: Plugin name
- `version`: Version (semver)
- `description`: Description
- `category`: Category
- `wasm_file`: WASM file (binary)

**Response:**
```json
{
  "name": "my-plugin",
  "version": "1.0.0",
  "message": "Plugin published successfully"
}
```

#### Search Templates

```http
POST /api/v1/templates/search
```

**Request:**
```json
{
  "query": "chaos",
  "category": "testing",
  "limit": 20
}
```

**Response:**
```json
{
  "templates": [
    {
      "name": "chaos-testing",
      "version": "1.0.0",
      "description": "Chaos testing template",
      "category": "testing",
      "downloads": 500
    }
  ],
  "total": 1
}
```

#### Get Template

```http
GET /api/v1/templates/:name/:version
```

**Response:**
```json
{
  "name": "chaos-testing",
  "version": "1.0.0",
  "description": "Chaos testing template",
  "manifest": {},
  "download_url": "https://storage.mockforge.dev/templates/chaos-testing-1.0.0.tar.gz"
}
```

#### Publish Template

```http
POST /api/v1/templates/publish
```

**Authentication:** Required

**Request:** (multipart/form-data)
- `manifest`: Template manifest (JSON string)
- `package`: Template package (tar.gz file)

**Response:**
```json
{
  "name": "my-template",
  "version": "1.0.0",
  "message": "Template published successfully"
}
```

#### Search Scenarios

```http
POST /api/v1/scenarios/search
```

**Request:**
```json
{
  "query": "payment",
  "category": "ecommerce",
  "limit": 20
}
```

**Response:**
```json
{
  "scenarios": [
    {
      "name": "payment-flow",
      "version": "1.0.0",
      "description": "Payment processing scenario",
      "category": "ecommerce",
      "downloads": 200
    }
  ],
  "total": 1
}
```

#### Get Scenario

```http
GET /api/v1/scenarios/:name
```

**Response:**
```json
{
  "name": "payment-flow",
  "description": "Payment processing scenario",
  "latest_version": "1.0.0",
  "downloads_total": 200,
  "rating_avg": 4.0
}
```

#### Get Scenario Version

```http
GET /api/v1/scenarios/:name/versions/:version
```

**Response:**
```json
{
  "name": "payment-flow",
  "version": "1.0.0",
  "manifest": {},
  "download_url": "https://storage.mockforge.dev/scenarios/payment-flow-1.0.0.tar.gz"
}
```

#### Publish Scenario

```http
POST /api/v1/scenarios/publish
```

**Authentication:** Required

**Request:** (multipart/form-data)
- `name`: Scenario name
- `version`: Version (semver)
- `manifest`: Scenario manifest (JSON string)
- `package`: Scenario package (tar.gz file)

**Response:**
```json
{
  "name": "my-scenario",
  "version": "1.0.0",
  "message": "Scenario published successfully"
}
```

---

### Settings

#### Get BYOK Configuration

```http
GET /api/v1/settings/byok
```

**Authentication:** Required

**Response:**
```json
{
  "enabled": true,
  "provider": "openai",
  "api_key_masked": "sk-...***",
  "monthly_token_limit": 100000
}
```

#### Update BYOK Configuration

```http
PUT /api/v1/settings/byok
```

**Authentication:** Required

**Request:**
```json
{
  "provider": "openai",
  "api_key": "sk-...",
  "monthly_token_limit": 100000
}
```

**Response:**
```json
{
  "enabled": true,
  "provider": "openai",
  "api_key_masked": "sk-...***",
  "monthly_token_limit": 100000
}
```

#### Delete BYOK Configuration

```http
DELETE /api/v1/settings/byok
```

**Authentication:** Required

**Response:**
```json
{
  "success": true,
  "message": "BYOK configuration deleted"
}
```

---

### Security

#### Get Suspicious Activities

```http
GET /api/v1/security/suspicious-activities
```

**Authentication:** Required (Org Admin only)

**Query Parameters:**
- `org_id` (optional): Filter by organization
- `user_id` (optional): Filter by user
- `severity` (optional): Filter by severity (low, medium, high, critical)
- `limit` (optional): Number of results (default: 50)

**Response:**
```json
[
  {
    "id": "123e4567-e89b-12d3-a456-426614174008",
    "org_id": "123e4567-e89b-12d3-a456-426614174000",
    "user_id": "123e4567-e89b-12d3-a456-426614174001",
    "activity_type": "multiple_failed_logins",
    "severity": "high",
    "description": "Multiple failed login attempts detected",
    "metadata": {
      "email": "john@example.com",
      "ip_address": "192.168.1.1"
    },
    "ip_address": "192.168.1.1",
    "resolved": false,
    "created_at": "2025-01-20T12:00:00Z"
  }
]
```

#### Resolve Suspicious Activity

```http
POST /api/v1/security/suspicious-activities/:activity_id/resolve
```

**Authentication:** Required (Org Admin only)

**Request:**
```json
{
  "resolved_by_user_id": "123e4567-e89b-12d3-a456-426614174001"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Suspicious activity resolved successfully",
  "activity_id": "123e4567-e89b-12d3-a456-426614174008"
}
```

---

### GDPR

#### Export Data

```http
GET /api/v1/gdpr/export
```

**Authentication:** Required

**Response:**
```json
{
  "user_data": {
    "id": "123e4567-e89b-12d3-a456-426614174001",
    "username": "johndoe",
    "email": "john@example.com",
    "created_at": "2025-01-01T00:00:00Z"
  },
  "organizations": [...],
  "deployments": [...],
  "api_tokens": [...],
  "exported_at": "2025-01-20T12:00:00Z"
}
```

#### Delete Data

```http
POST /api/v1/gdpr/delete
```

**Authentication:** Required

**Request:**
```json
{
  "confirm": true
}
```

**Response:**
```json
{
  "message": "Data deletion initiated. This action cannot be undone.",
  "deletion_scheduled_at": "2025-01-21T12:00:00Z"
}
```

---

### Legal & Support

#### Get Terms of Service

```http
GET /api/v1/legal/terms
```

**Response:**
```json
{
  "title": "Terms of Service",
  "content": "# MockForge Terms of Service\n\n...",
  "last_updated": "2024-07-20"
}
```

#### Get Privacy Policy

```http
GET /api/v1/legal/privacy
```

**Response:**
```json
{
  "title": "Privacy Policy",
  "content": "# MockForge Privacy Policy\n\n...",
  "last_updated": "2024-07-20"
}
```

#### Get DPA

```http
GET /api/v1/legal/dpa
```

**Response:**
```json
{
  "title": "Data Processing Agreement",
  "content": "# MockForge DPA\n\n...",
  "last_updated": "2024-07-20"
}
```

#### Get FAQ

```http
GET /api/v1/faq
```

**Response:**
```json
{
  "categories": ["Getting Started", "Billing", "Features"],
  "items": [
    {
      "id": "getting-started-1",
      "question": "What is MockForge Cloud?",
      "answer": "MockForge Cloud is...",
      "category": "Getting Started"
    }
  ]
}
```

#### Submit Support Request

```http
POST /api/v1/support/contact
```

**Request:**
```json
{
  "category": "technical",
  "priority": "high",
  "subject": "Deployment issue",
  "message": "My deployment is failing..."
}
```

**Response:**
```json
{
  "ticket_id": "TKT-123456",
  "message": "Your support request has been received."
}
```

---

### Admin

#### Verify Plugin

```http
POST /api/v1/admin/plugins/:name/verify
```

**Authentication:** Required (Admin only)

**Response:**
```json
{
  "name": "my-plugin",
  "verified": true,
  "verified_at": "2025-01-20T12:00:00Z"
}
```

#### Get Admin Stats

```http
GET /api/v1/admin/stats
```

**Authentication:** Required (Admin only)

**Response:**
```json
{
  "total_users": 1000,
  "total_organizations": 500,
  "total_deployments": 2000,
  "total_plugins": 100
}
```

#### Get Analytics

```http
GET /api/v1/admin/analytics
```

**Authentication:** Required (Admin only)

**Query Parameters:**
- `start_date` (optional): Start date (ISO 8601)
- `end_date` (optional): End date (ISO 8601)
- `metric` (optional): Metric type (users, subscriptions, usage, features)

**Response:**
```json
{
  "users": {
    "total": 1000,
    "new_this_month": 100,
    "active_this_month": 800
  },
  "subscriptions": {
    "free": 700,
    "pro": 250,
    "team": 50
  },
  "usage": {
    "total_requests": 10000000,
    "total_storage_gb": 500
  }
}
```

---

## Examples

### cURL Examples

#### Register User

```bash
curl -X POST https://api.mockforge.dev/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "johndoe",
    "email": "john@example.com",
    "password": "securepassword123"
  }'
```

#### Create API Token

```bash
curl -X POST https://api.mockforge.dev/api/v1/tokens \
  -H "Authorization: Bearer <jwt_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "CI/CD Token",
    "scopes": ["read", "write"]
  }'
```

#### Create Deployment

```bash
curl -X POST https://api.mockforge.dev/api/v1/deployments \
  -H "Authorization: Bearer mfx_<api_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "User API Mock",
    "slug": "user-api",
    "openapi_spec_url": "https://example.com/openapi.json"
  }'
```

### JavaScript/TypeScript Examples

```typescript
const API_BASE = 'https://api.mockforge.dev';

// Register
const register = async (username: string, email: string, password: string) => {
  const response = await fetch(`${API_BASE}/api/v1/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, email, password }),
  });
  return response.json();
};

// Create deployment
const createDeployment = async (token: string, deployment: any) => {
  const response = await fetch(`${API_BASE}/api/v1/deployments`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(deployment),
  });
  return response.json();
};
```

---

## SDKs and Libraries

Official SDKs are available for:

- **Rust**: `mockforge-cli` crate
- **JavaScript/TypeScript**: Coming soon
- **Python**: Coming soon
- **Go**: Coming soon

---

## Support

For API support:

- **Email**: support@mockforge.dev
- **Documentation**: [docs.mockforge.dev](https://docs.mockforge.dev)
- **GitHub**: [github.com/SaaSy-Solutions/mockforge](https://github.com/SaaSy-Solutions/mockforge)

---

**Last Updated:** January 2025
