#!/bin/bash

# MockForge LLM Studio Demo Script
# This script demonstrates creating a complete workspace from natural language

set -e

echo "=== MockForge LLM Studio Demo ==="
echo ""
echo "This demo showcases creating a complete workspace from natural language."
echo "We'll create an e-commerce workspace with customers, orders, and payments."
echo ""

# Check if mockforge is available
if ! command -v mockforge &> /dev/null; then
    echo "❌ Error: mockforge CLI not found. Please install MockForge first."
    exit 1
fi

echo "📝 Creating workspace from natural language description..."
echo ""

# Create workspace with auto-confirm
mockforge voice create-workspace \
  --command "Create an e-commerce workspace with customers, orders, and payments. I need a happy path checkout, a failed payment path, and a slow-shipping scenario. Make this 80% mock, 20% real prod for catalog only, with strict drift budget." \
  --yes

echo ""
echo "=== Demo Complete ==="
echo ""
echo "✅ Workspace created successfully!"
echo ""
echo "The workspace includes:"
echo "  • Endpoints for customers, orders, and payments"
echo "  • Personas with relationships"
echo "  • Behavioral scenarios (happy path, failure, slow path)"
echo "  • Reality continuum configuration (80% mock, 20% real for catalog)"
echo "  • Drift budget configuration (strict)"
echo ""
echo "Next steps:"
echo "  1. Start MockForge server: mockforge serve"
echo "  2. Access workspace via Admin UI"
echo "  3. View personas and scenarios"
echo "  4. Test the generated endpoints"
echo ""
