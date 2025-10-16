#!/bin/bash

# MockForge Crates Publishing Script
# This script publishes crates to crates.io in the correct dependency order

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
DRY_RUN=${DRY_RUN:-false}
WAIT_TIME=${WAIT_TIME:-30}  # Seconds to wait between publishes
CRATES_IO_TOKEN=${CRATES_IO_TOKEN:-""}

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if crates.io token is set
check_token() {
    if [ -z "$CRATES_IO_TOKEN" ]; then
        print_error "CRATES_IO_TOKEN environment variable is not set!"
        print_status "Please set it with: export CRATES_IO_TOKEN=your_token_here"
        print_status "Get your token from: https://crates.io/me"
        exit 1
    fi
}

# Function to wait for crates.io to process
wait_for_processing() {
    if [ "$DRY_RUN" = "false" ]; then
        print_status "Waiting ${WAIT_TIME}s for crates.io to process..."
        sleep $WAIT_TIME
    fi
}

# Function to check if a crate already exists on crates.io
crate_exists() {
    local crate_name=$1
    local version="0.1.0"  # Current version from workspace

    # Check if the crate exists by trying to get its info
    if cargo search "$crate_name" --limit 1 | grep -q "^$crate_name ="; then
        return 0  # Crate exists
    else
        return 1  # Crate doesn't exist
    fi
}

# Function to publish a crate
publish_crate() {
    local crate_name=$1
    local dry_run_flag=""

    if [ "$DRY_RUN" = "true" ]; then
        dry_run_flag="--dry-run"
        print_status "DRY RUN: Would publish $crate_name"
    else
        # Check if crate already exists
        if crate_exists "$crate_name"; then
            print_warning "$crate_name already exists on crates.io, skipping..."
            return 0
        fi

        print_status "Publishing $crate_name..."
    fi

    if cargo publish -p "$crate_name" $dry_run_flag --allow-dirty; then
        print_success "Successfully published $crate_name"
    else
        # Check if the error is because the crate already exists
        if [ "$DRY_RUN" = "false" ] && cargo publish -p "$crate_name" --dry-run --allow-dirty 2>&1 | grep -q "already exists"; then
            print_warning "$crate_name already exists on crates.io, skipping..."
            return 0
        else
            print_error "Failed to publish $crate_name"
            exit 1
        fi
    fi
}

# Function to convert dependencies for a specific crate
convert_crate_dependencies() {
    local crate_name=$1
    local cargo_toml="crates/$crate_name/Cargo.toml"

    if [ -f "$cargo_toml" ]; then
        print_status "Converting dependencies for $crate_name..."
        # Convert mockforge-core path dependency to version dependency
        sed -i 's|mockforge-core = { path = "../mockforge-core" }|mockforge-core = "0.1.0"|g' "$cargo_toml"
        # Convert mockforge-data path dependency to version dependency (if it exists)
        sed -i 's|mockforge-data = { path = "../mockforge-data" }|mockforge-data = "0.1.0"|g' "$cargo_toml"
        # Convert mockforge-plugin-core path dependency to version dependency (if it exists)
        sed -i 's|mockforge-plugin-core = { path = "../mockforge-plugin-core" }|mockforge-plugin-core = "0.1.0"|g' "$cargo_toml"
    fi
}

# Function to convert path dependencies to version dependencies (legacy - converts all at once)
convert_dependencies() {
    print_status "Converting path dependencies to version dependencies..."

    # List of crates that need dependency conversion
    local crates_to_convert=(
        "mockforge-data"
        "mockforge-http"
        "mockforge-grpc"
        "mockforge-ws"
        "mockforge-graphql"
        "mockforge-mqtt"
        "mockforge-smtp"
        "mockforge-amqp"
        "mockforge-kafka"
        "mockforge-ftp"
        "mockforge-bench"
        "mockforge-plugin-loader"
        "mockforge-k8s-operator"
        "mockforge-registry-server"
    )

    for crate in "${crates_to_convert[@]}"; do
        convert_crate_dependencies "$crate"
    done

    print_success "Dependency conversion completed"
}

# Function to restore path dependencies (for development)
restore_dependencies() {
    print_status "Restoring path dependencies for development..."

    local crates_to_restore=(
        "mockforge-data"
        "mockforge-http"
        "mockforge-grpc"
        "mockforge-ws"
        "mockforge-graphql"
        "mockforge-mqtt"
        "mockforge-smtp"
        "mockforge-amqp"
        "mockforge-kafka"
        "mockforge-ftp"
        "mockforge-bench"
        "mockforge-plugin-loader"
        "mockforge-k8s-operator"
        "mockforge-registry-server"
    )

    for crate in "${crates_to_restore[@]}"; do
        local cargo_toml="crates/$crate/Cargo.toml"
        if [ -f "$cargo_toml" ]; then
            # Restore mockforge-core path dependency
            sed -i 's|mockforge-core = "0.1.0"|mockforge-core = { path = "../mockforge-core" }|g' "$cargo_toml"
            # Restore mockforge-data path dependency
            sed -i 's|mockforge-data = "0.1.0"|mockforge-data = { path = "../mockforge-data" }|g' "$cargo_toml"
            # Restore mockforge-plugin-core path dependency
            sed -i 's|mockforge-plugin-core = "0.1.0"|mockforge-plugin-core = { path = "../mockforge-plugin-core" }|g' "$cargo_toml"
        fi
    done

    print_success "Path dependencies restored"
}

# Function to show usage
show_usage() {
    echo "MockForge Crates Publishing Script"
    echo ""
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --dry-run              Run in dry-run mode (don't actually publish)"
    echo "  --convert-only         Only convert dependencies, don't publish"
    echo "  --restore              Restore path dependencies for development"
    echo "  --resume               Resume publishing (skip already published crates)"
    echo "  --wait-time SECONDS    Wait time between publishes (default: 30)"
    echo "  --help                 Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  CRATES_IO_TOKEN        Your crates.io API token (required for publishing)"
    echo "  DRY_RUN                Set to 'true' for dry-run mode"
    echo "  WAIT_TIME              Wait time between publishes in seconds"
    echo ""
    echo "Examples:"
    echo "  $0 --dry-run                    # Test the publishing process"
    echo "  $0 --convert-only               # Only convert dependencies"
    echo "  $0 --restore                    # Restore development dependencies"
    echo "  $0 --resume                     # Resume publishing (skip existing crates)"
    echo "  DRY_RUN=true $0                 # Dry run using environment variable"
    echo ""
    echo "Resumable Publishing:"
    echo "  The script can be run multiple times safely. It will:"
    echo "  - Skip crates that already exist on crates.io"
    echo "  - Continue from where it left off"
    echo "  - Handle dependency conversion automatically"
}

# Main execution
main() {
    print_status "MockForge Crates Publishing Script"
    print_status "=================================="

    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --convert-only)
                CONVERT_ONLY=true
                shift
                ;;
            --restore)
                RESTORE_ONLY=true
                shift
                ;;
            --resume)
                RESUME=true
                shift
                ;;
            --wait-time)
                WAIT_TIME="$2"
                shift 2
                ;;
            --help)
                show_usage
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done

    # Handle restore-only mode
    if [ "$RESTORE_ONLY" = "true" ]; then
        restore_dependencies
        exit 0
    fi

    # Handle convert-only mode
    if [ "$CONVERT_ONLY" = "true" ]; then
        convert_dependencies
        exit 0
    fi

    # Check for crates.io token if not in dry-run mode
    if [ "$DRY_RUN" = "false" ]; then
        check_token
    fi

    # Phase 1: Publish base crates (no internal dependencies)
    print_status "Phase 1: Publishing base crates..."

    publish_crate "mockforge-core"
    wait_for_processing

    # Convert dependencies for mockforge-data and publish it
    convert_crate_dependencies "mockforge-data"
    publish_crate "mockforge-data"
    wait_for_processing

    # Convert dependencies for mockforge-plugin-core and publish it
    convert_crate_dependencies "mockforge-plugin-core"
    publish_crate "mockforge-plugin-core"
    wait_for_processing

    # Convert dependencies for mockforge-plugin-sdk and publish it
    convert_crate_dependencies "mockforge-plugin-sdk"
    publish_crate "mockforge-plugin-sdk"
    wait_for_processing

    # Phase 2: Publish remaining dependent crates
    print_status "Phase 2: Publishing remaining dependent crates..."

    # Publish plugin system crates
    convert_crate_dependencies "mockforge-plugin-loader"
    publish_crate "mockforge-plugin-loader"
    wait_for_processing

    # Publish protocol crates
    convert_crate_dependencies "mockforge-http"
    publish_crate "mockforge-http"
    wait_for_processing

    convert_crate_dependencies "mockforge-grpc"
    publish_crate "mockforge-grpc"
    wait_for_processing

    convert_crate_dependencies "mockforge-ws"
    publish_crate "mockforge-ws"
    wait_for_processing

    convert_crate_dependencies "mockforge-graphql"
    publish_crate "mockforge-graphql"
    wait_for_processing

    convert_crate_dependencies "mockforge-mqtt"
    publish_crate "mockforge-mqtt"
    wait_for_processing

    convert_crate_dependencies "mockforge-smtp"
    publish_crate "mockforge-smtp"
    wait_for_processing

    convert_crate_dependencies "mockforge-amqp"
    publish_crate "mockforge-amqp"
    wait_for_processing

    convert_crate_dependencies "mockforge-kafka"
    publish_crate "mockforge-kafka"
    wait_for_processing

    convert_crate_dependencies "mockforge-ftp"
    publish_crate "mockforge-ftp"
    wait_for_processing

    # Publish utility crates
    convert_crate_dependencies "mockforge-bench"
    publish_crate "mockforge-bench"
    wait_for_processing

    convert_crate_dependencies "mockforge-k8s-operator"
    publish_crate "mockforge-k8s-operator"
    wait_for_processing

    convert_crate_dependencies "mockforge-registry-server"
    publish_crate "mockforge-registry-server"
    wait_for_processing

    print_success "All crates published successfully!"

    if [ "$DRY_RUN" = "false" ]; then
        print_warning "Remember to restore path dependencies for development:"
        print_status "$0 --restore"
    fi
}

# Run main function
main "$@"
