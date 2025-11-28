# Chaos Lab - Interactive Network Condition Simulation

**Pillars:** [Reality]

[Reality] - Makes mocks feel like real backends through network condition simulation and chaos engineering

Chaos Lab is an interactive module in MockForge that enables developers to simulate various real-world network conditions and errors directly from the UI. This feature helps test application resilience, debug network-related issues, and validate error handling logic.

## Overview

Chaos Lab provides:

- **Real-time latency visualization** - Visual graph showing request latency over time
- **Network profile management** - Predefined and custom profiles for common network conditions
- **Error pattern scripting** - Configure burst, random, or sequential error injection
- **Profile export/import** - Share and version control chaos configurations
- **CLI integration** - Apply profiles and manage configurations from the command line

## Quick Start

### Using the UI

1. Navigate to the **Chaos Engineering** page in the MockForge Admin UI
2. Use the **Network Profiles** section to apply predefined conditions (slow 3G, flaky Wi-Fi, etc.)
3. Monitor real-time latency in the **Latency Metrics** graph
4. Configure error patterns in the **Error Pattern Editor**

### Using the CLI

```bash
# Apply a network profile
mockforge serve --chaos-profile slow_3g

# List available profiles
mockforge chaos profile list

# Export a profile
mockforge chaos profile export slow_3g --format json --output profile.json

# Import a profile
mockforge chaos profile import --file profile.json
```

## Features

### Real-Time Latency Graph

The latency graph displays request latency over time with:

- **Time-series visualization** - See latency trends in real-time
- **Statistics overlay** - Min, max, average, P95, P99 percentiles
- **Auto-refresh** - Updates every 500ms for live monitoring
- **Configurable history** - View last 100 samples by default

**Usage:**
- Enable latency injection in the Quick Controls section
- The graph automatically populates as requests are made
- Hover over data points to see exact latency values

### Network Profiles

Network profiles are pre-configured chaos settings that simulate specific network conditions:

#### Built-in Profiles

- **slow_3g** - Simulates slow 3G connection (high latency, low bandwidth)
- **flaky_wifi** - Intermittent connection issues with packet loss
- **high_latency** - Consistent high latency for all requests
- **unstable_connection** - Random connection drops and timeouts

#### Custom Profiles

Create your own profiles:

1. Configure chaos settings in the Quick Controls
2. Use the Profile Exporter to save your configuration
3. Import it later or share with your team

**Applying Profiles:**

```bash
# Via UI
Click "Apply Profile" on any profile card

# Via CLI
mockforge chaos profile apply slow_3g
```

### Error Pattern Editor

Configure sophisticated error injection patterns:

#### Burst Pattern

Inject multiple errors within a time window:

```json
{
  "type": "burst",
  "count": 5,
  "interval_ms": 1000
}
```

This injects 5 errors within 1 second, then waits for the next interval.

#### Random Pattern

Inject errors with a probability:

```json
{
  "type": "random",
  "probability": 0.1
}
```

Each request has a 10% chance of receiving an error.

#### Sequential Pattern

Inject errors in a specific order:

```json
{
  "type": "sequential",
  "sequence": [500, 502, 503, 504]
}
```

Errors are injected in the specified order, then the sequence repeats.

**Usage:**
1. Enable Fault Injection in Quick Controls
2. Open the Error Pattern Editor
3. Select pattern type and configure parameters
4. Click "Save Pattern"

### Profile Export/Import

Export and import chaos configurations for:

- **Version control** - Track chaos configurations in git
- **Team sharing** - Share tested configurations
- **CI/CD integration** - Apply profiles in automated tests
- **Backup** - Save working configurations

**Export Format:**

```json
{
  "name": "custom_profile",
  "description": "Custom network condition",
  "chaos_config": {
    "latency": {
      "enabled": true,
      "fixed_delay_ms": 500,
      "probability": 1.0
    },
    "fault_injection": {
      "enabled": true,
      "http_errors": [500, 502, 503],
      "http_error_probability": 0.1
    }
  },
  "tags": ["custom", "testing"],
  "builtin": false
}
```

**Import:**
- Via UI: Use the Profile Exporter component
- Via CLI: `mockforge chaos profile import --file profile.json`

## API Endpoints

### Latency Metrics

```http
GET /api/chaos/metrics/latency
```

Returns time-series latency data:

```json
{
  "samples": [
    {
      "timestamp": "2024-01-01T12:00:00Z",
      "latency_ms": 150
    }
  ]
}
```

```http
GET /api/chaos/metrics/latency/stats
```

Returns aggregated statistics:

```json
{
  "avg_latency_ms": 145.5,
  "min_latency_ms": 100,
  "max_latency_ms": 200,
  "total_requests": 100,
  "p50_ms": 140,
  "p95_ms": 180,
  "p99_ms": 195
}
```

### Profile Management

```http
GET /api/chaos/profiles
```

List all available profiles.

```http
GET /api/chaos/profiles/{name}
```

Get a specific profile.

```http
POST /api/chaos/profiles/{name}/apply
```

Apply a profile to the current configuration.

```http
POST /api/chaos/profiles
```

Create a custom profile.

```http
DELETE /api/chaos/profiles/{name}
```

Delete a custom profile.

```http
GET /api/chaos/profiles/{name}/export?format=json
```

Export a profile.

```http
POST /api/chaos/profiles/import
```

Import a profile.

### Error Pattern Configuration

Update error patterns via the fault injection config endpoint:

```http
PUT /api/chaos/config/faults
```

```json
{
  "enabled": true,
  "http_errors": [500, 502, 503],
  "error_pattern": {
    "type": "burst",
    "count": 5,
    "interval_ms": 1000
  }
}
```

## CLI Commands

### Profile Management

```bash
# List all profiles
mockforge chaos profile list

# Apply a profile
mockforge chaos profile apply slow_3g

# Export a profile
mockforge chaos profile export slow_3g --format json --output profile.json

# Import a profile
mockforge chaos profile import --file profile.json
```

### Server Startup

```bash
# Start server with a profile applied
mockforge serve --chaos-profile slow_3g --spec openapi.json
```

## Use Cases

### Testing Resilience

1. Apply a "flaky_wifi" profile
2. Monitor your application's retry logic
3. Verify error handling and recovery

### Debugging Network Issues

1. Reproduce reported network conditions
2. Use the latency graph to identify patterns
3. Test fixes under controlled conditions

### Load Testing Preparation

1. Create profiles matching production network conditions
2. Export profiles for CI/CD pipelines
3. Apply profiles during automated tests

### Team Collaboration

1. Export tested chaos configurations
2. Share profiles via version control
3. Standardize testing across environments

## Best Practices

### Profile Naming

- Use descriptive names: `production_like_network`, `mobile_edge_conditions`
- Include tags for categorization: `["mobile", "edge", "testing"]`
- Document profile purpose in the description field

### Error Pattern Design

- Start with low probabilities (0.05-0.1) and increase gradually
- Use burst patterns to test rate limiting and circuit breakers
- Use sequential patterns to test specific error code handling

### Monitoring

- Always monitor the latency graph when chaos is active
- Set up alerts for unexpected latency spikes
- Review statistics regularly to understand impact

### Version Control

- Export profiles before making changes
- Commit profiles to version control
- Tag profiles with application versions

## Troubleshooting

### Latency Graph Not Updating

- Ensure latency injection is enabled
- Check that requests are being made to the server
- Verify the API endpoint is accessible: `GET /api/chaos/metrics/latency`

### Profile Not Applying

- Verify profile name is correct: `mockforge chaos profile list`
- Check server logs for errors
- Ensure chaos engineering is enabled in configuration

### Error Pattern Not Working

- Verify fault injection is enabled
- Check error pattern configuration is valid JSON
- Ensure HTTP error codes are configured: `http_errors: [500, 502, 503]`

## Configuration

Chaos Lab settings can be configured in `mockforge.yaml`:

```yaml
observability:
  chaos:
    enabled: true
    latency:
      enabled: true
      fixed_delay_ms: 200
      probability: 0.5
    fault_injection:
      enabled: true
      http_errors: [500, 502, 503]
      http_error_probability: 0.1
      error_pattern:
        type: random
        probability: 0.1
```

## Integration with Test Automation

### CI/CD Integration

```yaml
# Example GitHub Actions workflow
- name: Test with chaos profile
  run: |
    mockforge serve --chaos-profile slow_3g &
    sleep 5
    pytest tests/
    mockforge chaos profile apply none
```

### Test Scripts

```bash
#!/bin/bash
# Apply profile and run tests
mockforge chaos profile apply flaky_wifi --base-url http://localhost:3000
npm test
mockforge chaos profile apply none --base-url http://localhost:3000
```

## Performance Considerations

- Latency metrics are stored in memory (last 100 samples)
- Profile application is instant (no server restart required)
- Error pattern evaluation adds minimal overhead (< 1ms per request)
- Real-time graph updates every 500ms (configurable)

## Limitations

- Latency samples are limited to the last 100 requests
- Custom profiles are stored in memory (not persisted across restarts)
- Error patterns apply globally (not per-endpoint)
- MockAI integration requires MockAI to be enabled

## Future Enhancements

- Per-endpoint error patterns
- Persistent profile storage
- Profile templates and marketplace
- Advanced visualization (heatmaps, distribution charts)
- Integration with external monitoring tools

## Related Documentation

- [Chaos Engineering Guide](./CHAOS_ENGINEERING.md) - General chaos engineering concepts
- [Network Profiles](./NETWORK_PROFILES_AND_CHAOS.md) - Network profile details
- [Resilience Patterns](./RESILIENCE_PATTERNS.md) - Resilience testing patterns
- [API Documentation](./API_DOCUMENTATION_STATUS.md) - Complete API reference
