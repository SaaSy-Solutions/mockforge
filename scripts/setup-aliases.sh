#!/bin/bash
# Setup command aliases for MockForge
# This script creates convenient aliases like 'mf' for 'mockforge'

set -e

SHELL_RC=""
if [ -n "$ZSH_VERSION" ]; then
    SHELL_RC="$HOME/.zshrc"
elif [ -n "$BASH_VERSION" ]; then
    SHELL_RC="$HOME/.bashrc"
else
    echo "⚠️  Unsupported shell. Please manually add aliases to your shell configuration."
    exit 1
fi

echo "🔧 Setting up MockForge aliases..."

# Check if aliases already exist
if grep -q "# MockForge aliases" "$SHELL_RC" 2>/dev/null; then
    echo "✅ Aliases already configured in $SHELL_RC"
    echo "   Run 'source $SHELL_RC' or restart your terminal to use them"
    exit 0
fi

# Add aliases
cat >> "$SHELL_RC" << 'EOF'

# MockForge aliases
alias mf='mockforge'
alias mf-serve='mockforge serve'
alias mf-init='mockforge init'
alias mf-wizard='mockforge wizard'
alias mf-gen='mockforge generate'
EOF

echo "✅ Aliases added to $SHELL_RC"
echo ""
echo "📝 Added aliases:"
echo "   mf         → mockforge"
echo "   mf-serve   → mockforge serve"
echo "   mf-init    → mockforge init"
echo "   mf-wizard  → mockforge wizard"
echo "   mf-gen     → mockforge generate"
echo ""
echo "🔄 To use them now, run:"
echo "   source $SHELL_RC"
echo ""
echo "Or restart your terminal."
