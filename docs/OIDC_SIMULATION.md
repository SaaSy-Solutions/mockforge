# OIDC Simulation Guide

**Pillars:** [Reality][DevX]

[Reality] - Makes mocks feel like real backends through realistic OIDC simulation
[DevX] - Improves developer experience with easy-to-use authentication testing

MockForge provides OpenID Connect (OIDC) simulation capabilities to help you test applications that integrate with identity providers. This guide explains how to configure and use OIDC simulation features.

## Overview

OIDC simulation in MockForge includes:

- **Discovery Document**: `/.well-known/openid-configuration` endpoint
- **JWKS Endpoint**: `/.well-known/jwks.json` for public key distribution
- **Signed JWT Generation**: Configurable JWT tokens with custom claims
- **Multi-tenant Support**: Organization and tenant ID claims
- **Identity Provider Simulation**: Different claim structures for different providers

## Configuration

### Basic OIDC Configuration

```yaml
auth:
  oidc:
    enabled: true
    issuer: "https://mockforge.example.com"
    jwks:
      keys:
        - kid: "key-1"
          alg: "RS256"
          kty: "RSA"
          use: "sig"
          public_key: |
            -----BEGIN PUBLIC KEY-----
            MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA...
            -----END PUBLIC KEY-----
          private_key: |
            -----BEGIN PRIVATE KEY-----
            MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC...
            -----END PRIVATE KEY-----
    claims:
      default:
        - sub
        - iss
        - exp
        - iat
        - email
        - name
      custom:
        email: "test@example.com"
        name: "Test User"
    multi_tenant:
      enabled: true
      org_id_claim: "org_id"
      tenant_id_claim: "tenant_id"
```

### Supported Algorithms

- **RS256, RS384, RS512**: RSA with SHA-256/384/512
- **ES256, ES384, ES512**: ECDSA with SHA-256/384/512
- **HS256, HS384, HS512**: HMAC with SHA-256/384/512

## Discovery Document

The discovery document is available at `/.well-known/openid-configuration` and provides information about the OIDC provider's capabilities:

```json
{
  "issuer": "https://mockforge.example.com",
  "authorization_endpoint": "https://mockforge.example.com/oauth2/authorize",
  "token_endpoint": "https://mockforge.example.com/oauth2/token",
  "userinfo_endpoint": "https://mockforge.example.com/oauth2/userinfo",
  "jwks_uri": "https://mockforge.example.com/.well-known/jwks.json",
  "response_types_supported": ["code", "id_token", "token id_token"],
  "subject_types_supported": ["public"],
  "id_token_signing_alg_values_supported": ["RS256", "ES256", "HS256"],
  "scopes_supported": ["openid", "profile", "email", "address", "phone"],
  "claims_supported": ["sub", "iss", "aud", "exp", "iat", ...],
  "grant_types_supported": ["authorization_code", "implicit", "refresh_token", "client_credentials"]
}
```

## JWKS Endpoint

The JWKS (JSON Web Key Set) endpoint at `/.well-known/jwks.json` provides public keys for token verification:

```json
{
  "keys": [
    {
      "kid": "key-1",
      "kty": "RSA",
      "alg": "RS256",
      "use": "sig",
      "n": "...",
      "e": "AQAB"
    }
  ]
}
```

## Generating Signed JWTs

### Using the OIDC Module

```rust
use mockforge_http::auth::oidc::{OidcState, generate_oidc_token};

// Generate token with default claims
let token = generate_oidc_token(
    &oidc_state,
    "user-123".to_string(),
    None,
    Some(3600), // 1 hour expiration
)?;

// Generate token with custom claims
let mut custom_claims = HashMap::new();
custom_claims.insert("email".to_string(), json!("user@example.com"));
custom_claims.insert("role".to_string(), json!("admin"));

let token = generate_oidc_token(
    &oidc_state,
    "user-123".to_string(),
    Some(custom_claims),
    Some(3600),
)?;
```

### Direct JWT Generation

```rust
use mockforge_http::auth::oidc::generate_signed_jwt;
use jsonwebtoken::{Algorithm, EncodingKey};

let mut claims = HashMap::new();
claims.insert("sub".to_string(), json!("user-123"));
claims.insert("email".to_string(), json!("user@example.com"));

let token = generate_signed_jwt(
    claims,
    Some("key-1".to_string()),
    Algorithm::RS256,
    &encoding_key,
    Some(3600),
    Some("https://mockforge.example.com".to_string()),
    Some("my-app".to_string()),
)?;
```

## Multi-tenant Support

When multi-tenant mode is enabled, tokens automatically include organization and tenant identifiers:

```yaml
auth:
  oidc:
    multi_tenant:
      enabled: true
      org_id_claim: "org_id"
      tenant_id_claim: "tenant_id"
```

Generated tokens will include:
```json
{
  "sub": "user-123",
  "org_id": "org-456",
  "tenant_id": "tenant-789",
  ...
}
```

## Identity Provider Simulation

You can simulate different identity providers by configuring different claim structures:

### Azure AD Simulation

```yaml
auth:
  oidc:
    claims:
      default:
        - sub
        - iss
        - exp
        - iat
        - oid
        - tid
        - upn
      custom:
        oid: "object-id-123"
        tid: "tenant-id-456"
        upn: "user@tenant.onmicrosoft.com"
```

### Google OAuth Simulation

```yaml
auth:
  oidc:
    claims:
      default:
        - sub
        - iss
        - exp
        - iat
        - email
        - email_verified
        - name
        - picture
      custom:
        email: "user@gmail.com"
        email_verified: true
        name: "John Doe"
        picture: "https://example.com/avatar.jpg"
```

## Testing with OIDC

### Using curl

```bash
# Get discovery document
curl https://mockforge.example.com/.well-known/openid-configuration

# Get JWKS
curl https://mockforge.example.com/.well-known/jwks.json
```

### Using OIDC Client Libraries

Most OIDC client libraries can automatically discover configuration:

```python
from oic import rndstr
from oic.oic import Client
from oic.utils.authn.client import CLIENT_AUTHN_METHOD

# Auto-discovery
client = Client(client_authn_method=CLIENT_AUTHN_METHOD)
client.provider_config("https://mockforge.example.com")
```

## Best Practices

1. **Key Rotation**: Regularly rotate signing keys and update JWKS
2. **Claim Validation**: Always validate claims in your application
3. **Token Expiration**: Set appropriate expiration times for different token types
4. **Multi-tenant Isolation**: Use org_id and tenant_id for proper isolation
5. **Security**: Never expose private keys in configuration files

## See Also

- [Consent & Risk Simulation Guide](CONSENT_RISK_SIMULATION.md)
- [Token Lifecycle Scenarios Guide](TOKEN_LIFECYCLE_SCENARIOS.md)
- [OAuth2 Documentation](../book/src/user-guide/security.md)
