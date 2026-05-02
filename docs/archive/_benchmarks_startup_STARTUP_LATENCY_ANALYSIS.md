# MockForge Startup Latency Analysis

## Overview

This document analyzes MockForge's startup latency and provides instrumentation to measure and optimize startup performance.

## Instrumentation Added

### HTTP Server Startup (mockforge-http/src/lib.rs)

Added timing measurements for:

1. **OpenAPI Spec Loading** - Time to read and parse the OpenAPI specification file (JSON/YAML)
2. **Route Registry Creation** - Time to create the OpenAPI route registry and generate all routes
3. **Route Extraction** - Time to extract route information for introspection
4. **Overrides Loading** - Time to load override rules (if configured)
5. **Router Building** - Time to build the Axum router with all endpoints
6. **Total Startup Time** - Total time for HTTP router initialization

### gRPC Server Startup (mockforge-grpc/src/dynamic/mod.rs)

Added timing measurements for:

1. **Proto File Parsing** - Time to discover and parse all .proto files using protoc
2. **Registry Creation** - Time to create the service registry with descriptor pool
3. **Service Registration** - Time to register all discovered services
4. **Reflection Proxy Creation** - Time to create the gRPC reflection proxy
5. **Total Discovery Time** - Total time for service discovery
6. **Total Startup Time** - Total time for gRPC server initialization

## Test Scenarios Created

### 1. Large OpenAPI Specification

**File**: `benchmarks/startup/large_api_100_endpoints.json`

- **Endpoints**: 100 HTTP endpoints (20 paths with 5 methods each: GET, POST, PUT, DELETE)
- **Categories**: 10 resource categories (users, products, orders, customers, invoices, payments, shipments, inventory, analytics, reports)
- **Operations**: Full CRUD operations with parameters, request bodies, and response schemas
- **Schemas**: 10 component schemas with realistic properties

**Generated with**: `benchmarks/startup/generate_large_spec.py`

### 2. Multiple Proto Files

**Directory**: `benchmarks/startup/proto/`

Three comprehensive proto service definitions:

1. **user_service.proto**
   - 14 unary methods (GetUser, CreateUser, UpdateUser, etc.)
   - 2 server streaming methods
   - 1 client streaming method
   - 1 bidirectional streaming method
   - Total: ~18 methods

2. **product_service.proto**
   - 10 unary methods (GetProduct, CreateProduct, inventory management, etc.)
   - 2 server streaming methods
   - Total: ~12 methods

3. **order_service.proto**
   - 10 unary methods (CreateOrder, GetOrder, payment processing, etc.)
   - 2 server streaming methods
   - Total: ~12 methods

**Combined**: 3 services with ~42 methods total

## Expected Performance Characteristics

### HTTP Server Startup

Based on the implementation:

1. **OpenAPI Spec Loading**: O(n) where n = file size
   - JSON/YAML parsing using serde
   - For 100 endpoints: ~43KB file
   - Expected: 1-5ms on modern hardware

2. **Route Registry Creation**: O(m) where m = number of operations
   - Iterates through all paths and operations
   - Generates route handlers for each
   - Compiles regex patterns for path matching
   - For 100 endpoints: Expected 10-50ms

3. **Route Extraction**: O(m)
   - Simple iteration and cloning
   - For 100 endpoints: Expected 1-5ms

4. **Router Building**: O(m)
   - Axum router construction
   - For 100 endpoints: Expected 5-20ms

**Total HTTP Estimated**: 20-100ms for 100 endpoints

### gRPC Server Startup

1. **Proto File Discovery**: O(f) where f = number of files
   - Recursive filesystem scan
   - For 3 files: Expected <1ms

2. **Proto File Parsing**: O(f * c) where c = complexity per file
   - Executes `protoc` for each file
   - Spawns subprocess, compiles proto
   - This is the **slowest operation**
   - For 3 files: Expected 50-200ms (depends on protoc availability)

3. **Service Registration**: O(s) where s = number of services
   - For 3 services: Expected 1-5ms

4. **Reflection Service**: O(s)
   - Encodes descriptor pool
   - For 3 services: Expected 1-10ms

**Total gRPC Estimated**: 50-250ms for 3 services (dominated by protoc compilation)

## Performance Bottlenecks Identified

### 1. Proto File Compilation (gRPC)

**Issue**: Each proto file requires spawning `protoc` subprocess

**Impact**: HIGH - Can add 20-100ms per file depending on system

**Mitigation Options**:
- Cache compiled descriptor sets
- Batch compile multiple files in single protoc invocation
- Use pre-compiled descriptor sets at build time
- Lazy loading: defer compilation until service is first accessed

### 2. Route Registry Creation (HTTP)

**Issue**: Synchronous iteration through all operations with regex compilation

**Impact**: MEDIUM - Linear with number of endpoints

**Mitigation Options**:
- Parallel route generation using rayon
- Lazy route generation (generate on first request)
- Pre-compile regex patterns at build time
- Cache compiled route registries

### 3. Overrides Loading (HTTP)

**Issue**: Glob pattern matching and file system scans

**Impact**: LOW-MEDIUM - Only when `MOCKFORGE_HTTP_OVERRIDES_GLOB` is set

**Mitigation Options**:
- Async/parallel file loading
- Watch mode with hot reload instead of startup load
- Lazy loading with on-demand compilation

## Optimization Recommendations

### High Priority (Quick Wins)

1. **Log Startup Metrics**
   - ✅ DONE: Added timing instrumentation
   - Enables data-driven optimization decisions

2. **Batch Proto Compilation**
   ```rust
   // Instead of:
   for proto_file in proto_files {
       compile_proto(proto_file);
   }

   // Do:
   compile_protos_batch(&proto_files);
   ```

3. **Parallel Route Generation** (if beneficial)
   ```rust
   use rayon::prelude::*;

   let routes: Vec<_> = paths.par_iter()
       .flat_map(|path| generate_routes(path))
       .collect();
   ```

### Medium Priority

4. **Descriptor Set Caching**
   - Cache compiled proto descriptor sets
   - Store with hash of source files
   - Regenerate only when protos change

5. **Lazy Route Generation**
   - Generate routes on first request
   - Trade startup time for first-request latency
   - Suitable for development mode

### Low Priority (Optimization After Measurement)

6. **Pre-compiled Resources**
   - Build-time OpenAPI route generation
   - Build-time proto compilation
   - Requires code generation step

7. **Async/Parallel File I/O**
   - Use tokio::fs for parallel spec/proto loading
   - Benefit minimal for single files

## Benchmarking Scripts

### Quick Test

```bash
./benchmarks/startup/quick_test.sh
```

Builds and runs MockForge with the large OpenAPI spec, showing timing breakdown.

### Full Benchmark Suite

```bash
./benchmarks/startup/measure_startup.sh
```

Runs four test scenarios:
1. Baseline (no specs)
2. Large OpenAPI spec (100 endpoints)
3. gRPC with multiple protos (3 services, ~42 methods)
4. Combined (HTTP + gRPC)

## Example Output

With the instrumentation added, you'll see logs like:

```
[INFO] Successfully loaded OpenAPI spec from benchmarks/startup/large_api_100_endpoints.json (took 3.2ms)
[INFO] Created OpenAPI route registry with 100 routes (took 15.7ms)
[DEBUG] Extracted route information (took 1.1ms)
[DEBUG] Built OpenAPI router (took 8.3ms)
[INFO] HTTP router startup completed (total time: 28.5ms)
```

For gRPC:

```
[INFO] Discovering gRPC services from proto directory: benchmarks/startup/proto
[INFO] Found 3 proto files
[INFO] Proto file parsing completed (took 127.4ms)
[DEBUG] Registry creation completed (took 2.1ms)
[INFO] Service registration completed for 3 services (took 3.8ms)
[INFO] Service discovery completed (total time: 133.5ms)
[INFO] gRPC reflection proxy created (took 5.2ms)
[INFO] gRPC server startup completed (total time: 138.9ms)
```

## Acceptable Performance Targets

Based on typical use cases:

- **Development Mode**: <500ms total startup acceptable
- **CI/CD Testing**: <1s acceptable
- **Production**: Not critical (one-time cost at deploy)

For the test scenarios:
- 100 HTTP endpoints: **Target <50ms** ✅ (easily achievable with current implementation)
- 3 gRPC services: **Target <200ms** ⚠️ (depends on protoc performance)
- Combined: **Target <250ms** ⚠️

## Rust Performance Advantages

MockForge benefits from Rust's performance characteristics:

1. **Zero-cost abstractions**: No runtime overhead for generics/traits
2. **Efficient serialization**: serde is highly optimized
3. **No GC pauses**: Predictable performance
4. **Parallel compilation**: Cargo builds dependencies in parallel

## Next Steps

1. ✅ Add timing instrumentation (COMPLETED)
2. ✅ Create test scenarios (COMPLETED)
3. ⏳ Run benchmarks and collect data (IN PROGRESS)
4. ⏳ Identify actual bottlenecks from data (IN PROGRESS)
5. ⏳ Implement high-priority optimizations (PENDING)
6. Test performance improvements
7. Document findings and recommendations

## Conclusion

The timing instrumentation added to MockForge provides visibility into startup performance. The test scenarios enable measurement of realistic workloads. Initial analysis suggests:

- **HTTP startup is fast** (likely <50ms for 100 endpoints)
- **gRPC startup may be slower** due to protoc compilation (50-200ms)
- **Optimization opportunities exist** but should be data-driven

The current implementation is likely acceptable for most use cases. Any optimizations should be made only after confirming actual performance issues with real measurements.
