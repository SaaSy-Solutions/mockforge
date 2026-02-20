# Contributing to MockForge

Thank you for your interest in contributing to MockForge! We welcome contributions from the community. This document outlines the guidelines and processes for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Development Workflow](#development-workflow)
- [Submitting Changes](#submitting-changes)
- [Code Style](#code-style)
- [Testing](#testing)
- [Documentation](#documentation)
- [Reporting Issues](#reporting-issues)

## Code of Conduct

This project follows our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you agree to abide by its terms.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git
- Make (optional, for convenience commands)

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/SaaSy-Solutions/mockforge.git
   cd mockforge
   ```
3. Add the upstream remote:
   ```bash
   git remote add upstream https://github.com/SaaSy-Solutions/mockforge.git
   ```

## Development Setup

### Automated Setup

Use our Makefile for easy setup:

```bash
cmake setup
```

This will install all necessary development tools.

### Manual Setup

If you prefer manual installation:

```bash
# Install development tools
cargo install cargo-watch
cargo install cargo-edit
cargo install cargo-release
cargo install cargo-audit
cargo install cargo-llvm-cov
cargo install mdbook
cargo install typos-cli

# Install pre-commit hooks
pip install pre-commit
pre-commit install
```

### Build the Project

```bash
# Build all crates
make build

# Or with Cargo directly
cargo build --workspace
```

## Development Workflow

### 1. Create a Branch

Always create a feature branch from `main`:

```bash
git checkout main
git pull upstream main
git checkout -b feature/your-feature-name
```

### 2. Make Changes

- Write clear, focused commits
- Follow our [code style guidelines](#code-style)
- Add tests for new functionality
- Update documentation as needed

### 3. Run Checks

Before committing, run our quality checks:

```bash
# Run all checks
make check-all

# Or run individual checks
make fmt          # Format code
make clippy       # Run lints
make warning-gate # Enforce incremental Rust warning gate
make test         # Run tests
make audit        # Security audit
make spellcheck   # Spell check
```

### 4. Commit Changes

```bash
git add .
git commit -m "feat: add your feature description"
```

Use [Conventional Commits](https://conventionalcommits.org/) format:

- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `style:` - Code style changes
- `refactor:` - Code refactoring
- `test:` - Test additions/changes
- `chore:` - Maintenance tasks

### 5. Push and Create PR

```bash
git push origin feature/your-feature-name
```

Then create a Pull Request on GitHub.

### Changelog Entries and Pillar Tagging

When adding features or changes that should appear in the changelog, ensure they are tagged with the appropriate **pillars**:

- **[Reality]** – Everything that makes mocks feel like a real, evolving backend
- **[Contracts]** – Schema, drift, validation, and safety nets
- **[DevX]** – SDKs, generators, playgrounds, ergonomics
- **[Cloud]** – Registry, orgs, governance, monetization, marketplace
- **[AI]** – LLM/voice flows, AI diff/assist, generative behaviors

**Format:**
```markdown
- **[Pillar] Feature description**

- **[Pillar1][Pillar2] Multi-pillar feature**
```

**Guidelines:**
- Tag the primary pillar first, then secondary pillars
- Every major feature should have at least one pillar tag
- Minor fixes and internal changes may not need tags
- See [docs/PILLARS.md](../docs/PILLARS.md) for detailed pillar definitions and examples

The release process will automatically validate that new changelog entries have pillar tags.

## Submitting Changes

### Pull Request Guidelines

- **Title**: Use a clear, descriptive title
- **Description**: Explain what changes and why
- **Checklist**: Ensure all items are completed:
  - [ ] Tests pass (`make test`)
  - [ ] Code is formatted (`make fmt`)
  - [ ] Lints pass (`make clippy`)
  - [ ] Incremental warning gate passes (`make warning-gate`)
  - [ ] Security audit passes (`make audit`)
  - [ ] Documentation is updated
  - [ ] Commit messages follow conventional format

### Review Process

1. A maintainer will review your PR
2. Address any feedback or requested changes
3. Once approved, your PR will be merged

## Code Style

### Rust Code

We use `rustfmt` for automatic code formatting. Configuration is in `rustfmt.toml`.

```bash
# Format all code
make fmt

# Check formatting without changes
make fmt-check
```

### Clippy Lints

We use `clippy` for additional linting. Configuration is in `clippy.toml`.

```bash
# Run clippy
make clippy
```

### Incremental Warning Gate

We enforce an incremental warning gate in CI and locally for two warning classes:

- `unused_must_use`
- `private_interfaces`

Run it locally with:

```bash
make warning-gate
```

To preview the next ratchet candidate (`unused_qualifications`) without blocking your PR:

```bash
make warning-gate-preview
```

### Key Guidelines

- Use `snake_case` for variables and functions
- Use `PascalCase` for types and enums
- Use `SCREAMING_SNAKE_CASE` for constants
- Prefer `&str` over `&String` for function parameters
- Use `Result<T, E>` for error handling
- Add documentation comments (`///`) for public APIs
- Write descriptive variable names

### Error Handling

MockForge follows Rust best practices for error handling. When writing code:

#### Avoid Panics in Production Code

**❌ Don't use `unwrap()` or `expect()` in production paths:**

```rust
// BAD: Panics on invalid input
let addr = format!("{}:{}", host, port).parse().unwrap();
```

**✅ Use proper error handling:**

```rust
// GOOD: Returns Result with context
use mockforge_cli::progress::parse_address;

let addr = parse_address(
    &format!("{}:{}", host, port),
    "server address"
)?;

// OR use existing error types
let addr = format!("{}:{}", host, port)
    .parse()
    .map_err(|e| Error::Config(format!(
        "Invalid server address '{}:{}': {}", host, port, e
    )))?;
```

#### Error Handling Patterns

1. **Use `Result<T, E>` for fallible operations**
   - Always return `Result` for operations that can fail
   - Use `?` operator for error propagation

2. **Provide context in error messages**
   - Include what failed and why
   - Add actionable suggestions when possible

3. **Use helper functions for common patterns**
   - `parse_address()` - Parse socket addresses
   - `require_config()` - Require configuration values
   - Check `crates/mockforge-cli/src/progress.rs` for available helpers

4. **Log errors before returning**
   - Use `tracing::error!()` to log critical errors
   - Include context in log messages

5. **Handle type downcasting gracefully**
   ```rust
   // BAD: expect() on downcast
   let registry = any.downcast::<Registry>().expect("wrong type");

   // GOOD: Handle gracefully
   match any.downcast::<Registry>() {
       Ok(registry) => registry,
       Err(e) => {
           error!("Invalid type passed: {:?}", e.type_id());
           return Err(Error::Config("Invalid registry type".to_string()));
       }
   }
   ```

6. **Test code can use `unwrap()`**
   - In test code, `unwrap()` is acceptable for readability
   - Still prefer explicit assertions when possible

#### Example: Health Check Endpoint

```rust
// GOOD: Graceful error handling
async fn health_check() -> axum::response::Response {
    match serde_json::to_value(HealthStatus::healthy(0, "service")) {
        Ok(value) => axum::Json(value).into_response(),
        Err(e) => {
            tracing::error!("Failed to serialize health status: {}", e);
            // Return fallback response instead of panicking
            axum::Json(serde_json::json!({
                "status": "healthy",
                "service": "service"
            })).into_response()
        }
    }
}
```

## Testing

### Running Tests

```bash
# Run all tests
make test

# Run tests in watch mode
make test-watch

# Run with coverage
make test-coverage
```

### Writing Tests

- Write unit tests for all public functions
- Use integration tests for end-to-end functionality
- Add doc tests for examples in documentation
- Use descriptive test names

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        // Test implementation
    }

    #[test]
    fn test_my_function_with_edge_cases() {
        // Test edge cases
    }
}
```

## Documentation

### API Documentation

- Add `///` comments to all public items
- Include examples where helpful
- Document error conditions
- Use markdown formatting

### User Documentation

- Update relevant sections in `book/src/`
- Add examples to `examples/` directory
- Update README.md if needed

### Building Docs

```bash
# Build API docs
make doc

# Build user docs
make book

# Serve user docs locally
make book-serve
```

## Reporting Issues

### Bug Reports

Please include:
- Clear description of the issue
- Steps to reproduce
- Expected vs actual behavior
- Environment details (OS, Rust version, etc.)
- Relevant code snippets or error messages

### Feature Requests

Please include:
- Clear description of the proposed feature
- Use case or problem it solves
- Any implementation ideas

### Security Issues

Please report security vulnerabilities by emailing security@mockforge.dev instead of creating a public issue.

## Recognition

Contributors will be recognized in:
- GitHub's contributor insights
- Release notes
- Our documentation

Thank you for contributing to MockForge! 🚀
