#!/bin/bash

# Script to publish remaining MockForge crates
# This script publishes crates in dependency order

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
        # Convert mockforge-plugin-sdk path dependency to version dependency (if it exists)
        sed -i 's|mockforge-plugin-sdk = { path = "../mockforge-plugin-sdk" }|mockforge-plugin-sdk = "0.1.0"|g' "$cargo_toml"
        # Convert mockforge-plugin-loader path dependency to version dependency (if it exists)
        sed -i 's|mockforge-plugin-loader = { path = "../mockforge-plugin-loader" }|mockforge-plugin-loader = "0.1.0"|g' "$cargo_toml"
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

print_status "MockForge Remaining Crates Publishing Script"
print_status "============================================="

# Check for token
if [ -z "$CRATES_IO_TOKEN" ]; then
    print_error "CRATES_IO_TOKEN environment variable is not set!"
    print_status "Please set it with: export CRATES_IO_TOKEN=your_token_here"
    exit 1
fi

# Already published: mockforge-core, mockforge-data, mockforge-plugin-core, mockforge-plugin-sdk, mockforge-plugin-loader, mockforge-ftp
print_warning "Already published: mockforge-core, mockforge-data, mockforge-plugin-core, mockforge-plugin-sdk, mockforge-plugin-loader, mockforge-ftp"

# Publish remaining crates in dependency order
print_status "Publishing remaining crates..."

# Crates that only depend on mockforge-core
publish_single_crate "mockforge-smtp"
publish_single_crate "mockforge-mqtt"
publish_single_crate "mockforge-amqp"
publish_single_crate "mockforge-kafka"

# Crates that depend on mockforge-core and mockforge-data
publish_single_crate "mockforge-graphql"
publish_single_crate "mockforge-grpc"
publish_single_crate "mockforge-ws"

# Crates that depend on multiple internal crates
publish_single_crate "mockforge-http"

# Utility crates
publish_single_crate "mockforge-bench"

print_success "All remaining crates published successfully!"
print_status ""
print_warning "Remember to restore path dependencies for development:"
print_status "./scripts/publish-crates.sh --restore"
