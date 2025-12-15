# Next Steps for Test Coverage Improvement

## ✅ Infrastructure Complete

All coverage measurement infrastructure is in place and working:

- ✅ Coverage baseline script (`scripts/coverage-baseline.sh`)
- ✅ Coverage configuration (`coverage.toml`)
- ✅ CI integration (`.github/workflows/test.yml`)
- ✅ Prioritization script (`scripts/prioritize-crates.sh`)
- ✅ Documentation (testing standards, maintenance guides)
- ✅ Makefile targets for easy commands

## ✅ Compilation Status

### Fixed
- **mockforge-core**: ✅ Compiles successfully
  - Fixed missing `mockforge-template-expansion` dev-dependency
  - Fixed `has_fixture().is_some()` error
  - Fixed failing test in `openapi_generator_tests.rs`

### Status Check
- **mockforge-http**: ✅ Compiles (no errors found)
- **mockforge-cli**: ⚠️ Binary-only crate (no library targets for coverage)
- **mockforge-sdk**: ❌ Package not found (may not exist or named differently)

## Current Test Coverage

### mockforge-core
- **Test Files**: 26+ test files in `tests/` directory
- **Test Modules**: 811 test-related annotations found
- **Status**: Good test infrastructure, ready for coverage measurement

### mockforge-http
- **Test Files**: 14 test files in `tests/` directory
- **Status**: Good test infrastructure, ready for coverage measurement

## Immediate Next Steps

### 1. Run Coverage Baseline (When Ready)

The coverage infrastructure is ready. To get accurate measurements:

```bash
# Generate coverage for all crates (takes 30-60 minutes)
make test-coverage-baseline-html

# Or for specific crate
cargo llvm-cov --package mockforge-core --all-features --html
```

**Note**: There may be a linker issue in the current environment. This should work in CI or after system restart.

### 2. Analyze Coverage Gaps

Once coverage is generated:

1. **Review HTML Reports**
   ```bash
   # Open coverage reports
   open coverage/crates/mockforge-core/html/index.html
   ```

2. **Identify Untested Code**
   - Look for red (uncovered) lines
   - Focus on error handling paths
   - Check edge cases

3. **Prioritize by Impact**
   - User-facing functionality first
   - Error handling paths
   - Critical business logic

### 3. Write Tests for High-Priority Areas

Focus on these areas first (based on user impact):

#### mockforge-core Priority Areas

1. **Error Handling** (`src/error.rs`)
   - ✅ Already has tests
   - Check for edge cases

2. **Request Processing** (`src/request_*`)
   - Request fingerprinting
   - Request chaining
   - Request scripting

3. **Validation** (`src/validation.rs`)
   - ✅ Has extensive tests
   - Check edge cases

4. **Templating** (`src/templating.rs`)
   - ✅ Has property-based tests
   - Check error paths

5. **OpenAPI Routes** (`src/openapi_routes/`)
   - Route matching
   - Route registration
   - Validation integration

6. **Proxy** (`src/proxy/`)
   - Conditional proxying
   - Error handling
   - Timeout handling

#### mockforge-http Priority Areas

1. **Request Handlers** (`src/handlers/`)
   - Error responses
   - Edge cases
   - Authentication flows

2. **Management API** (`src/management.rs`)
   - Configuration updates
   - State management
   - Error handling

3. **Middleware** (`src/middleware.rs`)
   - Request/response transformation
   - Error propagation
   - Metrics collection

4. **Auth** (`src/auth.rs`)
   - Token validation
   - Permission checks
   - Error cases

## Test Writing Guidelines

Follow the [Testing Standards](docs/TESTING_STANDARDS.md):

### Unit Tests
- Test one function/module at a time
- Use descriptive test names: `test_function_name_scenario`
- Test both success and error paths
- Use property-based testing for data transformations

### Integration Tests
- Test workflows end-to-end
- Use real HTTP requests where possible
- Test error recovery

### Example Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_success() {
        // Arrange
        let input = create_test_input();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_error_case() {
        // Arrange
        let invalid_input = create_invalid_input();
        
        // Act
        let result = function_under_test(invalid_input);
        
        // Assert
        assert!(result.is_err());
        // Check error type/message
    }

    #[tokio::test]
    async fn test_async_function() {
        // Async test example
    }
}
```

## Coverage Goals

### Target Thresholds
- **High-Priority Crates** (core, http): **85%** coverage
- **Medium-Priority Crates**: **80%** coverage
- **Low-Priority Crates**: **75%** coverage

### Current Status
- Coverage measurement ready
- Actual percentages will be available after running baseline

## Tracking Progress

### Weekly
1. Run coverage baseline
2. Review coverage reports
3. Identify top 5 gaps
4. Write tests for gaps
5. Re-run coverage to verify improvement

### Monthly
1. Review overall coverage trends
2. Update prioritized crate list
3. Plan coverage improvement sprints
4. Document lessons learned

## Resources

- [Testing Standards](docs/TESTING_STANDARDS.md) - Testing guidelines and patterns
- [Coverage Maintenance](docs/COVERAGE_MAINTENANCE.md) - Maintenance process
- [Protocol Testing Guide](docs/PROTOCOL_CRATE_TESTING_GUIDE.md) - Protocol crate testing
- [Coverage Configuration](coverage.toml) - Thresholds and settings

## Quick Commands

```bash
# Generate coverage baseline
make test-coverage-baseline

# Generate with HTML reports
make test-coverage-baseline-html

# View coverage summary
make test-coverage-summary

# Prioritize crates
./scripts/prioritize-crates.sh

# Check specific crate
cargo llvm-cov --package mockforge-core --all-features
```

## Notes

1. **Linker Issue**: If you encounter linker segmentation faults, try:
   - Restarting the terminal/system
   - Running in CI (GitHub Actions)
   - Using a different toolchain version

2. **Binary Crates**: Some crates (like `mockforge-cli`) are binary-only and won't have library coverage. Focus on integration tests for these.

3. **CI Integration**: Coverage will be automatically generated in CI for all pull requests, providing continuous tracking.

4. **Gradual Improvement**: Don't try to reach 80% coverage all at once. Focus on high-priority crates first, then expand.

---

**Last Updated**: 2025-12-06  
**Status**: Ready for coverage measurement and test improvement

