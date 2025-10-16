#!/bin/bash

# Manual publishing script for MockForge crates
# This script publishes crates one by one in the correct order

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
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

# Function to publish a single crate
publish_single_crate() {
    local crate_name=$1

    print_status "Publishing $crate_name..."

    # Convert dependencies
    convert_crate_dependencies "$crate_name"

    # Publish the crate
    if cargo publish -p "$crate_name" --allow-dirty; then
        print_success "Successfully published $crate_name"
    else
        print_error "Failed to publish $crate_name"
        exit 1
    fi

    # Wait a bit for crates.io to process
    print_status "Waiting 30s for crates.io to process..."
    sleep 30
}

print_status "MockForge Manual Publishing Script"
print_status "=================================="

# Check for token
if [ -z "$CRATES_IO_TOKEN" ]; then
    print_error "CRATES_IO_TOKEN environment variable is not set!"
    print_status "Please set it with: export CRATES_IO_TOKEN=your_token_here"
    exit 1
fi

# Publish crates in order
print_status "Phase 1: Publishing base crates..."

# mockforge-core (already published)
print_warning "mockforge-core already published, skipping..."

# mockforge-data (already published)
print_warning "mockforge-data already published, skipping..."

# mockforge-plugin-core
publish_single_crate "mockforge-plugin-core"

# mockforge-plugin-sdk
publish_single_crate "mockforge-plugin-sdk"

print_status "Phase 2: Publishing dependent crates..."

# mockforge-plugin-loader
publish_single_crate "mockforge-plugin-loader"

# mockforge-http
publish_single_crate "mockforge-http"

# mockforge-grpc
publish_single_crate "mockforge-grpc"

# mockforge-ws
publish_single_crate "mockforge-ws"

# mockforge-graphql
publish_single_crate "mockforge-graphql"

# mockforge-mqtt
publish_single_crate "mockforge-mqtt"

# mockforge-smtp
publish_single_crate "mockforge-smtp"

# mockforge-amqp
publish_single_crate "mockforge-amqp"

# mockforge-kafka
publish_single_crate "mockforge-kafka"

# mockforge-ftp
publish_single_crate "mockforge-ftp"

# mockforge-bench
publish_single_crate "mockforge-bench"

# mockforge-k8s-operator
publish_single_crate "mockforge-k8s-operator"

# mockforge-registry-server
publish_single_crate "mockforge-registry-server"

print_success "All crates published successfully!"
print_status ""
print_warning "Remember to restore path dependencies for development:"
print_status "./scripts/publish-crates.sh --restore"
