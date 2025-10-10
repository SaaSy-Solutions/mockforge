# MockForge Security Whitepaper

**Version:** 1.0
**Date:** 2025-10-09
**Status:** Draft

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Threat Model](#threat-model)
3. [Security Architecture](#security-architecture)
4. [WASM Plugin Sandbox](#wasm-plugin-sandbox)
5. [Encryption & Key Management](#encryption--key-management)
6. [Authentication & Authorization](#authentication--authorization)
7. [Network Security](#network-security)
8. [Input Validation](#input-validation)
9. [Audit & Compliance](#audit--compliance)
10. [Incident Response](#incident-response)

---

## Executive Summary

MockForge is a comprehensive API mocking and testing platform built with security as a fundamental design principle. This whitepaper describes the security architecture, threat model, and defensive mechanisms implemented throughout the system.

### Key Security Features

- **WASM Sandbox:** Isolated plugin execution with resource limits
- **End-to-End Encryption:** AES-256-GCM and ChaCha20-Poly1305
- **Multi-Factor Authentication:** JWT, OAuth2, API keys, Basic auth
- **Rate Limiting:** DDoS protection and abuse prevention
- **Input Validation:** OpenAPI schema validation and sanitization
- **Audit Logging:** Comprehensive security event tracking

---

## Threat Model

### Assets

1. **Configuration Data:** API mocks, fixtures, routing rules
2. **Secrets:** API keys, JWT secrets, OAuth2 credentials
3. **User Data:** Authentication credentials, session tokens
4. **System Integrity:** WASM plugin sandbox, core services
5. **Availability:** API endpoints, admin UI, plugin system

### Threat Actors

1. **External Attackers:** Internet-based adversaries
2. **Malicious Plugins:** Untrusted third-party code
3. **Insider Threats:** Compromised credentials or malicious users
4. **Automated Attacks:** Bots, scrapers, DDoS

### Attack Vectors

| Vector | Threat | Mitigation |
|--------|--------|------------|
| Plugin Code Injection | Malicious WASM execution | Sandbox isolation, resource limits |
| Authentication Bypass | Unauthorized access | Multi-method auth, token validation |
| XSS | UI code injection | Output escaping, CSP headers |
| DDoS | Service disruption | Rate limiting, connection limits |
| Data Exfiltration | Sensitive data theft | Encryption at rest, TLS in transit |
| Privilege Escalation | Admin access abuse | RBAC, audit logging |

---

## Security Architecture

### Defense in Depth

```
┌─────────────────────────────────────────────┐
│         Network Perimeter                    │
│  ┌───────────────────────────────────────┐  │
│  │      TLS/HTTPS Layer                   │  │
│  │  ┌─────────────────────────────────┐  │  │
│  │  │   Rate Limiting & WAF            │  │  │
│  │  │  ┌───────────────────────────┐  │  │  │
│  │  │  │  Authentication Layer      │  │  │  │
│  │  │  │  ┌─────────────────────┐  │  │  │  │
│  │  │  │  │  Input Validation    │  │  │  │  │
│  │  │  │  │  ┌───────────────┐  │  │  │  │  │
│  │  │  │  │  │ WASM Sandbox  │  │  │  │  │  │
│  │  │  │  │  │  (Isolated)   │  │  │  │  │  │
│  │  │  │  │  └───────────────┘  │  │  │  │  │
│  │  │  │  └─────────────────────┘  │  │  │  │
│  │  │  └───────────────────────────┘  │  │  │
│  │  └─────────────────────────────────┘  │  │
│  └───────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

### Security Principles

1. **Least Privilege:** Minimal permissions by default
2. **Fail Secure:** Deny access on errors
3. **Complete Mediation:** All requests validated
4. **Separation of Duties:** Distinct roles and permissions
5. **Defense in Depth:** Multiple security layers
6. **Audit & Accountability:** All actions logged

---

## WASM Plugin Sandbox

### Architecture

The WASM sandbox uses **Wasmtime** to provide secure, isolated plugin execution:

```rust
pub struct WasmSandbox {
    engine: Engine,
    store: Store<()>,
    limits: ResourceLimits,
}

pub struct ResourceLimits {
    max_memory_bytes: usize,      // 10MB default
    max_executions: u64,           // 1000 default
    max_cpu_seconds: u64,          // 5 seconds default
    max_lifetime_hours: u64,       // 1 hour default
}
```

### Security Guarantees

1. **Memory Isolation:** Plugins cannot access host memory
2. **CPU Limits:** Execution time capped per invocation
3. **Resource Quotas:** Memory, execution count, lifetime limits
4. **Capability-based:** Explicit permissions required
5. **No Unsafe Code:** Workspace-wide `unsafe_code = "deny"`

### Capability System

```yaml
capabilities:
  - template                    # Low risk
  - network:http               # Requires approval
  - filesystem:read:/tmp       # Scoped access
  - resource:memory=50MB       # Resource limit
```

### Attack Scenarios

| Attack | Mitigation |
|--------|------------|
| Memory exhaustion | `max_memory_bytes` enforced |
| Infinite loops | `max_cpu_seconds` timeout |
| Fork bombs | Single-threaded execution |
| Host escape | Wasmtime sandbox boundary |
| Side channels | Limited WASI capabilities |

---

## Encryption & Key Management

### Algorithms

**Primary:** AES-256-GCM (authenticated encryption)
- 256-bit keys
- 96-bit nonces (cryptographically random)
- AEAD for integrity and confidentiality

**Alternative:** ChaCha20-Poly1305
- Software-optimized for non-AES-NI platforms
- Same security properties as AES-GCM

### Key Lifecycle

```
Generation → Storage → Usage → Rotation → Destruction
    ↓            ↓        ↓         ↓          ↓
  Secure     Encrypted  Limited  Versioned  Zeroized
   Random     at Rest   Access    Keys      Memory
```

### Implementation

```rust
pub struct EncryptionKey {
    key: Vec<u8>,                    // Secure storage
    algorithm: EncryptionAlgorithm,
}

// Key validation
fn validate_key_strength(key: &EncryptionKey) -> Result<()> {
    if key.as_bytes().iter().all(|&b| b == 0) {
        return Err("Key cannot be all zeros");
    }
    Ok(())
}

// Memory cleanup
pub fn zeroize(data: &mut [u8]) {
    for byte in data.iter_mut() {
        *byte = 0;
    }
}
```

### Key Rotation (Planned v1.1)

- Automatic rotation every 30 days
- Version-tracked keys for backward compatibility
- Re-encryption of existing data
- Zero-downtime rollover

---

## Authentication & Authorization

### Multi-Method Support

```
┌──────────────┐
│   Request    │
└──────┬───────┘
       │
   ┌───▼────┐
   │ Auth   │
   │ Layer  │
   └┬──┬──┬─┘
    │  │  │
┌───▼──▼──▼───┐
│JWT│OAuth│API│
│   │  2  │Key│
└───┴─────┴───┘
```

### JWT Validation

```rust
pub async fn authenticate_jwt(
    state: &AuthState,
    auth_header: &str,
) -> Option<AuthResult> {
    let validation = Validation {
        validate_exp: true,      // Expiration
        validate_iss: true,      // Issuer
        validate_aud: true,      // Audience
        algorithms: vec![...],   // Allowed algos
    };

    decode::<Claims>(token, &key, &validation)
}
```

### OAuth2 Token Introspection

- Client credentials for introspection
- Token caching with TTL
- Revocation support
- Rate-limited introspection calls

### Admin UI Protection

- Basic authentication (v1.0)
- Configurable credentials
- Separate from API auth
- Rate-limited login attempts

---

## Network Security

### TLS/HTTPS

- **Minimum Version:** TLS 1.2
- **Cipher Suites:** Strong ciphers only (AEAD preferred)
- **Certificate Validation:** Full chain validation
- **HSTS:** Strict-Transport-Security headers

### Rate Limiting

```rust
pub struct RateLimitConfig {
    requests_per_minute: u32,    // 100 default
    burst: u32,                  // 200 default
    per_ip: bool,                // true
    per_endpoint: bool,          // false
}
```

### CORS Policy

- Configurable allowed origins
- Credentials support optional
- Preflight caching
- Method/header restrictions

---

## Input Validation

### OpenAPI Schema Validation

```yaml
request_validation: "enforce"  # off | warn | enforce
validate_responses: true
aggregate_validation_errors: true
```

### Path Traversal Prevention

```rust
pub fn validate_path(path: &str) -> Result<()> {
    if path.contains("..") || path.contains("~") {
        return Err(ValidationError::PathTraversal);
    }
    Ok(())
}
```

### XSS Protection

- Output encoding via `textContent` (not `innerHTML`)
- Content-Security-Policy headers
- X-XSS-Protection headers

---

## Audit & Compliance

### Audit Logging

```rust
pub struct AuthAuditLog {
    timestamp: DateTime<Utc>,
    ip_address: String,
    user_agent: Option<String>,
    auth_method: String,
    result: AuthResult,
}
```

### Compliance Mappings

| Standard | Requirement | Implementation |
|----------|-------------|----------------|
| SOC 2 | Access controls | Multi-method auth |
| SOC 2 | Audit logging | Comprehensive logs |
| SOC 2 | Encryption | AES-256-GCM |
| GDPR | Data minimization | Configurable retention |
| GDPR | Right to erasure | API endpoints |
| HIPAA | Access controls | RBAC, audit logs |
| HIPAA | Encryption | TLS + at-rest encryption |

---

## Incident Response

### Response Plan

1. **Detection:** Audit logs, alerts, monitoring
2. **Containment:** Rate limiting, IP blocking, service isolation
3. **Eradication:** Patch vulnerability, rotate credentials
4. **Recovery:** Restore services, validate integrity
5. **Post-Incident:** Root cause analysis, security improvements

### SLAs

- **Critical:** 24-48 hours to patch
- **High:** 1 week to patch
- **Medium:** 1 month to patch
- **Low:** Next release

### Communication

- security@mockforge.dev for reports
- Security advisories published to GitHub
- CVE assignments for public vulnerabilities

---

## Conclusion

MockForge implements a comprehensive, defense-in-depth security architecture suitable for enterprise deployments. Regular security audits, penetration testing, and continuous improvement ensure the platform remains secure against evolving threats.

---

**For Questions:** security@mockforge.dev
**For Vulnerability Reports:** See [SECURITY.md](../SECURITY.md)
**Last Updated:** 2025-10-09
