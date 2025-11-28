# Deceptive Deploy Guide

Deceptive Deploy is a feature that allows you to deploy mock APIs that look identical to production endpoints. This is perfect for:

- **Front-end demos**: Show your front-end working with a "real" API
- **PoCs (Proof of Concepts)**: Demonstrate functionality without backend dependencies
- **Investor prototypes**: Present a complete-looking product without full infrastructure
- **Client presentations**: Show working integrations without exposing production systems

## What is Deceptive Deploy?

Deceptive Deploy configures MockForge to automatically:

- ✅ Add production-like headers to all responses
- ✅ Configure CORS to match production settings
- ✅ Apply production-like rate limiting
- ✅ Support OAuth flows identical to production
- ✅ Deploy to public URLs via tunneling

The result: mock APIs that are indistinguishable from production endpoints to your application and users.

## Quick Start

### 1. Basic Deployment

```bash
# Deploy with production preset
mockforge deploy deploy --production-preset --spec api.yaml

# Deploy with custom config
mockforge deploy deploy --config config.yaml --spec api.yaml
```

### 2. Configuration File

Create a `config.yaml` file:

```yaml
http:
  port: 3000
  openapi_spec: "./api-spec.yaml"

deceptive_deploy:
  enabled: true
  auto_tunnel: true
```

### 3. Start the Server

```bash
mockforge serve --config config.yaml
```

The server will automatically:
- Apply production-like headers
- Configure CORS
- Set up rate limiting
- Start a tunnel (if `auto_tunnel: true`)

## Configuration

### Basic Configuration

```yaml
deceptive_deploy:
  enabled: true
  auto_tunnel: true
```

### Full Configuration

```yaml
deceptive_deploy:
  enabled: true

  # Production-like CORS
  cors:
    allowed_origins: ["*"]
    allowed_methods: ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"]
    allowed_headers: ["*"]
    allow_credentials: true

  # Production-like rate limiting
  rate_limit:
    requests_per_minute: 1000
    burst: 2000
    per_ip: true

  # Production headers (supports templates)
  headers:
    X-API-Version: "1.0"
    X-Request-ID: "{{uuid}}"
    X-Powered-By: "MockForge"

  # OAuth configuration (optional)
  oauth:
    client_id: "your-client-id"
    client_secret: "your-client-secret"
    introspection_url: "https://auth.example.com/introspect"

  # Custom domain (optional)
  custom_domain: "api.example.com"

  # Auto-start tunnel
  auto_tunnel: true
```

## Production Headers

Deceptive Deploy automatically adds configured headers to all responses. Headers support template expansion:

### Supported Templates

- `{{uuid}}` - Generates a unique UUID v4 for each request
- `{{now}}` - Current timestamp in RFC3339 format
- `{{timestamp}}` - Current Unix timestamp (seconds)

### Example

```yaml
headers:
  X-Request-ID: "{{uuid}}"           # Unique ID per request
  X-Timestamp: "{{timestamp}}"      # Unix timestamp
  X-Request-Time: "{{now}}"         # RFC3339 timestamp
  X-API-Version: "1.0"               # Static value
```

### Common Production Headers

```yaml
headers:
  # Request tracking
  X-Request-ID: "{{uuid}}"
  X-Correlation-ID: "{{uuid}}"

  # API information
  X-API-Version: "1.0"
  X-Environment: "production"

  # Server information
  X-Powered-By: "MockForge"
  Server: "MockForge/1.0"

  # Custom headers
  X-Rate-Limit-Remaining: "999"
  X-Rate-Limit-Reset: "{{timestamp}}"
```

## CORS Configuration

Deceptive Deploy can configure CORS to match production settings:

```yaml
cors:
  # Allow all origins (use specific origins in production)
  allowed_origins:
    - "*"
    # Or specific origins:
    # - "https://app.example.com"
    # - "https://staging.example.com"

  # Allowed HTTP methods
  allowed_methods:
    - "GET"
    - "POST"
    - "PUT"
    - "DELETE"
    - "PATCH"
    - "OPTIONS"

  # Allowed headers
  allowed_headers:
    - "*"
    # Or specific headers:
    # - "Content-Type"
    # - "Authorization"
    # - "X-API-Key"

  # Allow credentials (cookies, authorization headers)
  allow_credentials: true
```

## Rate Limiting

Configure production-like rate limiting:

```yaml
rate_limit:
  # Requests per minute
  requests_per_minute: 1000

  # Burst capacity (maximum requests in a short burst)
  burst: 2000

  # Enable per-IP rate limiting
  per_ip: true
```

### Rate Limit Headers

When rate limiting is enabled, responses include rate limit headers:

- `X-Rate-Limit-Limit`: Maximum requests per minute
- `X-Rate-Limit-Remaining`: Remaining requests in current window
- `X-Rate-Limit-Reset`: Unix timestamp when limit resets

## OAuth Configuration

Configure OAuth flows to match production:

```yaml
oauth:
  client_id: "your-client-id"
  client_secret: "your-client-secret"
  introspection_url: "https://auth.example.com/introspect"
  auth_url: "https://auth.example.com/authorize"
  token_url: "https://auth.example.com/token"
  token_type_hint: "access_token"
```

This enables:
- Token introspection
- Authorization code flow
- Client credentials flow
- Token validation

## Tunneling

Deceptive Deploy can automatically start a tunnel to expose your mock API via a public URL:

```yaml
deceptive_deploy:
  auto_tunnel: true
  custom_domain: "api.example.com"  # Optional
```

### Tunnel Providers

- **Self-hosted**: Use your own tunnel server
- **Cloud**: Use MockForge Cloud (if available)
- **Cloudflare**: Use Cloudflare Tunnel (coming soon)

### Manual Tunnel

```bash
# Start tunnel manually
mockforge tunnel start \
  --local-url http://localhost:3000 \
  --subdomain my-api
```

## CLI Commands

### Deploy

```bash
# Deploy with production preset
mockforge deploy deploy --production-preset --spec api.yaml

# Deploy with custom config
mockforge deploy deploy --config config.yaml --spec api.yaml

# Deploy with auto-tunnel
mockforge deploy deploy --config config.yaml --auto-tunnel

# Deploy with custom domain
mockforge deploy deploy --config config.yaml --custom-domain api.example.com
```

### Status

```bash
# Get deployment status
mockforge deploy status --config config.yaml
```

### Stop

```bash
# Stop deployment
mockforge deploy stop --config config.yaml
```

## Use Cases

### Front-End Demo

```yaml
# config.yaml
http:
  port: 3000
  openapi_spec: "./api.yaml"

deceptive_deploy:
  enabled: true
  auto_tunnel: true
  headers:
    X-API-Version: "1.0"
    X-Request-ID: "{{uuid}}"
```

```bash
# Deploy
mockforge deploy deploy --config config.yaml

# Start server
mockforge serve --config config.yaml

# Front-end connects to public URL
# https://abc123.tunnel.mockforge.dev
```

### Investor Prototype

```yaml
deceptive_deploy:
  enabled: true
  cors:
    allowed_origins: ["*"]
    allow_credentials: true
  rate_limit:
    requests_per_minute: 1000
    burst: 2000
  headers:
    X-API-Version: "1.0"
    X-Environment: "production"
  auto_tunnel: true
  custom_domain: "api.demo.example.com"
```

### PoC with OAuth

```yaml
deceptive_deploy:
  enabled: true
  oauth:
    client_id: "demo-client"
    client_secret: "demo-secret"
    introspection_url: "https://auth.example.com/introspect"
  headers:
    X-Request-ID: "{{uuid}}"
    X-Auth-Provider: "OAuth2"
```

## Best Practices

### 1. Use Specific Origins

Instead of `*`, use specific origins:

```yaml
cors:
  allowed_origins:
    - "https://app.example.com"
    - "https://staging.example.com"
```

### 2. Set Realistic Rate Limits

Match production rate limits:

```yaml
rate_limit:
  requests_per_minute: 1000  # Match production
  burst: 2000
```

### 3. Use Meaningful Headers

Add headers that match production:

```yaml
headers:
  X-API-Version: "1.0"
  X-Request-ID: "{{uuid}}"
  X-Environment: "production"
```

### 4. Secure OAuth Credentials

Never commit OAuth secrets to version control:

```yaml
oauth:
  client_id: "${OAUTH_CLIENT_ID}"
  client_secret: "${OAUTH_CLIENT_SECRET}"
```

### 5. Use Custom Domains

For professional presentations:

```yaml
deceptive_deploy:
  custom_domain: "api.example.com"
```

## Troubleshooting

### Headers Not Appearing

Check that deceptive deploy is enabled:

```yaml
deceptive_deploy:
  enabled: true
  headers:
    X-Request-ID: "{{uuid}}"
```

### CORS Errors

Verify CORS configuration:

```yaml
cors:
  allowed_origins: ["*"]  # Or specific origins
  allow_credentials: true
```

### Rate Limiting Too Strict

Adjust rate limits:

```yaml
rate_limit:
  requests_per_minute: 1000  # Increase if needed
  burst: 2000
```

### Tunnel Not Starting

Check tunnel configuration:

```yaml
deceptive_deploy:
  auto_tunnel: true
```

Or start manually:

```bash
mockforge tunnel start --local-url http://localhost:3000
```

## Definition of Done

✅ You can deploy to a URL that feels real to the app and users:

- ✅ Production-like headers are automatically added
- ✅ CORS is configured to match production
- ✅ Rate limits are set to production-like values
- ✅ OAuth flows work identically to production
- ✅ Deployment to public URL is one command

## Examples

See `examples/deceptive-deploy-config.yaml` for a complete configuration example.

## Related Documentation

- [Tunneling Guide](./TUNNELING.md) - Detailed tunnel setup
- [Authentication Guide](./AUTHENTICATION.md) - OAuth configuration
- [Configuration Reference](../config.example.yaml) - Full config options
