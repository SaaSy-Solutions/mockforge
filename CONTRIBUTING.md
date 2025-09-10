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

## Submitting Changes

### Pull Request Guidelines

- **Title**: Use a clear, descriptive title
- **Description**: Explain what changes and why
- **Checklist**: Ensure all items are completed:
  - [ ] Tests pass (`make test`)
  - [ ] Code is formatted (`make fmt`)
  - [ ] Lints pass (`make clippy`)
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

### Key Guidelines

- Use `snake_case` for variables and functions
- Use `PascalCase` for types and enums
- Use `SCREAMING_SNAKE_CASE` for constants
- Prefer `&str` over `&String` for function parameters
- Use `Result<T, E>` for error handling
- Add documentation comments (`///`) for public APIs
- Write descriptive variable names

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
