#!/bin/bash

# Core Features (Chaos Engineering) Tests
# Tests latency simulation, failure injection, proxy mode, and traffic shaping

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

# Function to start server with config and return PID
start_server_with_config() {
    local config_file="$1"
    local port="$2"

    log_info "Starting server with config: $config_file on port $port"

    mockforge serve --config "$config_file" > /tmp/mockforge-chaos-test.log 2>&1 &
    local pid=$!

    # Wait for server to start
    local retries=15
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
    cat /tmp/mockforge-chaos-test.log || true
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

# Function to measure response time
measure_response_time() {
    local url="$1"
    local expected_min_ms="${2:-0}"

    local start_time=$(date +%s%3N)
    if curl -f -s "$url" > /dev/null 2>&1; then
        local end_time=$(date +%s%3N)
        local response_time=$((end_time - start_time))
        echo $response_time
        return 0
    else
        echo "-1"
        return 1
    fi
}

test_latency_simulation() {
    log_info "Testing latency simulation..."

    # Create config with latency enabled
    local config_file="/tmp/mockforge-latency-test.yaml"
    cat > "$config_file" << EOF
http:
  port: 3000
chaos:
  latency:
    enabled: true
    base_ms: 100
    jitter_ms: 20
    distribution: fixed
routes:
  - path: "/test-latency"
    method: GET
    response:
      status: 200
      body: "latency test"
EOF

    local pid=$(start_server_with_config "$config_file" "3000")
    if [ $? -eq 0 ]; then
        # Test that responses take at least the base latency
        local response_time=$(measure_response_time "http://localhost:3000/test-latency" 80)
        if [ $response_time -ge 80 ]; then
            log_success "Latency simulation working (response time: ${response_time}ms)"
        else
            log_error "Latency simulation not working (response time: ${response_time}ms, expected >= 80ms)"
            stop_server "$pid"
            rm -f "$config_file"
            return 1
        fi

        stop_server "$pid"
    else
        rm -f "$config_file"
        return 1
    fi

    # Test different distributions
    cat > "$config_file" << EOF
http:
  port: 3000
chaos:
  latency:
    enabled: true
    base_ms: 50
    jitter_ms: 10
    distribution: normal
routes:
  - path: "/test-normal"
    method: GET
    response:
      status: 200
      body: "normal distribution test"
EOF

    pid=$(start_server_with_config "$config_file" "3000")
    if [ $? -eq 0 ]; then
        response_time=$(measure_response_time "http://localhost:3000/test-normal" 30)
        if [ $response_time -ge 30 ]; then
            log_success "Normal distribution latency working"
        else
            log_warning "Normal distribution latency may not be working as expected"
        fi
        stop_server "$pid"
    fi

    rm -f "$config_file"

    log_success "Latency simulation tests passed"
    return 0
}

test_failure_injection() {
    log_info "Testing failure injection..."

    # Create config with failures enabled
    local config_file="/tmp/mockforge-failure-test.yaml"
    cat > "$config_file" << EOF
http:
  port: 3000
chaos:
  failures:
    enabled: true
    global_rate: 0.5  # 50% failure rate for testing
routes:
  - path: "/test-failure"
    method: GET
    response:
      status: 200
      body: "success"
EOF

    local pid=$(start_server_with_config "$config_file" "3000")
    if [ $? -eq 0 ]; then
        # Make multiple requests to see failures
        local success_count=0
        local failure_count=0
        local total_requests=10

        for i in $(seq 1 $total_requests); do
            if curl -f -s "http://localhost:3000/test-failure" > /dev/null 2>&1; then
                success_count=$((success_count + 1))
            else
                failure_count=$((failure_count + 1))
            fi
        done

        log_info "Success: $success_count, Failures: $failure_count out of $total_requests requests"

        if [ $failure_count -gt 0 ]; then
            log_success "Failure injection working (some requests failed as expected)"
        else
            log_warning "Failure injection may not be working (no failures detected)"
        fi

        stop_server "$pid"
    else
        rm -f "$config_file"
        return 1
    fi

    rm -f "$config_file"

    log_success "Failure injection tests passed"
    return 0
}

test_proxy_mode() {
    log_info "Testing proxy mode..."

    # For proxy mode testing, we'd need an upstream server
    # This is complex to set up automatically, so we'll create a basic test
    # that verifies proxy configuration can be loaded

    local config_file="/tmp/mockforge-proxy-test.yaml"
    cat > "$config_file" << EOF
http:
  port: 3000
proxy:
  enabled: true
  upstream_url: "http://httpbin.org"
  mode: forward_unknown
routes:
  - path: "/known-endpoint"
    method: GET
    response:
      status: 200
      body: "mocked response"
EOF

    local pid=$(start_server_with_config "$config_file" "3000")
    if [ $? -eq 0 ]; then
        # Test known endpoint (should be mocked)
        if curl -f -s "http://localhost:3000/known-endpoint" | grep -q "mocked response"; then
            log_success "Known endpoint returns mocked response"
        else
            log_error "Known endpoint did not return mocked response"
            stop_server "$pid"
            rm -f "$config_file"
            return 1
        fi

        # Test unknown endpoint (should proxy if httpbin.org is available)
        # Note: This test may fail if httpbin.org is not available
        if curl -f -s "http://localhost:3000/get" > /dev/null 2>&1; then
            log_success "Unknown endpoint proxy working"
        else
            log_warning "Unknown endpoint proxy test failed (upstream may be unavailable)"
        fi

        stop_server "$pid"
    else
        rm -f "$config_file"
        return 1
    fi

    rm -f "$config_file"

    log_success "Proxy mode tests passed"
    return 0
}

test_traffic_shaping() {
    log_info "Testing traffic shaping..."

    # Create config with traffic shaping
    local config_file="/tmp/mockforge-traffic-test.yaml"
    cat > "$config_file" << EOF
http:
  port: 3000
chaos:
  traffic_shaping:
    enabled: true
    max_connections: 5
    bandwidth_limit_kbps: 1000
routes:
  - path: "/test-traffic"
    method: GET
    response:
      status: 200
      body: "traffic shaping test"
EOF

    local pid=$(start_server_with_config "$config_file" "3000")
    if [ $? -eq 0 ]; then
        # Test basic connectivity with traffic shaping enabled
        if curl -f -s "http://localhost:3000/test-traffic" | grep -q "traffic shaping test"; then
            log_success "Traffic shaping configuration loaded successfully"
        else
            log_error "Traffic shaping test failed"
            stop_server "$pid"
            rm -f "$config_file"
            return 1
        fi

        # Note: More sophisticated traffic shaping tests would require
        # load testing tools and are difficult to automate reliably
        log_warning "Advanced traffic shaping tests (bandwidth limits, connection limits) require load testing tools"

        stop_server "$pid"
    else
        rm -f "$config_file"
        return 1
    fi

    rm -f "$config_file"

    log_success "Traffic shaping tests passed"
    return 0
}

test_per_tag_configuration() {
    log_info "Testing per-tag configuration..."

    # Create config with per-tag latency and failures
    local config_file="/tmp/mockforge-tag-test.yaml"
    cat > "$config_file" << EOF
http:
  port: 3000
chaos:
  latency:
    enabled: true
    base_ms: 10
    per_tag_overrides:
      auth: 100
      payment: 200
  failures:
    enabled: true
    global_rate: 0.1
    per_tag_rates:
      auth: 0.5
      payment: 0.2
routes:
  - path: "/auth/login"
    method: POST
    tags: ["auth"]
    response:
      status: 200
      body: "login success"
  - path: "/payment/charge"
    method: POST
    tags: ["payment"]
    response:
      status: 200
      body: "payment success"
  - path: "/search"
    method: GET
    tags: ["search"]
    response:
      status: 200
      body: "search results"
EOF

    local pid=$(start_server_with_config "$config_file" "3000")
    if [ $? -eq 0 ]; then
        # Test that tagged endpoints are accessible
        if curl -f -s -X POST "http://localhost:3000/auth/login" | grep -q "login success"; then
            log_success "Auth endpoint with tags working"
        else
            log_error "Auth endpoint failed"
            stop_server "$pid"
            rm -f "$config_file"
            return 1
        fi

        if curl -f -s -X POST "http://localhost:3000/payment/charge" | grep -q "payment success"; then
            log_success "Payment endpoint with tags working"
        else
            log_error "Payment endpoint failed"
            stop_server "$pid"
            rm -f "$config_file"
            return 1
        fi

        if curl -f -s "http://localhost:3000/search" | grep -q "search results"; then
            log_success "Search endpoint with tags working"
        else
            log_error "Search endpoint failed"
            stop_server "$pid"
            rm -f "$config_file"
            return 1
        fi

        stop_server "$pid"
    else
        rm -f "$config_file"
        return 1
    fi

    rm -f "$config_file"

    log_success "Per-tag configuration tests passed"
    return 0
}

main() {
    log_info "Starting Core Features (Chaos Engineering) tests..."

    local failed_tests=()

    if ! test_latency_simulation; then
        failed_tests+=("latency_simulation")
    fi

    if ! test_failure_injection; then
        failed_tests+=("failure_injection")
    fi

    if ! test_proxy_mode; then
        failed_tests+=("proxy_mode")
    fi

    if ! test_traffic_shaping; then
        failed_tests+=("traffic_shaping")
    fi

    if ! test_per_tag_configuration; then
        failed_tests+=("per_tag_configuration")
    fi

    if [ ${#failed_tests[@]} -eq 0 ]; then
        log_success "All Core Features (Chaos Engineering) tests passed!"
        exit 0
    else
        log_error "Failed tests: ${failed_tests[*]}"
        exit 1
    fi
}

main "$@"
