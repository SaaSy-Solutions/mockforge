# Chaos Engineering

MockForge's chaos engineering capabilities enable you to test system resilience by injecting controlled failures, delays, and resource constraints into your API testing environment.

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [CLI Usage](#cli-usage)
- [Chaos Scenarios](#chaos-scenarios)
- [Management API](#management-api)
- [Latency Injection](#latency-injection)
- [Fault Injection](#fault-injection)
- [Rate Limiting](#rate-limiting)
- [Traffic Shaping](#traffic-shaping)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

Chaos engineering is the discipline of experimenting on a system to build confidence in its capability to withstand turbulent conditions. MockForge provides comprehensive chaos engineering tools to test how your applications handle:

- Network latency and delays
- Service failures and errors
- Rate limiting and throttling
- Bandwidth constraints
- Packet loss
- Cascading failures

### Key Capabilities

- **Latency Injection**: Fixed delays, random delays, jitter simulation
- **Fault Injection**: HTTP errors, connection errors, timeouts, partial responses
- **Rate Limiting**: Global, per-IP, and per-endpoint rate limits
- **Traffic Shaping**: Bandwidth throttling, packet loss, connection limits
- **Predefined Scenarios**: Ready-to-use chaos patterns
- **Dynamic Control**: Real-time configuration via REST API
- **Zero Impact When Disabled**: No performance overhead when chaos is off

## Features

### Latency Injection

Simulate network delays and slow backends:
- Fixed delay (e.g., always add 500ms)
- Random delay range (e.g., 100-500ms)
- Jitter percentage (e.g., ±10% variance)
- Probability-based application (e.g., 50% of requests)

### Fault Injection

Introduce failures and errors:
- HTTP error codes (500, 502, 503, 504, etc.)
- Connection errors (simulate network failures)
- Timeout errors (simulate hung requests)
- Partial responses (incomplete data)
- Configurable probability for each fault type

### Rate Limiting

Control request throughput:
- Global rate limits
- Per-IP rate limits
- Per-endpoint rate limits
- Burst allowance
- Configurable requests per second

### Traffic Shaping

Simulate poor network conditions:
- Bandwidth throttling (bytes per second)
- Packet loss percentage
- Maximum concurrent connections
- Connection timeouts

## Quick Start

### Basic Usage

```bash
# Enable chaos with default settings
mockforge serve --chaos

# Use a predefined scenario
mockforge serve --chaos --chaos-scenario network_degradation

# Custom latency injection
mockforge serve --chaos --chaos-latency-ms 500

# Custom fault injection
mockforge serve --chaos \
  --chaos-http-errors 500,503 \
  --chaos-http-error-probability 0.2
```

### Quick Examples

#### Test with High Latency

```bash
# Add 1-second delay to all requests
mockforge serve --chaos --chaos-latency-ms 1000
```

#### Test with Random Errors

```bash
# 10% of requests return 500 errors
mockforge serve --chaos \
  --chaos-http-errors 500 \
  --chaos-http-error-probability 0.1
```

#### Test Rate Limiting

```bash
# Limit to 10 requests per second
mockforge serve --chaos --chaos-rate-limit 10
```

#### Test Bandwidth Constraints

```bash
# Limit bandwidth to 100KB/s with 5% packet loss
mockforge serve --chaos \
  --chaos-bandwidth-limit 100000 \
  --chaos-packet-loss 5
```

## Configuration

### YAML Configuration

```yaml
observability:
  chaos:
    enabled: true
    scenario: "network_degradation"  # Optional predefined scenario

    latency:
      enabled: true
      fixed_delay_ms: 500
      jitter_percent: 10.0
      probability: 0.8

    fault_injection:
      enabled: true
      http_errors: [500, 502, 503, 504]
      http_error_probability: 0.1
      connection_errors: true
      connection_error_probability: 0.05
      timeout_errors: true
      timeout_ms: 5000
      timeout_probability: 0.05

    rate_limit:
      enabled: true
      requests_per_second: 100
      burst_size: 10
      per_ip: true
      per_endpoint: false

    traffic_shaping:
      enabled: true
      bandwidth_limit_bps: 1000000  # 1MB/s
      packet_loss_percent: 2.0
      max_connections: 100
```

### Environment Variables

```bash
MOCKFORGE_CHAOS_ENABLED=true
MOCKFORGE_CHAOS_SCENARIO=network_degradation
MOCKFORGE_CHAOS_LATENCY_MS=500
MOCKFORGE_CHAOS_HTTP_ERRORS=500,503
MOCKFORGE_CHAOS_RATE_LIMIT=100
```

## CLI Usage

### Complete Flag Reference

```bash
mockforge serve \
  # Enable chaos engineering
  --chaos \

  # Predefined scenarios
  --chaos-scenario network_degradation \

  # Latency injection
  --chaos-latency-ms 500 \
  --chaos-latency-range "100-500" \
  --chaos-latency-probability 0.8 \

  # Fault injection
  --chaos-http-errors "500,502,503" \
  --chaos-http-error-probability 0.1 \

  # Rate limiting
  --chaos-rate-limit 100 \

  # Traffic shaping
  --chaos-bandwidth-limit 1000000 \
  --chaos-packet-loss 5.0
```

### CLI Examples

#### Development Testing

```bash
# Test with moderate latency
mockforge serve --chaos --chaos-latency-ms 200
```

#### Staging Environment

```bash
# Simulate production-like conditions
mockforge serve --chaos \
  --chaos-scenario service_instability \
  --chaos-rate-limit 1000
```

#### Extreme Stress Test

```bash
# Test with cascading failures
mockforge serve --chaos \
  --chaos-scenario cascading_failure
```

## Chaos Scenarios

MockForge includes 5 predefined chaos scenarios that simulate real-world failure patterns.

### 1. Network Degradation

Simulates poor network conditions with high latency and packet loss.

**Configuration:**
- Fixed delay: 500ms
- Jitter: 20%
- Packet loss: 5%
- Bandwidth: 100KB/s

**Use Case:** Test application behavior under poor network conditions

**Example:**
```bash
mockforge serve --chaos --chaos-scenario network_degradation
```

### 2. Service Instability

Simulates an unstable service with random errors and timeouts.

**Configuration:**
- HTTP errors (500, 502, 503, 504): 20% probability
- Timeout errors: 10% probability

**Use Case:** Test error handling and retry logic

**Example:**
```bash
mockforge serve --chaos --chaos-scenario service_instability
```

### 3. Cascading Failure

Simulates a cascading failure with multiple simultaneous issues.

**Configuration:**
- Random delay: 1-5 seconds
- Jitter: 30%
- HTTP errors: 30% probability
- Timeout errors: 20% probability
- Connection errors: 10% probability
- Rate limit: 10 req/s with burst of 2

**Use Case:** Test system resilience under multiple failure modes

**Example:**
```bash
mockforge serve --chaos --chaos-scenario cascading_failure
```

### 4. Peak Traffic

Simulates peak traffic conditions with aggressive rate limiting.

**Configuration:**
- Rate limit: 50 req/s
- Burst: 10
- Per-endpoint limiting enabled

**Use Case:** Test rate limiting and backpressure handling

**Example:**
```bash
mockforge serve --chaos --chaos-scenario peak_traffic
```

### 5. Slow Backend

Simulates a consistently slow backend service.

**Configuration:**
- Fixed delay: 2 seconds
- Jitter: 10%
- Applies to 100% of requests

**Use Case:** Test timeout handling and slow query performance

**Example:**
```bash
mockforge serve --chaos --chaos-scenario slow_backend
```

## Management API

The chaos management API allows dynamic control of chaos engineering features without restarting the server.

### Configuration Endpoints

#### Get Current Configuration

```http
GET /api/chaos/config
```

Response:
```json
{
  "enabled": true,
  "latency": {
    "enabled": true,
    "fixed_delay_ms": 500,
    "random_delay_range_ms": null,
    "jitter_percent": 10.0,
    "probability": 0.8
  },
  "fault_injection": {
    "enabled": true,
    "http_errors": [500, 502, 503],
    "http_error_probability": 0.1
  },
  "rate_limit": {
    "enabled": true,
    "requests_per_second": 100,
    "burst_size": 10,
    "per_ip": true,
    "per_endpoint": false
  },
  "traffic_shaping": {
    "enabled": true,
    "bandwidth_limit_bps": 1000000,
    "packet_loss_percent": 2.0,
    "max_connections": 100
  }
}
```

#### Update Full Configuration

```http
PUT /api/chaos/config
Content-Type: application/json

{
  "enabled": true,
  "latency": {...},
  "fault_injection": {...}
}
```

#### Update Latency Configuration

```http
PUT /api/chaos/config/latency
Content-Type: application/json

{
  "enabled": true,
  "fixed_delay_ms": 1000,
  "random_delay_range_ms": null,
  "jitter_percent": 15.0,
  "probability": 0.9
}
```

#### Update Fault Injection Configuration

```http
PUT /api/chaos/config/faults
Content-Type: application/json

{
  "enabled": true,
  "http_errors": [500, 503],
  "http_error_probability": 0.2,
  "connection_errors": true,
  "connection_error_probability": 0.1,
  "timeout_errors": true,
  "timeout_ms": 5000,
  "timeout_probability": 0.1
}
```

#### Update Rate Limit Configuration

```http
PUT /api/chaos/config/rate-limit
Content-Type: application/json

{
  "enabled": true,
  "requests_per_second": 50,
  "burst_size": 5,
  "per_ip": true,
  "per_endpoint": true
}
```

#### Update Traffic Shaping Configuration

```http
PUT /api/chaos/config/traffic
Content-Type: application/json

{
  "enabled": true,
  "bandwidth_limit_bps": 500000,
  "packet_loss_percent": 10.0,
  "max_connections": 50
}
```

### Control Endpoints

#### Enable Chaos

```http
POST /api/chaos/enable
```

#### Disable Chaos

```http
POST /api/chaos/disable
```

#### Reset Configuration

```http
POST /api/chaos/reset
```

### Scenario Endpoints

#### List Predefined Scenarios

```http
GET /api/chaos/scenarios/predefined
```

Response:
```json
[
  {
    "name": "network_degradation",
    "description": "Simulates degraded network conditions with high latency and packet loss",
    "tags": ["network", "latency"]
  },
  {
    "name": "service_instability",
    "description": "Simulates an unstable service with random errors and timeouts",
    "tags": ["service", "errors"]
  }
]
```

#### List Active Scenarios

```http
GET /api/chaos/scenarios
```

#### Start a Scenario

```http
POST /api/chaos/scenarios/network_degradation
```

#### Stop a Scenario

```http
DELETE /api/chaos/scenarios/network_degradation
```

#### Stop All Scenarios

```http
DELETE /api/chaos/scenarios
```

### Status Endpoint

#### Get Chaos Status

```http
GET /api/chaos/status
```

Response:
```json
{
  "enabled": true,
  "active_scenarios": ["network_degradation"],
  "latency_enabled": true,
  "fault_injection_enabled": true,
  "rate_limit_enabled": false,
  "traffic_shaping_enabled": true
}
```

## Latency Injection

### Fixed Delay

Add a consistent delay to all requests:

```yaml
latency:
  enabled: true
  fixed_delay_ms: 1000  # 1 second
  probability: 1.0      # Apply to 100% of requests
```

**CLI:**
```bash
mockforge serve --chaos --chaos-latency-ms 1000
```

### Random Delay Range

Add a random delay within a range:

```yaml
latency:
  enabled: true
  random_delay_range_ms: [100, 500]  # 100-500ms
  probability: 1.0
```

**CLI:**
```bash
mockforge serve --chaos --chaos-latency-range "100-500"
```

### Jitter

Add random variance to delays:

```yaml
latency:
  enabled: true
  fixed_delay_ms: 500
  jitter_percent: 20.0  # ±20% variance (400-600ms)
  probability: 1.0
```

### Probability-Based

Apply latency to a percentage of requests:

```yaml
latency:
  enabled: true
  fixed_delay_ms: 500
  probability: 0.5  # Apply to 50% of requests
```

**CLI:**
```bash
mockforge serve --chaos \
  --chaos-latency-ms 500 \
  --chaos-latency-probability 0.5
```

## Fault Injection

### HTTP Errors

Inject HTTP error responses:

```yaml
fault_injection:
  enabled: true
  http_errors: [500, 502, 503, 504]
  http_error_probability: 0.1  # 10% of requests
```

**CLI:**
```bash
mockforge serve --chaos \
  --chaos-http-errors "500,502,503,504" \
  --chaos-http-error-probability 0.1
```

### Connection Errors

Simulate network connection failures:

```yaml
fault_injection:
  enabled: true
  connection_errors: true
  connection_error_probability: 0.05  # 5% of requests
```

### Timeout Errors

Simulate request timeouts:

```yaml
fault_injection:
  enabled: true
  timeout_errors: true
  timeout_ms: 5000
  timeout_probability: 0.1  # 10% of requests
```

### Partial Responses

Simulate incomplete responses:

```yaml
fault_injection:
  enabled: true
  partial_responses: true
  partial_response_probability: 0.05  # 5% of requests
```

## Rate Limiting

### Global Rate Limit

Limit total requests per second:

```yaml
rate_limit:
  enabled: true
  requests_per_second: 100
  burst_size: 10
  per_ip: false
  per_endpoint: false
```

**CLI:**
```bash
mockforge serve --chaos --chaos-rate-limit 100
```

### Per-IP Rate Limit

Limit requests per IP address:

```yaml
rate_limit:
  enabled: true
  requests_per_second: 10
  burst_size: 2
  per_ip: true
  per_endpoint: false
```

### Per-Endpoint Rate Limit

Limit requests per endpoint:

```yaml
rate_limit:
  enabled: true
  requests_per_second: 50
  burst_size: 5
  per_ip: false
  per_endpoint: true
```

### Combined Rate Limits

Apply multiple rate limits:

```yaml
rate_limit:
  enabled: true
  requests_per_second: 100  # Global limit
  burst_size: 10
  per_ip: true              # Also limit per IP
  per_endpoint: true        # Also limit per endpoint
```

## Traffic Shaping

### Bandwidth Throttling

Limit bandwidth to simulate slow connections:

```yaml
traffic_shaping:
  enabled: true
  bandwidth_limit_bps: 100000  # 100 KB/s
```

**CLI:**
```bash
mockforge serve --chaos --chaos-bandwidth-limit 100000
```

### Packet Loss

Simulate packet loss:

```yaml
traffic_shaping:
  enabled: true
  packet_loss_percent: 5.0  # 5% packet loss
```

**CLI:**
```bash
mockforge serve --chaos --chaos-packet-loss 5.0
```

### Connection Limits

Limit concurrent connections:

```yaml
traffic_shaping:
  enabled: true
  max_connections: 100
  connection_timeout_ms: 30000
```

### Combined Traffic Shaping

Apply multiple constraints:

```yaml
traffic_shaping:
  enabled: true
  bandwidth_limit_bps: 100000  # 100 KB/s
  packet_loss_percent: 2.0      # 2% loss
  max_connections: 50           # Max 50 concurrent
```

## Best Practices

### 1. Start Simple

Begin with single failure modes:

```bash
# Start with just latency
mockforge serve --chaos --chaos-latency-ms 500

# Then add errors
mockforge serve --chaos \
  --chaos-latency-ms 500 \
  --chaos-http-errors 500 \
  --chaos-http-error-probability 0.1
```

### 2. Use Scenarios for Common Patterns

Leverage predefined scenarios instead of manual configuration:

```bash
# Use predefined scenarios
mockforge serve --chaos --chaos-scenario network_degradation
```

### 3. Test Gradually

Increase chaos intensity gradually:

```bash
# Light chaos (10% errors)
mockforge serve --chaos --chaos-http-error-probability 0.1

# Medium chaos (30% errors)
mockforge serve --chaos --chaos-http-error-probability 0.3

# Heavy chaos (50% errors)
mockforge serve --chaos --chaos-http-error-probability 0.5
```

### 4. Monitor Impact

Use observability features to monitor chaos impact:

```bash
# Enable chaos with metrics
mockforge serve \
  --chaos --chaos-scenario service_instability \
  --metrics
```

### 5. Automate Chaos Testing

Integrate chaos testing into CI/CD:

```bash
#!/bin/bash
# chaos-test.sh

# Start MockForge with chaos
mockforge serve --chaos --chaos-scenario network_degradation &
SERVER_PID=$!

# Run tests
npm test

# Stop server
kill $SERVER_PID
```

### 6. Document Expectations

Document expected behavior under chaos:

```yaml
# Expected behaviors:
# - Client should retry 500 errors
# - Client should timeout after 5 seconds
# - Client should handle partial responses gracefully
chaos:
  enabled: true
  fault_injection:
    http_errors: [500]
    http_error_probability: 0.2
```

### 7. Combine with Recording

Record chaos experiments for analysis:

```bash
# Record chaos experiments
mockforge serve \
  --chaos --chaos-scenario cascading_failure \
  --recorder
```

## Troubleshooting

### Chaos Not Working

**Issue**: Chaos features don't seem to be applying

**Solutions**:

1. Check if chaos is enabled:
   ```bash
   curl http://localhost:3000/api/chaos/status
   ```

2. Enable chaos:
   ```bash
   curl -X POST http://localhost:3000/api/chaos/enable
   ```

3. Verify configuration:
   ```bash
   curl http://localhost:3000/api/chaos/config
   ```

4. Check logs:
   ```bash
   mockforge serve --chaos --chaos-scenario network_degradation -v
   ```

### Rate Limits Too Aggressive

**Issue**: All requests are being rate limited

**Solutions**:

1. Increase rate limit:
   ```bash
   curl -X PUT http://localhost:3000/api/chaos/config/rate-limit \
     -H "Content-Type: application/json" \
     -d '{"enabled": true, "requests_per_second": 1000, "burst_size": 100}'
   ```

2. Disable rate limiting:
   ```bash
   curl -X PUT http://localhost:3000/api/chaos/config/rate-limit \
     -H "Content-Type: application/json" \
     -d '{"enabled": false}'
   ```

### Latency Too High

**Issue**: Requests are too slow

**Solutions**:

1. Reduce latency:
   ```bash
   curl -X PUT http://localhost:3000/api/chaos/config/latency \
     -H "Content-Type: application/json" \
     -d '{"enabled": true, "fixed_delay_ms": 100, "probability": 0.5}'
   ```

2. Disable latency injection:
   ```bash
   curl -X PUT http://localhost:3000/api/chaos/config/latency \
     -H "Content-Type: application/json" \
     -d '{"enabled": false}'
   ```

### Too Many Errors

**Issue**: Application receiving too many errors

**Solutions**:

1. Reduce error probability:
   ```bash
   curl -X PUT http://localhost:3000/api/chaos/config/faults \
     -H "Content-Type: application/json" \
     -d '{"enabled": true, "http_error_probability": 0.05}'
   ```

2. Disable fault injection:
   ```bash
   curl -X PUT http://localhost:3000/api/chaos/config/faults \
     -H "Content-Type: application/json" \
     -d '{"enabled": false}'
   ```

### Reset Everything

**Issue**: Need to start fresh

**Solution**:

```bash
# Reset all chaos configuration
curl -X POST http://localhost:3000/api/chaos/reset
```

## Integration with Other Features

### Chaos + Metrics

Monitor chaos impact with Prometheus metrics:

```bash
mockforge serve \
  --chaos --chaos-scenario service_instability \
  --metrics --metrics-port 9090
```

View metrics:
```bash
curl http://localhost:9090/metrics | grep mockforge
```

### Chaos + Tracing

Trace requests through chaos layers:

```bash
mockforge serve \
  --chaos --chaos-scenario network_degradation \
  --tracing --jaeger-endpoint http://localhost:14268/api/traces
```

### Chaos + Recording

Record chaos experiments for analysis:

```bash
mockforge serve \
  --chaos --chaos-scenario cascading_failure \
  --recorder --recorder-db chaos-test.db
```

Query recordings:
```bash
curl http://localhost:3000/api/recorder/search \
  -H "Content-Type: application/json" \
  -d '{"status_code": 500}'
```

## Advanced Usage

### Custom Scenario via API

Create a custom chaos scenario dynamically:

```bash
# Define custom configuration
curl -X PUT http://localhost:3000/api/chaos/config \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true,
    "latency": {
      "enabled": true,
      "random_delay_range_ms": [500, 2000],
      "jitter_percent": 25.0,
      "probability": 0.7
    },
    "fault_injection": {
      "enabled": true,
      "http_errors": [503],
      "http_error_probability": 0.15,
      "timeout_errors": true,
      "timeout_ms": 3000,
      "timeout_probability": 0.1
    }
  }'
```

### Progressive Chaos

Gradually increase chaos intensity:

```bash
#!/bin/bash
# progressive-chaos.sh

# Phase 1: Light chaos (5 minutes)
curl -X POST http://localhost:3000/api/chaos/scenarios/network_degradation
sleep 300

# Phase 2: Medium chaos (5 minutes)
curl -X POST http://localhost:3000/api/chaos/scenarios/service_instability
sleep 300

# Phase 3: Heavy chaos (5 minutes)
curl -X POST http://localhost:3000/api/chaos/scenarios/cascading_failure
sleep 300

# Reset
curl -X POST http://localhost:3000/api/chaos/reset
```

### Chaos Testing Framework

Integrate with testing frameworks:

```javascript
// chaos-test.js
const axios = require('axios');

describe('Chaos Tests', () => {
  beforeEach(async () => {
    // Enable chaos
    await axios.post('http://localhost:3000/api/chaos/enable');
  });

  afterEach(async () => {
    // Reset chaos
    await axios.post('http://localhost:3000/api/chaos/reset');
  });

  it('should handle network degradation', async () => {
    await axios.post('http://localhost:3000/api/chaos/scenarios/network_degradation');

    // Your test code here
    const response = await axios.get('http://localhost:3000/api/test');
    expect(response.status).toBe(200);
  });

  it('should handle service instability', async () => {
    await axios.post('http://localhost:3000/api/chaos/scenarios/service_instability');

    // Test error handling
    try {
      await axios.get('http://localhost:3000/api/test');
    } catch (error) {
      expect([500, 502, 503]).toContain(error.response.status);
    }
  });
});
```

## AI-Powered Recommendations

MockForge includes an intelligent recommendation engine that analyzes your chaos engineering experiments and provides actionable recommendations for improving system resilience.

### Overview

The AI-powered recommendation engine:
- Automatically detects patterns in chaos events
- Identifies system weaknesses and coverage gaps
- Generates prioritized, actionable recommendations
- Provides concrete examples and commands
- Scores recommendations by severity, confidence, and impact

### Getting Recommendations

#### Analyze Current Chaos Data

```bash
# Analyze last 24 hours and get recommendations
curl -X POST http://localhost:3000/api/chaos/recommendations/analyze | jq
```

Response:
```json
{
  "total_recommendations": 8,
  "high_priority": 3,
  "recommendations": [
    {
      "id": "rec-latency-a1b2c3d4",
      "category": "latency",
      "severity": "high",
      "confidence": 0.85,
      "title": "Increase latency testing for endpoint: /api/users",
      "description": "Endpoint /api/users shows high average latency (750ms)",
      "rationale": "High latency detected consistently across experiments",
      "action": "Test with latencies up to 1500ms to validate timeout handling",
      "example": "mockforge serve --chaos --chaos-latency-ms 1500",
      "affected_endpoints": ["/api/users"],
      "expected_impact": 0.85
    }
  ]
}
```

#### Filter by Severity

```bash
# Get only critical and high priority recommendations
curl http://localhost:3000/api/chaos/recommendations/severity/high | jq

# Get all recommendations
curl http://localhost:3000/api/chaos/recommendations | jq
```

#### Filter by Category

```bash
# Get latency recommendations
curl http://localhost:3000/api/chaos/recommendations/category/latency | jq

# Get coverage recommendations
curl http://localhost:3000/api/chaos/recommendations/category/coverage | jq

# Available categories:
# - latency
# - fault_injection
# - rate_limit
# - traffic_shaping
# - circuit_breaker
# - bulkhead
# - scenario
# - coverage
```

### Recommendation Categories

1. **Latency**: Recommendations for latency testing
   - Identifies endpoints with high latency
   - Suggests more aggressive latency scenarios
   - Provides timeout validation recommendations

2. **Fault Injection**: Error handling recommendations
   - Detects endpoints with high fault rates
   - Recommends diverse fault type testing
   - Suggests retry logic improvements

3. **Rate Limit**: Backpressure recommendations
   - Identifies rate limiting issues
   - Recommends retry logic with exponential backoff
   - Suggests rate limit testing strategies

4. **Traffic Shaping**: Network condition testing
   - Recommends bandwidth and packet loss testing
   - Suggests connection limit scenarios

5. **Circuit Breaker**: Circuit breaker pattern recommendations
   - Identifies cascading failure risks
   - Recommends circuit breaker implementation
   - Provides testing scenarios

6. **Bulkhead**: Bulkhead pattern recommendations
   - Recommends resource isolation
   - Suggests bulkhead testing strategies

7. **Scenario**: Chaos scenario recommendations
   - Recommends progressive testing strategies
   - Suggests scenario combinations
   - Provides orchestration recommendations

8. **Coverage**: Test coverage recommendations
   - Identifies untested protocols
   - Recommends missing fault types
   - Suggests coverage improvements

### Severity Levels

Recommendations are prioritized by severity:

- **Critical**: Must be addressed immediately
  - System shows severe degradation
  - No chaos testing detected
  - Major resilience patterns missing

- **High**: Should be addressed soon
  - Significant weakness detected
  - High fault or latency rates
  - Important patterns missing

- **Medium**: Should be planned for improvement
  - Moderate issues detected
  - Coverage gaps identified
  - Optimization opportunities

- **Low**: Nice to have improvements
  - Minor issues
  - Optional enhancements

- **Info**: Informational only
  - Best practice suggestions
  - General recommendations

### Example Recommendations

#### No Chaos Testing Detected

```json
{
  "severity": "critical",
  "confidence": 1.0,
  "title": "Start chaos engineering testing",
  "description": "No chaos testing detected",
  "action": "Start with the 'network_degradation' scenario",
  "example": "mockforge serve --chaos --chaos-scenario network_degradation"
}
```

#### High Latency Endpoint

```json
{
  "severity": "high",
  "confidence": 0.85,
  "title": "Increase latency testing for endpoint: /api/orders",
  "description": "Endpoint shows 800ms average latency",
  "action": "Test with latencies up to 1600ms",
  "example": "mockforge serve --chaos --chaos-latency-ms 1600"
}
```

#### Insufficient Fault Coverage

```json
{
  "severity": "high",
  "confidence": 0.80,
  "title": "Insufficient fault type coverage",
  "description": "Testing with limited fault types",
  "action": "Add diverse fault injection scenarios",
  "example": "mockforge serve --chaos --chaos-scenario service_instability"
}
```

#### Low System Resilience

```json
{
  "severity": "critical",
  "confidence": 0.85,
  "title": "System shows low resilience - implement resilience patterns",
  "description": "System degradation of 75% under chaos",
  "action": "Implement circuit breaker and bulkhead patterns",
  "example": "mockforge serve --chaos --chaos-scenario cascading_failure"
}
```

### Best Practices

#### 1. Regular Analysis

Run analysis regularly to identify new issues:

```bash
# Daily cron job
0 6 * * * curl -X POST http://localhost:3000/api/chaos/recommendations/analyze
```

#### 2. Focus on High-Priority First

Address critical and high-severity recommendations first:

```bash
curl http://localhost:3000/api/chaos/recommendations/severity/critical
```

#### 3. Track Progress

Clear recommendations after addressing them:

```bash
# After implementing fixes
curl -X DELETE http://localhost:3000/api/chaos/recommendations

# Re-analyze to see improvement
curl -X POST http://localhost:3000/api/chaos/recommendations/analyze
```

#### 4. Category-Focused Improvements

Target specific areas in each sprint:

```bash
# This sprint: improve latency handling
curl http://localhost:3000/api/chaos/recommendations/category/latency

# Next sprint: improve fault coverage
curl http://localhost:3000/api/chaos/recommendations/category/coverage
```

#### 5. Automate in CI/CD

Fail builds if critical recommendations are found:

```bash
#!/bin/bash
# ci-check-recommendations.sh

RECS=$(curl -s -X POST http://localhost:3000/api/chaos/recommendations/analyze)
CRITICAL=$(echo "$RECS" | jq '[.recommendations[] | select(.severity == "critical")] | length')

if [ "$CRITICAL" -gt 0 ]; then
    echo "Critical chaos recommendations found!"
    echo "$RECS" | jq '.recommendations[] | select(.severity == "critical")'
    exit 1
fi
```

### Recommendation Scoring

Recommendations are scored using a weighted algorithm:

```
Score = (Severity × 0.4) + (Confidence × 0.3) + (Expected Impact × 0.3)
```

Where:
- **Severity**: Critical=1.0, High=0.8, Medium=0.6, Low=0.4, Info=0.2
- **Confidence**: 0.0 - 1.0 (algorithm confidence in the recommendation)
- **Expected Impact**: 0.0 - 1.0 (estimated improvement from implementing)

Recommendations are sorted by score (highest first) and limited to top 20 by default.

### API Endpoints

```
GET    /api/chaos/recommendations                      - Get all recommendations
POST   /api/chaos/recommendations/analyze              - Analyze and generate recommendations
GET    /api/chaos/recommendations/category/:category   - Get by category
GET    /api/chaos/recommendations/severity/:severity   - Get by severity
DELETE /api/chaos/recommendations                      - Clear recommendations
```

### Pattern Detection

The engine detects the following patterns:

1. **High Latency Pattern**
   - Threshold: Average latency > 500ms
   - Generates latency testing recommendations
   - Includes affected endpoints

2. **High Fault Rate Pattern**
   - Threshold: Fault rate > 20%
   - Generates error handling recommendations
   - Suggests comprehensive fault testing

3. **Frequent Rate Limits**
   - Threshold: Violation rate > 10%
   - Generates backpressure recommendations
   - Suggests retry logic improvements

4. **Increasing Fault Trend**
   - Threshold: 50% increase between time periods
   - Detects cascading failures
   - Recommends resilience patterns

5. **Coverage Gaps**
   - Detects missing protocols
   - Identifies insufficient fault types
   - Recommends comprehensive testing

For complete documentation, see [AI Recommendations Guide](../PHASE_8_AI_RECOMMENDATIONS_COMPLETE.md).

## Next Steps

- **Combine Features**: Use chaos engineering with metrics, tracing, and recording
- **Automate Testing**: Integrate chaos tests into CI/CD pipelines
- **Document Learnings**: Record findings from chaos experiments
- **Expand Scenarios**: Create custom scenarios for your specific use cases

For more information, see:
- [Observability Guide](./OBSERVABILITY.md)
- [OpenTelemetry Integration](./OPENTELEMETRY.md)
- [API Flight Recorder](./API_FLIGHT_RECORDER.md)
