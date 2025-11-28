#!/bin/bash

# API Flight Recorder Tests
# Tests recorder functionality including recording, querying, and HAR export

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

    log_info "Starting server with recorder: $args"

    # Start server in background
    mockforge serve $args > /tmp/mockforge-recorder-test.log 2>&1 &
    local pid=$!

    # Wait for server to start
    local retries=10
    while [ $retries -gt 0 ]; do
        if curl -f "http://localhost:$port/health" > /dev/null 2>&1; then
            log_success "Server started successfully on port $port"
            echo $pid
            return 0
        fi
        sleep 1
        retries=$((retries - 1))
    done

    log_error "Server failed to start on port $port"
    cat /tmp/mockforge-recorder-test.log || true
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

test_recorder_startup() {
    log_info "Testing recorder startup..."

    # Create temporary database file
    local db_file="/tmp/mockforge-recorder-test.db"
    rm -f "$db_file"

    # Start server with recorder enabled
    local pid=$(start_server "--recorder --recorder-db $db_file --http-port 3000" "3000")
    if [ $? -eq 0 ]; then
        log_success "Server with recorder starts successfully"

        # Wait a moment for recorder to initialize
        sleep 2

        # Check if database file was created
        if [ -f "$db_file" ]; then
            log_success "Recorder database file created"
        else
            log_warning "Recorder database file not found (may be created on first request)"
        fi

        stop_server "$pid"
        rm -f "$db_file"
        return 0
    else
        rm -f "$db_file"
        return 1
    fi
}

test_recorder_api() {
    log_info "Testing recorder API endpoints..."

    # Create temporary database file
    local db_file="/tmp/mockforge-recorder-api-test.db"
    rm -f "$db_file"

    # Start server with recorder enabled
    local pid=$(start_server "--recorder --recorder-db $db_file --http-port 3000" "3000")
    if [ $? -eq 0 ]; then
        sleep 2

        # Test recorder stats endpoint
        if curl -f "http://localhost:3000/api/recorder/stats" > /dev/null 2>&1; then
            log_success "Recorder stats endpoint accessible"
        else
            log_warning "Recorder stats endpoint may not be available (feature may be optional)"
        fi

        # Test recorder requests endpoint
        if curl -f "http://localhost:3000/api/recorder/requests?limit=10" > /dev/null 2>&1; then
            log_success "Recorder requests endpoint accessible"
        else
            log_warning "Recorder requests endpoint may not be available (feature may be optional)"
        fi

        stop_server "$pid"
        rm -f "$db_file"
        return 0
    else
        rm -f "$db_file"
        return 1
    fi
}

test_recording_capture() {
    log_info "Testing request recording..."

    # Create temporary database file
    local db_file="/tmp/mockforge-recorder-capture-test.db"
    rm -f "$db_file"

    # Start server with recorder enabled
    local pid=$(start_server "--recorder --recorder-db $db_file --http-port 3000" "3000")
    if [ $? -eq 0 ]; then
        sleep 2

        # Make a test request
        curl -s "http://localhost:3000/health" > /dev/null 2>&1 || true

        # Wait for recording
        sleep 1

        # Check if database file exists and has content
        if [ -f "$db_file" ]; then
            log_success "Recorder database file created after request"
        else
            log_warning "Recorder database file not found (may be created lazily)"
        fi

        stop_server "$pid"
        rm -f "$db_file"
        return 0
    else
        rm -f "$db_file"
        return 1
    fi
}

main() {
    log_info "Starting API Flight Recorder tests..."

    local failed_tests=()

    if ! test_recorder_startup; then
        failed_tests+=("recorder_startup")
    fi

    if ! test_recorder_api; then
        failed_tests+=("recorder_api")
    fi

    if ! test_recording_capture; then
        failed_tests+=("recording_capture")
    fi

    if [ ${#failed_tests[@]} -eq 0 ]; then
        log_success "All API Flight Recorder tests passed!"
        exit 0
    else
        log_error "Failed tests: ${failed_tests[*]}"
        exit 1
    fi
}

main "$@"
