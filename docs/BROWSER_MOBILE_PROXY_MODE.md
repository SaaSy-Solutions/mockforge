# Browser/Mobile Proxy Mode

MockForge's Browser/Mobile Proxy Mode provides an intercepting proxy that allows you to test frontend applications and mobile apps by routing their API calls through MockForge for mocking, logging, and analysis.

## Overview

The proxy mode enables you to:

- **Intercept API calls** from browsers and mobile apps
- **Route requests** through MockForge for mocking and logging
- **Support HTTPS** with automatic certificate generation and injection
- **Log requests and responses** for debugging and analysis
- **Work with any client** that supports HTTP proxy configuration

## Quick Start

### Basic Proxy Mode

Start a simple HTTP proxy:

```bash
mockforge proxy --port 8081
```

This will start a proxy server on port 8081 that forwards requests to a default target.

### HTTPS Proxy Mode

Start a proxy with HTTPS support and certificate injection:

```bash
mockforge proxy --port 8081 --https --cert-dir ./certs
```

This will:
- Generate self-signed certificates for HTTPS interception
- Create certificates in the `./certs` directory
- Provide instructions for installing certificates

### With Logging

Enable request and response logging:

```bash
mockforge proxy --port 8081 --log-requests --log-responses
```

### With Admin UI

Enable the admin UI for proxy management:

```bash
mockforge proxy --port 8081 --admin --admin-port 9080
```

## Configuration

### Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `--port` | Proxy server port | 8081 |
| `--host` | Host to bind to | 127.0.0.1 |
| `--https` | Enable HTTPS support | false |
| `--cert-dir` | Certificate directory | ./proxy-certs |
| `--log-requests` | Enable request logging | false |
| `--log-responses` | Enable response logging | false |
| `--admin` | Enable admin UI | false |
| `--admin-port` | Admin UI port | 9080 |
| `--config` | Configuration file | none |

### Configuration File

Create a `proxy-config.yaml` file for advanced configuration:

```yaml
# Proxy configuration
proxy:
  enabled: true
  target_url: "http://127.0.0.1:3000"  # Default upstream URL
  timeout_seconds: 30
  follow_redirects: true
  prefix: "/proxy/"  # URL prefix for proxy requests
  passthrough_by_default: true

  # Additional headers to add to proxied requests
  headers:
    "X-Proxy-Source": "mockforge-dev"
    "X-Proxy-Version": "1.0"

  # Per-route proxy rules
  rules:
    - pattern: "/api/users/*"
      upstream_url: "http://127.0.0.1:3001"
      enabled: true
    - pattern: "/api/posts/*"
      upstream_url: "http://127.0.0.1:3002"
      enabled: true
```

Use the configuration file:

```bash
mockforge proxy --config proxy-config.yaml
```

## Client Configuration

### Browser Configuration

#### Chrome/Edge
1. Open Settings → Advanced → System
2. Click "Open proxy settings"
3. Enable "Use a proxy server"
4. Set HTTP proxy to `127.0.0.1:8081`

#### Firefox
1. Open Settings → General → Network Settings
2. Select "Manual proxy configuration"
3. Set HTTP proxy to `127.0.0.1:8081`

#### Safari
1. Open System Preferences → Network
2. Select your network connection
3. Click "Advanced" → "Proxies"
4. Enable "Web Proxy (HTTP)"
5. Set server to `127.0.0.1` and port to `8081`

### Mobile App Configuration

#### Android
1. Go to Settings → Wi-Fi
2. Long-press your connected network
3. Select "Modify Network"
4. Tap "Advanced Options"
5. Set Proxy to "Manual"
6. Set Proxy hostname to `127.0.0.1`
7. Set Proxy port to `8081`

#### iOS
1. Go to Settings → Wi-Fi
2. Tap the "i" next to your connected network
3. Scroll down to "Configure Proxy"
4. Select "Manual"
5. Set Server to `127.0.0.1`
6. Set Port to `8081`

### Programmatic Configuration

#### JavaScript/Node.js
```javascript
// Configure proxy for HTTP requests
const proxyAgent = new HttpsProxyAgent('http://127.0.0.1:8081');

fetch('https://api.example.com/users', {
  agent: proxyAgent
});
```

#### Python
```python
import requests

proxies = {
    'http': 'http://127.0.0.1:8081',
    'https': 'http://127.0.0.1:8081'
}

response = requests.get('https://api.example.com/users', proxies=proxies)
```

#### Go
```go
package main

import (
    "net/http"
    "net/url"
)

func main() {
    proxyURL, _ := url.Parse("http://127.0.0.1:8081")
    client := &http.Client{
        Transport: &http.Transport{
            Proxy: http.ProxyURL(proxyURL),
        },
    }

    resp, _ := client.Get("https://api.example.com/users")
    defer resp.Body.Close()
}
```

## HTTPS Certificate Installation

When using HTTPS mode, MockForge generates self-signed certificates that need to be installed as trusted root certificates.

### Certificate Generation

Certificates are automatically generated when you start the proxy with `--https`:

```bash
mockforge proxy --port 8081 --https --cert-dir ./certs
```

This creates:
- `proxy.crt` - Certificate file
- `proxy.key` - Private key file

### Installation Instructions

#### macOS
1. Open Keychain Access
2. Go to File → Import Items
3. Select the `proxy.crt` file
4. Find the certificate in "login" keychain
5. Double-click the certificate
6. Expand "Trust" section
7. Set "When using this certificate" to "Always Trust"

#### Windows
1. Double-click the `proxy.crt` file
2. Click "Install Certificate"
3. Select "Local Machine"
4. Place certificate in "Trusted Root Certification Authorities"
5. Complete the installation

#### Linux
```bash
# Copy certificate to system trust store
sudo cp proxy.crt /usr/local/share/ca-certificates/mockforge-proxy.crt
sudo update-ca-certificates
```

#### Android
1. Transfer `proxy.crt` to your device
2. Go to Settings → Security → Install certificates
3. Select "CA certificate"
4. Choose the `proxy.crt` file
5. Give it a name (e.g., "MockForge Proxy")

#### iOS
1. Transfer `proxy.crt` to your device (via AirDrop, email, etc.)
2. Open the certificate file
3. Go to Settings → General → VPN & Device Management
4. Find the certificate under "Downloaded Profile"
5. Tap "Install" and follow the prompts

## Usage Examples

### Testing a React App

1. Start the proxy:
```bash
mockforge proxy --port 8081 --log-requests --log-responses
```

2. Configure your React app to use the proxy:
```javascript
// In your React app's package.json
{
  "proxy": "http://127.0.0.1:8081"
}
```

3. Start your React app:
```bash
npm start
```

4. All API calls will be intercepted and logged by MockForge.

### Testing a Mobile App

1. Start the proxy with HTTPS:
```bash
mockforge proxy --port 8081 --https --log-requests
```

2. Install the generated certificate on your mobile device

3. Configure your mobile app to use the proxy:
```kotlin
// Android example
val proxy = Proxy(Proxy.Type.HTTP, InetSocketAddress("127.0.0.1", 8081))
val client = OkHttpClient.Builder()
    .proxy(proxy)
    .build()
```

4. Run your mobile app - all API calls will be intercepted.

### API Mocking with Proxy

1. Start MockForge with your API spec:
```bash
mockforge serve --spec api.yaml --http-port 3000
```

2. Start the proxy pointing to MockForge:
```bash
mockforge proxy --port 8081 --config proxy-config.yaml
```

3. Configure your frontend to use the proxy

4. All API calls will be routed through MockForge for mocking.

## Advanced Features

### Request/Response Logging

Enable detailed logging:

```bash
mockforge proxy --port 8081 --log-requests --log-responses
```

This will log:
- Request method, URL, headers, and body
- Response status, headers, and body
- Timing information
- Client IP addresses

### Admin UI and Proxy Inspector

Access the proxy management interface:

```bash
mockforge proxy --port 8081 --admin --admin-port 9080
```

Visit `http://127.0.0.1:9080` to:
- View proxy statistics
- Monitor request/response logs
- Configure proxy rules
- Manage certificates
- **Use the Proxy Inspector** for body transformation management

#### Proxy Inspector Features

The Proxy Inspector UI provides:

1. **Replacement Rules Management**
   - Create, edit, and delete body transformation rules
   - Filter rules by type (request/response) and pattern
   - Enable/disable rules without deleting
   - Visual status indicators

2. **Intercepted Traffic Viewing**
   - Real-time view of intercepted requests and responses
   - Request/response body inspection
   - Auto-refresh every 2 seconds
   - Search and filter capabilities

3. **JSONPath Transformation Editor**
   - Visual editor for JSONPath expressions
   - Template expansion support (UUIDs, faker data, etc.)
   - Operation selection (Replace, Add, Remove)
   - Status code filtering for response rules

Access the Proxy Inspector:
- Navigate to the "Proxy Inspector" tab in the admin UI
- Or access directly via the `proxy-inspector` route

### Body Transformation

MockForge supports JSONPath-based body transformation for both requests and responses. This allows you to:

- **Modify request bodies** before they reach the upstream server
- **Modify response bodies** before they reach the client
- **Use JSONPath expressions** to target specific fields
- **Apply template expansion** for dynamic values

See [Proxy Body Transformation Guide](./PROXY_BODY_TRANSFORMATION.md) for detailed documentation.

Quick example:

```bash
# Create a transformation rule via API
curl -X POST http://127.0.0.1:9080/__mockforge/api/proxy/rules \
  -H "Content-Type: application/json" \
  -d '{
    "pattern": "/api/users/*",
    "type": "request",
    "body_transforms": [
      {
        "path": "$.userId",
        "replace": "{{uuid}}",
        "operation": "replace"
      }
    ],
    "enabled": true
  }'
```

This rule replaces the `userId` field in request bodies with a generated UUID.

### Custom Proxy Rules

Create complex routing rules:

```yaml
proxy:
  rules:
    - pattern: "/api/v1/*"
      upstream_url: "http://127.0.0.1:3001"
      enabled: true
    - pattern: "/api/v2/*"
      upstream_url: "http://127.0.0.1:3002"
      enabled: true
    - pattern: "/static/*"
      upstream_url: "http://127.0.0.1:8080"
      enabled: true
```

### Header Manipulation

Add custom headers to proxied requests:

```yaml
proxy:
  headers:
    "X-Proxy-Source": "mockforge-dev"
    "X-Proxy-Version": "1.0"
    "X-Request-ID": "{{uuid}}"
```

## Troubleshooting

### Common Issues

#### Certificate Errors
- **Problem**: "Certificate not trusted" errors
- **Solution**: Install the generated certificate as a trusted root CA

#### Connection Refused
- **Problem**: "Connection refused" errors
- **Solution**: Ensure the proxy is running and the port is correct

#### Proxy Not Intercepting
- **Problem**: Requests bypass the proxy
- **Solution**: Verify client proxy configuration and check proxy rules

### Debug Mode

Enable debug logging:

```bash
RUST_LOG=debug mockforge proxy --port 8081
```

This will show detailed information about:
- Proxy configuration
- Request routing decisions
- Certificate generation
- Error conditions

### Health Check

Check if the proxy is running:

```bash
curl http://127.0.0.1:8081/proxy/health
```

Expected response:
```json
{
  "status": "healthy",
  "service": "mockforge-proxy"
}
```

## Security Considerations

### Certificate Security
- Generated certificates are for development/testing only
- Never use in production environments
- Certificates are valid for 10 years by default

### Network Security
- Proxy runs on localhost by default
- Use firewall rules to restrict access if needed
- Consider using VPN for remote testing

### Data Privacy
- Request/response logging may contain sensitive data
- Use appropriate log retention policies
- Consider data encryption for stored logs

## Best Practices

1. **Use HTTPS in production-like environments** to test real-world scenarios
2. **Enable logging** for debugging but disable in production
3. **Use configuration files** for complex proxy setups
4. **Test with real clients** to ensure compatibility
5. **Monitor proxy performance** using the admin UI
6. **Clean up certificates** when done testing
7. **Use body transformations** for testing edge cases and data masking
8. **Leverage the Proxy Inspector UI** for visual rule management and traffic inspection
9. **Document transformation rules** to maintain clarity on what each rule does
10. **Test transformations** before enabling them in production-like scenarios

## Integration with CI/CD

### GitHub Actions
```yaml
- name: Start MockForge Proxy
  run: |
    mockforge proxy --port 8081 --log-requests &
    sleep 5

- name: Run Tests with Proxy
  run: |
    export HTTP_PROXY=http://127.0.0.1:8081
    export HTTPS_PROXY=http://127.0.0.1:8081
    npm test
```

### Docker
```dockerfile
FROM node:18
COPY . /app
WORKDIR /app
RUN npm install
EXPOSE 3000 8081
CMD ["sh", "-c", "mockforge proxy --port 8081 & npm start"]
```

This completes the Browser/Mobile Proxy Mode implementation with comprehensive testing and documentation.
