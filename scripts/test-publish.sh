#!/bin/bash

# Simple test script to verify publishing readiness
# This script tests the first few crates without actually publishing

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

print_status "Testing MockForge publishing readiness..."

# Test 1: Check if workspace compiles
print_status "Testing workspace compilation..."
if cargo check --workspace; then
    print_success "Workspace compiles successfully"
else
    print_error "Workspace compilation failed"
    exit 1
fi

# Test 2: Test mockforge-core dry-run
print_status "Testing mockforge-core dry-run..."
if cargo publish -p mockforge-core --dry-run --allow-dirty; then
    print_success "mockforge-core is ready for publishing"
else
    print_error "mockforge-core publishing test failed"
    exit 1
fi

# Test 3: Test dependency conversion
print_status "Testing dependency conversion..."

# Backup original files
cp crates/mockforge-data/Cargo.toml crates/mockforge-data/Cargo.toml.backup

# Convert dependencies
sed -i 's|mockforge-core = { path = "../mockforge-core" }|mockforge-core = "0.1.0"|g' crates/mockforge-data/Cargo.toml

# Test if it compiles with version dependency
if cargo check -p mockforge-data; then
    print_success "Dependency conversion works correctly"
else
    print_error "Dependency conversion failed"
    # Restore backup
    mv crates/mockforge-data/Cargo.toml.backup crates/mockforge-data/Cargo.toml
    exit 1
fi

# Restore original file
mv crates/mockforge-data/Cargo.toml.backup crates/mockforge-data/Cargo.toml

print_success "All publishing readiness tests passed!"
print_status "Your crates are ready for publishing to crates.io"
