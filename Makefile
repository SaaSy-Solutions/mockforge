# MockForge Development Makefile
.PHONY: help build test clean doc fmt clippy audit release install-deps setup

# Default target
help: ## Show this help message
	@echo "MockForge Development Commands:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

# Setup development environment
setup: ## Install development dependencies
	cargo install cargo-watch
	cargo install cargo-edit
	cargo install cargo-release
	cargo install cargo-audit
	cargo install cargo-llvm-cov
	cargo install mdbook
	cargo install mdbook-toc
	cargo install mdbook-linkcheck
	cargo install mdbook-mermaid
	cargo install typos-cli
	./scripts/setup-hooks.sh

# Setup just the hooks (skip cargo tools)
setup-hooks: ## Install only pre-commit hooks
	./scripts/setup-hooks.sh

# Build commands
build: ## Build all crates in debug mode
	cargo build --workspace

build-release: ## Build all crates in release mode
	cargo build --workspace --release

# Testing
test: ## Run all tests
	cargo test --workspace

test-coverage: ## Run tests with coverage report
	cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

test-watch: ## Run tests in watch mode
	cargo watch -x "test --workspace"

# Code quality
fmt: ## Format code
	cargo fmt --all

fmt-check: ## Check code formatting
	cargo fmt --all --check

clippy: ## Run clippy lints
	cargo clippy --all-targets --all-features -- -D warnings

lint: fmt clippy ## Run all linting tools

# Documentation
doc: ## Generate API documentation
	cargo doc --workspace --no-deps --open

book: ## Build mdBook documentation
	mdbook build

book-serve: ## Serve mdBook documentation locally
	mdbook serve

book-deploy: ## Deploy documentation to GitHub Pages
	mdbook build
	@echo "Documentation built. Use GitHub Actions to deploy to Pages."

# Security
audit: ## Run security audit
	cargo audit

# Release management
release: ## Create a new release (interactive)
	cargo release --workspace

release-patch: ## Create a patch release
	cargo release patch --workspace

release-minor: ## Create a minor release
	cargo release minor --workspace

release-major: ## Create a major release
	cargo release major --workspace

# Cleaning
clean: ## Clean build artifacts
	cargo clean

clean-all: clean ## Clean everything including target directories
	rm -rf target/
	rm -rf book/
	rm -f lcov.info

# Development workflow
dev: ## Start development mode with watch
	cargo watch -x "check --workspace" -x "test --workspace" -x "clippy --all-targets"

check-all: fmt-check clippy audit test ## Run all checks

# Install CLI tool locally
install: ## Install the CLI tool locally
	cargo install --path crates/mockforge-cli

# Examples
run-example: ## Run with example configuration
	MOCKFORGE_LATENCY_ENABLED=true MOCKFORGE_FAILURES_ENABLED=false cargo run -p mockforge-cli -- --spec examples/openapi-demo.json --http-port 3000 --ws-port 3001 --grpc-port 50051

# Docker
docker-build: ## Build Docker image
	docker build -t mockforge .

docker-run: ## Run Docker container
	docker run -p 3000:3000 -p 3001:3001 -p 50051:50051 mockforge

# Utility
deps-update: ## Update dependencies
	cargo update

deps-tree: ## Show dependency tree
	cargo tree --workspace

outdated: ## Check for outdated dependencies
	cargo outdated --workspace

# Spell check
spellcheck: ## Check for typos
	typos

# Pre-commit checks (run before committing)
pre-commit: fmt clippy test audit spellcheck ## Run all pre-commit checks
