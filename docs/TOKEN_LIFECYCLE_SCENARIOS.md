# Token Lifecycle Scenarios Guide

MockForge provides first-class support for testing token lifecycle edge cases, including token revocation, key rotation, clock skew, and other scenarios that are difficult to test in production.

## Overview

Token Lifecycle Scenarios include:

- **Token Revocation**: Simulate token revocation mid-session
- **Key Rotation**: Test behavior during signing key rotation
- **Clock Skew**: Simulate time synchronization issues
- **Prebuilt Test Scenarios**: Easy-to-use endpoints for common test cases

## Configuration

### Token Lifecycle Configuration

```yaml
auth:
  token_lifecycle:
    revocation_enabled: true
    key_rotation_enabled: true
    clock_skew_seconds: 0
    revocation_store: "memory"  # or "redis", "database"
    key_rotation_grace_period_seconds: 3600
```

## Token Revocation

### Revoking Tokens

Tokens can be revoked at any time, simulating scenarios like:
- User logout
- Security breach
- Token compromise
- Administrative action

#### Revoke Single Token

```bash
POST /api/v1/auth/tokens/revoke
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "reason": "user_logout"
}
```

#### Revoke All Tokens for User

```bash
POST /api/v1/auth/tokens/revoke/user
{
  "user_id": "user-123",
  "reason": "security_breach"
}
```

#### Revoke Tokens by Scope

```bash
POST /api/v1/auth/tokens/revoke/scope
{
  "scope": "admin",
  "reason": "permission_revoked"
}
```

### Checking Token Revocation Status

```bash
GET /api/v1/auth/tokens/status?token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

Response:
```json
{
  "revoked": true,
  "revoked_at": "2025-01-27T12:00:00Z",
  "reason": "user_logout"
}
```

### Testing Revoked Token Behavior

When a revoked token is used, the API should return `401 Unauthorized`:

```bash
# Use revoked token
curl -H "Authorization: Bearer <revoked_token>" https://api.example.com/protected

# Expected response
HTTP/1.1 401 Unauthorized
{
  "error": "token_revoked",
  "error_description": "The token has been revoked"
}
```

## Key Rotation

### Rotating Signing Keys

Key rotation is critical for security. MockForge supports:

1. **Add New Key**: Add a new signing key to JWKS
2. **Mark Old Key as Inactive**: Old keys remain valid during grace period
3. **Remove Old Key**: After grace period, remove old keys

#### Rotate Keys

```bash
POST /api/v1/auth/keys/rotate
{
  "new_key": {
    "kid": "key-2",
    "alg": "RS256",
    "public_key": "...",
    "private_key": "..."
  },
  "grace_period_seconds": 3600
}
```

#### Get Active Keys

```bash
GET /api/v1/auth/keys/active
```

Response:
```json
{
  "keys": [
    {
      "kid": "key-1",
      "status": "active",
      "created_at": "2025-01-01T00:00:00Z"
    },
    {
      "kid": "key-2",
      "status": "active",
      "created_at": "2025-01-27T12:00:00Z"
    }
  ]
}
```

### Testing Key Rotation

1. **Generate token with old key**
2. **Rotate to new key**
3. **Verify old token still works** (during grace period)
4. **Generate new token with new key**
5. **After grace period, old tokens should fail**

### Key Rotation Scenarios

#### Scenario 1: Graceful Rotation

```yaml
steps:
  1. Generate token with key-1
  2. Rotate to key-2 (grace period: 1 hour)
  3. Old token still valid
  4. New tokens use key-2
  5. After 1 hour, key-1 removed
```

#### Scenario 2: Immediate Rotation

```yaml
steps:
  1. Generate token with key-1
  2. Rotate to key-2 (grace period: 0)
  3. Old token immediately invalid
  4. New tokens use key-2
```

## Clock Skew

### Simulating Clock Skew

Clock skew occurs when the server and client have different system times, causing token validation to fail even with valid tokens.

#### Set Clock Skew

```bash
POST /api/v1/auth/clock/skew
{
  "skew_seconds": 300,  # Server is 5 minutes ahead
  "apply_to": "all"     # or "issuance", "validation"
}
```

#### Get Current Clock Skew

```bash
GET /api/v1/auth/clock/skew
```

Response:
```json
{
  "skew_seconds": 300,
  "server_time": "2025-01-27T12:05:00Z",
  "adjusted_time": "2025-01-27T12:00:00Z"
}
```

### Testing Clock Skew Scenarios

#### Scenario 1: Server Ahead

```yaml
skew_seconds: 300
expected_behavior: Tokens issued with server time appear expired to clients
```

#### Scenario 2: Server Behind

```yaml
skew_seconds: -300
expected_behavior: Tokens appear not yet valid (nbf claim)
```

## Prebuilt Test Scenarios

MockForge provides prebuilt endpoints for common test scenarios:

### Force Refresh Token Failure

Simulate refresh token failure scenarios:

```bash
POST /api/v1/auth/test/force-refresh-failure
{
  "user_id": "user-123",
  "failure_type": "expired"  # or "revoked", "invalid", "network_error"
}
```

### Simulate Token Revocation Mid-Session

```bash
POST /api/v1/auth/test/revoke-mid-session
{
  "user_id": "user-123",
  "delay_seconds": 30  # Revoke after 30 seconds
}
```

### Simulate Key Rotation

```bash
POST /api/v1/auth/test/rotate-keys
{
  "grace_period_seconds": 60,
  "auto_remove_old": true
}
```

### Simulate Clock Skew

```bash
POST /api/v1/auth/test/clock-skew
{
  "skew_seconds": 300,
  "duration_seconds": 3600  # Apply for 1 hour
}
```

## Front-End Testing Helpers

### React Hook Example

```typescript
import { useTokenLifecycle } from '@mockforge/auth';

function TestComponent() {
  const {
    revokeToken,
    rotateKeys,
    setClockSkew,
    forceRefreshFailure
  } = useTokenLifecycle();

  const handleTestRevocation = async () => {
    await revokeToken('current-token', 'user_logout');
    // Next API call should fail with 401
  };

  const handleTestKeyRotation = async () => {
    await rotateKeys({ gracePeriod: 60 });
    // Old tokens work for 60 seconds, then fail
  };

  return (
    <div>
      <button onClick={handleTestRevocation}>
        Test Token Revocation
      </button>
      <button onClick={handleTestKeyRotation}>
        Test Key Rotation
      </button>
    </div>
  );
}
```

### Vue Composable Example

```typescript
import { useTokenLifecycle } from '@mockforge/auth';

export default {
  setup() {
    const { revokeToken, rotateKeys } = useTokenLifecycle();

    const testRevocation = async () => {
      await revokeToken('current-token', 'user_logout');
    };

    return { testRevocation };
  }
};
```

## Testing Workflows

### Workflow 1: Token Revocation Test

```yaml
steps:
  1. User logs in, receives token
  2. User makes API calls (should succeed)
  3. Admin revokes token
  4. User makes API call (should fail with 401)
  5. User refreshes token
  6. User makes API call (should succeed)
```

### Workflow 2: Key Rotation Test

```yaml
steps:
  1. Generate token with key-1
  2. Make API calls (should succeed)
  3. Rotate to key-2 (grace period: 5 minutes)
  4. Make API calls with old token (should still succeed)
  5. Wait 5 minutes
  6. Make API call with old token (should fail)
  7. Refresh token (gets new token with key-2)
  8. Make API calls (should succeed)
```

### Workflow 3: Clock Skew Test

```yaml
steps:
  1. Set clock skew: +5 minutes
  2. Generate token
  3. Token appears expired to client (iat/exp claims)
  4. Client adjusts for skew
  5. Token validation succeeds
```

## Best Practices

1. **Test All Scenarios**: Test revocation, rotation, and clock skew separately and together
2. **Grace Periods**: Use realistic grace periods for key rotation
3. **Error Handling**: Verify proper error responses for all failure scenarios
4. **Client Resilience**: Test that clients handle token lifecycle events gracefully
5. **Monitoring**: Monitor token revocation and key rotation events

## See Also

- [OIDC Simulation Guide](OIDC_SIMULATION.md)
- [Consent & Risk Simulation Guide](CONSENT_RISK_SIMULATION.md)
- [OAuth2 Documentation](../book/src/user-guide/security.md)
