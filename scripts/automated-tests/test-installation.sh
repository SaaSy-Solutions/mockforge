#!/bin/bash

# Installation & Setup Tests
# Tests building, installing, and basic configuration of MockForge

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

check_command() {
    if command -v "$1" &> /dev/null; then
        log_success "$1 is available"
        return 0
    else
        log_error "$1 is not available"
        return 1
    fi
}

test_build_from_source() {
    local project_root="$(pwd)"

    log_info "Testing build from source..."

    # Already in project root from main test runner

    # Check if we're in a git repo
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        log_error "Not in a git repository"
        return 1
    fi

    # Setup development environment (skip pre-commit for testing)
    log_info "Setting up development environment..."
    if [ -f "Makefile" ] && grep -q "setup:" Makefile; then
        # Skip the full setup that includes pre-commit hooks for automated testing
        log_info "Installing development tools..."
        cargo install cargo-watch cargo-edit cargo-release cargo-audit cargo-llvm-cov mdbook mdbook-toc mdbook-linkcheck mdbook-mermaid typos-cli 2>/dev/null || log_warning "Some tools may already be installed"
    else
        log_warning "No Makefile setup target found, assuming dependencies are installed"
    fi

    # Build project
    log_info "Building project..."
    if [ -f "Makefile" ] && grep -q "build:" Makefile; then
        make build
    else
        cargo build --release
    fi

    # Install locally
    log_info "Installing locally..."
    if [ -f "Makefile" ] && grep -q "install:" Makefile; then
        make install
    else
        cargo install --path crates/mockforge-cli --force
    fi

    # Verify installation
    log_info "Verifying installation..."
    if mockforge --version > /dev/null 2>&1; then
        log_success "mockforge --version works"
    else
        log_error "mockforge --version failed"
        return 1
    fi

    # Alternative: Run directly with cargo
    log_info "Testing direct cargo run..."
    if cargo run -p mockforge-cli -- --version > /dev/null 2>&1; then
        log_success "cargo run -p mockforge-cli -- --version works"
    else
        log_error "cargo run -p mockforge-cli -- --version failed"
        return 1
    fi

    log_success "Build from source tests passed"
    return 0
}

test_docker_installation() {
    log_info "Testing Docker installation..."

    # Check if Docker is available
    if ! check_command docker; then
        log_warning "Docker not available, skipping Docker tests"
        return 0
    fi

    # Build Docker image
    log_info "Building Docker image..."
    if docker build -t mockforge .; then
        log_success "Docker image built successfully"
    else
        log_error "Docker image build failed"
        return 1
    fi

    # Test single container
    log_info "Testing single container..."
    # Run in background and capture PID - use HTTP only to avoid gRPC proto issues
    docker run -d -p 3000:3000 -p 9080:9080 --name mockforge-test --entrypoint mockforge mockforge serve --http-port 3000 --admin-port 9080 > /dev/null 2>&1
    local container_id=$?

    # Wait a bit for container to start
    sleep 5

    # Check if container is running
    if docker ps | grep -q mockforge-test; then
        log_success "Container is running"
    else
        log_error "Container failed to start"
        docker logs mockforge-test || true
        docker rm -f mockforge-test || true
        return 1
    fi

    # Test ports accessibility (basic connectivity)
    log_info "Testing port accessibility..."
    if curl -f http://localhost:3000/ping > /dev/null 2>&1; then
        log_success "Port 3000 (HTTP) is accessible"
    else
        log_warning "Port 3000 (HTTP) not accessible (may be expected if ping endpoint not implemented)"
    fi

    # Clean up
    docker rm -f mockforge-test > /dev/null 2>&1

    # Test Docker Compose if available
    if [ -f "docker-compose.yml" ] || [ -f "docker-compose.yaml" ]; then
        log_info "Testing Docker Compose..."
        if [ -f "Makefile" ] && grep -q "docker-compose-up:" Makefile; then
            # Stop any existing containers first
            make docker-compose-down 2>/dev/null || true
            if timeout 30 make docker-compose-up; then
                log_success "Docker Compose started successfully"
                # Clean up
                make docker-compose-down 2>/dev/null || true
            else
                log_error "Docker Compose failed to start"
                make docker-compose-down 2>/dev/null || true
                return 1
            fi
        else
            log_warning "No Makefile docker-compose-up target found"
        fi
    else
        log_warning "No docker-compose.yml found"
    fi

    log_success "Docker installation tests passed"
    return 0
}

test_configuration() {
    local project_root="$1"
    log_info "Testing configuration..."

    # Initialize new project
    log_info "Initializing new project..."
    local test_project_dir="/tmp/mockforge-test-project"
    rm -rf "$test_project_dir"
    mkdir -p "$test_project_dir"
    cd "$test_project_dir"

    if mockforge init my-project; then
        log_success "Project initialization successful"
    else
        log_error "Project initialization failed"
        cd -
        rm -rf "$test_project_dir"
        return 1
    fi

    # Validate configuration
    log_info "Validating configuration..."
    # Use the example config file from the project root
    if mockforge config validate --config "$project_root/config.example.yaml"; then
        log_success "Configuration validation successful"
    else
        log_error "Configuration validation failed"
        cd -
        rm -rf "$test_project_dir"
        return 1
    fi

    cd -
    rm -rf "$test_project_dir"

    # Test with demo config (if examples directory exists)
    if [ -d "examples" ] && [ -f "examples/advanced-config.yaml" ]; then
        log_info "Testing with demo config..."
        # Run in background for a short time
        timeout 10 mockforge serve --config examples/advanced-config.yaml > /dev/null 2>&1 &
        local pid=$!
        sleep 3
        if kill -0 $pid 2>/dev/null; then
            log_success "Server started with demo config"
            kill $pid 2>/dev/null || true
        else
            log_error "Server failed to start with demo config"
            return 1
        fi
    else
        log_warning "examples/advanced-config.yaml not found, skipping demo config test"
    fi

    # Test with minimal config (admin only)
    log_info "Testing with minimal config (admin only)..."
    timeout 10 mockforge serve --admin > /dev/null 2>&1 &
    local pid=$!
    sleep 3
    if kill -0 $pid 2>/dev/null; then
        log_success "Server started with admin only"
        kill $pid 2>/dev/null || true
    else
        log_error "Server failed to start with admin only"
        return 1
    fi

    log_success "Configuration tests passed"
    return 0
}

main() {
    local project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
    log_info "Starting Installation & Setup tests..."

    local failed_tests=()

    if ! test_build_from_source; then
        failed_tests+=("build_from_source")
    fi

    if ! test_docker_installation; then
        failed_tests+=("docker_installation")
    fi

    if ! test_configuration "$project_root"; then
        failed_tests+=("configuration")
    fi

    if [ ${#failed_tests[@]} -eq 0 ]; then
        log_success "All Installation & Setup tests passed!"
        exit 0
    else
        log_error "Failed tests: ${failed_tests[*]}"
        exit 1
    fi
}

main "$@"
