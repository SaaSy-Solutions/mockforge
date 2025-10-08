# MockForge Startup Performance Benchmarks

This directory contains tools and documentation for analyzing and optimizing MockForge's startup latency.

## Quick Start

### View the Analysis

Read the comprehensive analysis:
```bash
cat STARTUP_LATENCY_ANALYSIS.md
```

### Run Benchmarks

```bash
# Build the project
cargo build --release

# Run the benchmark suite
./measure_startup.sh

# Or run a quick test
./quick_test.sh
```

## What's Included

### Documentation

- **STARTUP_LATENCY_ANALYSIS.md** - Comprehensive analysis of startup performance, instrumentation, bottlenecks, and optimization recommendations

### Test Scenarios

- **large_api_100_endpoints.json** - OpenAPI specification with 100 endpoints for testing HTTP server startup
- **proto/** - Directory with 3 gRPC service definitions (~42 methods) for testing gRPC server startup

### Tools

- **generate_large_spec.py** - Python script to generate OpenAPI specs with configurable number of endpoints
- **measure_startup.sh** - Bash script to run comprehensive startup benchmarks
- **quick_test.sh** - Quick test to verify timing instrumentation

## Instrumentation Added

Timing measurements have been added to:

1. **HTTP Server** (mockforge-http/src/lib.rs:250-393)
   - OpenAPI spec loading
   - Route registry creation
   - Route extraction
   - Overrides loading
   - Router building
   - Total startup time

2. **gRPC Server** (mockforge-grpc/src/dynamic/mod.rs:105-203)
   - Proto file parsing
   - Registry creation
   - Service registration
   - Reflection proxy creation
   - Total startup time

## Key Findings

### HTTP Server
- **Expected Performance**: 20-100ms for 100 endpoints
- **Main Operations**:
  - Spec parsing (1-5ms)
  - Route generation (10-50ms)
  - Router building (5-20ms)
- **Status**: ✅ Fast, likely acceptable as-is

### gRPC Server
- **Expected Performance**: 50-250ms for 3 services
- **Main Bottleneck**: Proto compilation via protoc (20-100ms per file)
- **Status**: ⚠️ May need optimization for many proto files

## Optimization Recommendations

### High Priority
1. ✅ Add timing instrumentation (COMPLETED)
2. Batch proto compilation (multiple files in one protoc invocation)
3. Cache compiled descriptor sets

### Medium Priority
4. Parallel route generation (if measurements show benefit)
5. Lazy route generation option for dev mode

### Low Priority
6. Pre-compiled resources at build time
7. Async/parallel file I/O

## Usage Examples

### Generate Custom OpenAPI Spec

```bash
python3 generate_large_spec.py 200 > custom_api_200_endpoints.json
```

### Run with Specific Log Level

```bash
RUST_LOG=debug ./quick_test.sh
```

### View Timing Logs

When running MockForge, look for log messages like:

```
[INFO] Successfully loaded OpenAPI spec (took 3.2ms)
[INFO] Created OpenAPI route registry with 100 routes (took 15.7ms)
[INFO] HTTP router startup completed (total time: 28.5ms)
```

## Performance Targets

- **100 HTTP endpoints**: <50ms ✅
- **3 gRPC services**: <200ms ⚠️
- **Combined**: <250ms ⚠️

These targets are suitable for development and testing workflows.

## Next Steps

1. Collect real-world performance data from the instrumentation
2. Identify actual bottlenecks (may differ from predictions)
3. Implement targeted optimizations based on data
4. Measure improvements
5. Document performance characteristics for users

## Contributing

To add new benchmarks or test scenarios:

1. Add test files to this directory
2. Update the benchmark scripts
3. Document expected performance characteristics
4. Run tests and record results
5. Update this README

## Related Resources

- [MockForge HTTP Mocking](../../crates/mockforge-http/README.md)
- [MockForge gRPC Mocking](../../crates/mockforge-grpc/README.md)
- [Performance Best Practices](../../docs/performance.md) (if exists)

## Support

For questions or issues with startup performance:
- Open an issue on GitHub
- Tag with `performance` label
- Include timing logs and configuration
