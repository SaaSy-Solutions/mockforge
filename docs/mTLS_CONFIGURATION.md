# Mutual TLS (mTLS) Configuration Guide

MockForge supports **Mutual TLS (mTLS)** for enhanced security, requiring both server and client certificates for authentication. This provides an additional layer of security beyond standard TLS.

## Overview

Mutual TLS (mTLS) is a security protocol where both the server and client authenticate each other using certificates. This is particularly useful for:

- **API-to-API Communication**: Secure service-to-service authentication
- **Enterprise Environments**: Additional security layer for internal APIs
- **Zero Trust Architectures**: Certificate-based authentication for all connections
- **Compliance Requirements**: Meeting security standards that require client authentication

## How mTLS Works

In standard TLS:
1. Client verifies server certificate ✅
2. Server accepts any client ❌

In mTLS:
1. Client verifies server certificate ✅
2. Server verifies client certificate ✅
3. Both parties are authenticated ✅

## Configuration

### Basic mTLS Setup

Enable mTLS in your MockForge configuration:

```yaml
http:
  tls:
    enabled: true
    cert_file: "./certs/server.crt"      # Server certificate
    key_file: "./certs/server.key"       # Server private key
    ca_file: "./certs/ca.crt"           # CA certificate for client verification
    require_client_cert: true            # Enable mTLS (require client certificates)
    min_version: "1.2"                   # Minimum TLS version
```

### Certificate Requirements

For mTLS to work, you need:

1. **Server Certificate** (`cert_file`): The server's TLS certificate
2. **Server Private Key** (`key_file`): The server's private key
3. **CA Certificate** (`ca_file`): Certificate Authority certificate that signed the client certificates

### Environment Variables

You can also configure mTLS via environment variables:

```bash
export MOCKFORGE_HTTP_TLS_ENABLED=true
export MOCKFORGE_HTTP_TLS_CERT_FILE=./certs/server.crt
export MOCKFORGE_HTTP_TLS_KEY_FILE=./certs/server.key
export MOCKFORGE_HTTP_TLS_CA_FILE=./certs/ca.crt
export MOCKFORGE_HTTP_TLS_REQUIRE_CLIENT_CERT=true
```

## Certificate Generation

### Using OpenSSL

#### 1. Create CA Certificate

```bash
# Generate CA private key
openssl genrsa -out ca.key 4096

# Generate CA certificate
openssl req -new -x509 -days 365 -key ca.key -out ca.crt \
  -subj "/CN=MockForge CA"
```

#### 2. Create Server Certificate

```bash
# Generate server private key
openssl genrsa -out server.key 4096

# Generate server certificate signing request
openssl req -new -key server.key -out server.csr \
  -subj "/CN=localhost"

# Sign server certificate with CA
openssl x509 -req -days 365 -in server.csr -CA ca.crt -CAkey ca.key \
  -CAcreateserial -out server.crt
```

#### 3. Create Client Certificate

```bash
# Generate client private key
openssl genrsa -out client.key 4096

# Generate client certificate signing request
openssl req -new -key client.key -out client.csr \
  -subj "/CN=client"

# Sign client certificate with CA
openssl x509 -req -days 365 -in client.csr -CA ca.crt -CAkey ca.key \
  -CAcreateserial -out client.crt
```

### Using mkcert (Recommended for Development)

For local development, `mkcert` simplifies certificate generation:

```bash
# Install mkcert
brew install mkcert  # macOS
# or
choco install mkcert  # Windows

# Install local CA
mkcert -install

# Generate server certificate
mkcert localhost 127.0.0.1 ::1

# Generate client certificate
mkcert -client localhost 127.0.0.1 ::1
```

## Client Configuration

### Using cURL

Test mTLS with cURL:

```bash
curl --cert client.crt --key client.key --cacert ca.crt \
  https://localhost:3000/api/endpoint
```

### Using HTTP Clients

#### Python (requests)

```python
import requests

cert = ('client.crt', 'client.key')
verify = 'ca.crt'

response = requests.get(
    'https://localhost:3000/api/endpoint',
    cert=cert,
    verify=verify
)
```

#### Node.js (axios)

```javascript
const axios = require('axios');
const fs = require('fs');
const https = require('https');

const agent = new https.Agent({
  cert: fs.readFileSync('client.crt'),
  key: fs.readFileSync('client.key'),
  ca: fs.readFileSync('ca.crt')
});

const response = await axios.get('https://localhost:3000/api/endpoint', {
  httpsAgent: agent
});
```

#### Go

```go
package main

import (
    "crypto/tls"
    "crypto/x509"
    "io/ioutil"
    "net/http"
)

func main() {
    // Load client certificate
    cert, err := tls.LoadX509KeyPair("client.crt", "client.key")
    if err != nil {
        panic(err)
    }

    // Load CA certificate
    caCert, err := ioutil.ReadFile("ca.crt")
    if err != nil {
        panic(err)
    }
    caCertPool := x509.NewCertPool()
    caCertPool.AppendCertsFromPEM(caCert)

    // Configure TLS
    tlsConfig := &tls.Config{
        Certificates: []tls.Certificate{cert},
        RootCAs:      caCertPool,
    }

    // Create HTTP client
    client := &http.Client{
        Transport: &http.Transport{
            TLSClientConfig: tlsConfig,
        },
    }

    // Make request
    resp, err := client.Get("https://localhost:3000/api/endpoint")
    // ... handle response
}
```

## Verification

### Test Server Configuration

Start MockForge with mTLS:

```bash
mockforge serve --config config.yaml
```

The server will log:
```
INFO Loading TLS certificate from ./certs/server.crt and key from ./certs/server.key
INFO TLS acceptor configured successfully
```

### Test Client Connection

Attempt to connect without a client certificate (should fail):

```bash
curl https://localhost:3000/api/endpoint
# Error: SSL peer certificate or SSH remote key was not OK
```

Connect with client certificate (should succeed):

```bash
curl --cert client.crt --key client.key --cacert ca.crt \
  https://localhost:3000/api/endpoint
# Success!
```

## Troubleshooting

### Common Issues

#### 1. "Client certificate required but no CA file provided"

**Error:**
```
Client certificate required (require_client_cert=true) but no CA file provided
```

**Solution:**
Ensure `ca_file` is specified when `require_client_cert` is `true`:

```yaml
http:
  tls:
    require_client_cert: true
    ca_file: "./certs/ca.crt"  # Must be provided
```

#### 2. "Certificate verification failed"

**Error:**
```
certificate verify failed: self signed certificate
```

**Solution:**
- Ensure the CA certificate is trusted by the client
- Use `--cacert` with cURL to specify the CA certificate
- For production, use certificates from a trusted CA

#### 3. "No certificates found"

**Error:**
```
No certificates found in ./certs/server.crt
```

**Solution:**
- Verify certificate file exists and is readable
- Check certificate format (should be PEM)
- Ensure certificate is not empty

### Certificate Format

MockForge expects certificates in **PEM format**:

```
-----BEGIN CERTIFICATE-----
MIIFjTCCA3WgAwIBAgIQK...
...
-----END CERTIFICATE-----
```

To convert from other formats:

```bash
# Convert DER to PEM
openssl x509 -inform DER -in cert.der -out cert.pem

# Convert PKCS#12 to PEM
openssl pkcs12 -in cert.p12 -out cert.pem -nodes
```

## Security Best Practices

### Production Recommendations

1. **Use Trusted CAs**: Use certificates from trusted Certificate Authorities (not self-signed)
2. **Certificate Rotation**: Implement regular certificate rotation (30-90 days)
3. **Key Management**: Store private keys securely (key management service, HSM)
4. **TLS Version**: Use TLS 1.3 when possible (minimum TLS 1.2)
5. **Certificate Pinning**: Consider certificate pinning for additional security
6. **Revocation**: Implement certificate revocation checking (OCSP/CRL)

### Development Recommendations

1. **Separate CAs**: Use different CAs for development and production
2. **Short Expiration**: Use short-lived certificates for development
3. **Automated Generation**: Automate certificate generation in CI/CD
4. **Documentation**: Document certificate requirements clearly

## Advanced Configuration

### Multiple CA Certificates

You can use multiple CA certificates by combining them in a single file:

```bash
# Combine multiple CA certificates
cat ca1.crt ca2.crt ca3.crt > combined-ca.crt
```

Then reference the combined file:

```yaml
http:
  tls:
    ca_file: "./certs/combined-ca.crt"
```

### Certificate Validation

MockForge uses `rustls` with safe defaults:
- Validates certificate chain
- Checks expiration dates
- Verifies certificate signatures
- Supports TLS 1.2 and 1.3

### Custom Cipher Suites

While MockForge accepts cipher suite configuration, `rustls` uses safe defaults. Custom cipher suites may not be applied directly:

```yaml
http:
  tls:
    cipher_suites:
      - "TLS13_AES_256_GCM_SHA384"
      - "TLS13_CHACHA20_POLY1305_SHA256"
```

Note: `rustls` uses safe defaults that include both TLS 1.2 and 1.3 cipher suites.

## Integration with Other Features

### Proxy Mode with mTLS

When using proxy mode with mTLS, ensure upstream servers support mTLS:

```yaml
proxy:
  upstream_url: "https://api.example.com"
  tls:
    enabled: true
    cert_file: "./certs/client.crt"
    key_file: "./certs/client.key"
    ca_file: "./certs/ca.crt"
```

### Admin UI with mTLS

The Admin UI will automatically use mTLS when configured:

```yaml
http:
  tls:
    enabled: true
    require_client_cert: true
    ca_file: "./certs/ca.crt"
admin:
  enabled: true
  port: 9080
```

Access the Admin UI with client certificate:

```bash
curl --cert client.crt --key client.key --cacert ca.crt \
  https://localhost:3000
```

## Examples

### Complete mTLS Configuration

```yaml
# config.yaml
http:
  port: 3000
  host: "0.0.0.0"

  tls:
    enabled: true
    cert_file: "./certs/server.crt"
    key_file: "./certs/server.key"
    ca_file: "./certs/ca.crt"
    require_client_cert: true
    min_version: "1.3"

admin:
  enabled: true
  port: 9080
```

### Docker Deployment with mTLS

```yaml
# docker-compose.yml
version: '3.8'
services:
  mockforge:
    image: mockforge:latest
    ports:
      - "3000:3000"
      - "9080:9080"
    volumes:
      - ./certs:/certs
      - ./config.yaml:/config.yaml
    environment:
      - MOCKFORGE_HTTP_TLS_ENABLED=true
      - MOCKFORGE_HTTP_TLS_CERT_FILE=/certs/server.crt
      - MOCKFORGE_HTTP_TLS_KEY_FILE=/certs/server.key
      - MOCKFORGE_HTTP_TLS_CA_FILE=/certs/ca.crt
      - MOCKFORGE_HTTP_TLS_REQUIRE_CLIENT_CERT=true
```

## Related Documentation

- [Security Guide](../book/src/user-guide/security.md) - General security features
- [Configuration Reference](../config.template.yaml) - Complete configuration options
- [TLS Implementation](../TLS_IMPLEMENTATION_TEST_SUMMARY.md) - Implementation details

## Support

For issues or questions:
- [GitHub Issues](https://github.com/SaaSy-Solutions/mockforge/issues)
- [GitHub Discussions](https://github.com/SaaSy-Solutions/mockforge/discussions)
- [Discord](https://discord.gg/2FxXqKpa)

---

**Last Updated:** 2025-01-27
