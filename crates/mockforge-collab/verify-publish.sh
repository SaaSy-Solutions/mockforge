#!/bin/bash
# Verification script to ensure .sqlx directory is included in published crate

set -e

echo "Verifying mockforge-collab package includes .sqlx directory..."

# Check that .sqlx directory exists
if [ ! -d ".sqlx" ]; then
    echo "❌ ERROR: .sqlx directory not found!"
    echo "   Run: cargo sqlx prepare --database-url <your-database-url>"
    exit 1
fi

# Count query cache files
QUERY_COUNT=$(ls .sqlx/*.json 2>/dev/null | wc -l)
if [ "$QUERY_COUNT" -eq 0 ]; then
    echo "❌ ERROR: No query cache files found in .sqlx/"
    echo "   Run: cargo sqlx prepare --database-url <your-database-url>"
    exit 1
fi

echo "✅ Found $QUERY_COUNT query cache files in .sqlx/"

# Check that Cargo.toml includes .sqlx
if ! grep -q '".sqlx/\*\*/\*"' Cargo.toml; then
    echo "❌ ERROR: Cargo.toml does not include .sqlx/**/*"
    exit 1
fi

echo "✅ Cargo.toml includes .sqlx/**/*"

# Check that .gitignore doesn't exclude .sqlx
if grep -q "^\.sqlx" .gitignore 2>/dev/null; then
    echo "⚠️  WARNING: .gitignore excludes .sqlx - this will prevent publishing the query cache"
    exit 1
fi

echo "✅ .sqlx is not ignored by .gitignore"

# Verify files are included in package
echo ""
echo "Checking package contents..."
PACKAGE_COUNT=$(cargo package --list 2>&1 | grep "\.sqlx/query-" | wc -l)

if [ "$PACKAGE_COUNT" -eq 0 ]; then
    echo "❌ ERROR: No .sqlx files found in package list"
    echo "   This means the published crate won't include query cache"
    exit 1
fi

echo "✅ Package includes $PACKAGE_COUNT .sqlx query cache files"
echo ""
echo "✅ All checks passed! The published crate will include the .sqlx directory."
echo "   Users installing from crates.io will be able to compile without a database connection."

