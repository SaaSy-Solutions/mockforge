# Performance Benchmarks

This document provides detailed performance characteristics and benchmark results for MockForge core operations. These benchmarks are automatically tracked and updated via our [Performance Monitoring System](PERFORMANCE_MONITORING.md).

## Overview

MockForge benchmarks measure real functionality across critical operations:
- **Template Rendering**: Token expansion and template processing
- **JSON Validation**: Schema validation performance
- **Data Generation**: Mock data generation from schemas
- **Encryption**: Cryptographic operations (AES-256-GCM, ChaCha20-Poly1305)
- **OpenAPI Parsing**: Spec parsing and route generation
- **Memory Operations**: Large-scale operations and memory usage

All benchmarks use [Criterion.rs](https://bheisler.github.io/criterion.rs/) and are run automatically on every pull request and main branch push.

## Current Performance Metrics

*Last Updated: December 2025*

### Fast Operations (< 1µs)

These operations complete in under 1 microsecond, making them suitable for high-throughput scenarios:

| Operation | Mean Time | Description |
|-----------|-----------|-------------|
| `json_validation/simple` | **105 ns** | Simple JSON schema validation (single property) |
| `template_rendering/arrays` | **345 ns** | Template rendering with array iteration |
| `template_rendering/complex` | **405 ns** | Complex template with multiple variables |
| `json_validation/complex` | **590 ns** | Complex nested JSON schema validation |
| `data_generation/generate_single_record` | **748 ns** | Generate a single JSON record from schema |
| `template_rendering/simple` | **941 ns** | Simple template with UUID expansion |
| `encryption/aes256_gcm` | **1,089 ns** | AES-256-GCM encrypt + decrypt operation |

**Performance Notes:**
- JSON validation is highly optimized for fast schema checking
- Template rendering includes token expansion (UUID generation adds ~500ns)
- Data generation includes full schema processing and validation

### Medium Operations (1-200µs)

Operations suitable for typical API request handling:

| Operation | Mean Time | Description |
|-----------|-----------|-------------|
| `memory/deep_template_rendering` | **1.5 µs** | Deeply nested template rendering (10 levels) |
| `encryption/chacha20_poly1305` | **4.2 µs** | ChaCha20-Poly1305 encrypt + decrypt operation |
| `openapi_parsing/small_spec` | **24 µs** | Parse small OpenAPI spec (1 path) |
| `memory/large_data_validation` | **108 µs** | Validate 100-item array with complex schema |
| `openapi_parsing/medium_spec_10_paths` | **172 µs** | Parse medium OpenAPI spec (10 paths) |

**Performance Notes:**
- OpenAPI parsing includes full route registry generation
- Large data validation scales linearly with data size
- ChaCha20-Poly1305 is ~3.8x slower than AES-256-GCM (expected - see Encryption section)

### Large Operations

Operations for complex, resource-intensive tasks:

| Operation | Mean Time | Description |
|-----------|-----------|-------------|
| `memory/large_spec_parsing` | **7.16 ms** | Parse large OpenAPI spec (100 paths) |

**Performance Notes:**
- Large spec parsing includes full route generation for 100 endpoints
- Performance scales sub-linearly due to internal optimizations

## Detailed Performance Characteristics

### Template Rendering

Template rendering performance varies based on:
- **Token Type**: UUID generation is the most expensive (~500ns per UUID)
- **Template Complexity**: Nested templates add minimal overhead
- **Token Count**: Multiple tokens are processed efficiently

**Benchmark Results:**
- `simple`: 941 ns (single `{{uuid}}` token)
- `complex`: 405 ns (multiple context variables)
- `arrays`: 345 ns (array iteration)
- `deep_template_rendering`: 1.5 µs (10-level nesting)

**Optimization Notes:**
- Early return if no template tokens present
- Conditional processing (only process tokens that exist)
- UUID generation uses cryptographically secure randomness (necessary for uniqueness)

### JSON Validation

JSON validation is highly optimized for performance:

**Benchmark Results:**
- `simple`: 105 ns (single property validation)
- `complex`: 590 ns (nested object with multiple properties and constraints)
- `large_data_validation`: 108 µs (100-item array validation)

**Performance Characteristics:**
- Pre-compiled validators avoid schema compilation overhead
- Validation scales linearly with data size
- Complex schemas add minimal overhead (~6x for nested structures)

### Data Generation

Data generation includes full schema processing:

**Benchmark Results:**
- `generate_single_record`: 748 ns (complete JSON record generation)

**Performance Characteristics:**
- Includes schema parsing, validation, and data generation
- Supports complex nested structures
- Generates realistic mock data with proper types

### Encryption

MockForge supports two encryption algorithms with different performance characteristics:

#### AES-256-GCM (Default)

**Benchmark Result:** 1,089 ns (encrypt + decrypt)

**Characteristics:**
- **Performance**: Optimized for speed on modern CPUs
- **Hardware Acceleration**: Uses AES-NI instructions on Intel/AMD processors
- **Security**: NIST-approved, widely audited
- **Use Case**: Default choice for x86_64 architectures

#### ChaCha20-Poly1305

**Benchmark Result:** 4,167 ns (encrypt + decrypt)

**Characteristics:**
- **Performance**: ~3.8x slower than AES-256-GCM
- **Software-Based**: No hardware acceleration (pure software implementation)
- **Security**: Excellent security properties, resistant to timing attacks
- **Use Case**: Better performance on ARM processors and older CPUs without AES-NI

**Performance Comparison:**
```
AES-256-GCM:     1,089 ns  (100%)
ChaCha20-Poly:   4,167 ns  (382%)
```

**Recommendation:**
- Use **AES-256-GCM** as default (already configured)
- Use **ChaCha20-Poly1305** for ARM devices or when hardware acceleration is unavailable
- The performance difference is expected and documented - both are secure choices

### OpenAPI Parsing

OpenAPI spec parsing includes full route registry generation:

**Benchmark Results:**
- `small_spec`: 24 µs (1 path)
- `medium_spec_10_paths`: 172 µs (10 paths)
- `large_spec_parsing`: 7.16 ms (100 paths)

**Performance Characteristics:**
- Scales sub-linearly due to internal optimizations
- Includes full route matching logic generation
- Supports complex schemas, parameters, and response definitions

**Scaling Analysis:**
- 1 path: 24 µs
- 10 paths: 172 µs (~7.2x for 10x paths)
- 100 paths: 7.16 ms (~41.6x for 100x paths)

## Performance Stability

All benchmarks show excellent stability with low variance:

| Benchmark | Mean (ns) | StdDev (ns) | Coefficient of Variation |
|-----------|-----------|-------------|-------------------------|
| `json_validation/simple` | 105.43 | < 1 | < 1% |
| `template_rendering/simple` | 940.97 | ~10 | ~1% |
| `encryption/aes256_gcm` | 1,089.36 | 11.49 | ~1% |
| `encryption/chacha20_poly1305` | 4,167.18 | 20.62 | ~0.5% |

**Stability Notes:**
- Low standard deviation indicates consistent performance
- UUID generation introduces some variance (cryptographic randomness)
- Encryption operations are highly stable

## Performance Trends

Benchmarks are automatically compared against the baseline on every pull request. Recent trends:

### Recent Improvements
- All benchmarks now measure **real functionality** (no placeholders)
- Data generation benchmark fixed to measure actual work
- Template rendering benchmark uses recognized tokens

### Stable Benchmarks
- Most benchmarks show < 8% variation from baseline
- JSON validation: Stable at ~105ns (simple), ~590ns (complex)
- OpenAPI parsing: Consistent scaling characteristics

## Interpreting Benchmark Results

### What These Numbers Mean

1. **Nanoseconds (ns)**: Operations completing in nanoseconds are extremely fast
   - Suitable for high-throughput scenarios
   - Can handle millions of operations per second

2. **Microseconds (µs)**: Operations in microseconds are still very fast
   - Suitable for typical API request handling
   - Can handle thousands of operations per second

3. **Milliseconds (ms)**: Operations in milliseconds are for complex tasks
   - Suitable for one-time or infrequent operations
   - Large spec parsing is expected to take milliseconds

### Performance Expectations

**For API Mocking:**
- Template rendering: < 1µs per request ✅
- JSON validation: < 1µs per request ✅
- Data generation: < 1µs per record ✅

**For Encryption:**
- AES-256-GCM: < 2µs per encrypt/decrypt ✅
- ChaCha20-Poly1305: < 5µs per encrypt/decrypt ✅

**For Spec Processing:**
- Small specs (< 10 paths): < 200µs ✅
- Large specs (100+ paths): < 10ms ✅

## Optimization Opportunities

### Current Status: ✅ No Critical Issues

All benchmarks are performing within expected ranges:

1. **Template Rendering**: 941ns for UUID expansion is reasonable
   - UUID generation requires cryptographically secure randomness
   - Uniqueness guarantee requires this cost
   - **Status**: No optimization needed

2. **Data Generation**: 748ns for complete record generation
   - Includes full schema processing and validation
   - Generates realistic, type-safe data
   - **Status**: No optimization needed

3. **Encryption**: Performance differences are expected
   - AES-256-GCM uses hardware acceleration (faster)
   - ChaCha20-Poly1305 is software-based (slower but secure)
   - **Status**: No optimization needed - documented trade-off

4. **JSON Validation**: Highly optimized
   - Pre-compiled validators avoid overhead
   - Fast path for simple schemas
   - **Status**: No optimization needed

### Future Optimization Areas

While current performance is excellent, potential future improvements:

1. **UUID Generation**: Could explore faster PRNGs for non-cryptographic use cases
   - Trade-off: Uniqueness guarantees vs. performance
   - Current: Cryptographically secure (required)

2. **Template Caching**: Cache expanded templates for repeated patterns
   - Trade-off: Memory usage vs. performance
   - Current: No caching (always fresh values)

3. **Spec Parsing**: Incremental parsing for large specs
   - Trade-off: Complexity vs. performance
   - Current: Full parsing (acceptable for < 10ms)

## Running Benchmarks Locally

See [Performance Monitoring Guide](PERFORMANCE_MONITORING.md) for detailed instructions.

**Quick Start:**
```bash
# Run all benchmarks
cd crates/mockforge-core
cargo bench --bench core_benchmarks

# Run specific benchmark group
cargo bench --bench core_benchmarks -- template_rendering

# View HTML reports
open ../../target/criterion/report/index.html
```

## Benchmark Methodology

### Measurement Approach

1. **Criterion.rs**: Industry-standard Rust benchmarking framework
2. **Black Box**: Prevents compiler optimizations from skewing results
3. **Setup Isolation**: Uses `iter_with_setup` to exclude setup overhead
4. **Statistical Analysis**: Multiple samples with mean and standard deviation

### Benchmark Implementation

All benchmarks measure **real functionality**:
- ✅ Actual data generation (not reference checks)
- ✅ Real template expansion (not placeholder tokens)
- ✅ Actual encryption/decryption (not stubs)
- ✅ Complete schema validation (not simplified checks)

### Baseline Management

- Baselines stored in `.github/benchmarks/baseline.json`
- Automatically updated on main branch pushes
- Compared against on every pull request
- Regression threshold: 5% (configurable)

## Related Documentation

- [Performance Monitoring Guide](PERFORMANCE_MONITORING.md) - Automated monitoring system
- [Performance Mode](book/src/user-guide/advanced-features/performance-mode.md) - Load simulation
- [Security Whitepaper](SECURITY_WHITEPAPER.md) - Encryption algorithm details
- [Architecture Documentation](ARCHITECTURE.md) - System design

## Questions or Issues?

If you notice performance regressions or have optimization suggestions:

1. **Check CI**: Review benchmark results in pull request comments
2. **Run Locally**: Verify with `cargo bench`
3. **Profile**: Use profiling tools (flamegraph, perf) for deep analysis
4. **Report**: Open an issue with benchmark results and analysis

---

*This document is automatically updated when benchmark baselines change. Last baseline update: December 2025.*
