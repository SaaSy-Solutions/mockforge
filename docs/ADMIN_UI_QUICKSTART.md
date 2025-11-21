# MockForge Admin UI - Quick Start Guide

**Pillars:** [DevX]

[DevX] - Interactive playgrounds and developer experience tools

## Overview

The MockForge Admin UI provides a comprehensive web interface for managing chaos engineering, observability, and API testing features.

## Features

### 1. Real-Time Observability Dashboard
- Live metrics streaming
- Active alerts monitoring
- Chaos impact scoring
- Top affected endpoints
- Real-time event timeline

### 2. Distributed Trace Viewer
- OpenTelemetry trace visualization
- Hierarchical span tree
- Service dependency mapping
- Detailed span attributes

### 3. Chaos Scenario Control
- 5 predefined scenarios
- One-click activation
- Real-time configuration
- Quick parameter controls

### 4. API Flight Recorder
- Request/response recording
- Scenario replay
- Export to JSON/YAML
- Protocol filtering

## Getting Started

### 1. Start MockForge with Admin UI

```bash
# Basic admin UI
mockforge serve --admin --admin-port 9080

# With all observability features
mockforge serve \
  --admin --admin-port 9080 \
  --metrics --metrics-port 9090 \
  --tracing --jaeger-endpoint http://localhost:14268/api/traces \
  --recorder --recorder-db mockforge.db \
  --chaos
```

### 2. Access the Admin UI

Open your browser to:
```
http://localhost:9080
```

### 3. Navigate the UI

#### Main Dashboard
- **Dashboard**: System overview and performance metrics
- **Metrics**: Detailed performance analytics
- **Logs**: Request/response logs

#### Observability Features (Phase 5)
- **Observability**: Real-time chaos metrics and alerts
- **Traces**: Distributed trace viewer
- **Chaos Engineering**: Scenario control interface
- **API Flight Recorder**: Recording and replay

## Usage Examples

### Example 1: Monitor Chaos Engineering Impact

1. Navigate to **Observability** page
2. Observe the connection status (should show "Connected")
3. Start a chaos scenario from the **Chaos Engineering** page
4. Return to **Observability** page
5. Watch real-time metrics update:
   - Events counter increases
   - Average latency changes
   - Impact score rises
   - Affected endpoints tracked

### Example 2: View Distributed Traces

1. Enable tracing when starting MockForge:
   ```bash
   mockforge serve --admin --tracing --jaeger-endpoint http://localhost:14268/api/traces
   ```

2. Make some API requests to generate traces
3. Navigate to **Traces** page
4. Browse the list of traces
5. Click a trace to view:
   - Span hierarchy
   - Timing information
   - Attributes and events
   - Error status

### Example 3: Test with Chaos Scenarios

1. Navigate to **Chaos Engineering** page
2. Choose a scenario:
   - **Network Degradation**: 500ms latency + packet loss
   - **Service Instability**: Random errors and timeouts
   - **Cascading Failure**: Multiple failure modes
   - **Peak Traffic**: Aggressive rate limiting
   - **Slow Backend**: Consistent high latency

3. Click "Start Scenario"
4. Make API requests to test your application's resilience
5. Monitor impact on **Observability** page
6. Click "Stop All Chaos" when done

### Example 4: Record and Replay API Calls

1. Navigate to **API Flight Recorder** page
2. Click "Start Recording"
3. Execute your test sequence (make API calls)
4. Click "Stop Recording"
5. Browse recorded requests:
   - Filter by protocol (HTTP, gRPC, WebSocket, GraphQL)
   - Search by path or method
   - View detailed request/response data
6. Click "Replay" to re-execute the scenario
7. Click "Export" to save for CI/CD integration

### Example 5: Analyze System Performance

1. Navigate to **Metrics** page (existing feature)
2. View key performance indicators:
   - Total requests
   - Average response time
   - Error rate
   - Active endpoints
3. Examine charts:
   - Request distribution by endpoint
   - Response time percentiles
   - Error rates
   - System resource usage

## WebSocket Real-Time Updates

The Observability Dashboard uses WebSocket for live updates:

- **Auto-reconnect**: Automatically reconnects if disconnected
- **Connection indicator**: Shows connection status
- **Live metrics**: Updates every second
- **Alerts**: Fires immediately when thresholds exceeded

## API Endpoints

All UI features are backed by REST APIs:

### Observability
```
GET  /api/observability/stats         # Dashboard statistics
GET  /api/observability/alerts        # Active alerts
GET  /api/observability/ws            # WebSocket endpoint
GET  /api/observability/traces        # List traces
```

### Chaos Control
```
GET  /api/chaos/scenarios             # List scenarios
POST /api/chaos/scenarios/:name       # Start scenario
GET  /api/chaos/status                # Get status
POST /api/chaos/disable               # Disable chaos
POST /api/chaos/reset                 # Reset configuration
```

### Recording & Replay
```
POST /api/chaos/recording/start       # Start recording
POST /api/chaos/recording/stop        # Stop recording
GET  /api/chaos/recording/status      # Recording status
GET  /api/chaos/recording/list        # List recordings
POST /api/chaos/recording/export      # Export scenario
POST /api/chaos/replay/start          # Start replay
POST /api/chaos/replay/stop           # Stop replay
GET  /api/chaos/replay/status         # Replay status
```

### API Flight Recorder
```
POST /api/recorder/search             # Search requests
```

## Configuration

### Admin UI Configuration

Via YAML:
```yaml
admin:
  enabled: true
  port: 9080
  host: "127.0.0.1"
  auth_required: false
  api_enabled: true
```

Via CLI:
```bash
mockforge serve --admin --admin-port 9080
```

Via Environment Variables:
```bash
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_ADMIN_PORT=9080
mockforge serve
```

## Troubleshooting

### Admin UI doesn't load
- Verify MockForge is running: `ps aux | grep mockforge`
- Check the port is correct: default is 9080
- Try accessing: `curl http://localhost:9080`

### WebSocket disconnected
- Check browser console for errors
- Verify network connectivity
- WebSocket auto-reconnects after 3 seconds

### No metrics/traces visible
- Ensure observability features are enabled:
  - `--metrics` for Prometheus metrics
  - `--tracing` for OpenTelemetry tracing
  - `--recorder` for API Flight Recorder
  - `--chaos` for chaos engineering

### Chaos scenarios not starting
- Check backend logs for errors
- Verify chaos is enabled: `--chaos` flag
- Ensure no conflicting scenarios are active

## Best Practices

1. **Start Simple**: Begin with the Dashboard to understand system state
2. **Enable Tracing**: Use `--tracing` for full observability
3. **Record First**: Record scenarios before replaying in CI/CD
4. **Monitor Impact**: Keep Observability page open when running chaos
5. **Export Scenarios**: Save successful test scenarios for reuse

## Next Steps

- Read [CHAOS_ENGINEERING.md](CHAOS_ENGINEERING.md) for detailed chaos features
- See [OBSERVABILITY.md](OBSERVABILITY.md) for metrics and tracing setup
- Check [API_FLIGHT_RECORDER.md](API_FLIGHT_RECORDER.md) for recording details
- Review [OPENTELEMETRY.md](OPENTELEMETRY.md) for distributed tracing

## Support

For issues or questions:
- GitHub Issues: https://github.com/mockforge/mockforge/issues
- Documentation: https://docs.mockforge.dev
- Examples: `examples/` directory in the repository
