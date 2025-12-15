# Coverage Maintenance Guide

This guide describes how to maintain test coverage across the MockForge codebase.

## Regular Maintenance Tasks

### Weekly Tasks

1. **Generate Coverage Report**
   ```bash
   make test-coverage-baseline
   ```

2. **Review Coverage Summary**
   ```bash
   make test-coverage-summary
   # Or view directly
   cat coverage/summary.txt
   ```

3. **Identify Coverage Gaps**
   - Review crates below threshold
   - Check for new untested code
   - Identify regressions

### Monthly Tasks

1. **Coverage Trend Analysis**
   - Compare current coverage with previous month
   - Identify crates with declining coverage
   - Set improvement goals

2. **Update Coverage Thresholds**
   - Review `coverage.toml` for threshold adjustments
   - Update thresholds based on crate maturity
   - Document threshold changes

3. **Coverage Improvement Planning**
   - Prioritize crates for improvement
   - Assign coverage improvement tasks
   - Track progress

### Quarterly Tasks

1. **Coverage Review Meeting**
   - Review overall coverage trends
   - Discuss coverage goals
   - Plan coverage improvements

2. **Enforcement Mode Review**
   - Review current enforcement mode in `coverage.toml`
   - Consider moving to next phase (report_only → warn → enforce)
   - Update enforcement settings

## Coverage Workflow

### For New Code

1. **Write Tests First** (TDD approach)
   - Write tests before implementation
   - Ensure tests cover all code paths
   - Include error case tests

2. **Verify Coverage**
   ```bash
   # Generate coverage for specific crate
   cargo llvm-cov --package mockforge-{name} --all-features --lcov --output-path coverage/{name}.lcov.info
   ```

3. **Check Coverage Threshold**
   - Ensure new code meets crate threshold
   - Review coverage report for gaps
   - Add tests for uncovered code

### For Existing Code

1. **Identify Low Coverage Areas**
   ```bash
   # View crates below threshold
   cat coverage/summary.txt | grep -A 20 "Crates Below Threshold"
   ```

2. **Analyze Coverage Gaps**
   - Review HTML coverage reports
   - Identify untested functions
   - Prioritize by user impact

3. **Add Missing Tests**
   - Write unit tests for uncovered functions
   - Add integration tests for workflows
   - Include error handling tests

4. **Verify Improvements**
   ```bash
   # Re-run coverage for improved crate
   cargo llvm-cov --package mockforge-{name} --all-features
   ```

## Coverage Monitoring

### Automated Monitoring

Coverage is automatically monitored in CI:

- **PR Coverage**: Coverage reports generated for all PRs
- **Coverage Comments**: PR comments with coverage summary
- **Coverage Artifacts**: Coverage reports uploaded as artifacts

### Manual Monitoring

1. **Local Coverage Generation**
   ```bash
   make test-coverage-baseline
   ```

2. **Coverage Comparison**
   ```bash
   # Compare with previous baseline
   diff coverage/summary.txt coverage/history/previous-summary.txt
   ```

3. **Coverage Trends**
   - Track coverage over time
   - Identify regressions early
   - Set improvement goals

## Coverage Improvement Process

### Step 1: Identify Target Crate

Review coverage summary to identify crates below threshold:

```bash
cat coverage/summary.txt
```

### Step 2: Analyze Coverage Gaps

View detailed coverage for the crate:

```bash
# HTML report (if generated)
open coverage/crates/mockforge-{name}/index.html

# JSON report
cat coverage/crates/mockforge-{name}/coverage.json | jq .
```

### Step 3: Prioritize Improvements

Focus on:
1. High-impact, low-coverage areas
2. User-facing functionality
3. Error handling paths
4. Edge cases

### Step 4: Write Tests

Follow testing standards in [TESTING_STANDARDS.md](TESTING_STANDARDS.md):

- Use AAA pattern (Arrange-Act-Assert)
- Test error cases
- Test edge cases
- Use property-based testing where appropriate

### Step 5: Verify Coverage

Re-run coverage to verify improvements:

```bash
cargo llvm-cov --package mockforge-{name} --all-features
```

### Step 6: Update Baseline

Regenerate baseline to update coverage reports:

```bash
make test-coverage-baseline
```

## Coverage Thresholds

### Default Thresholds

- **Default**: 80%
- **High-Priority**: 85% (core, http, cli, sdk)
- **Protocol**: 75% (grpc, ws, graphql, kafka, mqtt, amqp, smtp, ftp, tcp)
- **Infrastructure**: 70-75% (observability, tracing, analytics)

### Updating Thresholds

Edit `coverage.toml` to update thresholds:

```toml
[coverage.crates]
mockforge-core = 85
mockforge-http = 85
# ... etc
```

## Coverage Enforcement

### Current Mode: `report_only`

Coverage is currently in reporting-only mode:

- Coverage reports are generated
- No enforcement or blocking
- Warnings are informational only

### Future Modes

1. **Warn Mode**: Coverage warnings in CI, no blocking
2. **Enforce Mode**: CI blocks PRs if coverage drops below threshold

To change enforcement mode, update `coverage.toml`:

```toml
[coverage]
enforcement = "warn"  # or "enforce"
```

## Troubleshooting

### Coverage Not Generating

**Issue**: Coverage script fails

**Solution**:
1. Ensure `cargo-llvm-cov` is installed: `cargo install cargo-llvm-cov`
2. Check crate has tests: `cargo test --package mockforge-{name}`
3. Verify crate compiles: `cargo build --package mockforge-{name}`

### Coverage Shows 0%

**Issue**: Crate shows 0% coverage

**Possible Causes**:
1. Crate has no tests
2. Tests don't compile
3. Coverage tool issue

**Solution**:
1. Check if crate has tests: `ls crates/mockforge-{name}/tests/`
2. Run tests manually: `cargo test --package mockforge-{name}`
3. Check test compilation: `cargo test --package mockforge-{name} --no-run`

### Coverage Reports Missing

**Issue**: Coverage reports not found

**Solution**:
1. Run baseline script: `make test-coverage-baseline`
2. Check output directory: `ls coverage/`
3. Verify script permissions: `chmod +x scripts/coverage-baseline.sh`

## Best Practices

1. **Regular Monitoring**: Check coverage weekly
2. **Early Detection**: Catch coverage regressions early
3. **Incremental Improvement**: Improve coverage gradually
4. **Focus on Quality**: Prioritize meaningful tests over coverage percentage
5. **Document Exceptions**: Document any justified exceptions to thresholds

## Resources

- [Testing Standards](TESTING_STANDARDS.md) - Testing guidelines and patterns
- [Coverage Configuration](../coverage.toml) - Coverage thresholds and settings
- [Coverage Dashboard](COVERAGE.md) - Current coverage status
