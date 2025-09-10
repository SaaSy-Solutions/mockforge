#!/bin/bash

# MockForge Pre-commit Hook Setup Script
# This script sets up pre-commit hooks for code quality enforcement

set -e

echo "🚀 Setting up MockForge development environment..."

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Please run this script from the MockForge root directory"
    exit 1
fi

# Install pre-commit if not already installed
if ! command -v pre-commit &> /dev/null; then
    echo "📦 Installing pre-commit..."

    # Try different installation methods for different systems
    if command -v pipx &> /dev/null; then
        echo "📦 Using pipx to install pre-commit..."
        pipx install pre-commit
    elif command -v pacman &> /dev/null; then
        echo "📦 Installing pre-commit via pacman..."
        sudo pacman -S python-pre-commit
    elif command -v apt &> /dev/null; then
        echo "📦 Installing pre-commit via apt..."
        sudo apt update && sudo apt install -y pre-commit
    elif command -v brew &> /dev/null; then
        echo "📦 Installing pre-commit via Homebrew..."
        brew install pre-commit
    else
        echo "📦 Attempting pip install (may require virtual environment)..."
        if pip install --user pre-commit 2>/dev/null; then
            echo "✅ Pre-commit installed successfully with --user flag"
        elif python -m pip install --user pre-commit 2>/dev/null; then
            echo "✅ Pre-commit installed successfully with python -m pip"
        else
            echo "❌ Failed to install pre-commit automatically."
            echo ""
            echo "Please install pre-commit manually using one of these options:"
            echo ""
            echo "Option 1 - pipx (recommended for Arch Linux):"
            echo "  sudo pacman -S python-pipx"
            echo "  pipx install pre-commit"
            echo ""
            echo "Option 2 - pip with virtual environment:"
            echo "  python -m venv ~/.venv/mockforge"
            echo "  source ~/.venv/mockforge/bin/activate"
            echo "  pip install pre-commit"
            echo ""
            echo "Option 3 - System package manager:"
            echo "  sudo pacman -S python-pre-commit"
            echo ""
            echo "Option 4 - Force pip install (not recommended):"
            echo "  pip install --break-system-packages pre-commit"
            echo ""
            exit 1
        fi
    fi
fi

# Install the pre-commit hooks
echo "🔧 Installing pre-commit hooks..."
pre-commit install

# Install the commit-msg hook for conventional commits (optional)
echo "🔧 Installing commit-msg hook..."
pre-commit install --hook-type commit-msg

# Check for custom PyPI configuration that might cause issues
if pip config list | grep -q "index-url"; then
    echo "⚠️  Custom PyPI configuration detected. This may cause issues with pre-commit."
    echo "🔧 Attempting to run pre-commit with system Python to avoid pip config issues..."

    # Try to run pre-commit with system Python to bypass custom pip config
    if command -v python3 &> /dev/null; then
        echo "🔍 Running pre-commit checks with system Python..."
        PIP_INDEX_URL=https://pypi.org/simple/ pre-commit run --all-files || {
            echo "⚠️  Pre-commit run failed due to pip configuration."
            echo "📋 You can run checks manually later with:"
            echo "    PIP_INDEX_URL=https://pypi.org/simple/ pre-commit run --all-files"
            echo ""
            echo "✅ Pre-commit hooks are installed and will work on future commits."
        }
    else
        echo "⚠️  Skipping pre-commit run due to pip configuration issues."
        echo "📋 You can run checks manually later with:"
        echo "    PIP_INDEX_URL=https://pypi.org/simple/ pre-commit run --all-files"
    fi
else
    # No custom pip config, run normally
    echo "🔍 Running pre-commit checks on all files..."
    pre-commit run --all-files
fi

echo "✅ Pre-commit hooks setup complete!"
echo ""
echo "🎯 Next steps:"
echo "  - Run 'make check-all' to run all quality checks"
echo "  - Run 'make pre-commit' before committing"
echo "  - Pre-commit hooks will run automatically on each commit"
echo ""
echo "📚 Useful commands:"
echo "  - pre-commit run --all-files    # Run all checks manually"
echo "  - pre-commit run <hook-id>      # Run specific hook"
echo "  - pre-commit uninstall          # Remove hooks"
echo ""
echo "💡 If you encounter pip configuration issues, use:"
echo "  PIP_INDEX_URL=https://pypi.org/simple/ pre-commit run --all-files"
