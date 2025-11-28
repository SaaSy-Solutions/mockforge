# Webhook Example Plugin

A MockForge response plugin that demonstrates webhook functionality by making outbound HTTP calls to external endpoints.

## Overview

This plugin shows how to:
- Make outbound HTTP requests from a plugin
- Configure network capabilities for plugins
- Generate and sign webhook payloads
- Handle webhook events and responses
- Implement error handling and retries

## Features

- **Outbound HTTP Requests**: Makes HTTP POST requests to configured webhook URLs
- **Payload Signing**: Optional HMAC-SHA256 signature for webhook security
- **Event-Based**: Triggers webhooks based on configured events
- **Configurable**: Customize webhook URL, timeout, retries, and events
- **Error Handling**: Graceful handling of network errors and timeouts

## Installation

```bash
# Build the plugin
cd examples/plugins/webhook-example
cargo build --target wasm32-wasi --release

# Install the plugin
mockforge plugin install .
```

## Configuration

```yaml
# In your mockforge.yaml
plugins:
  - id: webhook-example
    config:
      webhook_url: "https://your-webhook-endpoint.com/webhook"
      secret: "your-webhook-secret"  # Optional
      timeout_ms: 5000
      enable_retries: true
      max_retries: 3
      events:
        - "payment.completed"
        - "order.created"
```

## Usage

The plugin automatically triggers webhooks when processing requests that match configured events. The webhook payload includes:

```json
{
  "event": "mockforge.request",
  "timestamp": "2025-01-20T12:00:00Z",
  "request": {
    "method": "POST",
    "path": "/api/orders",
    "query": {},
    "headers": {}
  },
  "source": "mockforge-webhook-plugin"
}
```

## Network Capabilities

This plugin requires network access to make outbound HTTP calls. The plugin manifest specifies:

```yaml
capabilities:
  network:
    allow_http_outbound: true
    allowed_hosts:
      - "*"  # Configure specific hosts in production
```

**Security Note**: In production, restrict `allowed_hosts` to specific domains rather than using `"*"`.

## Example Response

When a webhook is triggered, the plugin returns a response indicating the webhook status:

```json
{
  "message": "Webhook triggered successfully",
  "webhook": {
    "url": "https://example.com/webhook",
    "event": "mockforge.request",
    "status": "sent"
  },
  "response": {
    "status": "sent",
    "webhook_url": "https://example.com/webhook",
    "payload": { ... },
    "timestamp": "2025-01-20T12:00:00Z"
  },
  "timestamp": "2025-01-20T12:00:00Z"
}
```

## Development

### Building

```bash
cargo build --target wasm32-wasi --release
```

### Testing

```bash
cargo test
```

### Code Structure

- `src/lib.rs`: Main plugin implementation
  - `WebhookConfig`: Plugin configuration structure
  - `WebhookExamplePlugin`: Plugin implementation
  - `generate_webhook_payload()`: Creates webhook payload from request
  - `sign_payload()`: Signs payload with HMAC-SHA256 (if secret provided)

## Security Considerations

1. **Network Access**: This plugin requires network capabilities. Only enable for trusted plugins.
2. **Host Restrictions**: Configure `allowed_hosts` to restrict which domains can be called.
3. **Secret Management**: Store webhook secrets securely, not in version control.
4. **Timeouts**: Set appropriate timeouts to prevent resource exhaustion.
5. **Rate Limiting**: Consider implementing rate limiting for webhook calls.

## Limitations

- This is an example plugin. In production, implement:
  - Actual HTTP client (using wasm-compatible HTTP library)
  - Proper HMAC-SHA256 signing
  - Retry logic with exponential backoff
  - Request queuing for high-volume scenarios
  - Circuit breaker pattern for failing endpoints

## See Also

- [Plugin Development Guide](../../../docs/plugins/development-guide.md)
- [Response Plugin API](../../../docs/plugins/README.md)
- [Network Capabilities](../../../docs/plugins/README.md#security-considerations)

## License

MIT OR Apache-2.0
