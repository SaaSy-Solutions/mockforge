#!/bin/bash

# MockForge LLM Studio - Interactive Demo Script
# This script demonstrates the interactive workspace creation workflow

set -e

echo "=== MockForge LLM Studio - Interactive Demo ==="
echo ""
echo "This demo will guide you through creating a workspace from natural language."
echo "You can use voice input or type your description."
echo ""

# Check if mockforge is available
if ! command -v mockforge &> /dev/null; then
    echo "❌ Error: mockforge CLI not found. Please install MockForge first."
    exit 1
fi

echo "Starting interactive workspace creation..."
echo ""

# Create workspace interactively (will prompt for input)
mockforge voice create-workspace

echo ""
echo "=== Demo Complete ==="
echo ""
