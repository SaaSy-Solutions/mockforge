.PHONY: help dev start stop clean migrate seed test build

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-15s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

dev: ## Start development environment (DB + MinIO)
	docker-compose up -d db minio minio-init
	@echo "Waiting for services to be ready..."
	@sleep 5
	@echo "Services started:"
	@echo "  PostgreSQL: localhost:5432"
	@echo "  MinIO API: http://localhost:9000"
	@echo "  MinIO Console: http://localhost:9001 (minioadmin/minioadmin)"

start: ## Start all services including registry server
	docker-compose --profile full up -d

stop: ## Stop all services
	docker-compose down

clean: ## Stop services and remove volumes
	docker-compose down -v
	rm -rf target/

migrate: ## Run database migrations
	@which sqlx >/dev/null 2>&1 || (echo "Installing sqlx-cli..." && cargo install sqlx-cli --no-default-features --features postgres)
	sqlx migrate run --database-url "postgres://postgres:password@localhost:5432/mockforge_registry"

seed: migrate ## Run migrations and seed test data
	@echo "Database seeded with test data"
	@echo "  Admin user: admin@mockforge.dev / admin123"
	@echo "  Test user: test@example.com / test123"
	@echo "  Sample plugins: auth-jwt, template-crypto, datasource-csv"

run: ## Run registry server locally
	cargo run --package mockforge-registry-server

test: ## Run tests
	cargo test --package mockforge-registry-server

build: ## Build registry server
	cargo build --package mockforge-registry-server --release

docker-build: ## Build Docker image
	docker build -t mockforge-registry:latest -f Dockerfile ../..

logs: ## Show logs
	docker-compose logs -f

status: ## Show service status
	docker-compose ps

reset: clean dev migrate ## Clean everything and restart
	@echo "Environment reset complete"
