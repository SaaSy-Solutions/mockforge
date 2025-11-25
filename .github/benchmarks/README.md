# Benchmark Profiling Guide

This directory contains benchmark configuration and baseline data for MockForge performance testing.

## Files

- `baseline.json`: Baseline performance measurements for regression detection
- `config.json`: Benchmark configuration including thresholds and alerts

## Running Benchmarks

### Basic Benchmark Execution

```bash
# Run all benchmarks
cargo bench --bench core_benchmarks

# Run specific benchmark groups
cargo bench --bench core_benchmarks json_validation
cargo bench --bench core_benchmarks openapi_parsing
cargo bench --bench core_benchmarks memory

# Run with more iterations for stable results
cargo bench --bench core_benchmarks -- --sample-size 100
```

### Profiling with Criterion

Criterion generates HTML reports automatically. After running benchmarks, view reports at:
```
target/criterion/<benchmark-name>/report/index.html
```

### CPU Profiling with perf

For detailed CPU profiling to identify hot paths:

```bash
# Install perf (if not already available)
# On Arch: sudo pacman -S perf
# On Ubuntu: sudo apt-get install linux-perf

# Profile a specific benchmark
perf record --call-graph=dwarf cargo bench --bench core_benchmarks json_validation
perf report

# Generate flamegraph
perf script | stackcollapse-perf.pl | flamegraph.pl > flamegraph.svg
```

### Memory Profiling

For memory allocation analysis:

```bash
# Using valgrind (if available)
valgrind --tool=massif cargo bench --bench core_benchmarks memory

# Using heaptrack (if installed)
heaptrack cargo bench --bench core_benchmarks memory
```

### Comparing Before/After

To compare performance before and after changes:

```bash
# Checkout baseline commit
git checkout ab52b510^

# Run benchmarks and save results
cargo bench --bench core_benchmarks > baseline_results.txt

# Checkout current commit
git checkout main

# Run benchmarks again
cargo bench --bench core_benchmarks > current_results.txt

# Compare
diff baseline_results.txt current_results.txt
```

## Benchmark Configuration

The `config.json` file defines:
- `regression_threshold`: Percentage change that triggers regression alerts (default: 5.0%)
- `improvement_threshold`: Percentage change that triggers improvement alerts (default: 5.0%)
- `baseline_storage`: Path to baseline JSON file
- Individual benchmark limits and descriptions

## Regression Detection

The CI workflow automatically:
1. Runs benchmarks on each commit
2. Compares results against `baseline.json`
3. Updates baseline if improvements are detected
4. Alerts on regressions exceeding the threshold

## Manual Baseline Update

After fixing performance regressions:

```bash
# Run benchmarks
cargo bench --bench core_benchmarks

# Update baseline.json with new results
# (This is typically done automatically by CI)
```

## Profiling Tips

1. **Focus on regressed benchmarks**: Start with benchmarks showing >5% regression
2. **Use flamegraphs**: Visual representation helps identify hot paths quickly
3. **Profile in release mode**: Benchmarks run in release mode by default
4. **Multiple runs**: Run benchmarks multiple times to account for system variance
5. **Isolate changes**: Profile before and after specific commits to identify cause

## Common Performance Issues

- **Schema recompilation**: JSON schemas being recompiled on every validation
- **Unnecessary cloning**: Data structures being cloned when references would suffice
- **Validation overhead**: Excessive validation in hot paths
- **Memory allocations**: Frequent allocations in tight loops
- **Cache misses**: Poor data locality or missing caches
