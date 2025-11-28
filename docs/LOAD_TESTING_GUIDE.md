# Load Testing Guide

**Date**: 2025-01-27
**Status**: ✅ **Implemented**

## Overview

MockForge includes comprehensive load testing infrastructure to validate performance and scalability under high load conditions, including tests for 10,000+ concurrent connections.

## Quick Start

### Run Standard Load Tests

```bash
make load-test
```

### Run High-Scale Load Tests (10,000+ VUs)

```bash
make load-test-high-scale
```

### Run Individual Protocol Tests

```bash
make load-test-http
make load-test-websocket
make load-test-grpc
```

## Load Test Scenarios

### Standard Load Tests

Located in `tests/load/`, these tests validate:
- **HTTP**: Up to 100 concurrent users
- **WebSocket**: Up to 100 concurrent connections
- **gRPC**: Up to 100 concurrent RPCs

**Duration**: 5-10 minutes total
**Purpose**: Quick validation, CI/CD integration

### High-Scale Load Tests

New tests designed for production-scale validation:
- **HTTP**: Up to 10,000 concurrent users
- **WebSocket**: Up to 10,000 concurrent connections
- **gRPC**: Up to 10,000 concurrent RPCs

**Duration**: 16-20 minutes total
**Purpose**: Production scalability validation

## Test Configuration

### High-Scale HTTP Test

**File**: `tests/load/http_load_high_scale.js`

**Load Profile**:
- Ramp up: 0 → 5,000 users (5 minutes)
- Sustain: 5,000 users (3 minutes)
- Ramp up: 5,000 → 10,000 users (3 minutes)
- Sustain: 10,000 users (5 minutes)
- Ramp down: 10,000 → 0 users (6 minutes)

**Thresholds**:
- P95 latency < 1 second
- P99 latency < 2 seconds
- Error rate < 1%
- Minimum throughput: 100 req/s

**Test Scenarios**:
- GET /health (10%)
- GET /api/users (30%)
- GET /api/users/:id (25%)
- POST /api/users (20%)
- PUT /api/users/:id (10%)
- DELETE /api/users/:id (5%)

### High-Scale WebSocket Test

**File**: `tests/load/websocket_load_high_scale.js`

**Load Profile**:
- Ramp up: 0 → 5,000 connections (5 minutes)
- Sustain: 5,000 connections (3 minutes)
- Ramp up: 5,000 → 10,000 connections (3 minutes)
- Sustain: 10,000 connections (5 minutes)
- Ramp down: 10,000 → 0 connections (6 minutes)

**Thresholds**:
- P95 connection time < 2 seconds
- P99 connection time < 5 seconds
- P95 message latency < 500ms
- P99 message latency < 1 second
- Connection error rate < 1%
- Message error rate < 1%

**Behavior**:
- Establishes WebSocket connections
- Sends ping messages every 2 seconds
- Maintains connections for 5 minutes
- Measures round-trip message latency

## Prerequisites

### Required Tools

1. **k6** (Load testing tool)
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

### System Requirements

For high-scale tests (10,000+ connections):

1. **File Descriptor Limit**
   ```bash
   # Check current limit
   ulimit -n

   # Increase limit (recommended: 65536)
   ulimit -n 65536

   # Make permanent (add to /etc/security/limits.conf)
   * soft nofile 65536
   * hard nofile 65536
   ```

2. **Memory**: At least 8GB RAM recommended
3. **CPU**: Multi-core recommended (4+ cores)
4. **Network**: Sufficient bandwidth for high throughput

## Usage

### Basic Usage

```bash
# Run all high-scale tests
make load-test-high-scale

# Run specific protocol
PROTOCOL=http make load-test-high-scale
PROTOCOL=websocket make load-test-high-scale
```

### Custom Configuration

```bash
# Custom base URL
BASE_URL=http://localhost:8080 make load-test-high-scale

# Custom WebSocket URL
WS_URL=ws://localhost:8081/ws make load-test-high-scale

# Custom duration (modify test files)
DURATION=10m make load-test-high-scale
```

### Running in CI/CD

For CI/CD, use quick mode:

```bash
# Standard tests with quick mode
QUICK_MODE=true make load-test

# Or modify test stages for shorter duration
```

## Test Results

### Results Location

Test results are saved to:
```
tests/load/results/high_scale_YYYYMMDD_HHMMSS/
├── http_high_scale.json           # Detailed HTTP metrics
├── http_high_scale_summary.json   # HTTP summary
├── websocket_high_scale.json      # Detailed WebSocket metrics
└── websocket_high_scale_summary.json  # WebSocket summary
```

### Metrics Tracked

**HTTP Metrics**:
- Request count and rate
- Response times (avg, p90, p95, p99, max)
- Error rate
- Data transfer (sent/received)
- Throughput

**WebSocket Metrics**:
- Connection count
- Connection establishment time
- Message latency
- Connection error rate
- Message error rate
- Active connections over time

### Viewing Results

```bash
# View summary
cat tests/load/results/high_scale_*/http_high_scale_summary.json | jq

# View detailed metrics
cat tests/load/results/high_scale_*/http_high_scale.json | jq '.metrics'
```

## Performance Targets

### HTTP Targets

- **Throughput**: > 100 req/s minimum, > 1000 req/s at peak
- **Latency**: P95 < 1s, P99 < 2s
- **Error Rate**: < 1%
- **Concurrent Users**: Support 10,000+ simultaneous users

### WebSocket Targets

- **Connection Time**: P95 < 2s, P99 < 5s
- **Message Latency**: P95 < 500ms, P99 < 1s
- **Connection Stability**: 99%+ uptime during test
- **Concurrent Connections**: Support 10,000+ simultaneous connections

### System Targets

- **Memory Usage**: Stable under load (no memory leaks)
- **CPU Usage**: Efficient utilization (< 80% per core)
- **File Descriptors**: Handle 10,000+ open connections
- **Network**: Efficient bandwidth utilization

## Troubleshooting

### Test Fails with "Too Many Open Files"

**Problem**: File descriptor limit too low

**Solution**:
```bash
ulimit -n 65536
# Or add to /etc/security/limits.conf
```

### High Error Rate

**Problem**: Server can't handle load

**Solutions**:
1. Check server resources (CPU, memory)
2. Verify MockForge is running in release mode
3. Increase system limits (file descriptors, memory)
4. Check network bandwidth
5. Review server logs for errors

### Connection Timeouts

**Problem**: WebSocket connections timing out

**Solutions**:
1. Increase connection timeout in test
2. Check server connection limits
3. Verify network connectivity
4. Review server resource usage

### Out of Memory

**Problem**: Test runner or server runs out of memory

**Solutions**:
1. Reduce concurrent users
2. Increase system memory
3. Run tests on separate machines
4. Use distributed k6 execution

## Best Practices

### 1. Pre-Test Setup

- Start MockForge in release mode
- Monitor system resources (htop, iotop)
- Check file descriptor limits
- Verify network connectivity
- Clear previous test results

### 2. During Test Execution

- Monitor server metrics (Prometheus, logs)
- Watch system resources
- Check for errors in real-time
- Don't run other heavy processes

### 3. Post-Test Analysis

- Review all metrics
- Compare against targets
- Identify bottlenecks
- Document findings
- Create performance baselines

### 4. Iterative Testing

- Start with lower loads (100, 500, 1000 users)
- Gradually increase to target (10,000+)
- Document performance at each level
- Identify breaking points
- Optimize before scaling

## CI/CD Integration

### GitHub Actions Example

```yaml
load-test:
  name: Load Test
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v5
    - name: Install k6
      run: |
        sudo gpg -k
        sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg \
          --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
        echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | \
          sudo tee /etc/apt/sources.list.d/k6.list
        sudo apt-get update
        sudo apt-get install k6

    - name: Start MockForge
      run: cargo run --release -- serve &

    - name: Wait for server
      run: |
        timeout 30 bash -c 'until curl -f http://localhost:3000/health; do sleep 1; done'

    - name: Run load tests
      run: make load-test

    - name: Upload results
      uses: actions/upload-artifact@v3
      if: always()
      with:
        name: load-test-results
        path: tests/load/results/
```

## Advanced Usage

### Custom Test Scenarios

Modify test files to add custom scenarios:

```javascript
const scenarios = [
    {
        name: 'Custom Endpoint',
        method: 'GET',
        path: '/custom/path',
        weight: 50,
    },
    // Add more scenarios
];
```

### Distributed Load Testing

For very high loads, use k6 cloud or distributed execution:

```bash
# k6 cloud (requires Grafana Cloud account)
k6 cloud tests/load/http_load_high_scale.js

# Distributed execution (requires k6 instances)
k6 run --dist --address localhost:6565 tests/load/http_load_high_scale.js
```

### Performance Profiling

Combine load testing with profiling:

```bash
# Run with profiling
cargo run --release -- serve &
perf record -g cargo run --release -- serve
# Run load test
make load-test-high-scale
# Analyze profile
perf report
```

## Files Created

1. **`tests/load/http_load_high_scale.js`** (NEW)
   - High-scale HTTP load test (10,000+ VUs)
   - Comprehensive metrics and thresholds

2. **`tests/load/websocket_load_high_scale.js`** (NEW)
   - High-scale WebSocket load test (10,000+ connections)
   - Connection and message latency tracking

3. **`tests/load/run_high_scale_load.sh`** (NEW)
   - High-scale load test runner
   - System resource validation
   - Results aggregation

4. **`Makefile`** (MODIFIED)
   - Added load testing commands
   - Easy-to-use targets

5. **`docs/LOAD_TESTING_GUIDE.md`** (NEW)
   - Comprehensive documentation
   - Usage examples
   - Troubleshooting guide

## Next Steps

Potential enhancements:
1. **gRPC High-Scale Test**: Add 10,000+ concurrent gRPC test
2. **Mixed Protocol Test**: Test all protocols simultaneously
3. **Real-world Scenarios**: Traffic patterns from production
4. **Automated Performance Regression**: CI/CD integration
5. **Performance Baselines**: Track performance over time
6. **Load Test Reports**: HTML/PDF report generation
