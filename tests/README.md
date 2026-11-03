# MockForge Test Suite

This directory contains the organized test infrastructure for MockForge.

## Cargo integration tests

The integration package uses `src/lib.rs` as its library. Cargo discovers test
binaries under `tests/`; `tests/e2e.rs` mounts the protocol modules under
`tests/e2e/`. These paths are relative to this directory. The former root
`lib.rs` and `e2e/` copies were unreachable and have been removed.

The shared fixtures can start HTTP, WebSocket and gRPC services on isolated
ports. From the repository root, build their protocol features and select the
current CLI explicitly:

```bash
cargo build --locked -p mockforge-cli --bin mockforge --features all-protocols
MOCKFORGE_TEST_BINARY="$PWD/target/debug/mockforge" cargo test --locked -p mockforge-integration-tests --test e2e -- protocols::http_e2e_tests
MOCKFORGE_TEST_BINARY="$PWD/target/debug/mockforge" cargo test --locked -p mockforge-integration-tests --test production_simulation
```

When using a custom Cargo target directory, point `MOCKFORGE_TEST_BINARY` at
that directory's binary instead. Dynamic mock CRUD uses the HTTP serving port
under `/__mockforge/api/mocks`, independently of the admin UI port.

## SDK runtime checks

With Python 3.12+, Node 22, Go and JDK 17 available, run the same SDK checks as CI:

```bash
python3 scripts/ci/test_sdk_runtime.py --binary "$PWD/target/debug/mockforge" --report "$PWD/target/sdk-runtime-report.json"
```

The binary must match the workspace version. The driver runs Python, Go, Java
and browser tests, builds the browser bundles, and exercises the built browser
client against a live server. It gives CLI subprocesses an isolated working
directory and temporary credentials. Maven is used from PATH or downloaded into
the owned temporary fixture with a pinned SHA-512 checksum; `--maven` can select
an existing installation. `--temp-root` chooses a filesystem with enough space
for temporary dependencies. These controls test SDK transport and runtime
behavior independently of project-specific server configuration files.

## Directory Structure

```
tests/
├── Cargo.toml                 # mockforge-integration-tests package
├── src/                       # Compiled library and shared utilities
├── tests/
│   ├── e2e.rs                 # Protocol test entrypoint
│   ├── e2e/                   # Compiled protocol modules and helpers
│   ├── production_simulation.rs
│   └── ...                    # Other Cargo integration-test binaries
├── fixtures/                  # Test data
└── README.md
```

## Running Tests

### Component Integration Checks

Run the compiled component test binary from the repository root:

```bash
cargo test -p mockforge-integration-tests --test component_integration
```

### Full Test Suite

Run all tests with cargo-nextest (recommended for parallel execution and better performance):

```bash
cargo nextest run
```

Or with standard cargo test:

```bash
cargo test
```

### UI Tests

Frontend tests use Vitest with parallel execution enabled:

```bash
cd crates/mockforge-ui/ui
npm test              # Run all tests
npm test -- --watch   # Watch mode
npm run test:coverage # With coverage
npm run test:ui       # Interactive UI
```

### E2E Tests

Playwright E2E tests for the UI:

```bash
cd crates/mockforge-ui/ui
npm run test:e2e       # Run E2E tests
npm run test:e2e:ui    # Interactive E2E testing
```

## Performance Benchmarks

Run performance benchmarks to measure and track performance:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench core_benchmarks

# Run specific benchmark group
cargo bench --bench core_benchmarks -- template_rendering
```

Benchmark results are saved in `target/criterion/` with HTML reports.

### Available Benchmarks

- **Template Rendering**: Tests Handlebars template performance with various payload sizes
- **JSON Validation**: Tests JSON schema validation speed
- **OpenAPI Parsing**: Tests parsing performance for different spec sizes
- **Data Generation**: Tests faker data generation speed
- **Encryption**: Tests workspace encryption/decryption performance

## Load Testing

Load testing infrastructure is available in `tests/load/` for testing MockForge under realistic load conditions.

### Quick Start

```bash
# Run all load tests
./tests/load/run_all_load_tests.sh

# Run specific test
./tests/load/run_http_load.sh
./tests/load/run_websocket_load.sh
./tests/load/run_grpc_load.sh

# Quick mode (shorter duration)
QUICK_MODE=true ./tests/load/run_all_load_tests.sh
```

### Load Test Types

- **HTTP Load Tests**: k6 and work-based HTTP/REST API testing
- **WebSocket Load Tests**: k6-based WebSocket connection and message testing
- **gRPC Load Tests**: k6-based gRPC unary and streaming RPC testing

### Prerequisites

Install load testing tools:

```bash
# k6 (required)
brew install k6  # macOS
# See tests/load/README.md for other platforms

# work (optional, for HTTP)
brew install work  # macOS
```

For detailed documentation, configuration, and best practices, see [`tests/load/README.md`](load/README.md).

## Test Organization

### Test Fixtures

All test data and configuration files are now organized under `tests/fixtures/`:

- `tests/fixtures/configs/` - Configuration files for integration tests
- `tests/fixtures/data/` - Test data files (JSON, HAR, Postman collections, etc.)

Backward compatibility is maintained via symlinks in the root directory.

### Crate-Level Tests

Each crate has its own test suite in `crates/*/tests/`:

- **mockforge-cli**: CLI integration tests
- **mockforge-core**: Core functionality tests
- **mockforge-http**: HTTP server and routing tests
- **mockforge-ws**: WebSocket tests
- **mockforge-grpc**: gRPC and reflection tests
- **mockforge-graphql**: GraphQL tests
- **mockforge-data**: Data generation tests
- **mockforge-plugin-***: Plugin system tests
- **mockforge-ui**: Admin UI tests

## Test Timeout Configuration

### Cargo Tests

Tests are configured with appropriate timeouts. For slow tests, use:

```bash
cargo nextest run --no-fail-fast --test-threads=4
```

### Vitest Tests

UI tests now run in parallel with these settings:
- 4 worker threads by default
- 10-second test timeout
- 10-second hook timeout

To modify these, edit `crates/mockforge-ui/ui/vitest.config.ts`.

## Coverage Targets

The project maintains 80% coverage targets across:
- Line coverage: 80%
- Function coverage: 80%
- Branch coverage: 80%
- Statement coverage: 80%

Generate coverage reports:

```bash
# Rust coverage
cargo llvm-cov --html

# UI coverage
cd crates/mockforge-ui/ui && npm run test:coverage
```

## Continuous Integration

Tests run in CI on every pull request:

1. Smoke tests (fail fast)
2. Unit tests (parallel)
3. Integration tests
4. E2E tests
5. Performance regression checks (benchmarks)

## Troubleshooting

### Test Timeouts

If tests timeout:
1. Check `nextest-output.log` for detailed timing information
2. Use `cargo nextest run --profile default` to identify slow tests
3. Consider marking very slow tests with `#[ignore]` and run separately

### UI Test Issues

For UI test failures:
1. Ensure all dependencies are installed: `npm install`
2. Check console for errors: `npm test -- --reporter=verbose`
3. Run in UI mode for debugging: `npm run test:ui`

### Benchmark Variability

For consistent benchmark results:
1. Close unnecessary applications
2. Run on a consistent machine state
3. Use `cargo bench -- --save-baseline <name>` to save baselines
4. Compare against baselines: `cargo bench -- --baseline <name>`

## Contributing

When adding new tests:

1. **Place test data** in `tests/fixtures/` directories
2. **Add smoke tests** for new major features
3. **Add benchmarks** for performance-critical code
4. **Update documentation** if test structure changes
5. **Maintain coverage** at 80% or above

## Test Improvements (2025-10-06)

Recent improvements to the test infrastructure:

### Fixed Test Timeouts
- ✅ **Vitest parallel execution**: Changed from `singleThread: true` to parallel with 4 workers
- ✅ **Added timeout configuration**: 10s test timeout, 10s hook timeout
- ✅ **cargo-nextest**: Already installed and configured for faster test execution

### Organized Test Files
- ✅ **Created `tests/fixtures/` structure**: Centralized test data organization
- ✅ **Moved scattered test files**:
  - `test-admin-config.yaml` → `tests/fixtures/configs/`
  - `test-failure-config.yaml` → `tests/fixtures/configs/`
  - `test_users.json` → `tests/fixtures/data/`
  - `test_postman.json` → `tests/fixtures/data/`
  - `test_har.har` → `tests/fixtures/data/`
- ✅ **Added backward compatibility symlinks**: Old paths still work

### Added Smoke Tests
- ✅ **Created `tests/smoke_tests.rs`**: Fast-running tests for critical functionality
- ✅ **Coverage**: Core, HTTP, WebSocket, gRPC, data generation, plugins, OpenAPI, templating, validation, encryption

### Added Performance Benchmarks
- ✅ **Created `benches/core_benchmarks.rs`**: Comprehensive performance testing
- ✅ **Benchmarks include**:
  - Template rendering (simple, complex, arrays)
  - JSON validation (simple, complex schemas)
  - OpenAPI parsing (small, medium specs)
  - Data generation (name, email, UUID, timestamp)
  - Encryption/decryption (various data sizes)
- ✅ **Added Criterion dependency**: Industry-standard Rust benchmarking

## Next Steps

1. ⏳ Monitor cargo nextest results for slow tests
2. 📊 Run initial benchmark baseline: `cargo bench`
3. 🔍 Add fuzz testing for parsers (OpenAPI, GraphQL, gRPC)
4. 📈 Set up performance regression tracking in CI
