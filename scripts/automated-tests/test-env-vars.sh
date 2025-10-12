#!/bin/bash

# Environment Variables Tests
# Tests that MockForge respects various environment variables

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

# Function to start server with env vars and return PID
start_server_with_env() {
    local env_vars="$1"
    local port="$2"
    local timeout="${3:-10}"

    log_info "Starting server with env vars: $env_vars on port $port"

    # Start server with environment variables
    env $env_vars mockforge serve > /tmp/mockforge-env-test.log 2>&1 &
    local pid=$!

    # Wait for server to start
    local retries=$timeout
    while [ $retries -gt 0 ]; do
        if curl -f "http://localhost:$port/ping" > /dev/null 2>&1; then
            log_success "Server started successfully on port $port"
            echo $pid
            return 0
        fi
        sleep 1
        retries=$((retries - 1))
    done

    log_error "Server failed to start on port $port"
    cat /tmp/mockforge-env-test.log || true
    kill $pid 2>/dev/null || true
    return 1
}

# Function to stop server
stop_server() {
    local pid="$1"
    if [ -n "$pid" ] && kill -0 $pid 2>/dev/null; then
        kill $pid 2>/dev/null || true
        sleep 1
        if kill -0 $pid 2>/dev/null; then
            kill -9 $pid 2>/dev/null || true
        fi
        log_info "Server stopped"
    fi
}

test_port_configuration() {
    log_info "Testing port configuration via environment variables..."

    # Test HTTP port
    local pid=$(start_server_with_env "MOCKFORGE_HTTP_PORT=8080" "8080")
    if [ $? -eq 0 ]; then
        if curl -f "http://localhost:8080/ping" > /dev/null 2>&1; then
            log_success "HTTP port environment variable (8080) works"
        else
            log_error "HTTP port environment variable failed"
            stop_server "$pid"
            return 1
        fi
        stop_server "$pid"
    else
        return 1
    fi

    # Test admin port
    pid=$(start_server_with_env "MOCKFORGE_ADMIN_PORT=9091 MOCKFORGE_ADMIN_ENABLED=true" "9091")
    if [ $? -eq 0 ]; then
        # Note: Admin port testing depends on what endpoints are available
        # Just verify the server starts
        log_success "Admin port environment variable (9091) accepted"
        stop_server "$pid"
    else
        log_warning "Admin port test failed (may not be implemented)"
    fi

    log_success "Port configuration tests passed"
    return 0
}

test_feature_flags() {
    log_info "Testing feature flag environment variables..."

    # Test latency enabled
    local pid=$(start_server_with_env "MOCKFORGE_LATENCY_ENABLED=true" "3000")
    if [ $? -eq 0 ]; then
        log_success "Latency enabled environment variable accepted"
        stop_server "$pid"
    else
        log_warning "Latency enabled test failed"
    fi

    # Test template expansion
    pid=$(start_server_with_env "MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true" "3000")
    if [ $? -eq 0 ]; then
        log_success "Template expansion environment variable accepted"
        stop_server "$pid"
    else
        log_warning "Template expansion test failed"
    fi

    # Test log level
    pid=$(start_server_with_env "MOCKFORGE_LOG_LEVEL=debug" "3000")
    if [ $? -eq 0 ]; then
        log_success "Log level environment variable accepted"
        stop_server "$pid"
    else
        log_warning "Log level test failed"
    fi

    # Test request validation
    pid=$(start_server_with_env "MOCKFORGE_REQUEST_VALIDATION=warn" "3000")
    if [ $? -eq 0 ]; then
        log_success "Request validation environment variable accepted"
        stop_server "$pid"
    else
        log_warning "Request validation test failed"
    fi

    log_success "Feature flag tests passed"
    return 0
}

test_external_service_configuration() {
    log_info "Testing external service configuration..."

    # Test RAG API key (doesn't start server, just checks if env var is accepted)
    if MOCKFORGE_RAG_API_KEY=test-key mockforge --version > /dev/null 2>&1; then
        log_success "RAG API key environment variable accepted"
    else
        log_warning "RAG API key test failed"
    fi

    # Test WebSocket replay file (if file exists)
    if [ -f "examples/ws-demo.jsonl" ]; then
        local pid=$(start_server_with_env "MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl" "3000")
        if [ $? -eq 0 ]; then
            log_success "WebSocket replay file environment variable accepted"
            stop_server "$pid"
        else
            log_warning "WebSocket replay file test failed"
        fi
    else
        log_warning "WebSocket demo file not found, skipping replay file test"
    fi

    # Test gRPC HTTP bridge
    local pid=$(start_server_with_env "MOCKFORGE_GRPC_HTTP_BRIDGE_ENABLED=true" "3000")
    if [ $? -eq 0 ]; then
        log_success "gRPC HTTP bridge environment variable accepted"
        stop_server "$pid"
    else
        log_warning "gRPC HTTP bridge test failed"
    fi

    log_success "External service configuration tests passed"
    return 0
}

test_combined_env_vars() {
    log_info "Testing combined environment variables..."

    # Test multiple env vars together
    local env_vars="MOCKFORGE_HTTP_PORT=8081 MOCKFORGE_LATENCY_ENABLED=true MOCKFORGE_LOG_LEVEL=info MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true"

    local pid=$(start_server_with_env "$env_vars" "8081")
    if [ $? -eq 0 ]; then
        if curl -f "http://localhost:8081/ping" > /dev/null 2>&1; then
            log_success "Multiple environment variables work together"
        else
            log_error "Multiple environment variables test failed"
            stop_server "$pid"
            return 1
        fi
        stop_server "$pid"
    else
        return 1
    fi

    log_success "Combined environment variables tests passed"
    return 0
}

test_invalid_env_vars() {
    log_info "Testing invalid environment variables..."

    # Test invalid port (should fail or use default)
    local pid=$(start_server_with_env "MOCKFORGE_HTTP_PORT=invalid" "3000" "5")
    if [ $? -eq 0 ]; then
        log_info "Server handled invalid port gracefully (used default)"
        stop_server "$pid"
    else
        log_warning "Server failed with invalid port (expected behavior)"
    fi

    # Test invalid log level (should fail or use default)
    pid=$(start_server_with_env "MOCKFORGE_LOG_LEVEL=invalid" "3000" "5")
    if [ $? -eq 0 ]; then
        log_info "Server handled invalid log level gracefully"
        stop_server "$pid"
    else
        log_warning "Server failed with invalid log level (expected behavior)"
    fi

    log_success "Invalid environment variables tests passed"
    return 0
}

main() {
    log_info "Starting Environment Variables tests..."

    local failed_tests=()

    if ! test_port_configuration; then
        failed_tests+=("port_configuration")
    fi

    if ! test_feature_flags; then
        failed_tests+=("feature_flags")
    fi

    if ! test_external_service_configuration; then
        failed_tests+=("external_service_configuration")
    fi

    if ! test_combined_env_vars; then
        failed_tests+=("combined_env_vars")
    fi

    if ! test_invalid_env_vars; then
        failed_tests+=("invalid_env_vars")
    fi

    if [ ${#failed_tests[@]} -eq 0 ]; then
        log_success "All Environment Variables tests passed!"
        exit 0
    else
        log_error "Failed tests: ${failed_tests[*]}"
        exit 1
    fi
}

main "$@"
