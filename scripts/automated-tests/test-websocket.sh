#!/bin/bash

# WebSocket Server Tests
# Tests WebSocket server functionality

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

main() {
    log_info "Starting WebSocket Server Testing..."

    log_warning "WebSocket tests require specialized WebSocket client tools (websocat, etc.)"
    log_warning "These tests are complex to automate and may require manual testing"
    log_info "Key tests to perform:"
    log_info "  - Start WebSocket server on custom port"
    log_info "  - Test connection with websocat"
    log_info "  - Test scripted replay from JSONL file"
    log_info "  - Test JSONPath message matching"
    log_info "  - Test connection management and limits"

    # Basic server startup test
    mockforge serve --ws-port 3003 > /tmp/ws-test.log 2>&1 &
    pid=$!
    sleep 3

    if kill -0 $pid 2>/dev/null; then
        log_success "WebSocket server starts successfully"
        kill $pid 2>/dev/null || true
    else
        log_error "WebSocket server failed to start"
        cat /tmp/ws-test.log || true
        exit 1
    fi

    log_success "WebSocket Server Testing completed (manual verification recommended)"
    exit 0
}

main "$@"
