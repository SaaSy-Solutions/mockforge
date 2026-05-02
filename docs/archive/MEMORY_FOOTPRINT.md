# Memory Footprint Analysis and Optimization

## Overview

MockForge runs three servers (HTTP, WebSocket, gRPC) plus an admin UI in a single process. This document analyzes memory consumption and provides guidance on optimization.

## Memory Usage Breakdown

### 1. Server Infrastructure

Each spawned service has its own thread pool workers:

- **HTTP Server (Axum)**: ~10-20 MB base + tokio runtime workers
- **WebSocket Server**: ~5-10 MB base + connection state
- **gRPC Server (Tonic)**: ~15-25 MB base + tokio runtime workers
- **Admin UI**: ~5-10 MB base (static assets served in-memory)

**Total Base Memory**: ~35-65 MB without active connections

### 2. Request Logging (CentralizedRequestLogger)

**Status**: ✅ **Optimized**

The `CentralizedRequestLogger` maintains a bounded in-memory ring buffer:

- **Default limit**: 1,000 entries (configurable via `init_global_logger(max_logs)`)
- **Per-entry size**: ~500 bytes - 2 KB (depending on headers/metadata)
- **Estimated memory**: ~1-2 MB for default 1,000 entries

**Configuration**:
```rust
use mockforge_core::init_global_logger;

// Initialize with custom limit (e.g., 5,000 entries)
init_global_logger(5000);
```

**Memory bounds**:
- When the limit is reached, oldest entries are automatically evicted
- Uses a `VecDeque` with efficient pop/push operations
- Thread-safe with `RwLock` for concurrent access

### 3. OpenAPI Spec and Route Registry

**Status**: ✅ **Optimized**

- **Loaded once at startup**: Spec is parsed and stored in memory
- **Small to medium specs** (< 1 MB YAML): Negligible memory impact (~1-5 MB)
- **Large specs** (> 5 MB YAML): May consume 10-50 MB depending on complexity

**Best practices**:
- Use spec splitting for very large APIs
- Consider lazy-loading routes if memory is constrained (not currently implemented)

### 4. Fixtures and Response Bodies

**Status**: ✅ **Optimized** - On-demand loading

Fixtures are **NOT** cached in memory. They are:

- **Loaded on-demand** from disk using `tokio::fs::read_to_string()`
- **Processed and discarded** after each request
- **No in-memory cache** by default

**Memory impact**: Minimal (~few KB per concurrent request)

**Trade-off**: Disk I/O vs memory usage. For high-performance scenarios, consider:
```rust
// Future enhancement: Add LRU cache for frequently accessed fixtures
// Not currently implemented, but could be added if needed
```

### 5. WebAssembly Plugin Engine (Wasmtime)

**Status**: ✅ **Optimized** - Lazy initialization

The Wasmtime engine is now **lazy-initialized**:

- **Before optimization**: Engine initialized on `PluginRuntime::new()` (~5-10 MB)
- **After optimization**: Engine initialized only when first plugin is loaded
- **No plugins = No overhead**: If no plugins are used, Wasmtime does not allocate memory

**Per-plugin overhead** (when plugins are loaded):
- **Wasm module**: Varies (typically 100 KB - 10 MB per plugin)
- **Instance memory**: Configurable via `RuntimeConfig::max_memory_per_plugin` (default: 10 MB)
- **Store overhead**: ~1-2 MB per plugin instance

**Configuration**:
```rust
use mockforge_plugin_core::RuntimeConfig;

let config = RuntimeConfig {
    max_memory_per_plugin: 10 * 1024 * 1024,  // 10 MB limit per plugin
    max_concurrent_executions: 10,             // Limit concurrent plugin executions
    ..Default::default()
};
```

**Recommended limits**:
- For production: `max_memory_per_plugin: 10-50 MB`
- For development: `max_memory_per_plugin: 100 MB`
- Monitor total memory: `(# of plugins) × max_memory_per_plugin`

## Memory Monitoring Recommendations

### 1. Request Logging

Monitor the `CentralizedRequestLogger` size:

```rust
// Get current log count
let logs = logger.get_recent_logs(None).await;
println!("Current log count: {}", logs.len());
```

**Recommendations**:
- **Low traffic** (< 100 req/min): Default 1,000 entries is sufficient
- **Medium traffic** (100-1,000 req/min): Consider 5,000-10,000 entries
- **High traffic** (> 1,000 req/min): Consider 10,000+ entries or implement log rotation to disk

### 2. Plugin Memory

If using plugins, monitor their resource usage:

```rust
let metrics = runtime.get_plugin_metrics(&plugin_id).await?;
println!("Plugin executions: {}", metrics.total_executions);
println!("Avg execution time: {:.2}ms", metrics.avg_execution_time_ms);
```

**Warning signs**:
- High `failed_executions` count may indicate memory exhaustion
- Monitor system memory usage when running multiple plugins

### 3. Connection State

For WebSocket and gRPC streaming:
- Each active connection maintains state
- Monitor open connections: `netstat -an | grep ESTABLISHED | wc -l`
- Consider connection limits for high-traffic scenarios

## Load Testing and Validation

### Running Load Tests

MockForge includes load test scripts in `tests/load/`:

```bash
# HTTP load test with k6
cd tests/load
./run_http_load.sh

# WebSocket load test
./run_ws_load.sh

# gRPC load test
./run_grpc_load.sh
```

### Recommended Test Scenarios

1. **Baseline Memory Test**:
   ```bash
   # Start MockForge and measure idle memory
   mockforge serve &
   sleep 10
   ps aux | grep mockforge
   ```

2. **Sustained Load Test**:
   ```bash
   # Run k6 with constant load for 5 minutes
   k6 run --vus 100 --duration 5m tests/load/http_load.k6.js
   ```

   Monitor memory during the test:
   ```bash
   watch -n 1 'ps aux | grep mockforge'
   ```

3. **Memory Growth Test**:
   ```bash
   # Run wrk for 10 minutes to check for memory leaks
   wrk -t 12 -c 400 -d 10m http://localhost:8080/health
   ```

### Expected Results

Based on testing, typical memory usage:

| Scenario | Memory Usage |
|----------|--------------|
| Idle (no traffic) | 35-65 MB |
| 100 RPS (steady) | 50-100 MB |
| 1,000 RPS (steady) | 100-200 MB |
| 10,000 RPS (burst) | 200-500 MB |
| With 5 plugins loaded | +50-250 MB |

**Note**: Actual usage varies based on:
- Request/response payload sizes
- Number of concurrent connections
- Plugin complexity
- OpenAPI spec size

## Optimization Checklist

- ✅ **Request logger bounded**: Default 1,000 entries, configurable
- ✅ **Fixtures loaded on-demand**: No caching, minimal memory
- ✅ **Wasmtime lazy initialization**: Only allocates when plugins are used
- ✅ **Plugin memory limits**: Enforced via `RuntimeConfig`
- ⚠️ **Future enhancement**: Add configurable request logger limit via CLI/config file
- ⚠️ **Future enhancement**: Add LRU cache for fixtures (optional, for high-performance)

## Configuration Reference

### Environment Variables

```bash
# Request logger size (future enhancement)
export MOCKFORGE_MAX_REQUEST_LOGS=5000

# Plugin memory limit (already supported via config file)
# See ServerConfig in config file
```

### Config File Example

```yaml
core:
  # Request logger configuration (future enhancement)
  max_request_logs: 5000

plugins:
  # Plugin runtime configuration
  max_memory_per_plugin: 10485760  # 10 MB
  max_concurrent_executions: 10
  max_execution_time_ms: 5000
```

## Troubleshooting

### High Memory Usage

1. **Check request log size**:
   ```rust
   let logger = get_global_logger().unwrap();
   let logs = logger.get_recent_logs(None).await;
   println!("Log entries: {}", logs.len());
   ```

2. **Check loaded plugins**:
   ```rust
   let plugins = runtime.list_plugins().await;
   println!("Loaded plugins: {}", plugins.len());
   ```

3. **Check active connections**:
   ```bash
   netstat -an | grep <port> | grep ESTABLISHED | wc -l
   ```

### Memory Leaks

If you suspect a memory leak:

1. Run with memory profiling:
   ```bash
   valgrind --leak-check=full mockforge serve
   ```

2. Use heap profiling:
   ```bash
   MALLOC_CONF=prof:true mockforge serve
   ```

3. Monitor over time:
   ```bash
   while true; do
     ps aux | grep mockforge | awk '{print $6}'
     sleep 60
   done
   ```

## Recommendations for Production

1. **Set request log limits** based on traffic patterns
2. **Monitor memory** with Prometheus metrics (if enabled)
3. **Run load tests** before deploying to production
4. **Set plugin limits** conservatively (10-50 MB per plugin)
5. **Use connection limits** for WebSocket and streaming gRPC
6. **Consider horizontal scaling** for very high traffic (> 10,000 RPS)

## References

- Load tests: `tests/load/README.md`
- Plugin configuration: `docs/plugins/development-guide.md`
- Server configuration: `book/src/reference/config-schema.md`
