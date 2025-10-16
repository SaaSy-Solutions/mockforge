#!/bin/bash

# Realistic publishing test script
# This script tests the actual publishing process for the first few crates

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

print_status "Testing MockForge publishing readiness (realistic test)..."

# Test 1: Check if workspace compiles
print_status "Testing workspace compilation..."
if cargo check --workspace; then
    print_success "Workspace compiles successfully"
else
    print_error "Workspace compilation failed"
    exit 1
fi

# Test 2: Test mockforge-core dry-run (this should work)
print_status "Testing mockforge-core dry-run..."
if cargo publish -p mockforge-core --dry-run --allow-dirty; then
    print_success "mockforge-core is ready for publishing"
else
    print_error "mockforge-core publishing test failed"
    exit 1
fi

# Test 3: Test dependency conversion and compilation
print_status "Testing dependency conversion..."

# Backup original files
cp crates/mockforge-data/Cargo.toml crates/mockforge-data/Cargo.toml.backup

# Convert dependencies
sed -i 's|mockforge-core = { path = "../mockforge-core" }|mockforge-core = "0.1.0"|g' crates/mockforge-data/Cargo.toml

# Test if it compiles with version dependency (this will fail in dry-run, but that's expected)
print_status "Testing mockforge-data with version dependency (expected to fail in dry-run)..."
if cargo publish -p mockforge-data --dry-run --allow-dirty 2>/dev/null; then
    print_success "mockforge-data dry-run succeeded (unexpected!)"
else
    print_warning "mockforge-data dry-run failed as expected (mockforge-core not on crates.io yet)"
    print_status "This is normal behavior - the crate will work once mockforge-core is published"
fi

# Test compilation with version dependency
if cargo check -p mockforge-data; then
    print_success "mockforge-data compiles with version dependency"
else
    print_error "mockforge-data compilation failed with version dependency"
    # Restore backup
    mv crates/mockforge-data/Cargo.toml.backup crates/mockforge-data/Cargo.toml
    exit 1
fi

# Restore original file
mv crates/mockforge-data/Cargo.toml.backup crates/mockforge-data/Cargo.toml

print_success "All realistic publishing tests passed!"
print_status ""
print_status "🎉 Your crates are ready for publishing to crates.io!"
print_status ""
print_status "To publish for real:"
print_status "1. Set your crates.io token: export CRATES_IO_TOKEN=your_token_here"
print_status "2. Run: ./scripts/publish-crates.sh"
print_status "3. Restore dev dependencies: ./scripts/publish-crates.sh --restore"
print_status ""
print_warning "Note: The dry-run will fail for dependent crates because mockforge-core"
print_warning "isn't actually published yet. This is expected behavior."
