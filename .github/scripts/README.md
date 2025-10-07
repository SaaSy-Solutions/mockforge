# Performance Monitoring Scripts

This directory contains scripts for automated performance monitoring and benchmarking.

## Scripts

### `compare-benchmarks.js`

Compares benchmark results between PR and baseline, detects regressions, and generates reports.

**Usage:**

```bash
# Compare current results against baseline
node compare-benchmarks.js compare

# Save current results as new baseline
node compare-benchmarks.js save-baseline
```

**Environment Variables:**

- `REGRESSION_THRESHOLD` - Percentage threshold for regression detection (default: 5.0)
- `BASELINE_DIR` - Directory containing baseline.json (default: `.github/benchmarks`)
- `CRITERION_DIR` - Directory containing Criterion results (default: `target/criterion`)
- `REPORT_PATH` - Output path for markdown report (default: `benchmark-report.md`)

**Exit Codes:**

- `0` - No significant regressions detected
- `1` - Regressions exceeding threshold found

**Features:**

- Parses Criterion benchmark JSON output
- Compares against git-tracked baseline
- Generates detailed markdown reports
- Detects regressions, improvements, and new benchmarks
- Configurable threshold for alerts

### `generate-dashboard.js`

Generates an interactive HTML dashboard from benchmark results.

**Usage:**

```bash
# Generate dashboard
node generate-dashboard.js
```

**Environment Variables:**

- `CRITERION_DIR` - Directory containing Criterion results (default: `target/criterion`)
- `DASHBOARD_OUTPUT` - Output path for HTML file (default: `performance-dashboard.html`)
- `BASELINE_DIR` - Directory containing baseline.json (default: `.github/benchmarks`)

**Features:**

- Interactive HTML dashboard with Chart.js visualizations
- Performance bar charts
- Status distribution pie chart
- Detailed results table
- Responsive design with dark theme
- No external dependencies (self-contained HTML)

**Dashboard Components:**

1. **Summary Stats** - Total benchmarks, regressions, improvements, stable count
2. **Performance Chart** - Bar chart showing mean execution time for all benchmarks
3. **Status Distribution** - Pie chart showing breakdown by status (regression/improvement/stable/new)
4. **Detailed Table** - Sortable table with complete benchmark results

## Integration with CI/CD

These scripts are automatically executed by the GitHub Actions workflow (`.github/workflows/benchmarks.yml`):

### On Pull Requests:

1. Fetch baseline from main branch
2. Run benchmarks
3. `compare-benchmarks.js compare` - Compare and detect regressions
4. `generate-dashboard.js` - Create visual dashboard
5. Post results to PR as comment
6. Fail PR if regressions detected

### On Main Branch Push:

1. Run benchmarks
2. `compare-benchmarks.js save-baseline` - Save new baseline
3. Commit baseline.json to repository
4. `generate-dashboard.js` - Create dashboard
5. Upload as artifact

## Local Development

### Running Benchmarks Locally

```bash
# Run all benchmarks
cd crates/mockforge-core
cargo bench --bench core_benchmarks

# Compare against baseline
cd ../..
node .github/scripts/compare-benchmarks.js compare

# Generate dashboard
node .github/scripts/generate-dashboard.js
open performance-dashboard.html
```

### Testing Scripts

```bash
# Test comparison script
CRITERION_DIR=target/criterion node .github/scripts/compare-benchmarks.js compare

# Test dashboard generation
CRITERION_DIR=target/criterion node .github/scripts/generate-dashboard.js
```

## File Structure

```
.github/
├── benchmarks/
│   ├── baseline.json       # Git-tracked baseline results
│   └── config.json         # Configuration (thresholds, limits)
└── scripts/
    ├── compare-benchmarks.js   # Comparison and regression detection
    ├── generate-dashboard.js   # Dashboard generation
    └── README.md              # This file
```

## Configuration

See `.github/benchmarks/config.json` for:

- Regression/improvement thresholds
- Per-benchmark maximum execution time limits
- Alert settings
- Baseline storage location

## Troubleshooting

### "No baseline found" Warning

This is normal on first run or when baseline doesn't exist yet. Run benchmarks on main branch and save baseline:

```bash
node .github/scripts/compare-benchmarks.js save-baseline
```

### "Failed to parse" Warnings

Criterion generates many JSON files. Warnings about parsing some files are normal. The script looks for `benchmark.json` files specifically.

### Dashboard Shows No Data

Ensure benchmarks have been run and `target/criterion/` directory exists with results:

```bash
ls -la target/criterion/
```

## Dependencies

Both scripts use only Node.js built-in modules:

- `fs` - File system operations
- `path` - Path manipulation

The dashboard uses Chart.js loaded from CDN (no local installation required).

## Contributing

When adding new scripts:

1. Add documentation to this README
2. Update `.github/workflows/benchmarks.yml` if needed
3. Add appropriate error handling
4. Test locally before committing
5. Update `docs/PERFORMANCE_MONITORING.md` with usage info

## Related Documentation

- [Performance Monitoring Guide](../../docs/PERFORMANCE_MONITORING.md) - Complete guide to the performance monitoring system
- [Criterion.rs Book](https://bheisler.github.io/criterion.rs/book/) - Benchmarking framework documentation
