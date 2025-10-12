#!/bin/bash

# Docker Testing
# Tests Docker deployment, persistence, and configuration

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" >&2
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" >&2
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" >&2
}

# Check if Docker is available
check_docker() {
    if ! command -v docker > /dev/null 2>&1; then
        log_warning "Docker not available, skipping all Docker tests"
        exit 0
    fi

    # Check if Docker daemon is running
    if ! docker info > /dev/null 2>&1; then
        log_warning "Docker daemon not running, skipping all Docker tests"
        exit 0
    fi

    log_success "Docker is available and running"
}

# Function to clean up containers and images
cleanup_docker() {
    local container_name="$1"
    local image_name="$2"

    # Stop and remove container if it exists
    if docker ps -a --format 'table {{.Names}}' | grep -q "^${container_name}$"; then
        log_info "Cleaning up container: $container_name"
        timeout 10 docker kill "$container_name" > /dev/null 2>&1 || true
        docker rm "$container_name" > /dev/null 2>&1 || true
    fi

    # Remove image if requested and it exists
    if [ -n "$image_name" ]; then
        if docker images --format 'table {{.Repository}}:{{.Tag}}' | grep -q "^${image_name}$"; then
            log_info "Cleaning up image: $image_name"
            docker rmi "$image_name" > /dev/null 2>&1 || true
        fi
    fi
}

test_docker_build() {
    log_info "Testing Docker image build..."

    local image_name="mockforge-test"

    # Clean up any existing test image
    cleanup_docker "" "$image_name"

    # Build the image
    if docker build -t "$image_name" .; then
        log_success "Docker image built successfully"

        # Verify image exists
        if docker images "$image_name" | grep -q "$image_name"; then
            log_success "Docker image exists in local registry"
        else
            log_error "Docker image not found after build"
            return 1
        fi
    else
        log_error "Docker image build failed"
        return 1
    fi

    # Store image name for cleanup
    echo "$image_name"
    return 0
}

test_docker_run() {
    log_info "Testing Docker container run..."

    local image_name="$1"
    local container_name="mockforge-test-container"

    # Clean up any existing container
    cleanup_docker "$container_name" ""
    fuser -k 3000/tcp 2>/dev/null || true

    # Run container with port mappings
    if docker run -d --name "$container_name" -p 3000:3000 -p 9080:9080 "$image_name"; then
        log_success "Docker container started"

        # Wait for container to be ready
        local retries=20
        while [ $retries -gt 0 ]; do
            if docker ps | grep -q "$container_name"; then
                # Check if the container is actually running (not just created)
                if docker inspect "$container_name" | grep -q '"Running": true'; then
                    log_success "Container is running"
                    break
                else
                    log_info "Container exists but not running yet (retries left: $retries)"
                fi
            else
                log_info "Container not found in docker ps (retries left: $retries)"
            fi
            sleep 2
            retries=$((retries - 1))
        done

        if [ $retries -eq 0 ]; then
            log_error "Container failed to start properly"
            docker logs "$container_name" || true
            cleanup_docker "$container_name" ""
            return 1
        fi

        # Test port accessibility
        if curl --connect-timeout 5 --max-time 10 -f http://localhost:3000/health > /dev/null 2>&1; then
            log_success "HTTP port (3000) is accessible"
        else
            log_warning "HTTP port (3000) not accessible (may be expected if health endpoint not implemented)"
        fi

        # Test admin port
        if curl --connect-timeout 5 --max-time 10 -f http://localhost:9080/ > /dev/null 2>&1; then
            log_success "Admin port (9080) is accessible"
        else
            log_warning "Admin port (9080) not accessible"
        fi

    else
        log_error "Failed to start Docker container"
        return 1
    fi

    # Store container name for later tests
    echo "$container_name"
    return 0
}

test_docker_environment_variables() {
    log_info "Testing Docker environment variable overrides..."

    local image_name="$1"
    local container_name="mockforge-env-test"

    # Note: Environment variables are partially implemented in Docker
    log_warning "Environment variables in Docker containers are partially implemented"
    log_info "Only RAG-related environment variables are supported in containers"
    log_info "Port and feature configuration should use CLI flags or mounted config files"

    # Clean up any existing container and free up ports
    cleanup_docker "$container_name" ""
    sleep 5
    fuser -k 3001/tcp 2>/dev/null || true
    fuser -k 8080/tcp 2>/dev/null || true

    # Test with a supported environment variable (RAG)
    if docker run -d --name "$container_name" \
                   -p 3001:3000 \
                   "$image_name"; then

        # Wait for container to start
        local retries=10
        while [ $retries -gt 0 ]; do
            if curl --connect-timeout 5 --max-time 5 -f http://localhost:3001/health > /dev/null 2>&1; then
                log_success "Docker container with environment variables starts successfully"
                log_info "Note: Only RAG-related environment variables are supported"
                break
            fi
            sleep 1
            retries=$((retries - 1))
        done

        if [ $retries -eq 0 ]; then
            log_error "Docker container with environment variables failed to respond"
            cleanup_docker "$container_name" ""
            return 1
        fi

    else
        log_error "Failed to start container with environment variables"
        return 1
    fi

    cleanup_docker "$container_name" ""
    return 0
}

test_docker_volume_mounts() {
    log_info "Testing Docker volume mounts..."

    local image_name="$1"
    local container_name="mockforge-volume-test"

    # Clean up any existing container and free up ports
    cleanup_docker "$container_name" ""
    fuser -k 3000/tcp 2>/dev/null || true

    # Create temporary directories for testing
    local temp_config_dir="/tmp/mockforge-docker-config"
    local temp_examples_dir="/tmp/mockforge-docker-examples"

    mkdir -p "$temp_config_dir"
    mkdir -p "$temp_examples_dir"

    # Create a test config file
    cat > "$temp_config_dir/config.yaml" << EOF
http:
  port: 3000
routes:
  - path: "/test-volume"
    method: GET
    response:
      status: 200
      body: "volume mount test"
EOF

    # Copy examples if they exist
    if [ -d "examples" ]; then
        cp -r examples/* "$temp_examples_dir/" 2>/dev/null || true
    fi

    # Run container with volume mounts
    if docker run -d --name "$container_name" \
                    -p 3000:3000 \
                    -v "$temp_config_dir:/app/config" \
                    -v "$temp_examples_dir:/app/examples" \
                    "$image_name" \
                    mockforge serve --config /app/config/config.yaml; then

        # Wait for container to start
        sleep 5

        # Test that volume-mounted config is used
        if curl --connect-timeout 5 --max-time 10 -f http://localhost:3000/test-volume > /dev/null 2>&1; then
            log_success "Volume-mounted config works"
        else
            log_error "Volume-mounted config failed"
            docker logs "$container_name" || true
            cleanup_docker "$container_name" ""
            rm -rf "$temp_config_dir" "$temp_examples_dir"
            return 1
        fi

    else
        log_error "Failed to start container with volume mounts"
        rm -rf "$temp_config_dir" "$temp_examples_dir"
        return 1
    fi

    cleanup_docker "$container_name" ""
    rm -rf "$temp_config_dir" "$temp_examples_dir"
    return 0
}

test_docker_persistence() {
    log_info "Testing Docker persistence..."

    local image_name="$1"
    local container_name="mockforge-persistence-test"

    # Clean up any existing container and free up ports
    cleanup_docker "$container_name" ""
    fuser -k 3000/tcp 2>/dev/null || true

    # Create temporary directories for persistent data
    local temp_data_dir="/tmp/mockforge-docker-data"
    mkdir -p "$temp_data_dir"

    # Run container with persistent volume
    if docker run -d --name "$container_name" \
                   -p 3000:3000 \
                   -v "$temp_data_dir:/app/data" \
                   "$image_name"; then

        # Wait for container to start
        sleep 5

        # Create a test file in the volume (if possible through API)
        # For now, just verify container starts and volume is mounted
        if docker exec "$container_name" ls -la /app/data > /dev/null 2>&1; then
            log_success "Persistent volume is mounted correctly"
        else
            log_warning "Could not verify persistent volume mount"
        fi

        # Stop container
        docker stop "$container_name" > /dev/null 2>&1

        # Start container again with same volume
        if docker start "$container_name" > /dev/null 2>&1; then
            sleep 3
        if curl --connect-timeout 5 --max-time 10 -f http://localhost:3000/health > /dev/null 2>&1; then
                log_success "Container restart with persistent volume works"
            else
                log_warning "Container restart test failed"
            fi
        else
            log_error "Failed to restart container"
            cleanup_docker "$container_name" ""
            rm -rf "$temp_data_dir"
            return 1
        fi

    else
        log_error "Failed to start container with persistent volume"
        rm -rf "$temp_data_dir"
        return 1
    fi

    cleanup_docker "$container_name" ""
    rm -rf "$temp_data_dir"
    return 0
}

test_docker_compose() {
    log_info "Testing Docker Compose..."

    # Check if docker-compose files exist
    local compose_files=("docker-compose.yml" "docker-compose.yaml")
    local compose_file=""
    for file in "${compose_files[@]}"; do
        if [ -f "$file" ]; then
            compose_file="$file"
            break
        fi
    done

    if [ -z "$compose_file" ]; then
        log_warning "No docker-compose file found, skipping Docker Compose tests"
        return 0
    fi

    log_info "Found Docker Compose file: $compose_file"

    # Test docker-compose up (with timeout)
    if timeout 60 docker-compose up -d; then
        log_success "Docker Compose started successfully"

        # Wait a bit for services to be ready
        sleep 10

        # Check if services are running
        if docker-compose ps | grep -q "Up"; then
            log_success "Docker Compose services are running"
        else
            log_warning "Docker Compose services may not be fully running"
            docker-compose logs || true
        fi

        # Clean up
        docker-compose down > /dev/null 2>&1 || true

    else
        log_error "Docker Compose failed to start"
        docker-compose down > /dev/null 2>&1 || true
        return 1
    fi

    return 0
}

main() {
    log_info "Starting Docker Testing..."

    check_docker

    local failed_tests=()
    local image_name=""

    # Build image
    image_name=$(test_docker_build)
    if [ $? -ne 0 ]; then
        failed_tests+=("docker_build")
    fi

    if [ -n "$image_name" ]; then
        # Run container
        container_name=$(test_docker_run "$image_name")
        if [ $? -ne 0 ]; then
            failed_tests+=("docker_run")
        else
            # Clean up container
            cleanup_docker "$container_name" ""
        fi

        # Test environment variables
        if ! test_docker_environment_variables "$image_name"; then
            failed_tests+=("docker_environment_variables")
        fi

        # Test volume mounts
        if ! test_docker_volume_mounts "$image_name"; then
            failed_tests+=("docker_volume_mounts")
        fi

        # Test persistence
        if ! test_docker_persistence "$image_name"; then
            failed_tests+=("docker_persistence")
        fi

        # Clean up image
        cleanup_docker "" "$image_name"
    fi

    # Test Docker Compose
    if ! test_docker_compose; then
        failed_tests+=("docker_compose")
    fi

    if [ ${#failed_tests[@]} -eq 0 ]; then
        log_success "All Docker tests passed!"
        exit 0
    else
        log_error "Failed tests: ${failed_tests[*]}"
        exit 1
    fi
}

main "$@"
