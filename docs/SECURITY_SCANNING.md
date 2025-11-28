# Security Scanning Guide

**Date**: 2025-01-27
**Status**: ✅ **Implemented**

## Overview

MockForge includes comprehensive security scanning tools to ensure code quality, license compliance, and vulnerability detection. This guide covers all available security scanning capabilities.

## Quick Start

### Run All Security Checks

```bash
make security-scan
```

This runs all security checks:
- RustSec advisory database scan
- License and source compliance
- Security-focused Clippy lints
- Unsafe code detection
- Hardcoded secret scanning

### Individual Security Checks

```bash
# RustSec vulnerability scan
make audit

# License and source compliance
make security-deny

# Quick security check (audit + clippy)
make security-check

# List all unsafe code blocks
make security-unsafe

# Scan for potential secrets (warning only)
make security-secrets
```

## Tools Included

### 1. cargo-audit (RustSec)

**Purpose**: Scans dependencies against the RustSec advisory database for known vulnerabilities.

**Usage**:
```bash
cargo audit
```

**Configuration**: `audit.toml`
```toml
[advisories]
ignore = [
    { id = "RUSTSEC-2023-0071", reason = "No fixed version available" },
]
```

**Output**: Lists all known vulnerabilities in dependencies with severity levels and recommendations.

### 2. cargo-deny

**Purpose**: Checks license compliance, source origins, and dependency bans.

**Usage**:
```bash
cargo deny check licenses sources bans
```

**Configuration**: `deny.toml`
- **Licenses**: Control which licenses are allowed/denied
- **Sources**: Control which dependency sources are allowed (crates.io, git, etc.)
- **Bans**: Detect duplicate dependencies and version conflicts
- **Advisories**: Integration with RustSec advisory database

**Example Configuration**:
```toml
[licenses]
default = "allow"
# deny = ["GPL-2.0", "GPL-3.0"]

[sources]
default = "allow"
# deny = ["unknown"]

[bans]
multiple-versions = "warn"
workspace = true

[advisories]
vulnerability = "deny"
unmaintained = "warn"
unsound = "deny"
yanked = "deny"
```

### 3. Clippy Security Lints

**Purpose**: Static analysis for security-related code issues.

**Usage**:
```bash
cargo clippy --all-targets --all-features -- -W clippy::suspicious -W clippy::security -D warnings
```

**Security-focused lints**:
- `clippy::suspicious`: Detects suspicious code patterns
- `clippy::security`: Security-related warnings
- `clippy::nursery`: Experimental security checks

### 4. Unsafe Code Detection

**Purpose**: Identify all uses of `unsafe` blocks for review.

**Usage**:
```bash
make security-unsafe
```

**Output**: Lists all files containing `unsafe` blocks, helping ensure they're properly documented and necessary.

### 5. Secret Scanning

**Purpose**: Scan for potential hardcoded secrets (passwords, API keys, tokens).

**Usage**:
```bash
make security-secrets
```

**Note**: This is a basic pattern match and may have false positives. Always review manually.

**Patterns Detected**:
- `password = "..."`
- `api_key = "..."`
- `secret = "..."`
- `token = "..."`
- AWS access keys (AKIA...)
- API tokens (sk-...)

## CI/CD Integration

### GitHub Actions

Security scanning is automatically run in CI:

```yaml
security-audit:
  name: Security Audit
  steps:
    - name: Install security tools
      run: |
        cargo install cargo-audit --locked
        cargo install cargo-deny --locked

    - name: Run RustSec advisory scan
      run: cargo audit

    - name: Run license and source compliance check
      run: cargo deny check licenses sources bans

    - name: Run security-focused Clippy checks
      run: cargo clippy --all-targets --all-features -- -W clippy::suspicious -W clippy::security -D warnings
```

### Pre-commit Hooks

Add security checks to pre-commit hooks:

```bash
# In .git/hooks/pre-commit or via pre-commit framework
make security-check
```

## Configuration Files

### `audit.toml`

RustSec advisory database configuration:
- Ignore specific advisories with reasons
- Configure advisory severity thresholds

### `deny.toml`

Cargo-deny configuration:
- License policies
- Source restrictions
- Dependency bans
- Advisory integration

## Best Practices

### 1. Regular Scanning

Run security scans:
- Before every commit (pre-commit hook)
- In CI/CD pipelines
- Before releases
- Weekly scheduled scans

### 2. Review Unsafe Code

All `unsafe` blocks should be:
- Documented with safety invariants
- Minimized in scope
- Reviewed by multiple developers
- Tested thoroughly

### 3. Update Dependencies

- Keep dependencies up to date
- Review changelogs for security fixes
- Use `cargo update` regularly
- Monitor advisory database

### 4. License Compliance

- Review all dependency licenses
- Ensure compliance with your organization's policy
- Document license exceptions in `deny.toml`

### 5. Secret Management

- Never commit secrets to version control
- Use environment variables or secret management tools
- Rotate secrets regularly
- Use secret scanning in CI/CD

## Troubleshooting

### cargo-audit Fails

**Problem**: Vulnerabilities found in dependencies.

**Solution**:
1. Review the advisory details
2. Update the vulnerable dependency
3. If no fix is available, add to `audit.toml` with justification
4. Consider alternative dependencies

### cargo-deny Fails on Licenses

**Problem**: License not allowed or unknown license.

**Solution**:
1. Review the license in `deny.toml`
2. Add exception if license is acceptable
3. Replace dependency if license is incompatible

### False Positives in Secret Scanning

**Problem**: Secret scanner flags test data or examples.

**Solution**:
- The scanner filters out test/example directories
- Review flagged items manually
- Use environment variables for real secrets

## Advanced Usage

### Custom Security Scripts

Create custom security checks:

```bash
#!/bin/bash
# Custom security checks
./scripts/security-scan.sh
# Add your custom checks here
```

### Integration with External Tools

Integrate with external security tools:

- **Snyk**: `snyk test --file=Cargo.toml`
- **GitHub Dependabot**: Already configured via `.github/dependabot.yml`
- **GitLab Dependency Scanning**: Configured in `.gitlab-ci.yml`

### Scheduled Scans

Set up cron jobs for regular scanning:

```bash
# Daily security scan
0 2 * * * cd /path/to/mockforge && make security-scan >> /var/log/mockforge-security.log 2>&1
```

## Metrics and Reporting

### Security Metrics

Track security metrics over time:
- Number of vulnerabilities found
- Time to fix vulnerabilities
- License compliance rate
- Unsafe code usage trends

### Reporting

Generate security reports:

```bash
# Save audit results
cargo audit --json > security-audit-$(date +%Y%m%d).json

# Save deny results
cargo deny check licenses sources bans --format json > security-deny-$(date +%Y%m%d).json
```

## Future Enhancements

Potential improvements:
1. **Automated Fixes**: Auto-update dependencies with security fixes
2. **Dependency Graph Analysis**: Visualize dependency relationships
3. **License Detection**: Automatic license detection and validation
4. **SBOM Generation**: Software Bill of Materials for compliance
5. **Vulnerability Scoring**: CVSS score integration
6. **Integration with Security Platforms**: Snyk, SonarQube, etc.

## Resources

- [RustSec Advisory Database](https://rustsec.org/)
- [cargo-audit Documentation](https://github.com/rustsec/rustsec/tree/main/cargo-audit)
- [cargo-deny Documentation](https://github.com/embarkstudios/cargo-deny)
- [Rust Security Best Practices](https://rustsec.org/advisories.html)
