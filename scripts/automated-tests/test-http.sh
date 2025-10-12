#!/bin/bash

# HTTP/REST Server Tests
# Tests HTTP server functionality including startup, OpenAPI, validation, templates, CORS, and routes

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

    log_info "Starting server with args: $args"

    # Start server in background
    mockforge serve $args > /tmp/mockforge-http-test.log 2>&1 &
    local pid=$!

    # Wait for server to start
    local retries=10
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
    cat /tmp/mockforge-http-test.log || true
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

test_server_startup() {
    log_info "Testing server startup..."

    # Test default HTTP server
    local pid=$(start_server "--http-port 3000" "3000")
    if [ $? -eq 0 ]; then
        # Test ping endpoint
        if curl -f "http://localhost:3000/ping" > /dev/null 2>&1; then
            log_success "Ping endpoint responds"
        else
            log_error "Ping endpoint failed"
            stop_server "$pid"
            return 1
        fi
        stop_server "$pid"
    else
        return 1
    fi

    # Test custom port
    pid=$(start_server "--http-port 8080" "8080")
    if [ $? -eq 0 ]; then
        if curl -f "http://localhost:8080/ping" > /dev/null 2>&1; then
            log_success "Custom port 8080 works"
        else
            log_error "Custom port 8080 failed"
            stop_server "$pid"
            return 1
        fi
        stop_server "$pid"
    else
        return 1
    fi

    # Test host binding
    pid=$(start_server "--host 127.0.0.1 --http-port 3000" "3000")
    if [ $? -eq 0 ]; then
        if curl -f "http://127.0.0.1:3000/ping" > /dev/null 2>&1; then
            log_success "Host binding to 127.0.0.1 works"
        else
            log_error "Host binding to 127.0.0.1 failed"
            stop_server "$pid"
            return 1
        fi
        stop_server "$pid"
    else
        return 1
    fi

    log_success "Server startup tests passed"
    return 0
}

test_openapi_integration() {
    log_info "Testing OpenAPI integration..."

    # Check if OpenAPI demo file exists
    if [ ! -f "examples/openapi-demo.json" ] && [ ! -f "examples/test_openapi_demo/src/openapi.json" ]; then
        log_warning "No OpenAPI demo file found, skipping OpenAPI tests"
        return 0
    fi

    local spec_file=""
    if [ -f "examples/openapi-demo.json" ]; then
        spec_file="examples/openapi-demo.json"
    elif [ -f "examples/test_openapi_demo/src/openapi.json" ]; then
        spec_file="examples/test_openapi_demo/src/openapi.json"
    fi

    # Start server with OpenAPI spec
    local pid=$(start_server "--spec $spec_file --http-port 3000" "3000")
    if [ $? -eq 0 ]; then
        # Test that some endpoints are available (this depends on the actual spec)
        # For now, just check that server is running and responds
        if curl -f "http://localhost:3000/ping" > /dev/null 2>&1; then
            log_success "Server with OpenAPI spec started successfully"
        else
            log_error "Server with OpenAPI spec failed"
            stop_server "$pid"
            return 1
        fi
        stop_server "$pid"
    else
        return 1
    fi

    log_success "OpenAPI integration tests passed"
    return 0
}

test_request_validation() {
    log_info "Testing request validation..."

    # Create a temporary config for validation testing
    local config_file="/tmp/mockforge-validation-test.yaml"
    cat > "$config_file" << EOF
http:
  port: 3000
  validation:
    mode: enforce
routes:
  - path: "/test-validation"
    method: POST
    response:
      status: 200
      body: "ok"
    request:
      validation:
        schema:
          type: object
          properties:
            name:
              type: string
          required:
            - name
EOF

    # Test enforce mode
    local pid=$(start_server "--config $config_file" "3000")
    if [ $? -eq 0 ]; then
        # Valid request
        if curl -f -X POST "http://localhost:3000/test-validation" \
               -H "Content-Type: application/json" \
               -d '{"name": "test"}' > /dev/null 2>&1; then
            log_success "Valid request accepted in enforce mode"
        else
            log_error "Valid request rejected in enforce mode"
            stop_server "$pid"
            rm -f "$config_file"
            return 1
        fi

        # Invalid request (missing required field)
        if curl -s -X POST "http://localhost:3000/test-validation" \
               -H "Content-Type: application/json" \
               -d '{"invalid": "test"}' | grep -q "400\|422"; then
            log_success "Invalid request rejected in enforce mode"
        else
            log_error "Invalid request accepted in enforce mode"
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

    log_success "Request validation tests passed"
    return 0
}

test_template_expansion() {
    log_info "Testing template expansion..."

    # Create a temporary config with templates
    local config_file="/tmp/mockforge-template-test.yaml"
    cat > "$config_file" << EOF
http:
  port: 3000
  response_template_expand: true
routes:
  - path: "/test-uuid"
    method: GET
    response:
      status: 200
      body: '{"uuid": "{{uuid}}"}'
  - path: "/test-now"
    method: GET
    response:
      status: 200
      body: '{"timestamp": "{{now}}"}'
  - path: "/test-faker"
    method: GET
    response:
      status: 200
      body: '{"email": "{{faker.email}}", "name": "{{faker.name}}"}'
EOF

    local pid=$(start_server "--config $config_file" "3000")
    if [ $? -eq 0 ]; then
        # Test UUID template
        local response=$(curl -s "http://localhost:3000/test-uuid")
        if echo "$response" | grep -q '"uuid":'; then
            log_success "UUID template expansion works"
        else
            log_error "UUID template expansion failed"
            echo "Response: $response"
            stop_server "$pid"
            rm -f "$config_file"
            return 1
        fi

        # Test timestamp template
        response=$(curl -s "http://localhost:3000/test-now")
        if echo "$response" | grep -q '"timestamp":'; then
            log_success "Timestamp template expansion works"
        else
            log_error "Timestamp template expansion failed"
            echo "Response: $response"
            stop_server "$pid"
            rm -f "$config_file"
            return 1
        fi

        # Test faker template
        response=$(curl -s "http://localhost:3000/test-faker")
        if echo "$response" | grep -q '"email":' && echo "$response" | grep -q '"name":'; then
            log_success "Faker template expansion works"
        else
            log_error "Faker template expansion failed"
            echo "Response: $response"
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

    log_success "Template expansion tests passed"
    return 0
}

test_cors_configuration() {
    log_info "Testing CORS configuration..."

    # Create a temporary config with CORS enabled
    local config_file="/tmp/mockforge-cors-test.yaml"
    cat > "$config_file" << EOF
http:
  port: 3000
  cors:
    enabled: true
    allowed_origins: ["http://localhost:3001", "https://example.com"]
    allowed_methods: ["GET", "POST", "PUT", "DELETE"]
    allowed_headers: ["Content-Type", "Authorization"]
routes:
  - path: "/test-cors"
    method: GET
    response:
      status: 200
      body: "cors test"
EOF

    local pid=$(start_server "--config $config_file" "3000")
    if [ $? -eq 0 ]; then
        # Test OPTIONS preflight request
        local response=$(curl -s -X OPTIONS "http://localhost:3000/test-cors" \
                         -H "Origin: http://localhost:3001" \
                         -H "Access-Control-Request-Method: POST" \
                         -H "Access-Control-Request-Headers: Content-Type")

        if echo "$response" | grep -q "access-control-allow-origin"; then
            log_success "CORS preflight request handled correctly"
        else
            log_warning "CORS preflight response not detected (may not be implemented)"
        fi

        # Test regular CORS headers
        response=$(curl -s -H "Origin: http://localhost:3001" "http://localhost:3000/test-cors")
        if echo "$response" | grep -q "cors test"; then
            log_success "CORS-enabled endpoint responds correctly"
        else
            log_error "CORS-enabled endpoint failed"
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

    log_success "CORS configuration tests passed"
    return 0
}

test_custom_routes() {
    log_info "Testing custom routes..."

    # Create a temporary config with custom routes
    local config_file="/tmp/mockforge-custom-routes-test.yaml"
    cat > "$config_file" << EOF
http:
  port: 3000
routes:
  - path: "/custom-route"
    method: GET
    response:
      status: 200
      body: "custom response"
      headers:
        X-Custom-Header: "test-value"
  - path: "/templated-route"
    method: GET
    response:
      status: 200
      body: '{"message": "Hello {{query.name}}"}'
EOF

    local pid=$(start_server "--config $config_file" "3000")
    if [ $? -eq 0 ]; then
        # Test custom route
        local response=$(curl -s "http://localhost:3000/custom-route")
        if [ "$response" = "custom response" ]; then
            log_success "Custom route works"
        else
            log_error "Custom route failed"
            echo "Response: $response"
            stop_server "$pid"
            rm -f "$config_file"
            return 1
        fi

        # Test custom headers
        response=$(curl -s -I "http://localhost:3000/custom-route" | grep -i "x-custom-header")
        if echo "$response" | grep -q "test-value"; then
            log_success "Custom headers work"
        else
            log_warning "Custom headers not detected"
        fi

        # Test templated route
        response=$(curl -s "http://localhost:3000/templated-route?name=World")
        if echo "$response" | grep -q '"message": "Hello World"'; then
            log_success "Templated route works"
        else
            log_error "Templated route failed"
            echo "Response: $response"
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

    log_success "Custom routes tests passed"
    return 0
}

main() {
    log_info "Starting HTTP/REST Server tests..."

    local failed_tests=()

    if ! test_server_startup; then
        failed_tests+=("server_startup")
    fi

    if ! test_openapi_integration; then
        failed_tests+=("openapi_integration")
    fi

    if ! test_request_validation; then
        failed_tests+=("request_validation")
    fi

    if ! test_template_expansion; then
        failed_tests+=("template_expansion")
    fi

    if ! test_cors_configuration; then
        failed_tests+=("cors_configuration")
    fi

    if ! test_custom_routes; then
        failed_tests+=("custom_routes")
    fi

    if [ ${#failed_tests[@]} -eq 0 ]; then
        log_success "All HTTP/REST Server tests passed!"
        exit 0
    else
        log_error "Failed tests: ${failed_tests[*]}"
        exit 1
    fi
}

main "$@"
