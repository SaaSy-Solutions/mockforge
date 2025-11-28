# Security & Encryption

MockForge provides enterprise-grade security features including end-to-end encryption, secure key management, and comprehensive authentication systems to protect your mock data and configurations.

## Overview

MockForge's security features include:

- **End-to-End Encryption**: AES-256-GCM and ChaCha20-Poly1305 algorithms
- **Hierarchical Key Management**: Master keys, workspace keys, and session keys
- **Auto-Encryption**: Automatic encryption of sensitive configuration data
- **Secure Storage**: OS keychain integration and file-based key storage
- **Template Encryption**: Built-in encryption/decryption functions in templates
- **Role-Based Access Control**: Admin and viewer roles in the UI
- **Plugin Security**: Sandboxed plugin execution with capability controls

## Encryption Setup

### Initial Configuration

Enable encryption when starting MockForge:

```bash
# Enable encryption with environment variables
export MOCKFORGE_ENCRYPTION_ENABLED=true
export MOCKFORGE_ENCRYPTION_ALGORITHM=aes-256-gcm
export MOCKFORGE_KEY_STORE_PATH=~/.mockforge/keys

# Start MockForge with encryption
mockforge serve --config config.yaml
```

### Configuration File

Configure encryption in your YAML configuration:

```yaml
# config.yaml
encryption:
  enabled: true
  algorithm: "aes-256-gcm"  # or "chacha20-poly1305"
  key_store:
    type: "file"  # or "os_keychain"
    path: "~/.mockforge/keys"
    auto_create: true
  
  # Auto-encryption rules
  auto_encrypt:
    enabled: true
    patterns:
      - "*.password"
      - "*.secret"
      - "*.key"
      - "*.token"
      - "auth.headers.*"
      - "database.connection_string"
  
  # Key rotation
  rotation:
    enabled: true
    interval_days: 30
    backup_count: 5
```

## Key Management

### Key Hierarchy

MockForge uses a hierarchical key system:

1. **Master Key**: Root encryption key stored securely
2. **Workspace Keys**: Per-workspace encryption keys derived from master key
3. **Session Keys**: Temporary keys for active sessions
4. **Data Keys**: Keys for encrypting specific data elements

### Key Storage Options

#### File-Based Storage

Store keys in encrypted files on the local filesystem:

```yaml
encryption:
  key_store:
    type: "file"
    path: "~/.mockforge/keys"
    permissions: "0600"  # Owner read/write only
    backup_enabled: true
    backup_path: "~/.mockforge/keys.backup"
```

#### OS Keychain Integration

Use the operating system's secure keychain:

```yaml
encryption:
  key_store:
    type: "os_keychain"
    service_name: "mockforge"
    account_prefix: "workspace_"
```

**Supported Platforms:**
- **macOS**: Uses Keychain Services
- **Windows**: Uses Windows Credential Manager
- **Linux**: Uses Secret Service API (GNOME Keyring, KWallet)

### Key Generation

MockForge automatically generates keys when needed:

```bash
# Initialize new key store
mockforge keys init --algorithm aes-256-gcm

# Generate workspace key
mockforge keys generate --workspace my-workspace

# Rotate all keys
mockforge keys rotate --all

# Export keys for backup (encrypted)
mockforge keys export --output keys-backup.enc
```

### Key Rotation

Implement automatic key rotation for enhanced security:

```yaml
encryption:
  rotation:
    enabled: true
    interval_days: 30
    max_key_age_days: 90
    backup_old_keys: true
    notify_before_rotation_days: 7
```

## Encryption Algorithms

### AES-256-GCM (Default)

```yaml
encryption:
  algorithm: "aes-256-gcm"
  config:
    key_size: 256
    iv_size: 12
    tag_size: 16
```

**Features:**
- **Performance**: Optimized for speed on modern CPUs
- **Security**: NIST-approved, widely audited
- **Authentication**: Built-in message authentication
- **Hardware Support**: AES-NI acceleration on Intel/AMD

### ChaCha20-Poly1305

```yaml
encryption:
  algorithm: "chacha20-poly1305"
  config:
    key_size: 256
    nonce_size: 12
    tag_size: 16
```

**Features:**
- **Performance**: Excellent on ARM and older CPUs
- **Security**: Modern, quantum-resistant design
- **Authentication**: Integrated Poly1305 MAC
- **Simplicity**: Fewer implementation pitfalls

## Auto-Encryption

MockForge automatically encrypts sensitive data based on configurable patterns:

### Configuration Patterns

```yaml
encryption:
  auto_encrypt:
    enabled: true
    patterns:
      # Password fields
      - "*.password"
      - "*.passwd"
      - "auth.password"
      
      # API keys and tokens
      - "*.api_key"
      - "*.secret_key"
      - "*.access_token"
      - "*.refresh_token"
      
      # Database connections
      - "database.password"
      - "database.connection_string"
      - "redis.password"
      
      # HTTP headers
      - "auth.headers.Authorization"
      - "auth.headers.X-API-Key"
      
      # Custom patterns
      - "custom.sensitive_data.*"
```

### Field-Level Encryption

Encrypt specific fields in your configurations:

```yaml
# Original configuration
database:
  host: "localhost"
  port: 5432
  username: "user"
  password: "secret123"  # Will be auto-encrypted
  
auth:
  jwt_secret: "my-secret"  # Will be auto-encrypted
  
# After auto-encryption
database:
  host: "localhost"
  port: 5432
  username: "user"
  password: "{{encrypted:AES256:base64-encrypted-data}}"
  
auth:
  jwt_secret: "{{encrypted:AES256:base64-encrypted-data}}"
```

## Template Encryption Functions

Use encryption functions directly in your templates:

### Encryption Functions

```yaml
# Encrypt data in templates
response:
  body:
    user_id: "{{uuid}}"
    encrypted_data: "{{encrypt('sensitive-data', 'workspace-key')}}"
    hashed_password: "{{hash('password123', 'sha256')}}"
    signed_token: "{{sign(user_data, 'signing-key')}}"
```

### Decryption Functions

```yaml
# Decrypt data in templates
request:
  headers:
    Authorization: "Bearer {{decrypt(encrypted_token, 'workspace-key')}}"
  body:
    password: "{{decrypt(user.encrypted_password, 'user-key')}}"
```

### Available Functions

| Function | Description | Example |
|----------|-------------|---------|
| `encrypt(data, key)` | Encrypt data with specified key | `{{encrypt('secret', 'my-key')}}` |
| `decrypt(data, key)` | Decrypt data with specified key | `{{decrypt(encrypted_data, 'my-key')}}` |
| `hash(data, algorithm)` | Hash data with algorithm | `{{hash('password', 'sha256')}}` |
| `hmac(data, key, algorithm)` | Generate HMAC signature | `{{hmac(message, 'secret', 'sha256')}}` |
| `sign(data, key)` | Sign data with private key | `{{sign(payload, 'private-key')}}` |
| `verify(data, signature, key)` | Verify signature with public key | `{{verify(data, sig, 'public-key')}}` |

## Mutual TLS (mTLS)

MockForge supports **Mutual TLS (mTLS)** for enhanced security, requiring both server and client certificates for authentication.

### Quick Start

Enable mTLS in your configuration:

```yaml
http:
  tls:
    enabled: true
    cert_file: "./certs/server.crt"
    key_file: "./certs/server.key"
    ca_file: "./certs/ca.crt"           # CA certificate for client verification
    require_client_cert: true            # Enable mTLS
```

### Client Configuration

Clients must provide a certificate signed by the CA:

```bash
# Using cURL
curl --cert client.crt --key client.key --cacert ca.crt \
  https://localhost:3000/api/endpoint
```

### Certificate Generation

For development, use `mkcert`:

```bash
# Install mkcert
brew install mkcert
mkcert -install

# Generate certificates
mkcert localhost 127.0.0.1 ::1
mkcert -client localhost 127.0.0.1 ::1
```

For production, use OpenSSL or a trusted Certificate Authority.

**Full Documentation:** See [mTLS Configuration Guide](../../docs/mTLS_CONFIGURATION.md) for complete setup instructions, certificate generation, client examples, and troubleshooting.

## Authentication & Authorization

### Admin UI Authentication

MockForge Admin UI v2 includes **complete role-based authentication** with JWT-based authentication:

```yaml
admin:
  auth:
    enabled: true
    jwt_secret: "{{encrypted:your-jwt-secret}}"
    session_timeout: 86400  # 24 hours
    
    # Built-in users
    users:
      admin:
        password: "{{encrypted:admin-password}}"
        role: "admin"
      viewer:
        password: "{{encrypted:viewer-password}}"
        role: "viewer"
        
    # Custom authentication provider
    provider: "custom"
    provider_config:
      ldap_url: "ldap://company.com"
      oauth2_client_id: "mockforge-client"
```

### Role Permissions

| Role | Permissions |
|------|------------|
| **Admin** | Full access to all features (workspace management, member management, all editing) |
| **Editor** | Create, edit, and delete mocks; view history; cannot manage workspace settings |
| **Viewer** | Read-only access to dashboard, logs, metrics, and mocks |

**Full Documentation:** See [RBAC Guide](../../docs/RBAC_GUIDE.md) for complete role and permission details.

### Custom Authentication

Implement custom authentication via plugins:

```rust
// Custom auth plugin
use mockforge_plugin_core::{AuthProvider, AuthResult};

pub struct LdapAuthProvider {
    ldap_url: String,
    base_dn: String,
}

impl AuthProvider for LdapAuthProvider {
    fn authenticate(&self, username: &str, password: &str) -> AuthResult {
        // LDAP authentication logic
        match self.ldap_authenticate(username, password) {
            Ok(user_info) => AuthResult::success(user_info),
            Err(e) => AuthResult::failure(e.to_string()),
        }
    }
}
```

## Plugin Security

### Capability System

Plugins must declare required capabilities:

```yaml
# plugin.yaml
capabilities:
  - "crypto.encrypt"      # Encryption functions
  - "crypto.decrypt"      # Decryption functions
  - "crypto.hash"         # Hashing functions
  - "crypto.random"       # Random number generation
  - "storage.encrypted"   # Encrypted storage access
  - "network.tls"         # TLS/SSL connections
```

### Resource Limits

Configure security limits for plugins:

```yaml
plugins:
  security:
    memory_limit_mb: 64
    cpu_limit_percent: 5
    network_timeout_ms: 5000
    file_access_paths:
      - "/app/data"
      - "/tmp/plugin-cache"
    
    # Encryption access
    encryption_access:
      allowed_algorithms: ["aes-256-gcm"]
      key_access_patterns: ["workspace.*", "plugin.*"]
```

### Sandboxing

Plugins run in secure sandboxes that:

- **Isolate Memory**: Separate memory space from host process
- **Limit File Access**: Restricted to declared paths only
- **Control Network**: Limited to specified endpoints
- **Monitor Resources**: CPU, memory, and execution time limits
- **Audit Operations**: Log all security-relevant operations

## Transport Security

### TLS Configuration

Enable TLS for all network communication:

```yaml
# Server TLS
server:
  tls:
    enabled: true
    cert_file: "/path/to/server.crt"
    key_file: "/path/to/server.key"
    min_version: "1.3"
    cipher_suites:
      - "TLS_AES_256_GCM_SHA384"
      - "TLS_CHACHA20_POLY1305_SHA256"

# Client TLS (for outbound requests)
client:
  tls:
    verify_certificates: true
    ca_bundle: "/path/to/ca-bundle.crt"
    client_cert: "/path/to/client.crt"
    client_key: "/path/to/client.key"
```

### Certificate Management

```bash
# Generate self-signed certificates for development
mockforge certs generate --domain localhost --output ./certs/

# Use Let's Encrypt for production
mockforge certs letsencrypt --domain api.mockforge.dev --email admin@company.com

# Import existing certificates
mockforge certs import --cert server.crt --key server.key --ca ca.crt
```

## Security Best Practices

### Configuration Security

1. **Encrypt Sensitive Data**: Use auto-encryption for passwords and keys
2. **Secure Key Storage**: Use OS keychain in production
3. **Regular Key Rotation**: Implement automatic key rotation
4. **Least Privilege**: Grant minimal necessary permissions
5. **Audit Logging**: Enable comprehensive security logging

### Deployment Security

1. **Use TLS**: Enable TLS for all network communication
2. **Network Isolation**: Deploy in isolated network segments
3. **Access Control**: Implement proper firewall rules
4. **Monitor Security**: Set up security monitoring and alerting
5. **Regular Updates**: Keep MockForge and dependencies updated

### Plugin Security

1. **Review Plugin Code**: Audit plugin source code before installation
2. **Limit Capabilities**: Grant only necessary plugin permissions
3. **Monitor Resources**: Watch plugin resource usage
4. **Isolate Environments**: Use separate configs for dev/prod
5. **Update Regularly**: Keep plugins updated for security fixes

## Security Monitoring

### Audit Logging

Enable comprehensive security logging:

```yaml
logging:
  security:
    enabled: true
    level: "info"
    destinations:
      - type: "file"
        path: "/var/log/mockforge/security.log"
        format: "json"
      - type: "syslog"
        facility: "local0"
        tag: "mockforge-security"
    
    events:
      - "auth_success"
      - "auth_failure"
      - "key_access"
      - "encryption_operation"
      - "plugin_security_violation"
      - "configuration_change"
```

### Security Metrics

Monitor security-related metrics:

```yaml
metrics:
  security:
    enabled: true
    metrics:
      - "auth_attempts_total"
      - "auth_failures_total"
      - "encryption_operations_total"
      - "key_rotations_total"
      - "plugin_security_violations_total"
```

### Alerting

Set up security alerts:

```yaml
alerts:
  security:
    enabled: true
    rules:
      - name: "High Authentication Failures"
        condition: "auth_failures_rate > 10/minute"
        action: "email_admin"
      
      - name: "Plugin Security Violation"
        condition: "plugin_security_violations > 0"
        action: "disable_plugin"
      
      - name: "Encryption Key Access Anomaly"
        condition: "key_access_rate > 100/minute"
        action: "alert_security_team"
```

## Compliance & Standards

### Standards Compliance

MockForge security features comply with:

- **FIPS 140-2**: Cryptographic standards compliance
- **Common Criteria**: Security evaluation criteria
- **SOC 2 Type II**: Security, availability, and confidentiality
- **ISO 27001**: Information security management

### Data Protection

Features for data protection compliance:

- **Data Encryption**: All sensitive data encrypted at rest and in transit
- **Key Management**: Secure key lifecycle management
- **Access Controls**: Role-based access and audit trails
- **Data Minimization**: Only collect and store necessary data
- **Right to Deletion**: Secure data deletion capabilities

## Audit Logging

MockForge provides comprehensive audit logging for security and compliance:

- **Authentication Audit Logs**: Track all authentication attempts (success/failure)
- **Request Logs**: Full request/response logging with metadata
- **Collaboration History**: Git-style version control for workspace changes
- **Configuration Changes**: Track all configuration modifications
- **Plugin Activity**: Monitor plugin execution and security events

**Full Documentation:** See [Audit Trails Guide](../../docs/AUDIT_TRAILS.md) for complete audit logging configuration and usage.

## Troubleshooting Security

### Common Issues

#### Encryption Not Working

```bash
# Check encryption status
mockforge encryption status

# Verify key store
mockforge keys list

# Test encryption/decryption
mockforge encrypt test-data --key workspace-key
```

#### Authentication Failures

```bash
# Check auth configuration
mockforge auth status

# Verify JWT secret
mockforge auth verify-jwt your-token

# Reset admin credentials
mockforge auth reset-admin
```

#### Key Store Issues

```bash
# Initialize key store
mockforge keys init --force

# Repair key store
mockforge keys repair

# Backup and restore
mockforge keys backup --output keys.backup
mockforge keys restore --input keys.backup
```

### Debug Mode

Enable security debug logging:

```bash
RUST_LOG=mockforge_core::encryption=debug,mockforge_core::auth=debug mockforge serve
```

This comprehensive security system ensures that MockForge can be safely used in enterprise environments while protecting sensitive mock data and configurations.