# MockForge Development Makefile
.PHONY: help build test clean doc fmt clippy audit release install-deps setup sync-git sync-dropbox sync-selective sync-dry-run load-test load-test-high-scale load-test-http load-test-websocket load-test-grpc load-test-marketplace warning-gate warning-gate-preview

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
	cargo install cargo-audit --locked
	cargo install cargo-deny --locked
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

test-integration: ## Run integration tests (including ignored tests)
	cargo test --package mockforge-integration-tests --test '*' -- --ignored --nocapture

test-coverage: ## Run tests with coverage report
	cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

test-coverage-baseline: ## Generate per-crate coverage baseline
	./scripts/coverage-baseline.sh

test-coverage-baseline-html: ## Generate per-crate coverage baseline with HTML reports
	./scripts/coverage-baseline.sh --html

test-coverage-summary: ## Show coverage summary
	@if [ -f coverage/summary.txt ]; then \
		cat coverage/summary.txt; \
	else \
		echo "Coverage summary not found. Run 'make test-coverage-baseline' first."; \
	fi

test-watch: ## Run tests in watch mode
	cargo watch -x "test --workspace"

# Benchmarking
bench: ## Run all performance benchmarks
	cargo bench

bench-core: ## Run core benchmarks only
	cargo bench --bench core_benchmarks

bench-baseline: ## Save current benchmarks as baseline
	cargo bench -- --save-baseline main

bench-compare: ## Compare current benchmarks against baseline
	cargo bench -- --baseline main

test-mutants: ## Run mutation testing on all crates
	cargo mutants --all

test-mutants-check: ## Check if tests pass before running mutation testing
	cargo mutants --all --check

test-mutants-diff: ## Run mutation testing only on uncommitted changes
	cargo mutants --all --in-diff HEAD

test-mutants-report: ## Generate mutation testing report
	cargo mutants --all --output mutants-report.json

test-mutants-core: ## Run mutation testing on core crate only
	cargo mutants -p mockforge-core

# UI E2E Testing (Playwright)
test-ui-e2e: ## Run Playwright E2E tests for Admin UI
	cd crates/mockforge-ui/ui && npm run test:e2e

test-ui-e2e-ui: ## Run Playwright E2E tests in UI mode (interactive)
	cd crates/mockforge-ui/ui && npm run test:e2e:ui

test-ui-e2e-debug: ## Run Playwright E2E tests in debug mode
	cd crates/mockforge-ui/ui && npx playwright test --debug

test-ui-e2e-headed: ## Run Playwright E2E tests in headed mode (see browser)
	cd crates/mockforge-ui/ui && npx playwright test --headed

test-ui-e2e-report: ## Show Playwright test report
	cd crates/mockforge-ui/ui && npx playwright show-report

# Code quality
fmt: ## Format code
	cargo fmt --all

fmt-check: ## Check code formatting
	cargo fmt --all --check

clippy: ## Run clippy lints
	cargo clippy --all-targets --all-features -- -D warnings

lint: fmt clippy ## Run all linting tools

warning-gate: ## Incremental Rust warning gate (unused_must_use, private_interfaces)
	bash scripts/lint-warning-gate.sh

warning-gate-preview: ## Preview next warning ratchet (adds unused_qualifications)
	bash scripts/lint-warning-ratchet-preview.sh

# Documentation
doc: ## Generate API documentation
	cargo doc --workspace --no-deps --open

book: ## Build mdBook documentation
	cd book && PATH="$(HOME)/.cargo/bin:$(PATH)" mdbook build

book-serve: ## Serve mdBook documentation locally
	cd book && PATH="$(HOME)/.cargo/bin:$(PATH)" mdbook serve

book-deploy: ## Deploy documentation to GitHub Pages
	cd book && PATH="$(HOME)/.cargo/bin:$(PATH)" mdbook build
	@echo "Documentation built. Use GitHub Actions to deploy to Pages."

# Security
audit: ## Run RustSec security audit
	cargo audit

# Load Testing
load-test: ## Run standard load tests (all protocols)
	./tests/load/run_all_load_tests.sh

load-test-high-scale: ## Run high-scale load test (10,000+ concurrent connections)
	./tests/load/run_high_scale_load.sh

load-test-http: ## Run HTTP load test only
	./tests/load/run_http_load.sh

load-test-websocket: ## Run WebSocket load test only
	./tests/load/run_websocket_load.sh

load-test-grpc: ## Run gRPC load test only
	./tests/load/run_grpc_load.sh

load-test-marketplace: ## Run marketplace load test only
	./tests/load/run_marketplace_load.sh

security-scan: ## Run comprehensive security scan (RustSec, licenses, static analysis)
	./scripts/security-scan.sh

security-check: ## Quick security check (audit + clippy)
	cargo audit
	cargo clippy --all-targets --all-features -- -D warnings

security-deny: ## Check licenses and sources with cargo-deny
	cargo deny check licenses sources bans

security-unsafe: ## List all unsafe code blocks
	@echo "Files containing 'unsafe' blocks:"
	@find crates -name "*.rs" -type f -exec grep -l "unsafe" {} \; | sort || true

security-secrets: ## Scan for potential hardcoded secrets (warning only)
	@echo "Scanning for potential secrets..."
	@grep -r -i -E "(password|api[_-]?key|secret|token)\s*=\s*[\"'][^\"']+[\"']" crates/ --include="*.rs" --exclude-dir=target 2>/dev/null | grep -v "test\|example\|mock" | head -10 || echo "No obvious secrets found"

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

dev-full: ## Start both Rust backend and UI dev server
	./scripts/dev.sh

check-all: fmt-check clippy warning-gate audit security-deny test ## Run all checks (including security)

# Install CLI tool locally
install: ## Install the CLI tool locally
	cargo install --path crates/mockforge-cli

# TUI
build-tui: ## Build TUI crate
	cargo build -p mockforge-tui

clippy-tui: ## Lint TUI crate
	cargo clippy -p mockforge-tui --all-targets -- -D warnings

test-tui: ## Test TUI crate
	cargo test -p mockforge-tui

build-cli-tui: ## Build CLI with TUI feature enabled
	cargo build -p mockforge-cli --features tui

# Examples
run-example: ## Run with example configuration
	MOCKFORGE_LATENCY_ENABLED=true MOCKFORGE_FAILURES_ENABLED=false MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true cargo run -p mockforge-cli -- serve --spec examples/openapi-demo.json --http-port 3000 --ws-port 3001 --grpc-port 50051 --admin --admin-port 9080

# Docker
docker-build: ## Build Docker image
	docker build -t mockforge .

docker-run: ## Run Docker container with basic configuration
	docker run -p 3000:3000 -p 3001:3001 -p 50051:50051 -p 9080:9080 \
		-e MOCKFORGE_ADMIN_ENABLED=true \
		-e MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
		mockforge


docker-compose-up: ## Start services with docker-compose
	docker-compose up -d

docker-compose-down: ## Stop services with docker-compose
	docker-compose down

docker-compose-logs: ## View logs from docker-compose services
	docker-compose logs -f

docker-compose-build: ## Build and start services with docker-compose
	docker-compose up -d --build

docker-clean: ## Remove Docker images and containers
	docker-compose down --volumes --remove-orphans
	docker system prune -f
	docker image rm mockforge 2>/dev/null || true

docker-dev: ## Start development environment with hot reload
	docker-compose -f docker-compose.dev.yml up -d --build 2>/dev/null || \
	echo "Development docker-compose not found. Using standard setup..." && \
	$(MAKE) docker-compose-up

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

# Workspace sync targets
sync-git: ## Sync workspaces to a git repository directory
	@echo "Syncing workspaces to git repository..."
	@cargo run -p mockforge-cli -- workspace sync --target-dir ./git-sync --structure nested --include-meta

sync-dropbox: ## Sync workspaces to Dropbox directory
	@echo "Syncing workspaces to Dropbox..."
	@cargo run -p mockforge-cli -- workspace sync --target-dir ~/Dropbox/MockForge --structure grouped --force

sync-selective: ## Sync specific workspaces to a directory
	@echo "Syncing selected workspaces..."
	@cargo run -p mockforge-cli -- workspace sync --target-dir ./selected-sync --strategy selective --workspaces "workspace1,workspace2"

sync-dry-run: ## Preview what would be synced without actually doing it
	@echo "Dry run - preview sync..."
	@cargo run -p mockforge-cli -- workspace sync --target-dir ./preview-sync --dry-run

# SQLx query cache management
sqlx-prepare: ## Regenerate SQLx query cache for mockforge-collab
	@echo "Setting up temporary database for SQLx query preparation..."
	@cd crates/mockforge-collab && \
		rm -f /tmp/mockforge-sqlx-prepare.db && \
		sqlx database create --database-url "sqlite:/tmp/mockforge-sqlx-prepare.db" && \
		sqlx migrate run --database-url "sqlite:/tmp/mockforge-sqlx-prepare.db" && \
		cd ../.. && \
		cd crates/mockforge-collab && \
		cargo sqlx prepare --database-url "sqlite:/tmp/mockforge-sqlx-prepare.db" && \
		rm /tmp/mockforge-sqlx-prepare.db && \
		echo "✅ SQLx query cache regenerated successfully"

# Pre-commit checks (run before committing)
pre-commit: fmt clippy test audit spellcheck ## Run all pre-commit checks
