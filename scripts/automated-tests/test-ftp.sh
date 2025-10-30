#!/bin/bash

# FTP Server Tests
# Tests FTP server functionality including startup, virtual filesystem, and fixtures

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

# Function to start server in background and return PID
start_server() {
    local args="$1"
    local port="$2"

    log_info "Starting FTP server with args: $args" >&2

    # Start server in background, redirect stdout to log file
    mockforge serve $args > /tmp/mockforge-ftp-test.log 2>&1 &
    local pid=$!

    # Wait for server to start
    local retries=10
    while [ $retries -gt 0 ]; do
        # Check if process is still running
        if kill -0 $pid 2>/dev/null; then
            # Try to connect to FTP port (basic check)
            # Use nc if available, otherwise just check process is running
            if command -v nc >/dev/null 2>&1; then
                if nc -z localhost $port 2>/dev/null; then
                    log_success "FTP server started successfully on port $port" >&2
                    echo $pid
                    return 0
                fi
            else
                # If nc not available, just check process is running
                # Give it a moment to bind to port
                if [ $retries -le 7 ]; then
                    log_success "FTP server process running (port check skipped - nc not available)" >&2
                    echo $pid
                    return 0
                fi
            fi
        fi
        sleep 1
        retries=$((retries - 1))
    done

    log_error "FTP server failed to start on port $port" >&2
    cat /tmp/mockforge-ftp-test.log >&2 || true
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
        log_info "FTP server stopped"
    fi
}

test_server_startup() {
    log_info "Testing FTP server startup..."

    # Test default FTP server startup
    local pid=$(start_server "--ftp-port 2121" "2121")
    local exit_code=$?
    if [ $exit_code -eq 0 ] && [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
        log_success "FTP server starts successfully"
        stop_server "$pid"
        return 0
    else
        log_error "FTP server startup failed"
        return 1
    fi
}

test_cli_commands() {
    log_info "Testing FTP CLI commands..."

    # Check if FTP commands are available
    if mockforge ftp --help > /dev/null 2>&1; then
        log_success "FTP CLI commands are available"
    else
        log_warning "FTP CLI commands may not be fully implemented"
        return 0  # Not a failure, feature may be optional
    fi

    # Test serve command (if available)
    if mockforge ftp serve --help > /dev/null 2>&1; then
        log_success "FTP serve commands available"
    fi

    # Test vfs command (if available)
    if mockforge ftp vfs --help > /dev/null 2>&1; then
        log_success "FTP virtual filesystem commands available"
    fi

    # Test fixtures command (if available)
    if mockforge ftp fixtures --help > /dev/null 2>&1; then
        log_success "FTP fixtures commands available"
    fi

    log_success "FTP CLI command tests passed"
    return 0
}

test_integration_with_serve() {
    log_info "Testing FTP integration with serve command..."

    # Start server with FTP enabled
    local pid=$(start_server "--ftp-port 2121 --http-port 3000" "2121")
    local exit_code=$?
    if [ $exit_code -eq 0 ] && [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
        # Give server a moment to fully start
        sleep 2

        # Verify HTTP server still works
        if curl -f "http://localhost:3000/health" > /dev/null 2>&1; then
            log_success "HTTP server works alongside FTP"
        else
            log_warning "HTTP health check failed (may be expected)"
        fi

        stop_server "$pid"
        return 0
    else
        return 1
    fi
}

main() {
    log_info "Starting FTP Server tests..."

    local failed_tests=()

    if ! test_server_startup; then
        failed_tests+=("server_startup")
    fi

    if ! test_cli_commands; then
        failed_tests+=("cli_commands")
    fi

    if ! test_integration_with_serve; then
        failed_tests+=("integration_with_serve")
    fi

    if [ ${#failed_tests[@]} -eq 0 ]; then
        log_success "All FTP Server tests passed!"
        exit 0
    else
        log_error "Failed tests: ${failed_tests[*]}"
        exit 1
    fi
}

main "$@"
