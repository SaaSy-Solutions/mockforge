# JWT Authentication Plugin

A MockForge plugin that provides JWT-based authentication with configurable token validation, claims extraction, and user role management.

## Features

- **Multiple Algorithms**: Support for HS256, RS256, and ES256
- **Claims Extraction**: Configurable extraction of standard and custom claims
- **Issuer/Audience Validation**: Optional validation of token issuer and audience
- **Clock Skew Tolerance**: Configurable tolerance for time-based claims
- **Role-Based Access**: Extract user roles and permissions from tokens

## Installation

```bash
# Build the plugin
cargo build --target wasm32-wasi --release

# Install to MockForge
mockforge plugin install ./target/wasm32-wasi/release/mockforge_plugin_auth_jwt.wasm
```

## Configuration

Add to your MockForge configuration:

```yaml
plugins:
  auth-jwt:
    verification_key: "your-secret-key-here"
    algorithms: ["HS256", "RS256"]
    required_issuer: "https://your-issuer.com"
    required_audience: "your-app"
    extract_claims: ["sub", "email", "roles", "department"]
```

## Usage

### Basic Authentication

```bash
# Make a request with JWT token
curl -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
     http://localhost:3000/api/protected
```

### Template Integration

Use extracted claims in response templates:

```json
{
  "user": "{{auth.user_id}}",
  "email": "{{auth.email}}",
  "roles": "{{auth.roles}}",
  "message": "Welcome back, {{auth.email}}!"
}
```

## Token Format

The plugin expects JWT tokens with the following standard claims:

```json
{
  "sub": "user123",
  "email": "user@example.com",
  "roles": ["admin", "user"],
  "permissions": ["read", "write"],
  "iss": "https://your-issuer.com",
  "aud": "your-app",
  "exp": 1640995200,
  "iat": 1640991600
}
```

## Security Considerations

- **Key Management**: Store verification keys securely
- **Algorithm Selection**: Choose appropriate algorithms for your use case
- **Clock Skew**: Adjust tolerance based on your infrastructure
- **Claims Validation**: Validate extracted claims before use

## Examples

### HS256 Token Generation (Node.js)

```javascript
const jwt = require('jsonwebtoken');

const payload = {
  sub: 'user123',
  email: 'user@example.com',
  roles: ['admin'],
  permissions: ['read', 'write']
};

const token = jwt.sign(payload, 'your-secret-key', {
  algorithm: 'HS256',
  issuer: 'https://your-app.com',
  audience: 'your-app'
});
```

### RS256 Token Generation (Python)

```python
import jwt
from cryptography.hazmat.primitives import serialization

# Load private key
with open('private.pem', 'rb') as f:
    private_key = serialization.load_pem_private_key(f.read(), password=None)

payload = {
    'sub': 'user123',
    'email': 'user@example.com',
    'roles': ['admin']
}

token = jwt.encode(payload, private_key, algorithm='RS256')
```

## Error Handling

The plugin returns appropriate error messages for common issues:

- **Invalid Token**: Malformed or corrupted JWT
- **Expired Token**: Token past expiration time
- **Invalid Signature**: Token signature verification failed
- **Missing Claims**: Required claims not present
- **Algorithm Not Supported**: Unsupported signing algorithm

## Development

### Building

```bash
# Install WASM target
rustup target add wasm32-wasi

# Build
cargo build --target wasm32-wasi --release
```

### Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_valid_token() {
        let plugin = JwtAuthPlugin::new();
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...";

        let result = plugin.authenticate(
            &PluginContext::default(),
            &AuthCredentials::Bearer(token.to_string())
        ).await;

        assert!(result.success);
    }
}
```

## License

MIT OR Apache-2.0
