# MockForge Proxy Configuration

This document explains how to configure and use the proxy feature to access firewall-blocked APIs in MockForge.

## Overview

MockForge includes a built-in proxy that can forward requests to external APIs that are blocked by corporate firewalls. This allows developers to work with external services during development without requiring firewall changes.

## Configuration

The proxy is configured in your `config.dev.yaml` or `config.prod.yaml` file under the `proxy` section.

### Basic Proxy Settings

```yaml
proxy:
  enabled: true  # Enable the proxy feature
  upstream_url: "https://api.external-service.com"  # Default upstream URL
  timeout_seconds: 30  # Request timeout in seconds
  prefix: "/external-api"  # URL prefix for proxy requests
  passthrough_by_default: false  # Only proxy explicitly configured routes
```

### Header Forwarding

Configure which headers to forward from the original request:

```yaml
proxy:
  forward_headers:
    - "authorization"
    - "content-type"
    - "user-agent"
    - "x-request-id"
    - "x-api-key"
```

### Additional Headers

Add custom headers to all proxied requests:

```yaml
proxy:
  additional_headers:
    "X-Proxy-Source": "mockforge-dev"
    "X-Proxy-Version": "1.0"
```

### Per-Route Proxy Rules

Define specific routes for different external APIs:

```yaml
proxy:
  rules:
    - pattern: "/external-api/weather/*"  # Matches /external-api/weather/... paths
      upstream_url: "https://api.openweathermap.org"
      enabled: true
    - pattern: "/external-api/maps/*"
      upstream_url: "https://maps.googleapis.com"
      enabled: true
    - pattern: "/external-api/social/*"
      upstream_url: "https://graph.facebook.com"
      enabled: true
```

## Usage Examples

### Making Proxied Requests

Once configured, you can make requests through the proxy by using the configured prefix:

```bash
# Weather API request
curl "http://localhost:3000/external-api/weather/data/2.5/weather?q=London&appid=YOUR_API_KEY"

# Maps API request
curl "http://localhost:3000/external-api/maps/api/directions/json?origin=Paris&destination=London&key=YOUR_API_KEY"

# Social media API request
curl "http://localhost:3000/external-api/social/v18.0/me?fields=id,name&access_token=YOUR_TOKEN"
```

### Programmatic Access

```javascript
// JavaScript example
const response = await fetch('/external-api/weather/data/2.5/weather?q=London', {
  method: 'GET',
  headers: {
    'Authorization': 'Bearer YOUR_API_KEY'
  }
});
```

```python
# Python example
import requests

response = requests.get(
    'http://localhost:3000/external-api/weather/data/2.5/weather',
    params={'q': 'London', 'appid': 'YOUR_API_KEY'}
)
```

## Security Considerations

1. **API Keys**: Include API keys and tokens in headers or query parameters as needed by the external service
2. **Request Validation**: The proxy respects MockForge's request validation settings
3. **HTTPS**: Configure the upstream URLs appropriately (use HTTPS for production)
4. **Rate Limiting**: Consider implementing rate limiting for the proxy routes

## Pattern Matching

The proxy supports wildcard patterns for route matching:

- `/external-api/weather/*` matches `/external-api/weather/forecast`, `/external-api/weather/current`, etc.
- `/external-api/maps/directions/*` matches `/external-api/maps/directions/driving`, etc.

## Environment Variables

You can also configure the proxy using environment variables:

- `MOCKFORGE_PROXY_UPSTREAM_URL`: Default upstream URL
- `MOCKFORGE_PROXY_ENABLED`: Enable/disable proxy mode

## Monitoring and Debugging

Enable debug logging to see proxy request details:

```yaml
logging:
  level: "debug"
```

Look for log entries containing `proxy` to monitor proxy activity.

## Integration with Other Features

The proxy works seamlessly with other MockForge features:

- **Request/response validation**: Applies to proxied requests
- **Latency simulation**: Can be applied to proxy responses
- **Error simulation**: Can simulate failures on proxy routes
- **Overrides**: Can override proxy responses with custom fixtures

## Common Use Cases

1. **Weather APIs**: OpenWeatherMap, AccuWeather
2. **Maps and Location**: Google Maps, Mapbox
3. **Social Media**: Facebook Graph API, Twitter API
4. **Payment Services**: Stripe, PayPal
5. **Analytics**: Mixpanel, Google Analytics
6. **Third-party Integrations**: Any external API that's blocked by corporate firewall

## Troubleshooting

1. **Requests not proxying**: Check that the path starts with the configured prefix
2. **Authentication failures**: Ensure API keys are properly forwarded or included
3. **Connection timeouts**: Increase the `timeout_seconds` value if needed
4. **CORS issues**: Enable CORS in the HTTP server configuration if making browser requests

## Next Steps

After configuring the proxy, test it with a few requests to ensure it's working correctly. You can use tools like `curl` or Postman to test the proxy functionality before integrating it into your application.
