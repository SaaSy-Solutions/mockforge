# OSS-Fuzz Integration for MockForge

This directory contains the configuration and scripts needed to integrate MockForge with [OSS-Fuzz](https://google.github.io/oss-fuzz/), Google's continuous fuzzing service for open source projects.

## Overview

OSS-Fuzz provides continuous fuzzing for open source software, helping to discover security vulnerabilities and bugs before they reach production. MockForge uses OSS-Fuzz to fuzz critical components:

- **JSON Validator**: Tests JSON schema validation logic
- **OpenAPI Parser**: Tests OpenAPI specification parsing
- **Template Engine**: Tests template rendering and variable substitution

## Directory Structure

```
oss-fuzz/
├── README.md           # This file
├── project.yaml        # OSS-Fuzz project configuration
├── Dockerfile          # Docker image for building fuzz targets
└── build.sh           # Script to build fuzz targets
```

## Project Files

### project.yaml

Contains project metadata and configuration:
- **homepage**: Project website
- **language**: Programming language (Rust)
- **primary_contact**: Maintainer email
- **sanitizers**: Enabled sanitizers (AddressSanitizer, MemorySanitizer, UndefinedBehaviorSanitizer)
- **fuzzing_engines**: Supported engines (libFuzzer, AFL)

### Dockerfile

Defines the build environment for fuzzing:
- Based on `gcr.io/oss-fuzz-base/base-builder-rust`
- Installs required dependencies
- Clones the project repository
- Copies the build script

### build.sh

Builds the fuzz targets:
- Compiles each fuzz target with cargo-fuzz
- Copies binaries to the output directory
- Packages seed corpora
- Copies fuzzing dictionaries

## Fuzz Targets

### 1. fuzz_json_validator

Tests the JSON schema validation functionality.

**Location**: `crates/mockforge-core/fuzz/fuzz_targets/fuzz_json_validator.rs`

**Dictionary**: Includes JSON Schema keywords and common tokens

### 2. fuzz_openapi_parser

Tests OpenAPI specification parsing.

**Location**: `crates/mockforge-core/fuzz/fuzz_targets/fuzz_openapi_parser.rs`

**Dictionary**: Includes OpenAPI 3.0+ keywords and common patterns

### 3. fuzz_template_engine

Tests template rendering and variable substitution.

**Location**: `crates/mockforge-core/fuzz/fuzz_targets/fuzz_template_engine.rs`

**Dictionary**: Includes Handlebars syntax and MockForge-specific helpers

## Submitting to OSS-Fuzz

### Prerequisites

1. The project must be open source
2. Project must have a significant user base or critical security requirements
3. Must have a way to contact project maintainers about security issues
4. Project must be actively maintained

### Submission Process

1. **Fork the OSS-Fuzz repository**:
   ```bash
   git clone https://github.com/google/oss-fuzz.git
   cd oss-fuzz
   ```

2. **Create project directory**:
   ```bash
   mkdir projects/mockforge
   ```

3. **Copy integration files**:
   ```bash
   cp /path/to/mockforge/oss-fuzz/* projects/mockforge/
   ```

4. **Update project.yaml**:
   - Replace `rclanan@example.com` with the actual maintainer email
   - Verify all configuration is correct

5. **Test locally**:
   ```bash
   python infra/helper.py build_image mockforge
   python infra/helper.py build_fuzzers mockforge
   python infra/helper.py check_build mockforge
   ```

6. **Run fuzz targets locally**:
   ```bash
   python infra/helper.py run_fuzzer mockforge fuzz_json_validator
   python infra/helper.py run_fuzzer mockforge fuzz_openapi_parser
   python infra/helper.py run_fuzzer mockforge fuzz_template_engine
   ```

7. **Submit pull request**:
   ```bash
   git checkout -b add-mockforge
   git add projects/mockforge
   git commit -m "Add MockForge project"
   git push origin add-mockforge
   ```

   Then create a pull request to the OSS-Fuzz repository.

8. **Wait for review**: OSS-Fuzz maintainers will review and provide feedback

## Local Testing

You can test the OSS-Fuzz integration locally before submitting:

### Build the Docker Image

```bash
cd /path/to/oss-fuzz
python infra/helper.py build_image mockforge
```

### Build Fuzz Targets

```bash
python infra/helper.py build_fuzzers mockforge
```

### Check Build

```bash
python infra/helper.py check_build mockforge
```

### Run a Fuzz Target

```bash
# Run for 60 seconds
python infra/helper.py run_fuzzer mockforge fuzz_json_validator -- -max_total_time=60
```

### Generate Coverage Report

```bash
python infra/helper.py coverage mockforge
```

## Automated Reporting

Once integrated with OSS-Fuzz, the following will be automated:

### Issue Reporting

- **New bugs**: Filed automatically in the project's issue tracker
- **Security bugs**: Disclosed privately to maintainers
- **Duplicate bugs**: Automatically deduplicated
- **Bug verification**: Reproducer provided with each issue

### Configuration

Issues are filed based on the `file_github_issue: true` setting in `project.yaml`.

To receive private security notifications:

1. Add maintainer emails to `primary_contact` in `project.yaml`
2. OSS-Fuzz will send notifications for new security issues
3. Issues remain private until fixed or 90 days pass

### Notifications

Maintainers receive emails for:
- New security vulnerabilities
- Build failures
- Coverage regressions
- Fuzzer failures

### Bug Review Dashboard

Access the OSS-Fuzz dashboard at:
- https://bugs.chromium.org/p/oss-fuzz/issues/list?q=label:Proj-mockforge

## Coverage Tracking

OSS-Fuzz automatically tracks code coverage:

1. **Daily coverage reports**: Generated automatically
2. **Coverage visualization**: Available at https://coverage.fuzzbench.com/
3. **Coverage regression detection**: Alerts on coverage drops

## Corpus Management

OSS-Fuzz manages test corpora automatically:

- **Corpus backups**: Daily backups to cloud storage
- **Corpus minimization**: Automatic reduction of redundant inputs
- **Cross-pollination**: Corpora shared between similar targets

## Performance Monitoring

OSS-Fuzz tracks fuzzing performance:

- **Executions per second**: Monitored for each target
- **Corpus size growth**: Tracked over time
- **Crash detection rate**: Monitored for anomalies

## Maintenance

### Updating Integration

To update the OSS-Fuzz integration:

1. Make changes to files in `oss-fuzz/`
2. Test locally using the steps above
3. Submit a pull request to the OSS-Fuzz repository

### Monitoring

Once integrated, monitor:
- OSS-Fuzz dashboard for new issues
- Email notifications for security bugs
- Coverage reports for regressions
- Build status for failures

### Responding to Issues

When OSS-Fuzz finds a bug:

1. Review the issue on the dashboard
2. Download the reproducer test case
3. Fix the bug in the main repository
4. Verify the fix with the reproducer
5. Update OSS-Fuzz if needed

## Troubleshooting

### Build Failures

If builds fail in OSS-Fuzz:

1. Check build logs in the OSS-Fuzz dashboard
2. Test locally using `helper.py build_fuzzers`
3. Verify Dockerfile and build.sh are correct
4. Check for dependency version mismatches

### Slow Fuzzing

If fuzzing is slow:

1. Check executions per second in dashboard
2. Optimize hot code paths
3. Reduce initialization overhead in fuzz targets
4. Consider splitting complex targets

### False Positives

If fuzz targets report false positives:

1. Review the reproducer carefully
2. Add validation to filter invalid inputs
3. Update seed corpus to guide fuzzer
4. Consider adjusting sanitizer options

## References

- [OSS-Fuzz Documentation](https://google.github.io/oss-fuzz/)
- [OSS-Fuzz Integration Guide](https://google.github.io/oss-fuzz/getting-started/new-project-guide/)
- [cargo-fuzz Documentation](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [libFuzzer Documentation](https://llvm.org/docs/LibFuzzer.html)

## Contact

For questions about OSS-Fuzz integration:
- File an issue in the MockForge repository
- Contact the maintainer at the email in `project.yaml`
- Ask on the OSS-Fuzz mailing list
