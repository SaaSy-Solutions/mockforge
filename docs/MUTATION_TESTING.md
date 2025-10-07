# Mutation Testing Guide

## Overview

Mutation testing is a technique to verify the quality of your test suite by introducing small changes (mutations) to the code and checking if the tests catch them. MockForge uses [cargo-mutants](https://mutants.rs/) for mutation testing.

## Setup

The project is already configured for mutation testing. The configuration is in `.cargo-mutants.toml`.

### Installation

If you haven't already installed cargo-mutants:

```bash
cargo install cargo-mutants
```

## Running Mutation Tests

### Quick Commands (via Makefile)

```bash
# Check if tests pass before mutation testing
make test-mutants-check

# Run mutation testing on all crates (WARNING: This can take hours!)
make test-mutants

# Run mutation testing only on changes since last commit
make test-mutants-diff

# Generate a mutation testing report
make test-mutants-report

# Run mutation testing on core crate only
make test-mutants-core
```

### Direct cargo Commands

```bash
# Check if unmutated tests pass
cargo mutants --check

# Run mutation testing on a specific package
cargo mutants -p mockforge-core

# Run mutation testing on uncommitted changes only
cargo mutants --in-diff HEAD

# Run mutation testing on specific files
cargo mutants --file src/template/engine.rs

# List all mutants without running tests
cargo mutants --list

# Run with different timeout multiplier
cargo mutants --timeout-multiplier 10

# Continue from where you left off after interruption
cargo mutants --workspace --continue
```

## Understanding Results

### Mutation Outcomes

- **Caught**: The test suite detected the mutation (GOOD!)
- **Missed**: No test failed when the code was mutated (BAD - indicates a gap in testing)
- **Timeout**: Tests took too long with the mutation (possibly indicates an infinite loop)
- **Unviable**: The mutated code doesn't compile (SKIP)

### Example Output

```
Found 3008 mutants to test
ok       Unmutated baseline in 100.3s check
MISSED   src/core/matcher.rs:45: replace == with != in match_request
CAUGHT   src/core/validator.rs:78: replace Some with None
TIMEOUT  src/engine/executor.rs:123: remove return statement
```

### Interpreting Scores

A good mutation score is typically:
- **90%+**: Excellent test coverage
- **75-90%**: Good test coverage
- **50-75%**: Moderate coverage, room for improvement
- **<50%**: Poor coverage, significant testing gaps

## Configuration

The `.cargo-mutants.toml` file configures:

- **Test tool**: Uses nextest for faster execution
- **Timeouts**: 5x multiplier for test timeouts
- **Exclusions**: Skips examples, UI code, and test files
- **Skip patterns**: Ignores logging calls, trivial constructors, etc.

### Customizing Configuration

Edit `.cargo-mutants.toml` to:

```toml
# Increase timeout for slow tests
timeout_multiplier = 10.0

# Exclude additional directories
exclude_globs = [
    "examples/**",
    "benches/**",
]

# Skip additional function calls
skip_calls_to = [
    "::debug",
    "::trace",
]
```

## Best Practices

### 1. Start Small

Don't run mutation testing on the entire workspace initially. Start with:

```bash
# Test a single file
cargo mutants --file src/core/matcher.rs

# Test a single package
cargo mutants -p mockforge-core
```

### 2. Incremental Testing

Use diff-based testing during development:

```bash
# Only test changed code
cargo mutants --in-diff HEAD
```

### 3. CI Integration

For continuous integration, use:

```bash
# Generate JSON report for CI
cargo mutants --output mutants-report.json --json

# Only test changes in PR
cargo mutants --in-diff origin/main
```

### 4. Time Management

Mutation testing is slow. Strategies:

- Run on specific packages during development
- Run full suite nightly in CI
- Use `--timeout-multiplier` to speed up at cost of accuracy
- Use `--continue` to resume interrupted runs

## Improving Missed Mutants

When mutants are missed, add tests that:

1. **Test edge cases**: Boundary conditions, empty inputs, null values
2. **Test error paths**: What happens when things go wrong?
3. **Test return values**: Verify exact values, not just that it doesn't panic
4. **Test side effects**: Verify state changes, not just return values

### Example

If this mutant is missed:

```rust
// Original
fn validate_port(port: u16) -> bool {
    port > 0 && port < 65536  // MISSED: replace && with ||
}
```

Add a test:

```rust
#[test]
fn test_validate_port_rejects_invalid() {
    assert!(!validate_port(0));       // Tests lower bound
    assert!(!validate_port(65536));   // Tests upper bound (impossible but good practice)
    assert!(validate_port(8080));     // Tests valid case
}
```

## Performance Tips

1. **Use nextest**: Already configured, faster test execution
2. **Limit scope**: Use `-p` for specific packages
3. **Parallel execution**: cargo-mutants runs tests in parallel automatically
4. **Incremental runs**: Use `--continue` after interruptions
5. **Filter mutants**: Use `--file` to focus on specific areas

## Troubleshooting

### Tests timeout during mutation testing

Increase the timeout multiplier:

```bash
cargo mutants --timeout-multiplier 10
```

Or in `.cargo-mutants.toml`:

```toml
timeout_multiplier = 10.0
minimum_test_timeout = 60
```

### Too many mutants

Focus testing:

```bash
# Test only changed files
cargo mutants --in-diff HEAD

# Test specific package
cargo mutants -p mockforge-core

# Test specific file
cargo mutants --file src/important.rs
```

### Build failures

Run a clean check first:

```bash
cargo mutants --check
```

## Resources

- [cargo-mutants documentation](https://mutants.rs/)
- [Mutation testing overview](https://en.wikipedia.org/wiki/Mutation_testing)
- [cargo-mutants GitHub](https://github.com/sourcefrog/cargo-mutants)

## Integration with Other Tools

### With Coverage Tools

Mutation testing complements code coverage:

```bash
# First check coverage
make test-coverage

# Then check mutation score
make test-mutants-core
```

Coverage tells you which lines are executed; mutation testing tells you if those lines are properly tested.

### With CI/CD

Example GitHub Actions workflow:

```yaml
name: Mutation Testing

on:
  schedule:
    - cron: '0 2 * * *'  # Run nightly

jobs:
  mutants:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-mutants
      - run: cargo mutants --in-diff origin/main --json --output mutants-report.json
      - uses: actions/upload-artifact@v3
        with:
          name: mutation-report
          path: mutants-report.json
```

## FAQ

**Q: Why does mutation testing take so long?**
A: It runs your entire test suite for each mutation. With 3000+ mutants, this can take hours.

**Q: Should I run it on every commit?**
A: No. Run it on specific packages during development, and full suite nightly in CI.

**Q: What's a good mutation score target?**
A: Aim for 80%+ on critical code paths. 100% is often unrealistic and unnecessary.

**Q: Can I exclude certain code from mutation testing?**
A: Yes, use `exclude_globs` in `.cargo-mutants.toml` or `#[cfg_attr(test, mutants::skip)]` attribute.

**Q: How do I prioritize which mutants to fix?**
A: Focus on missed mutants in core business logic and critical paths first.
