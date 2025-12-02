# MockForge Load Testing Suite

Comprehensive load testing infrastructure for MockForge using industry-standard tools.

## Overview

This directory contains load testing scripts and configurations for testing MockForge's performance under various load conditions across all supported protocols:

- **HTTP/REST**: Using k6 and work
- **WebSocket**: Using k6 with WebSocket support
- **gRPC**: Using k6 with gRPC support

## Directory Structure

```
tests/load/
├── http_load.js              # k6 HTTP load test scenarios
├── websocket_load.js         # k6 WebSocket stress test
├── grpc_load.js              # k6 gRPC load test
├── marketplace_load.js       # k6 marketplace load test (plugins, templates, scenarios)
├── work_http.lua              # work Lua script for HTTP testing
├── run_http_load.sh          # HTTP load test runner
├── run_websocket_load.sh     # WebSocket load test runner
├── run_grpc_load.sh          # gRPC load test runner
├── run_marketplace_load.sh   # Marketplace load test runner
├── run_all_load_tests.sh     # Run all load tests sequentially
├── results/                  # Test results directory (auto-created)
└── README.md                 # This file
```

## Prerequisites

### Required Tools

1. **k6** (for HTTP, WebSocket, and gRPC load testing)
   ```bash
   # macOS
   brew install k6

   # Linux
   sudo gpg -k
   sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg \
     --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
   echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | \
     sudo tee /etc/apt/sources.list.d/k6.list
   sudo apt-get update
   sudo apt-get install k6

   # Windows
   choco install k6
   ```

2. **work** (optional, for HTTP load testing)
   ```bash
   # macOS
   brew install work

   # Linux
   git clone https://github.com/wg/wrk.git
   cd work
   make
   sudo cp work /usr/local/bin/

   # Windows
   # Build from source or use WSL
   ```

### Optional Tools

- **grpcurl** (for gRPC server verification)
  ```bash
  # macOS
  brew install grpcurl

  # Linux
  go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest

  # Or download binary from releases
  ```

## Quick Start

### 1. Start MockForge Server

Before running load tests, ensure MockForge is running:

```bash
# Start with default configuration
cargo run --release

# Or with specific configuration
cargo run --release -- --config your-config.yaml
```

### 2. Run Individual Load Tests

#### HTTP Load Test (k6)
```bash
./tests/load/run_http_load.sh
```

With custom parameters:
```bash
BASE_URL=http://localhost:8080 \
DURATION=2m \
TOOL=k6 \
./tests/load/run_http_load.sh
```

#### HTTP Load Test (work)
```bash
TOOL=work \
DURATION=60s \
CONNECTIONS=200 \
THREADS=8 \
./tests/load/run_http_load.sh
```

#### WebSocket Load Test
```bash
./tests/load/run_websocket_load.sh
```

With custom parameters:
```bash
BASE_URL=ws://localhost:8080 \
DURATION=5m \
VUS=150 \
./tests/load/run_websocket_load.sh
```

#### gRPC Load Test
```bash
./tests/load/run_grpc_load.sh
```

With custom parameters:
```bash
GRPC_ADDR=localhost:50051 \
USE_TLS=false \
DURATION=5m \
VUS=100 \
./tests/load/run_grpc_load.sh
```

### 3. Run All Load Tests

Run the complete load test suite:

```bash
# Full mode (longer duration)
./tests/load/run_all_load_tests.sh

# Quick mode (shorter duration for CI)
QUICK_MODE=true ./tests/load/run_all_load_tests.sh
```

## Load Test Scenarios

### HTTP Load Test (`http_load.js`)

Tests various HTTP scenarios:

1. **Simple GET requests** - Basic endpoint testing
2. **POST with JSON payload** - Create operations
3. **Requests with query parameters** - Filtering and pagination
4. **Requests with headers** - Authentication and custom headers
5. **Batch requests** - Multiple concurrent requests

**Load Profile:**
- Ramp up: 20 → 50 → 100 users over 3.5 minutes
- Sustained: 100 users for 2 minutes
- Ramp down: 100 → 50 → 0 users over 1.5 minutes

**Thresholds:**
- 95% of requests < 500ms
- 99% of requests < 1000ms
- Error rate < 5%

### WebSocket Load Test (`websocket_load.js`)

Tests WebSocket connections and messaging:

1. **Connection establishment** - WebSocket handshake performance
2. **Message latency** - Round-trip time measurement
3. **Ping/pong handling** - Keepalive mechanism testing
4. **Burst messaging** - High-frequency message sending
5. **Long-lived connections** - Connection stability

**Load Profile:**
- Ramp up: 10 → 50 → 100 connections over 3.5 minutes
- Sustained: 100 connections for 2 minutes
- Ramp down: 100 → 50 → 0 connections over 1.5 minutes

**Thresholds:**
- 95% connection time < 1000ms
- 99% connection time < 2000ms
- 95% message latency < 200ms
- 99% message latency < 500ms
- Error rate < 5%

### gRPC Load Test (`grpc_load.js`)

Tests all gRPC call types:

1. **Unary calls** - Simple request-response
2. **Server streaming** - Single request, multiple responses
3. **Client streaming** - Multiple requests, single response
4. **Bidirectional streaming** - Full-duplex communication

**Load Profile:**
- Ramp up: 20 → 50 → 100 users over 3.5 minutes
- Sustained: 100 users for 2 minutes
- Ramp down: 100 → 50 → 0 users over 1.5 minutes

**Thresholds:**
- 95% of requests < 500ms
- 99% of requests < 1000ms
- Error rate < 5%

### work HTTP Test (`work_http.lua`)

Advanced work load test with:

1. **Mixed request types** - GET (60%), POST (20%), GET by ID (15%), DELETE (5%)
2. **Dynamic payloads** - Randomized test data
3. **Custom metrics** - Detailed latency distribution
4. **Status code tracking** - Response code analysis

## Configuration

### Environment Variables

All load test scripts support the following environment variables:

#### HTTP Load Tests
- `BASE_URL` - HTTP server URL (default: `http://localhost:8080`)
- `DURATION` - Test duration (default: `60s`)
- `CONNECTIONS` - Number of connections for work (default: `100`)
- `THREADS` - Number of threads for work (default: `4`)
- `TOOL` - Load testing tool: `k6` or `work` (default: `k6`)

#### WebSocket Load Tests
- `BASE_URL` - WebSocket server URL (default: `ws://localhost:8080`)
- `DURATION` - Test duration (default: `5m`)
- `VUS` - Number of virtual users (default: `100`)

#### gRPC Load Tests
- `GRPC_ADDR` - gRPC server address (default: `localhost:50051`)
- `USE_TLS` - Use TLS connection (default: `false`)
- `DURATION` - Test duration (default: `5m`)
- `VUS` - Number of virtual users (default: `100`)

#### Comprehensive Tests
- `BASE_URL` - HTTP server URL
- `WS_URL` - WebSocket server URL
- `GRPC_ADDR` - gRPC server address
- `QUICK_MODE` - Run shorter tests (default: `false`)

### Customizing Load Profiles

Edit the JavaScript files to customize load profiles:

```javascript
// In http_load.js, websocket_load.js, or grpc_load.js
export const options = {
  stages: [
    { duration: '30s', target: 20 },   // Adjust duration and target
    { duration: '1m', target: 50 },
    // ... add more stages
  ],
  thresholds: {
    'http_req_duration': ['p(95)<500'],  // Adjust thresholds
    // ... add more thresholds
  },
};
```

## Results and Analysis

### Results Directory Structure

All test results are saved in `tests/load/results/`:

```
results/
├── run_20250106_143022/          # Timestamped run directory
│   ├── k6-http-results.json      # k6 HTTP raw results
│   ├── k6-http-summary.json      # k6 HTTP summary
│   ├── k6-websocket-results.json # k6 WebSocket raw results
│   ├── k6-websocket-summary.json # k6 WebSocket summary
│   ├── k6-grpc-results.json      # k6 gRPC raw results
│   ├── k6-grpc-summary.json      # k6 gRPC summary
│   └── work-http-results.txt      # work results
└── ...
```

### Analyzing Results

#### k6 Results

k6 provides detailed output including:
- Request rate (requests/sec)
- Response time percentiles (p50, p75, p90, p95, p99)
- Error rate
- Data transfer rate
- Custom metrics

View summary JSON:
```bash
cat tests/load/results/k6-http-summary.json | jq
```

Analyze specific metrics:
```bash
# Get p95 response time
cat tests/load/results/k6-http-summary.json | jq '.metrics.http_req_duration.values.p95'

# Get error rate
cat tests/load/results/k6-http-summary.json | jq '.metrics.http_req_failed.values.rate'
```

#### work Results

work provides:
- Requests per second
- Transfer rate
- Latency distribution
- Status code distribution

Results are saved in `work-http-results.txt`.

### Performance Baselines

Recommended baselines for MockForge:

| Metric | HTTP | WebSocket | gRPC |
|--------|------|-----------|------|
| p95 latency | < 500ms | < 200ms | < 500ms |
| p99 latency | < 1000ms | < 500ms | < 1000ms |
| Error rate | < 5% | < 5% | < 5% |
| Throughput | > 1000 req/s | > 500 msg/s | > 500 req/s |

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Load Tests

on:
  schedule:
    - cron: '0 2 * * *'  # Daily at 2 AM
  workflow_dispatch:

jobs:
  load-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install k6
        run: |
          sudo gpg -k
          sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg \
            --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
          echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | \
            sudo tee /etc/apt/sources.list.d/k6.list
          sudo apt-get update
          sudo apt-get install k6

      - name: Build and run MockForge
        run: |
          cargo build --release
          cargo run --release &
          sleep 10

      - name: Run load tests
        run: |
          QUICK_MODE=true ./tests/load/run_all_load_tests.sh

      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: load-test-results
          path: tests/load/results/
```

## Troubleshooting

### Common Issues

1. **Server not running**
   ```
   Error: Server is not running at http://localhost:8080
   ```
   Solution: Start MockForge before running load tests

2. **k6 not found**
   ```
   Error: k6 is not installed
   ```
   Solution: Install k6 using instructions in Prerequisites

3. **Connection refused**
   ```
   Error: dial tcp [::1]:8080: connect: connection refused
   ```
   Solution: Check if the server is listening on the correct port

4. **High error rate**
   - Check server logs for errors
   - Reduce load (lower VUs or connections)
   - Increase server resources
   - Check network connectivity

5. **Slow response times**
   - Check server resource usage (CPU, memory)
   - Optimize server configuration
   - Check for database bottlenecks
   - Review application logs

### Debug Mode

Run k6 with verbose logging:
```bash
k6 run --verbose tests/load/http_load.js
```

Enable debug logging in scripts:
```bash
K6_LOG_OUTPUT=stdout \
K6_LOG_LEVEL=debug \
k6 run tests/load/http_load.js
```

## Best Practices

1. **Baseline First**: Establish performance baselines before making changes
2. **Incremental Load**: Start with small loads and gradually increase
3. **Monitor Resources**: Watch CPU, memory, and network during tests
4. **Realistic Data**: Use production-like data and scenarios
5. **Consistent Environment**: Run tests in consistent environments
6. **Regular Testing**: Run load tests regularly to catch regressions
7. **Document Results**: Keep track of results over time
8. **Test Isolation**: Run load tests in isolation from other services

## Advanced Usage

### Custom Scenarios

Create custom k6 scenarios by editing the test files:

```javascript
export const scenarios = {
  spike_test: {
    executor: 'ramping-arrival-rate',
    startRate: 0,
    timeUnit: '1s',
    preAllocatedVUs: 50,
    maxVUs: 500,
    stages: [
      { duration: '10s', target: 10 },
      { duration: '1m', target: 10 },
      { duration: '10s', target: 100 },  // Spike
      { duration: '3m', target: 100 },
      { duration: '10s', target: 10 },
      { duration: '3m', target: 10 },
      { duration: '10s', target: 0 },
    ],
  },
};
```

### Distributed Load Testing

Run k6 in distributed mode using k6 Cloud or k6 Operator on Kubernetes.

### Performance Monitoring

Integrate with monitoring tools:
- Grafana + InfluxDB for real-time metrics
- Prometheus for metrics collection
- Datadog, New Relic for APM

## Contributing

When adding new load tests:

1. Follow the existing naming convention
2. Add appropriate thresholds
3. Document the test scenarios
4. Update this README
5. Test locally before committing

## References

- [k6 Documentation](https://k6.io/docs/)
- [work Documentation](https://github.com/wg/wrk)
- [Load Testing Best Practices](https://k6.io/docs/testing-guides/test-types/)
- [Performance Testing Types](https://k6.io/docs/test-types/introduction/)

## License

MIT OR Apache-2.0
