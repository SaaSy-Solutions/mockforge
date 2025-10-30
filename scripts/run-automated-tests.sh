#!/bin/bash

# MockForge Automated Testing Script
# This script runs automated tests for all sections in MANUAL_TESTING_CHECKLIST.md

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to cleanup running processes and ports
cleanup_processes() {
    log_info "Cleaning up running processes and ports..."

    # Kill any running mockforge processes owned by current user (with timeout)
    timeout 10 pkill -u $(id -u) -f mockforge 2>/dev/null || true

    # Wait a moment for processes to terminate
    sleep 2

    # Kill any remaining processes on test ports (may require root, with timeout per port)
    for port in 3000 3001 50051 9080 1025 1026 9092 1883 5672 2121; do
        timeout 5 fuser -k ${port}/tcp 2>/dev/null || true
    done

    # Clean up any Docker containers (with timeout)
    timeout 30 docker ps -q --filter "name=mockforge" | xargs -r docker rm -f 2>/dev/null || true

    # Wait for ports to be freed
    sleep 3

    log_info "Cleanup completed"
}

# Function to run a test section
run_test_section() {
    local section_name="$1"
    local script_path="$2"

    # Cleanup before each test section
    cleanup_processes

    log_info "Running $section_name tests..."
    if [ -f "$script_path" ]; then
        if bash "$script_path"; then
            log_success "$section_name tests passed"
            return 0
        else
            log_error "$section_name tests failed"
            return 1
        fi
    else
        log_warning "$section_name test script not found: $script_path"
        return 1
    fi
}

# Main function
main() {
    local base_dir="scripts"
    local project_root="."
    local test_scripts_dir="$base_dir/automated-tests"

    log_info "Starting MockForge automated testing"
    log_info "Test scripts directory: $test_scripts_dir"

    cd "$project_root" || {
        log_error "Failed to change to project root directory"
        exit 1
    }

    # Create test scripts directory if it doesn't exist
    mkdir -p "$test_scripts_dir"

    local failed_sections=()

    # Installation & Setup
    if ! run_test_section "Installation & Setup" "$test_scripts_dir/test-installation.sh"; then
        failed_sections+=("Installation & Setup")
    fi

    # Post-Release Testing
    if ! run_test_section "Post-Release Testing" "$test_scripts_dir/test-post-release.sh"; then
        failed_sections+=("Post-Release Testing")
    fi

    # HTTP/REST Server
    if ! run_test_section "HTTP/REST Server" "$test_scripts_dir/test-http.sh"; then
        failed_sections+=("HTTP/REST Server")
    fi

    # WebSocket Server
    if ! run_test_section "WebSocket Server" "$test_scripts_dir/test-websocket.sh"; then
        failed_sections+=("WebSocket Server")
    fi

    # gRPC Server
    if ! run_test_section "gRPC Server" "$test_scripts_dir/test-grpc.sh"; then
        failed_sections+=("gRPC Server")
    fi

    # GraphQL Server
    if ! run_test_section "GraphQL Server" "$test_scripts_dir/test-graphql.sh"; then
        failed_sections+=("GraphQL Server")
    fi

    # SMTP Email Testing
    if ! run_test_section "SMTP Email Testing" "$test_scripts_dir/test-smtp.sh"; then
        failed_sections+=("SMTP Email Testing")
    fi

    # Kafka Broker
    if ! run_test_section "Kafka Broker" "$test_scripts_dir/test-kafka.sh"; then
        failed_sections+=("Kafka Broker")
    fi

    # MQTT Broker
    if ! run_test_section "MQTT Broker" "$test_scripts_dir/test-mqtt.sh"; then
        failed_sections+=("MQTT Broker")
    fi

    # AMQP Broker
    if ! run_test_section "AMQP Broker" "$test_scripts_dir/test-amqp.sh"; then
        failed_sections+=("AMQP Broker")
    fi

    # FTP Server
    if ! run_test_section "FTP Server" "$test_scripts_dir/test-ftp.sh"; then
        failed_sections+=("FTP Server")
    fi

    # API Flight Recorder
    if ! run_test_section "API Flight Recorder" "$test_scripts_dir/test-recorder.sh"; then
        failed_sections+=("API Flight Recorder")
    fi

    # Data Generation
    if ! run_test_section "Data Generation" "$test_scripts_dir/test-data-generation.sh"; then
        failed_sections+=("Data Generation")
    fi

    # AI-Powered Features
    if ! run_test_section "AI-Powered Features" "$test_scripts_dir/test-ai.sh"; then
        failed_sections+=("AI-Powered Features")
    fi

    # Plugin System
    if ! run_test_section "Plugin System" "$test_scripts_dir/test-plugins.sh"; then
        failed_sections+=("Plugin System")
    fi

    # Security & Encryption
    if ! run_test_section "Security & Encryption" "$test_scripts_dir/test-security.sh"; then
        failed_sections+=("Security & Encryption")
    fi

    # Workspace Synchronization
    if ! run_test_section "Workspace Synchronization" "$test_scripts_dir/test-workspace-sync.sh"; then
        failed_sections+=("Workspace Synchronization")
    fi

    # Admin UI
    if ! run_test_section "Admin UI" "$test_scripts_dir/test-admin-ui.sh"; then
        failed_sections+=("Admin UI")
    fi

    # Core Features (Chaos Engineering)
    if ! run_test_section "Core Features (Chaos Engineering)" "$test_scripts_dir/test-chaos.sh"; then
        failed_sections+=("Core Features (Chaos Engineering)")
    fi

    # Observability
    if ! run_test_section "Observability" "$test_scripts_dir/test-observability.sh"; then
        failed_sections+=("Observability")
    fi

    # Advanced Features
    if ! run_test_section "Advanced Features" "$test_scripts_dir/test-advanced.sh"; then
        failed_sections+=("Advanced Features")
    fi

    # Import/Export Features
    if ! run_test_section "Import/Export Features" "$test_scripts_dir/test-import-export.sh"; then
        failed_sections+=("Import/Export Features")
    fi

    # CLI Commands
    if ! run_test_section "CLI Commands" "$test_scripts_dir/test-cli.sh"; then
        failed_sections+=("CLI Commands")
    fi

    # Environment Variables
    if ! run_test_section "Environment Variables" "$test_scripts_dir/test-env-vars.sh"; then
        failed_sections+=("Environment Variables")
    fi

    # Docker Testing
    if ! run_test_section "Docker Testing" "$test_scripts_dir/test-docker.sh"; then
        failed_sections+=("Docker Testing")
    fi

    # Performance & Load Testing
    if ! run_test_section "Performance & Load Testing" "$test_scripts_dir/test-performance.sh"; then
        failed_sections+=("Performance & Load Testing")
    fi

    # Edge Cases & Error Handling
    if ! run_test_section "Edge Cases & Error Handling" "$test_scripts_dir/test-edge-cases.sh"; then
        failed_sections+=("Edge Cases & Error Handling")
    fi

    # Documentation Verification
    if ! run_test_section "Documentation Verification" "$test_scripts_dir/test-docs.sh"; then
        failed_sections+=("Documentation Verification")
    fi

    # Final Checks
    if ! run_test_section "Final Checks" "$test_scripts_dir/test-final-checks.sh"; then
        failed_sections+=("Final Checks")
    fi

    # Summary
    if [ ${#failed_sections[@]} -eq 0 ]; then
        log_success "All automated tests passed!"
        exit 0
    else
        log_error "The following test sections failed:"
        for section in "${failed_sections[@]}"; do
            echo -e "  - $section"
        done
        exit 1
    fi
}

# Run main function
main "$@"
