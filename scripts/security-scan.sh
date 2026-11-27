#!/bin/bash
# Comprehensive security scanning script for MockForge
# Runs all security checks: RustSec, cargo-deny, clippy, and static analysis

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track if any checks failed
FAILED=0

echo "🔒 Running comprehensive security scan for MockForge..."
echo ""

# Check if required tools are installed
check_tool() {
    if ! command -v "$1" &> /dev/null; then
        echo -e "${YELLOW}⚠️  $1 not found. Installing...${NC}"
        case "$1" in
            cargo-audit)
                cargo install cargo-audit --locked
                ;;
            cargo-deny)
                cargo install cargo-deny --locked
                ;;
            *)
                echo -e "${RED}❌ Unknown tool: $1${NC}"
                return 1
                ;;
        esac
    fi
}

# 1. RustSec Advisory Database Scan (cargo-audit)
echo "1️⃣  Running RustSec advisory scan (cargo-audit)..."
if check_tool cargo-audit; then
    if cargo audit; then
        echo -e "${GREEN}✅ RustSec scan passed${NC}"
    else
        echo -e "${RED}❌ RustSec scan found vulnerabilities${NC}"
        FAILED=1
    fi
else
    echo -e "${YELLOW}⚠️  cargo-audit not available, skipping${NC}"
fi
echo ""

# 2. License and Source Compliance (cargo-deny)
echo "2️⃣  Running license and source compliance check (cargo-deny)..."
if check_tool cargo-deny; then
    if cargo deny check licenses sources bans; then
        echo -e "${GREEN}✅ License and source compliance passed${NC}"
    else
        echo -e "${RED}❌ License or source compliance issues found${NC}"
        FAILED=1
    fi
else
    echo -e "${YELLOW}⚠️  cargo-deny not available, skipping${NC}"
fi
echo ""

# 3. Security-focused Clippy lints
echo "3️⃣  Running security-focused Clippy checks..."
if cargo clippy --all-targets --all-features -- -W clippy::suspicious -W clippy::security -D warnings 2>&1 | tee /tmp/clippy-security.log; then
    echo -e "${GREEN}✅ Clippy security checks passed${NC}"
else
    echo -e "${RED}❌ Clippy security checks found issues${NC}"
    FAILED=1
fi
echo ""

# 4. Check for unsafe code blocks
echo "4️⃣  Checking for unsafe code blocks..."
UNSAFE_COUNT=$(find crates -name "*.rs" -type f -exec grep -l "unsafe" {} \; | wc -l)
if [ "$UNSAFE_COUNT" -gt 0 ]; then
    echo -e "${YELLOW}⚠️  Found $UNSAFE_COUNT files with unsafe blocks${NC}"
    echo "Files with unsafe code:"
    find crates -name "*.rs" -type f -exec grep -l "unsafe" {} \; | head -10
    echo "(Review these files to ensure unsafe code is properly documented and necessary)"
else
    echo -e "${GREEN}✅ No unsafe code blocks found${NC}"
fi
echo ""

# 5. Check for hardcoded secrets/credentials
echo "5️⃣  Scanning for potential hardcoded secrets..."
SECRET_PATTERNS=(
    "password\s*=\s*[\"'][^\"']+[\"']"
    "api[_-]?key\s*=\s*[\"'][^\"']+[\"']"
    "secret\s*=\s*[\"'][^\"']+[\"']"
    "token\s*=\s*[\"'][^\"']+[\"']"
    "sk-[a-zA-Z0-9]{32,}"
    "AKIA[0-9A-Z]{16}"
)

SECRET_FOUND=0
for pattern in "${SECRET_PATTERNS[@]}"; do
    if grep -r -i -E "$pattern" crates/ --include="*.rs" --exclude-dir=target 2>/dev/null | grep -v "test\|example\|mock" | head -5; then
        SECRET_FOUND=1
    fi
done

if [ "$SECRET_FOUND" -eq 0 ]; then
    echo -e "${GREEN}✅ No obvious hardcoded secrets found${NC}"
else
    echo -e "${YELLOW}⚠️  Potential hardcoded secrets found (review manually)${NC}"
    # Don't fail on this, just warn
fi
echo ""
# 5b. Keys of a known shape anywhere in the tree, and not only in the Rust sources:
# the table the exchange log and `range new` use, in a test.
echo "5b. Keys of a known shape in the whole tree..."
if cargo test -p mockforge-contracts --test keys_in_tree; then
    echo -e "${GREEN}No key of a known shape beyond the made-up examples the test names${NC}"
else
    echo -e "${RED}A value of a known key's shape is in the tree: look at it by hand${NC}"
    FAILED=1
fi
echo ""

# 6. Check for known vulnerable dependency patterns
echo "6️⃣  Checking for known vulnerable dependency patterns..."
# Check for specific vulnerable versions (add more as needed)
VULNERABLE_DEPS=(
    "serde = \"1.0.0\""  # Example - adjust based on actual advisories
)

VULN_FOUND=0
for dep in "${VULNERABLE_DEPS[@]}"; do
    if grep -r "$dep" Cargo.toml Cargo.lock 2>/dev/null; then
        echo -e "${RED}❌ Found potentially vulnerable dependency: $dep${NC}"
        VULN_FOUND=1
    fi
done

if [ "$VULN_FOUND" -eq 0 ]; then
    echo -e "${GREEN}✅ No known vulnerable dependency patterns found${NC}"
else
    FAILED=1
fi
echo ""

# 7. Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ "$FAILED" -eq 0 ]; then
    echo -e "${GREEN}✅ All security scans passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Security scans found issues. Please review and fix.${NC}"
    exit 1
fi
