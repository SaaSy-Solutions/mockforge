# Voice Workspace Creation Demo

This demo showcases the LLM Studio feature for creating complete MockForge workspaces from natural language descriptions.

## Demo Script

This script demonstrates the complete workflow from zero to a fully functional workspace.

## Prerequisites

- MockForge CLI installed
- LLM provider configured (Ollama, OpenAI, or Anthropic)
- Terminal with voice input capability (optional)

## Demo Flow

### 1. Start from Zero

Begin with a clean slate - no workspaces, no configuration.

### 2. Create Workspace via Voice/Text

Use natural language to describe the workspace:

```bash
mockforge voice create-workspace
```

**Voice/Text Input:**
```
Create an e-commerce workspace with customers, orders, and payments.
I need a happy path checkout, a failed payment path, and a slow-shipping scenario.
Make this 80% mock, 20% real prod for catalog only, with strict drift budget.
```

### 3. Review Preview

The system displays a preview showing:
- Workspace name and description
- Entities and their endpoints
- Personas with relationships
- Behavioral scenarios
- Reality continuum configuration
- Drift budget configuration

### 4. Confirm Creation

Review the preview and confirm to create the workspace.

### 5. View Creation Log

See detailed logs of what was created:
- Workspace created
- OpenAPI spec generated
- Personas created
- Scenarios created
- Reality continuum configured
- Drift budget configured

### 6. Verify Workspace

Access the created workspace and verify:
- Endpoints are available
- Personas are configured
- Scenarios can be executed
- Reality continuum settings are applied
- Drift budget is active

## Demo Script (Text Version)

For video recording, use this exact script:

```bash
#!/bin/bash

# Demo: Voice Workspace Creation
# This script demonstrates creating a complete workspace from natural language

echo "=== MockForge LLM Studio Demo ==="
echo ""
echo "We'll create a complete e-commerce workspace from a natural language description."
echo ""

# Create workspace
mockforge voice create-workspace \
  --command "Create an e-commerce workspace with customers, orders, and payments. I need a happy path checkout, a failed payment path, and a slow-shipping scenario. Make this 80% mock, 20% real prod for catalog only, with strict drift budget." \
  --yes

echo ""
echo "=== Demo Complete ==="
echo ""
echo "The workspace has been created with:"
echo "  • Endpoints for customers, orders, and payments"
echo "  • Personas with relationships"
echo "  • Behavioral scenarios (happy path, failure, slow path)"
echo "  • Reality continuum configuration"
echo "  • Drift budget configuration"
```

## Interactive Demo Script

For live demos with confirmation:

```bash
#!/bin/bash

# Interactive Demo: Voice Workspace Creation

echo "=== MockForge LLM Studio - Interactive Demo ==="
echo ""
echo "This demo will create a complete workspace from natural language."
echo ""

# Create workspace interactively
mockforge voice create-workspace

echo ""
echo "=== Demo Complete ==="
```

## Video Recording Tips

1. **Start Clean**: Begin with a fresh MockForge instance
2. **Clear Terminal**: Use a clean terminal window
3. **Show Preview**: Pause to show the preview screen
4. **Highlight Features**: Point out personas, scenarios, and configuration
5. **Show Creation Log**: Scroll through the creation log
6. **Verify Results**: Show the created workspace in the Admin UI

## Key Points to Highlight

1. **Natural Language Input**: Show how simple the description is
2. **Automatic Generation**: Emphasize that everything is generated automatically
3. **Complete Workspace**: Show that personas, scenarios, and configs are all created
4. **Validation**: Mention the guardrails and validation
5. **Reality Continuum**: Explain the mock-to-real blending
6. **Drift Budget**: Show contract monitoring configuration

## Example Output

```
🏗️  Workspace Creator - Natural Language to Complete Workspace

This will create a complete workspace with:
  • Endpoints and API structure
  • Personas with relationships
  • Behavioral scenarios (happy path, failure, slow path)
  • Reality continuum configuration
  • Drift budget configuration

📝 Command: Create an e-commerce workspace with customers, orders, and payments...
🤖 Parsing workspace creation command with LLM...
✅ Parsed command successfully

📋 Workspace Preview:
════════════════════════════════════════════════════════════
Name: E-commerce Workspace
Description: An e-commerce workspace with customers, orders, and payments

Entities: 3
  • Customer (3 endpoints)
  • Order (3 endpoints)
  • Payment (3 endpoints)

Personas: 3
  • premium-customer (2 relationships)
  • regular-customer (2 relationships)
  • vip-customer (2 relationships)

Scenarios: 3
  • happy-path-checkout (happy_path)
  • failed-payment (failure)
  • slow-shipping (slow_path)

Reality Continuum: Configured
Drift Budget: Configured
════════════════════════════════════════════════════════════

Create this workspace? [y/N]: y

🏗️  Creating workspace...

✅ Workspace created successfully!

📊 Creation Summary:
────────────────────────────────────────────────────────────
  Creating workspace: e-commerce-workspace
  ✓ Workspace 'e-commerce-workspace' created
  ✓ Generated OpenAPI spec with 9 endpoints
  ✓ Created 3 personas
  ✓ Created 3 scenarios
  ✓ Reality continuum configured
  ✓ Drift budget configured
────────────────────────────────────────────────────────────

📦 Workspace Details:
  ID: e-commerce-workspace
  Name: E-commerce Workspace
  OpenAPI Spec: 9 endpoints
  Personas: 3
  Scenarios: 3
  Reality Continuum: Enabled
  Drift Budget: Configured

🎉 Workspace 'e-commerce-workspace' is ready to use!

💡 Next steps:
  • Start the MockForge server to use this workspace
  • Access the workspace via: /workspace/e-commerce-workspace
  • View personas and scenarios in the Admin UI
```
