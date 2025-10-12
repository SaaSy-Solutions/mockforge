#!/bin/bash

# CLI Commands Tests
# Tests various MockForge CLI commands

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

# Function to test a CLI command
test_cli_command() {
    local description="$1"
    local command="$2"
    local expected_exit="${3:-0}"  # Default expected exit code is 0

    log_info "Testing: $description"
    log_info "Command: $command"

    if eval "$command"; then
        local actual_exit=$?
        if [ $actual_exit -eq $expected_exit ]; then
            log_success "$description passed"
            return 0
        else
            log_error "$description failed (expected exit code $expected_exit, got $actual_exit)"
            return 1
        fi
    else
        local actual_exit=$?
        if [ $actual_exit -eq $expected_exit ]; then
            log_success "$description passed (expected failure)"
            return 0
        else
            log_error "$description failed (unexpected exit code $actual_exit)"
            return 1
        fi
    fi
}

test_server_commands() {
    log_info "Testing server commands..."

    # Test mockforge --version (basic functionality)
    if ! test_cli_command "mockforge --version" "mockforge --version"; then
        return 1
    fi

    # Test mockforge --help
    if ! test_cli_command "mockforge --help" "mockforge --help"; then
        return 1
    fi

    # Test config validate (should work with default or example config)
    if [ -f "config.example.yaml" ]; then
        if ! test_cli_command "mockforge config validate with example config" "mockforge config validate --config config.example.yaml"; then
            return 1
        fi
    else
        log_warning "config.example.yaml not found, skipping config validate test"
    fi

    log_success "Server commands tests passed"
    return 0
}

test_data_commands() {
    log_info "Testing data commands..."

    # Test data template user
    if ! test_cli_command "mockforge data template user" "mockforge data template user --rows 5 --output /tmp/test-users.json"; then
        return 1
    fi

    # Verify output file was created and has content
    if [ -f "/tmp/test-users.json" ]; then
        local line_count=$(wc -l < /tmp/test-users.json)
        if [ $line_count -gt 1 ]; then
            log_success "User data generation created valid output"
        else
            log_error "User data generation output seems invalid"
            return 1
        fi
        rm -f /tmp/test-users.json
    else
        log_error "User data generation did not create output file"
        return 1
    fi

    # Test data template product with CSV format
    if ! test_cli_command "mockforge data template product CSV" "mockforge data template product --rows 3 --format csv --output /tmp/test-products.csv"; then
        return 1
    fi

    # Verify CSV output
    if [ -f "/tmp/test-products.csv" ]; then
        if grep -q "," /tmp/test-products.csv; then
            log_success "Product data generation CSV format works"
        else
            log_error "Product data generation CSV format seems invalid"
            return 1
        fi
        rm -f /tmp/test-products.csv
    else
        log_error "Product data generation did not create CSV file"
        return 1
    fi

    # Test data template order
    if ! test_cli_command "mockforge data template order" "mockforge data template order --rows 2 --output /tmp/test-orders.json"; then
        return 1
    fi

    if [ -f "/tmp/test-orders.json" ]; then
        rm -f /tmp/test-orders.json
        log_success "Order data generation works"
    else
        log_error "Order data generation did not create output file"
        return 1
    fi

    log_success "Data commands tests passed"
    return 0
}

test_config_commands() {
    log_info "Testing config commands..."

    # Test config validate with minimal config
    local temp_config="/tmp/test-config.yaml"
    echo "http:" > "$temp_config"
    echo "  port: 3000" >> "$temp_config"

    if ! test_cli_command "mockforge config validate minimal config" "mockforge config validate --config $temp_config"; then
        rm -f "$temp_config"
        return 1
    fi

    rm -f "$temp_config"

    # Test config diff (if two configs exist)
    if [ -f "config.example.yaml" ] && [ -f "config.dev.yaml" ]; then
        if ! test_cli_command "mockforge config diff" "mockforge config diff config.example.yaml config.dev.yaml"; then
            return 1
        fi
    else
        log_warning "Need both config.example.yaml and config.dev.yaml for diff test, skipping"
    fi

    log_success "Config commands tests passed"
    return 0
}

test_plugin_commands() {
    log_info "Testing plugin commands..."

    # Test plugin list (should work even with no plugins)
    if ! test_cli_command "mockforge plugin list" "mockforge plugin list"; then
        return 1
    fi

    # Note: plugin install/remove tests would require actual plugin URLs
    # and might have side effects, so we'll skip them in automated tests
    log_warning "Skipping plugin install/remove tests (require actual plugins)"

    log_success "Plugin commands tests passed"
    return 0
}

test_sync_commands() {
    log_info "Testing sync commands..."

    # Test sync status (should work even if no sync is running)
    if ! test_cli_command "mockforge sync status" "mockforge sync status"; then
        return 1
    fi

    # Note: sync start/stop tests would require setting up a directory
    # and might leave processes running, so we'll skip them
    log_warning "Skipping sync start/stop tests (would leave processes running)"

    log_success "Sync commands tests passed"
    return 0
}

test_ai_commands() {
    log_info "Testing AI test commands..."

    # Test AI commands if available (these might require API keys or special setup)
    # We'll test that the commands exist and show help/error appropriately

    # Test intelligent-mock help
    if mockforge test-ai intelligent-mock --help > /dev/null 2>&1; then
        log_success "mockforge test-ai intelligent-mock command available"
    else
        log_warning "mockforge test-ai intelligent-mock not available or requires setup"
    fi

    # Test drift help
    if mockforge test-ai drift --help > /dev/null 2>&1; then
        log_success "mockforge test-ai drift command available"
    else
        log_warning "mockforge test-ai drift not available or requires setup"
    fi

    # Test event-stream help
    if mockforge test-ai event-stream --help > /dev/null 2>&1; then
        log_success "mockforge test-ai event-stream command available"
    else
        log_warning "mockforge test-ai event-stream not available or requires setup"
    fi

    log_success "AI commands tests passed"
    return 0
}

main() {
    log_info "Starting CLI Commands tests..."

    local failed_tests=()

    if ! test_server_commands; then
        failed_tests+=("server_commands")
    fi

    if ! test_data_commands; then
        failed_tests+=("data_commands")
    fi

    if ! test_config_commands; then
        failed_tests+=("config_commands")
    fi

    if ! test_plugin_commands; then
        failed_tests+=("plugin_commands")
    fi

    if ! test_sync_commands; then
        failed_tests+=("sync_commands")
    fi

    if ! test_ai_commands; then
        failed_tests+=("ai_commands")
    fi

    if [ ${#failed_tests[@]} -eq 0 ]; then
        log_success "All CLI Commands tests passed!"
        exit 0
    else
        log_error "Failed tests: ${failed_tests[*]}"
        exit 1
    fi
}

main "$@"
