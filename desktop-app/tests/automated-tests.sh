#!/bin/bash
# Automated Cross-Platform Testing Script for MockForge Desktop
# This script runs automated tests across different platforms

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DESKTOP_APP_DIR="$PROJECT_ROOT/desktop-app"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test results
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running in CI environment
is_ci() {
    [ -n "$CI" ] || [ -n "$GITHUB_ACTIONS" ] || [ -n "$GITLAB_CI" ]
}

# Detect platform
detect_platform() {
    case "$(uname -s)" in
        Linux*)     echo "linux" ;;
        Darwin*)    echo "macos" ;;
        CYGWIN*|MINGW*|MSYS*) echo "windows" ;;
        *)          echo "unknown" ;;
    esac
}

PLATFORM=$(detect_platform)

log_info "Detected platform: $PLATFORM"
log_info "Project root: $PROJECT_ROOT"

# Test 1: Build verification
test_build() {
    log_info "Test 1: Verifying build..."

    cd "$DESKTOP_APP_DIR"

    if cargo check --release 2>&1 | tee /tmp/build.log; then
        log_info "✓ Build check passed"
        ((TESTS_PASSED++))
        return 0
    else
        log_error "✗ Build check failed"
        ((TESTS_FAILED++))
        return 1
    fi
}

# Test 2: Lint check
test_lint() {
    log_info "Test 2: Running linter..."

    cd "$DESKTOP_APP_DIR"

    if cargo clippy -- -D warnings 2>&1 | tee /tmp/lint.log; then
        log_info "✓ Lint check passed"
        ((TESTS_PASSED++))
        return 0
    else
        log_warn "✗ Lint check found issues (non-blocking)"
        ((TESTS_SKIPPED++))
        return 0  # Non-blocking
    fi
}

# Test 3: Unit tests
test_unit() {
    log_info "Test 3: Running unit tests..."

    cd "$DESKTOP_APP_DIR"

    if cargo test --lib 2>&1 | tee /tmp/unit-tests.log; then
        log_info "✓ Unit tests passed"
        ((TESTS_PASSED++))
        return 0
    else
        log_error "✗ Unit tests failed"
        ((TESTS_FAILED++))
        return 1
    fi
}

# Test 4: Frontend build
test_frontend_build() {
    log_info "Test 4: Verifying frontend build..."

    cd "$PROJECT_ROOT/crates/mockforge-ui/ui"

    if command -v pnpm &> /dev/null; then
        if pnpm build 2>&1 | tee /tmp/frontend-build.log; then
            log_info "✓ Frontend build passed"
            ((TESTS_PASSED++))
            return 0
        else
            log_error "✗ Frontend build failed"
            ((TESTS_FAILED++))
            return 1
        fi
    else
        log_warn "pnpm not found, skipping frontend build test"
        ((TESTS_SKIPPED++))
        return 0
    fi
}

# Test 5: Icon files exist
test_icons() {
    log_info "Test 5: Checking icon files..."

    ICON_DIR="$DESKTOP_APP_DIR/icons"
    MISSING_ICONS=0

    # Required icons
    REQUIRED_ICONS=(
        "icon.png"
        "32x32.png"
        "128x128.png"
    )

    # Platform-specific icons
    case "$PLATFORM" in
        windows)
            REQUIRED_ICONS+=("icon.ico")
            ;;
        macos)
            REQUIRED_ICONS+=("icon.icns")
            ;;
    esac

    for icon in "${REQUIRED_ICONS[@]}"; do
        if [ ! -f "$ICON_DIR/$icon" ]; then
            log_warn "Missing icon: $icon"
            ((MISSING_ICONS++))
        fi
    done

    if [ $MISSING_ICONS -eq 0 ]; then
        log_info "✓ All required icons present"
        ((TESTS_PASSED++))
        return 0
    else
        log_warn "✗ Missing $MISSING_ICONS icon(s) (non-blocking)"
        ((TESTS_SKIPPED++))
        return 0  # Non-blocking
    fi
}

# Test 6: Configuration files
test_config() {
    log_info "Test 6: Verifying configuration files..."

    CONFIG_FILES=(
        "$DESKTOP_APP_DIR/Cargo.toml"
        "$DESKTOP_APP_DIR/tauri.conf.json"
        "$DESKTOP_APP_DIR/build.rs"
    )

    MISSING=0
    for file in "${CONFIG_FILES[@]}"; do
        if [ ! -f "$file" ]; then
            log_error "Missing config file: $file"
            ((MISSING++))
        fi
    done

    if [ $MISSING -eq 0 ]; then
        log_info "✓ All configuration files present"
        ((TESTS_PASSED++))
        return 0
    else
        log_error "✗ Missing $MISSING configuration file(s)"
        ((TESTS_FAILED++))
        return 1
    fi
}

# Test 7: Tauri CLI availability
test_tauri_cli() {
    log_info "Test 7: Checking Tauri CLI..."

    if command -v cargo-tauri &> /dev/null; then
        log_info "✓ Tauri CLI found"
        ((TESTS_PASSED++))
        return 0
    else
        log_warn "Tauri CLI not found (install with: cargo install tauri-cli)"
        ((TESTS_SKIPPED++))
        return 0  # Non-blocking for CI
    fi
}

# Test 8: Dependencies check
test_dependencies() {
    log_info "Test 8: Checking dependencies..."

    MISSING_DEPS=0

    # Check Rust
    if ! command -v cargo &> /dev/null; then
        log_error "Rust/Cargo not found"
        ((MISSING_DEPS++))
    fi

    # Check Node.js (for frontend)
    if ! command -v node &> /dev/null; then
        log_warn "Node.js not found (needed for frontend)"
        ((MISSING_DEPS++))
    fi

    if [ $MISSING_DEPS -eq 0 ]; then
        log_info "✓ All critical dependencies present"
        ((TESTS_PASSED++))
        return 0
    else
        log_error "✗ Missing $MISSING_DEPS dependency(ies)"
        ((TESTS_FAILED++))
        return 1
    fi
}

# Test 9: File structure
test_file_structure() {
    log_info "Test 9: Verifying file structure..."

    REQUIRED_DIRS=(
        "$DESKTOP_APP_DIR/src"
        "$DESKTOP_APP_DIR/icons"
    )

    MISSING=0
    for dir in "${REQUIRED_DIRS[@]}"; do
        if [ ! -d "$dir" ]; then
            log_error "Missing directory: $dir"
            ((MISSING++))
        fi
    done

    if [ $MISSING -eq 0 ]; then
        log_info "✓ File structure correct"
        ((TESTS_PASSED++))
        return 0
    else
        log_error "✗ Missing $MISSING directory(ies)"
        ((TESTS_FAILED++))
        return 1
    fi
}

# Test 10: Platform-specific checks
test_platform_specific() {
    log_info "Test 10: Platform-specific checks..."

    case "$PLATFORM" in
        linux)
            # Check for WebKit2GTK
            if pkg-config --exists webkit2gtk-4.0 2>/dev/null; then
                log_info "✓ WebKit2GTK found"
            else
                log_warn "WebKit2GTK not found (install: sudo apt install libwebkit2gtk-4.0-dev)"
            fi
            ;;
        macos)
            # Check for Xcode Command Line Tools
            if xcode-select -p &> /dev/null; then
                log_info "✓ Xcode Command Line Tools found"
            else
                log_warn "Xcode Command Line Tools not found"
            fi
            ;;
        windows)
            # Check for Visual Studio Build Tools (basic check)
            log_info "Windows platform detected (manual verification needed)"
            ;;
    esac

    ((TESTS_PASSED++))
    return 0
}

# Run all tests
run_all_tests() {
    log_info "Starting automated cross-platform tests..."
    echo ""

    test_build
    test_lint
    test_unit
    test_frontend_build
    test_icons
    test_config
    test_tauri_cli
    test_dependencies
    test_file_structure
    test_platform_specific

    echo ""
    log_info "Test Summary:"
    log_info "  Passed: $TESTS_PASSED"
    log_info "  Failed: $TESTS_FAILED"
    log_info "  Skipped: $TESTS_SKIPPED"
    echo ""

    if [ $TESTS_FAILED -eq 0 ]; then
        log_info "✓ All critical tests passed!"
        return 0
    else
        log_error "✗ $TESTS_FAILED test(s) failed"
        return 1
    fi
}

# Main execution
main() {
    if [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
        echo "Usage: $0 [test_name]"
        echo ""
        echo "Available tests:"
        echo "  build              - Build verification"
        echo "  lint               - Lint check"
        echo "  unit               - Unit tests"
        echo "  frontend           - Frontend build"
        echo "  icons              - Icon files check"
        echo "  config             - Configuration files"
        echo "  tauri              - Tauri CLI check"
        echo "  deps               - Dependencies check"
        echo "  structure          - File structure"
        echo "  platform           - Platform-specific checks"
        echo "  all                - Run all tests (default)"
        exit 0
    fi

    if [ -n "$1" ] && [ "$1" != "all" ]; then
        # Run specific test
        case "$1" in
            build) test_build ;;
            lint) test_lint ;;
            unit) test_unit ;;
            frontend) test_frontend_build ;;
            icons) test_icons ;;
            config) test_config ;;
            tauri) test_tauri_cli ;;
            deps) test_dependencies ;;
            structure) test_file_structure ;;
            platform) test_platform_specific ;;
            *) log_error "Unknown test: $1"; exit 1 ;;
        esac
    else
        # Run all tests
        run_all_tests
    fi
}

main "$@"
