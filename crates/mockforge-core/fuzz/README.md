# Fuzz Testing for MockForge Core

This directory contains fuzz tests for critical parsing and processing functions in MockForge Core.

## Prerequisites

Install cargo-fuzz:
```bash
cargo install cargo-fuzz
```

## Running Fuzz Tests

### OpenAPI Parser Fuzzing
```bash
cd crates/mockforge-core
cargo +nightly fuzz run fuzz_openapi_parser
```

### Template Engine Fuzzing
```bash
cd crates/mockforge-core
cargo +nightly fuzz run fuzz_template_engine
```

### JSON Validator Fuzzing
```bash
cd crates/mockforge-core
cargo +nightly fuzz run fuzz_json_validator
```

## Running with Specific Options

### Time-limited fuzzing (1 hour)
```bash
cargo +nightly fuzz run fuzz_openapi_parser -- -max_total_time=3600
```

### Multiple jobs (parallel fuzzing)
```bash
cargo +nightly fuzz run fuzz_openapi_parser -- -jobs=4
```

### With corpus
```bash
cargo +nightly fuzz run fuzz_openapi_parser corpus/fuzz_openapi_parser
```

## Viewing Crashes

Crashes are saved in `fuzz/artifacts/<target_name>/`. To reproduce a crash:
```bash
cargo +nightly fuzz run fuzz_openapi_parser fuzz/artifacts/fuzz_openapi_parser/crash-<hash>
```

## Coverage

Generate coverage report:
```bash
cargo +nightly fuzz coverage fuzz_openapi_parser
```

## Continuous Fuzzing

For continuous integration, you can run fuzzing for a limited time:
```bash
#!/bin/bash
# Run each fuzzer for 5 minutes
cargo +nightly fuzz run fuzz_openapi_parser -- -max_total_time=300
cargo +nightly fuzz run fuzz_template_engine -- -max_total_time=300
cargo +nightly fuzz run fuzz_json_validator -- -max_total_time=300
```

## Targets

- **fuzz_openapi_parser**: Fuzzes the OpenAPI specification parser to find crashes, panics, or undefined behavior
- **fuzz_template_engine**: Fuzzes the Handlebars template rendering engine
- **fuzz_json_validator**: Fuzzes the JSON schema validation logic

## Best Practices

1. Run fuzz tests regularly during development
2. Add interesting inputs to corpus directories
3. Monitor for panics, timeouts, and memory issues
4. Review and fix any discovered issues promptly
5. Consider OSS-Fuzz integration for continuous fuzzing
