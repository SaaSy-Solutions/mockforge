# Consent & Risk Simulation Guide

MockForge provides consent screen simulation and risk-based authentication challenges to help you test applications that integrate with identity providers requiring user consent and risk assessment.

## Overview

Consent & Risk Simulation includes:

- **Consent Screen Simulation**: Mock consent screens with permissions/scopes toggles
- **Risk Engine**: Configurable risk assessment with sliders for MFA prompts, device challenges, and blocked logins
- **Admin UI Controls**: Visual controls for simulating different risk scenarios
- **OAuth2 Integration**: Seamless integration with OAuth2 authorization flows

## Configuration

### Consent Configuration

```yaml
auth:
  consent:
    enabled: true
    require_consent: true
    consent_screen_url: "/consent"
    default_scopes:
      - "openid"
      - "profile"
      - "email"
    consent_timeout_seconds: 300
```

### Risk Engine Configuration

```yaml
auth:
  risk:
    enabled: true
    mfa_threshold: 0.7
    device_challenge_threshold: 0.5
    blocked_login_threshold: 0.9
    risk_factors:
      - name: "new_device"
        weight: 0.3
      - name: "unusual_location"
        weight: 0.4
      - name: "suspicious_activity"
        weight: 0.5
    risk_rules:
      - condition: "risk_score > 0.9"
        action: "block"
      - condition: "risk_score > 0.7"
        action: "require_mfa"
      - condition: "risk_score > 0.5"
        action: "device_challenge"
```

## Consent Screen Simulation

### Basic Consent Flow

When a user authorizes an OAuth2 application, they may be presented with a consent screen showing:

- **Application Name**: The name of the requesting application
- **Requested Permissions**: List of scopes/permissions being requested
- **User Controls**: Toggle switches for each permission
- **Approve/Deny Buttons**: Final consent decision

### Consent Screen UI

The consent screen is available at the configured `consent_screen_url` (default: `/consent`):

```
GET /consent?client_id=my-app&scope=openid profile email&state=xyz123
```

Response includes an interactive consent form with:
- Application information
- Requested scopes with toggle switches
- Approve/Deny buttons
- Privacy policy and terms of service links

### Consent API Endpoints

```bash
# Get consent screen
GET /consent?client_id={client_id}&scope={scopes}&state={state}

# Submit consent decision
POST /consent/decision
{
  "client_id": "my-app",
  "state": "xyz123",
  "approved": true,
  "scopes": ["openid", "profile", "email"]
}
```

## Risk Engine

### Risk Assessment

The risk engine evaluates authentication requests based on multiple factors:

1. **Device Fingerprinting**: New or unrecognized devices
2. **Location Analysis**: Unusual geographic locations
3. **Behavioral Patterns**: Suspicious activity patterns
4. **Time-based Factors**: Unusual login times
5. **IP Reputation**: Known malicious IP addresses

### Risk Score Calculation

Risk scores range from 0.0 (low risk) to 1.0 (high risk):

```
risk_score = Σ(risk_factor_weight × risk_factor_value)
```

### Risk Actions

Based on the calculated risk score, different actions can be triggered:

- **Low Risk (0.0 - 0.5)**: Normal authentication flow
- **Medium Risk (0.5 - 0.7)**: Device challenge (e.g., email verification)
- **High Risk (0.7 - 0.9)**: MFA required (e.g., TOTP, SMS)
- **Critical Risk (0.9 - 1.0)**: Block login attempt

### Simulating Risk Scenarios

#### Via Admin UI

The Admin UI provides sliders and controls to simulate different risk levels:

1. Navigate to `/__mockforge/admin/risk-simulation`
2. Adjust risk factor sliders:
   - New Device: 0.0 - 1.0
   - Unusual Location: 0.0 - 1.0
   - Suspicious Activity: 0.0 - 1.0
3. Set overall risk score override
4. Test authentication flow with configured risk level

#### Via API

```bash
# Set risk score for a user
POST /api/v1/auth/risk/simulate
{
  "user_id": "user-123",
  "risk_score": 0.8,
  "risk_factors": {
    "new_device": 0.9,
    "unusual_location": 0.7
  }
}

# Trigger MFA prompt
POST /api/v1/auth/risk/trigger-mfa
{
  "user_id": "user-123",
  "mfa_type": "totp"
}

# Block login
POST /api/v1/auth/risk/block
{
  "user_id": "user-123",
  "reason": "Suspicious activity detected"
}
```

## MFA Simulation

### Supported MFA Methods

- **TOTP**: Time-based One-Time Password (Google Authenticator, Authy)
- **SMS**: SMS-based verification codes
- **Email**: Email-based verification codes
- **Push Notification**: Push notification approval
- **WebAuthn**: Hardware security keys

### MFA Flow

1. User attempts login
2. Risk engine determines MFA is required
3. User is prompted to select MFA method
4. Verification code/challenge is sent
5. User completes MFA verification
6. Authentication completes

### Simulating MFA

```bash
# Trigger MFA prompt
POST /api/v1/auth/mfa/trigger
{
  "user_id": "user-123",
  "method": "totp"
}

# Verify MFA code
POST /api/v1/auth/mfa/verify
{
  "user_id": "user-123",
  "code": "123456",
  "method": "totp"
}

# Bypass MFA (for testing)
POST /api/v1/auth/mfa/bypass
{
  "user_id": "user-123"
}
```

## Device Challenge Simulation

Device challenges verify that the user controls a trusted device:

- **Email Verification**: Send verification link to registered email
- **SMS Verification**: Send code to registered phone
- **Device Approval**: Require approval from previously trusted device

### Simulating Device Challenges

```bash
# Trigger device challenge
POST /api/v1/auth/device/challenge
{
  "user_id": "user-123",
  "device_id": "device-456",
  "challenge_type": "email"
}

# Complete device challenge
POST /api/v1/auth/device/verify
{
  "user_id": "user-123",
  "device_id": "device-456",
  "verification_code": "abc123"
}
```

## Blocked Login Simulation

When risk is critical, logins can be blocked:

```bash
# Block user login
POST /api/v1/auth/block
{
  "user_id": "user-123",
  "reason": "Multiple failed login attempts",
  "duration_seconds": 3600
}

# Unblock user
POST /api/v1/auth/unblock
{
  "user_id": "user-123"
}

# Check if user is blocked
GET /api/v1/auth/block/status?user_id=user-123
```

## Integration with OAuth2 Flow

Consent and risk simulation integrate seamlessly with OAuth2:

1. **Authorization Request**: User initiates OAuth2 flow
2. **Risk Assessment**: Risk engine evaluates the request
3. **Consent Screen**: If risk is acceptable, show consent screen
4. **MFA/Challenge**: If risk is elevated, require additional verification
5. **Authorization Code**: After consent and verification, issue authorization code

### Example Flow

```
1. GET /oauth2/authorize?client_id=my-app&scope=openid profile
   → Risk engine evaluates: risk_score = 0.6
   → Device challenge required

2. POST /oauth2/device/challenge
   → Email verification sent

3. POST /oauth2/device/verify?code=abc123
   → Device verified, risk_score = 0.3

4. GET /consent?client_id=my-app&scope=openid profile
   → User approves consent

5. GET /oauth2/authorize?client_id=my-app&scope=openid profile&consent=approved
   → Authorization code issued
```

## Testing Scenarios

### Test Case 1: Normal Flow

```yaml
risk_score: 0.2
expected_behavior: Normal authentication, consent screen shown
```

### Test Case 2: Medium Risk

```yaml
risk_score: 0.6
expected_behavior: Device challenge required, then consent screen
```

### Test Case 3: High Risk

```yaml
risk_score: 0.8
expected_behavior: MFA required, then consent screen
```

### Test Case 4: Critical Risk

```yaml
risk_score: 0.95
expected_behavior: Login blocked, no consent screen
```

## Best Practices

1. **Realistic Risk Scores**: Use realistic risk scores that match production scenarios
2. **Test All Paths**: Test all risk levels and corresponding actions
3. **Consent Granularity**: Test consent with different scope combinations
4. **MFA Methods**: Test all supported MFA methods
5. **Error Handling**: Test error scenarios (expired codes, invalid challenges)

## See Also

- [OIDC Simulation Guide](OIDC_SIMULATION.md)
- [Token Lifecycle Scenarios Guide](TOKEN_LIFECYCLE_SCENARIOS.md)
- [OAuth2 Documentation](../book/src/user-guide/security.md)
