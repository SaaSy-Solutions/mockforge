# MockForge Authentication Guide

MockForge now supports comprehensive authentication for your mocked APIs, automatically authorizing requests with OAuth 2.0, JWT tokens, Basic Auth, and API keys.

## Features

- ✅ **OAuth 2.0** with token introspection
- ✅ **JWT tokens** (HS256, RS256, ES256 algorithms)
- ✅ **Basic Authentication**
- ✅ **API Keys** (header or query parameter)
- ✅ **OpenAPI Security Scheme Integration**
- ✅ **Multiple Auth Methods** per endpoint
- ✅ **Automatic Request Authorization**

## Quick Start

### 1. Configuration File

Create a configuration file (e.g., `config.yaml`):

```yaml
server:
  http:
    port: 3000
    openapi_spec: "./api-spec.yaml"

    # Authentication configuration
    auth:
      # Require authentication for all requests
      require_auth: true

      # JWT configuration
      jwt:
        secret: "your-jwt-secret-key"
        issuer: "https://your-issuer.com"
        audience: "your-api"
        algorithms: ["HS256", "RS256"]

      # OAuth2 configuration
      oauth2:
        client_id: "your-oauth2-client-id"
        client_secret: "your-oauth2-client-secret"
        introspection_url: "https://your-oauth2-server.com/introspect"

      # Basic authentication
      basic_auth:
        credentials:
          admin: "admin-password"
          user: "user-password"

      # API key configuration
      api_key:
        header_name: "X-API-Key"
        query_name: "api_key"
        keys:
          - "api-key-1"
          - "api-key-2"
```

### 2. Start MockForge

```bash
mockforge serve --config config.yaml --spec api-spec.yaml
```

### 3. Test Authentication

#### JWT Bearer Token
```bash
curl -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." \
     http://localhost:3000/api/users
```

#### Basic Auth
```bash
curl -H "Authorization: Basic YWRtaW46YWRtaW4=" \
     http://localhost:3000/api/users
```

#### API Key
```bash
# Header
curl -H "X-API-Key: api-key-1" \
     http://localhost:3000/api/users

# Query parameter
curl "http://localhost:3000/api/users?api_key=api-key-1"
```

## Configuration Options

### JWT Configuration

```yaml
jwt:
  # HMAC secret for HS256 tokens
  secret: "your-secret-key"

  # Or RSA public key (PEM format)
  rsa_public_key: |
    -----BEGIN PUBLIC KEY-----
    MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA...
    -----END PUBLIC KEY-----

  # Expected issuer
  issuer: "https://your-issuer.com"

  # Expected audience
  audience: "your-api"

  # Supported algorithms (optional, defaults to HS256, RS256, ES256)
  algorithms: ["HS256", "RS256", "ES256"]
```

### OAuth2 Configuration

```yaml
oauth2:
  client_id: "your-client-id"
  client_secret: "your-client-secret"
  introspection_url: "https://oauth-server.com/introspect"

  # Optional URLs
  auth_url: "https://oauth-server.com/auth"
  token_url: "https://oauth-server.com/token"

  # Token type hint
  token_type_hint: "access_token"
```

### Basic Auth Configuration

```yaml
basic_auth:
  credentials:
    username1: "password1"
    username2: "password2"
    admin: "admin-password"
```

### API Key Configuration

```yaml
api_key:
  # Header name (default: X-API-Key)
  header_name: "X-API-Key"

  # Optional query parameter name
  query_name: "api_key"

  # List of valid API keys
  keys:
    - "key1"
    - "key2"
    - "sk_live_123456789"
```

## OpenAPI Integration

MockForge automatically integrates with OpenAPI security schemes:

### Bearer Authentication
```yaml
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
```

### JWT Authentication
```yaml
components:
  securitySchemes:
    jwtAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
```

### API Key Authentication
```yaml
components:
  securitySchemes:
    apiKey:
      type: apiKey
      in: header
      name: X-API-Key
```

### OAuth2 Authentication
```yaml
components:
  securitySchemes:
    oauth2:
      type: oauth2
      flows:
        authorizationCode:
          authorizationUrl: https://example.com/oauth/authorize
          tokenUrl: https://example.com/oauth/token
          scopes:
            read: Grants read access
            write: Grants write access
```

### OpenID Connect
```yaml
components:
  securitySchemes:
    openId:
      type: openIdConnect
      openIdConnectUrl: https://example.com/.well-known/openid-configuration
```

## Security Features

### Automatic Authorization
- Requests are automatically validated against configured auth methods
- Multiple authentication methods can be configured simultaneously
- Fallback authentication (try one method, then another)

### Token Validation
- JWT signature verification
- Token expiration checking
- Issuer and audience validation
- OAuth2 token introspection

### Error Handling
- Clear error messages for authentication failures
- Proper HTTP status codes (401 Unauthorized)
- WWW-Authenticate headers for client guidance

## Advanced Usage

### Multiple Authentication Methods

You can configure multiple authentication methods that work together:

```yaml
auth:
  require_auth: true
  jwt:
    secret: "jwt-secret"
  basic_auth:
    credentials:
      admin: "password"
  api_key:
    header_name: "X-API-Key"
    keys: ["key1", "key2"]
```

MockForge will try each configured method until one succeeds.

### Environment Variables

You can also configure authentication via environment variables:

```bash
export MOCKFORGE_JWT_SECRET="your-secret"
export MOCKFORGE_OAUTH2_CLIENT_ID="client-id"
export MOCKFORGE_OAUTH2_CLIENT_SECRET="client-secret"
```

### Programmatic Configuration

For programmatic usage:

```rust
use mockforge_core::config::{AuthConfig, JwtConfig, BasicAuthConfig};
use mockforge_http::build_router_with_auth;

let auth_config = AuthConfig {
    jwt: Some(JwtConfig {
        secret: Some("your-secret".to_string()),
        ..Default::default()
    }),
    basic_auth: Some(BasicAuthConfig {
        credentials: [("admin".to_string(), "password".to_string())].into(),
    }),
    require_auth: true,
    ..Default::default()
};

let router = build_router_with_auth(
    Some("api-spec.yaml".to_string()),
    None,
    Some(auth_config)
).await;
```

## Troubleshooting

### Common Issues

1. **401 Unauthorized**
   - Check that authentication is properly configured
   - Verify token format and validity
   - Ensure the correct header names are used

2. **Invalid JWT Signature**
   - Verify the JWT secret/key matches between client and server
   - Check algorithm compatibility
   - Ensure token is not expired

3. **OAuth2 Introspection Failed**
   - Verify OAuth2 server URLs are correct
   - Check client credentials
   - Ensure introspection endpoint is accessible

### Debug Logging

Enable debug logging to see authentication details:

```bash
RUST_LOG=mockforge_http=debug mockforge serve --config config.yaml
```

This will show:
- Authentication method attempts
- Token validation results
- Security scheme matching
- Error details

## Examples

### Complete Configuration

See `config.example.auth.yaml` for a complete configuration example with all authentication methods.

### Testing Script

```bash
#!/bin/bash

# Generate a test JWT
JWT_SECRET="your-secret"
HEADER=$(echo -n '{"alg":"HS256","typ":"JWT"}' | base64 -w 0 | tr '+/' '-_' | tr -d '=')
PAYLOAD=$(echo -n '{"sub":"test","iss":"test","aud":"test","exp":'$(( $(date +%s) + 3600 ))'}' | base64 -w 0 | tr '+/' '-_' | tr -d '=')
SIGNATURE=$(echo -n "$HEADER.$PAYLOAD" | openssl dgst -sha256 -hmac "$JWT_SECRET" -binary | base64 -w 0 | tr '+/' '-_' | tr -d '=')
TOKEN="$HEADER.$PAYLOAD.$SIGNATURE"

# Test JWT authentication
curl -H "Authorization: Bearer $TOKEN" http://localhost:3000/api/test

# Test Basic auth
curl -H "Authorization: Basic $(echo -n 'admin:password' | base64)" http://localhost:3000/api/test

# Test API key
curl -H "X-API-Key: api-key-1" http://localhost:3000/api/test
```

## Security Best Practices

1. **Use HTTPS** in production
2. **Rotate secrets regularly**
3. **Use strong, unique secrets**
4. **Limit token lifetimes**
5. **Validate issuers and audiences**
6. **Monitor authentication failures**
7. **Use environment variables** for secrets (not config files)

## API Reference

### Authentication Middleware

The authentication middleware is automatically applied to all routes except:
- Health check endpoints (`/health`)
- Admin API endpoints (`/__mockforge/*`)

### Error Responses

- `401 Unauthorized`: Authentication required or failed
- `WWW-Authenticate` header provided for client guidance
- Detailed error messages in development mode

### Claims Access

Authenticated claims are available in request extensions:

```rust
use axum::Extension;
use mockforge_http::auth::AuthClaims;

async fn handler(Extension(claims): Extension<AuthClaims>) -> impl IntoResponse {
    Json(serde_json::json!({
        "user": claims.sub,
        "roles": claims.roles
    }))
}
```
