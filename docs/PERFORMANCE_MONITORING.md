# Performance Monitoring Guide

This document describes the automated performance monitoring system for MockForge.

## Overview

MockForge uses an automated performance tracking system that:

- **Stores benchmark baselines in git** - Baseline results are committed to the repository for version control
- **Compares PR benchmarks against main** - Every pull request is benchmarked and compared against the main branch baseline
- **Automated alerts on >5% regression** - Pull requests fail CI if performance degrades by more than 5%
- **Performance dashboards** - Interactive HTML dashboards visualize benchmark results and trends

## Architecture

### Components

1. **Criterion Benchmarks** (`crates/mockforge-core/benches/core_benchmarks.rs`)
   - Core performance benchmarks using Criterion.rs
   - Covers critical operations: template rendering, JSON validation, OpenAPI parsing, data generation

2. **Baseline Storage** (`.github/benchmarks/baseline.json`)
   - Git-tracked baseline results from main branch
   - Automatically updated on main branch pushes
   - Used for comparison in pull requests

3. **Comparison Script** (`.github/scripts/compare-benchmarks.js`)
   - Parses Criterion benchmark results
   - Compares against baseline
   - Detects regressions/improvements
   - Generates detailed reports

4. **Dashboard Generator** (`.github/scripts/generate-dashboard.js`)
   - Creates interactive HTML dashboard
   - Visualizes performance metrics
   - Shows historical trends

5. **GitHub Actions Workflow** (`.github/workflows/benchmarks.yml`)
   - Runs on pull requests and main branch pushes
   - Orchestrates the entire monitoring pipeline
   - Posts results to pull requests

## Configuration

### Regression Threshold

The default regression threshold is **5%**. You can adjust this in:

- **Workflow**: `.github/workflows/benchmarks.yml` → `REGRESSION_THRESHOLD` environment variable
- **Config**: `.github/benchmarks/config.json` → `regression_threshold`

```json
{
  "regression_threshold": 5.0,
  "improvement_threshold": 5.0
}
```

### Per-Benchmark Limits

You can set maximum acceptable mean times for specific benchmarks in `.github/benchmarks/config.json`:

```json
{
  "benchmarks": {
    "template_rendering/simple": {
      "max_mean_ns": 50000,
      "description": "Simple template rendering"
    }
  }
}
```

## Usage

### Running Benchmarks Locally

```bash
# Run all benchmarks
cd crates/mockforge-core
cargo bench --bench core_benchmarks

# Run specific benchmark group
cargo bench --bench core_benchmarks -- template_rendering

# Run with baseline comparison
cargo bench --bench core_benchmarks -- --save-baseline current

# View HTML reports
open ../../target/criterion/report/index.html
```

### Comparing Against Baseline

```bash
# Compare current results against stored baseline
node .github/scripts/compare-benchmarks.js compare

# Save current results as new baseline
node .github/scripts/compare-benchmarks.js save-baseline
```

### Generating Dashboard

```bash
# Generate performance dashboard
node .github/scripts/generate-dashboard.js

# Open dashboard
open performance-dashboard.html
```

## CI/CD Integration

### Pull Request Workflow

1. **Trigger**: When a PR modifies Rust code or Cargo files
2. **Fetch Baseline**: Download baseline from main branch
3. **Run Benchmarks**: Execute all benchmarks on PR code
4. **Compare**: Compare results against baseline
5. **Report**: Post detailed comparison to PR as comment
6. **Alert**: Fail PR if regression >5% detected
7. **Upload**: Store results and dashboard as artifacts

### Main Branch Workflow

1. **Trigger**: When code is pushed to main
2. **Run Benchmarks**: Execute all benchmarks
3. **Update Baseline**: Save results as new baseline
4. **Commit**: Commit baseline.json to repository
5. **Upload**: Store dashboard as artifact

## Interpreting Results

### Benchmark Report

The PR comment includes:

```markdown
# 📊 Performance Benchmark Report

## Summary
- **Total Benchmarks**: 15
- **Regressions**: 0 ⚠️
- **Improvements**: 3 ✅
- **Stable**: 11 ➡️
- **New**: 1 🆕

## ⚠️ Performance Regressions
| Benchmark | Baseline | Current | Change | % Change |
|-----------|----------|---------|--------|----------|
| template_rendering/complex | 45.23 µs | 48.91 µs | +3.68 µs | **+8.14%** |
```

### Status Types

- **🆕 New**: Benchmark doesn't exist in baseline (new test)
- **➡️ Stable**: Performance within ±5% of baseline
- **✅ Improvement**: Performance improved by >5%
- **⚠️ Regression**: Performance degraded by >5%

### Dashboard

The interactive dashboard provides:

- **Summary Statistics**: Total benchmarks, regressions, improvements, stable
- **Performance Chart**: Bar chart of all benchmark mean times
- **Status Distribution**: Pie chart showing status breakdown
- **Detailed Table**: Sortable table with all benchmark details

## Best Practices

### Writing Benchmarks

1. **Use `black_box`**: Prevent compiler optimizations from skewing results
   ```rust
   b.iter(|| {
       let result = my_function(black_box(input));
       black_box(result)
   });
   ```

2. **Appropriate Sample Sizes**: Use `group.sample_size(N)` for expensive operations
   ```rust
   let mut group = c.benchmark_group("memory");
   group.sample_size(10); // Reduce for memory-intensive benchmarks
   ```

3. **Representative Workloads**: Test realistic scenarios, not micro-optimizations

4. **Consistent Setup**: Use `iter_with_setup` to exclude setup from measurements
   ```rust
   b.iter_with_setup(
       || create_test_data(),  // Setup (not measured)
       |data| process(data)     // Measured code
   );
   ```

### Performance Optimization Workflow

1. **Baseline**: Run benchmarks on main branch to establish baseline
2. **Optimize**: Make performance improvements
3. **Measure**: Run benchmarks to verify improvements
4. **Compare**: Use comparison script to see exact differences
5. **Review**: Check dashboard for regression in other areas
6. **Iterate**: Refine based on results

### Investigating Regressions

If a PR triggers a regression alert:

1. **Download Artifacts**: Get detailed Criterion reports from GitHub Actions
2. **Review HTML Reports**: Open `target/criterion/report/index.html`
3. **Check Dashboard**: Review performance-dashboard.html
4. **Analyze Code**: Identify changes that may cause regression
5. **Profile**: Use profiling tools (flamegraph, perf) if needed
6. **Fix or Justify**: Either fix the regression or document why it's acceptable

## Maintenance

### Updating Baselines

Baselines are automatically updated when code is merged to main. Manual updates should be rare, but you can:

```bash
# Run benchmarks
cargo bench --bench core_benchmarks

# Save as baseline
node .github/scripts/compare-benchmarks.js save-baseline

# Commit to repository
git add .github/benchmarks/baseline.json
git commit -m "chore: update benchmark baseline"
```

### Adding New Benchmarks

1. Add benchmark function to `crates/mockforge-core/benches/core_benchmarks.rs`
2. Add to criterion group in `criterion_group!` macro
3. (Optional) Add configuration to `.github/benchmarks/config.json`
4. Run benchmarks locally to verify
5. Merge to main - baseline will auto-update

### Troubleshooting

**Problem**: Benchmarks fail in CI but pass locally

- Check CPU/memory differences between local and CI
- Verify baseline is committed and up-to-date
- Review CI logs for specific errors

**Problem**: Consistent false positive regressions

- Adjust `REGRESSION_THRESHOLD` if too sensitive
- Check for non-deterministic code in benchmarks
- Increase sample size for more accurate measurements

**Problem**: Dashboard not generating

- Verify Node.js is available in CI
- Check for parsing errors in Criterion output
- Ensure `target/criterion/` directory exists

## Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Performance Testing Best Practices](https://rust-lang.github.io/rustc-guide/profiling.html)
- [GitHub Actions Artifacts](https://docs.github.com/en/actions/using-workflows/storing-workflow-data-as-artifacts)

## Future Enhancements

Potential improvements to the performance monitoring system:

- [ ] Historical trend tracking (store baselines over time)
- [ ] Performance regression bisection (identify exact commit)
- [ ] Memory profiling integration
- [ ] Continuous performance tracking dashboard (GitHub Pages)
- [ ] Slack/Discord notifications for regressions
- [ ] Benchmark against competitor tools
- [ ] Load testing integration
- [ ] Automated performance reports in releases
