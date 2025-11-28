# AI-First Onboarding

**Pillars:** [AI]

[AI] - LLM/voice flows, AI diff/assist, generative behaviors

## Start Here If...

You want **natural-language-driven mocks**. You want to generate mocks from descriptions, use voice commands, and leverage AI to automate mock creation and enhance data realism.

Perfect for:
- Teams wanting to generate mocks from natural language descriptions
- Developers who prefer conversational interfaces
- Teams needing AI-powered contract analysis
- Organizations wanting to automate mock generation

## Quick Start: 5 Minutes

Let's create a mock API using natural language:

```bash
# Install MockForge
cargo install mockforge-cli

# Start MockForge with AI features enabled
mockforge serve --ai-enabled

# Use MockAI to generate a mock from natural language
curl -X POST http://localhost:3000/__mockforge/ai/generate \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "Create a REST API for a todo app with endpoints to list, create, update, and delete todos. Todos have id, title, description, completed status, and created date."
  }'
```

Or use the voice interface:

```bash
# Start voice mode
mockforge voice

# Say: "Create a REST API for a todo app with CRUD operations"
```

## Key AI Features

### 1. MockAI - Natural Language Mock Generation

Generate complete mock APIs from natural language descriptions:

```bash
# Generate a mock API
curl -X POST http://localhost:3000/__mockforge/ai/generate \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "Create a user management API with endpoints for registration, login, and profile management"
  }'
```

**Why it matters:** Create mocks instantly without writing YAML or code. Describe what you need in plain English, and MockAI generates the complete API.

**Learn more:** [MockAI Guide](../../docs/MOCKAI_USAGE.md)

### 2. Voice + LLM Interface

Build mocks using voice commands:

```bash
# Start voice mode
mockforge voice

# Example commands:
# "Create a REST API for an e-commerce store"
# "Add an endpoint to get product details by ID"
# "Generate realistic product data with names, prices, and descriptions"
```

**Why it matters:** Build mocks hands-free using natural language. Perfect for rapid prototyping and iterative development.

**Learn more:** [Voice Interface Guide](../../docs/MOCKAI_USAGE.md#voice-interface)

### 3. AI Contract Diff

Intelligently analyze and compare contract changes:

```yaml
ai_contract_diff:
  enabled: true
  llm_provider: openai
  analysis_depth: detailed
  generate_recommendations: true
```

**Why it matters:** Understand the impact of contract changes with AI-powered analysis. Get recommendations for handling breaking changes.

**Learn more:** [AI Contract Diff Guide](../../docs/DRIFT_BUDGETS.md#ai-contract-diff)

### 4. Generative Schema Mode

Generate complete API ecosystems from JSON examples:

```yaml
generative_schema:
  enabled: true
  examples:
    - path: ./examples/user.json
      entity_type: user
    - path: ./examples/order.json
      entity_type: order
```

**Why it matters:** Create entire API ecosystems from a few example JSON payloads. Automatically infer routes, relationships, and schemas.

**Learn more:** [Generative Schema Guide](../../docs/AI_SCHEMA_EXTRAPOLATION.md)

### 5. AI Event Streams

Generate narrative-driven WebSocket events:

```yaml
websocket:
  ai_streams:
    enabled: true
    narrative: "A user browsing an e-commerce site, adding items to cart, and completing a purchase"
    event_types:
      - page_view
      - product_view
      - add_to_cart
      - checkout
```

**Why it matters:** Create realistic, narrative-driven event streams for testing real-time features.

**Learn more:** [AI Event Streams Guide](../../docs/AI_DRIVEN_MOCKING.md)

## Next Steps

### Explore AI Features

1. **MockAI**: [Complete Guide](../../docs/MOCKAI_USAGE.md)
   - Generate mocks from natural language
   - Refine and iterate on generated mocks
   - Understand AI generation rules

2. **Voice Interface**: [Complete Guide](../../docs/MOCKAI_USAGE.md#voice-interface)
   - Set up voice commands
   - Use conversational mode
   - Generate OpenAPI specs from voice

3. **AI Contract Diff**: [Complete Guide](../../docs/DRIFT_BUDGETS.md#ai-contract-diff)
   - Enable AI analysis
   - Understand recommendations
   - Handle breaking changes

4. **Generative Schema**: [Complete Guide](../../docs/AI_SCHEMA_EXTRAPOLATION.md)
   - Generate APIs from JSON examples
   - Configure entity relationships
   - Customize route generation

5. **AI Event Streams**: [Complete Guide](../../docs/AI_DRIVEN_MOCKING.md)
   - Create narrative-driven events
   - Configure event types
   - Test real-time features

### Related Pillars

Once you've mastered AI, explore these complementary pillars:

- **[Reality]** - Enhance AI-generated mocks with realistic behavior
  - [Reality-First Onboarding](reality-first.md)
  - [Smart Personas Guide](../../docs/PERSONAS.md)

- **[Contracts]** - Add validation to AI-generated mocks
  - [Contracts-First Onboarding](contracts-first.md)
  - [Validation Guide](../../user-guide/http-mocking/openapi.md)

- **[DevX]** - Improve your AI workflow with SDKs and tools
  - [DevX Features](../../user-guide/http-mocking.md)
  - [CLI Reference](../../reference/cli.md)

## Examples

### Example 1: Natural Language Mock Generation

```bash
# Generate a complete e-commerce API
curl -X POST http://localhost:3000/__mockforge/ai/generate \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "Create a REST API for an e-commerce store with products, cart, and checkout endpoints. Products have id, name, price, description, and stock. Cart items link to products and have quantity."
  }'
```

### Example 2: Voice-Driven Development

```bash
# Start voice mode
mockforge voice

# Interactive session:
# You: "Create a user authentication API"
# MockForge: "I've created a user authentication API with /register and /login endpoints. Would you like to add password reset?"
# You: "Yes, add password reset with email verification"
# MockForge: "Added /reset-password and /verify-email endpoints. The API is ready!"
```

### Example 3: AI-Enhanced Personas

```yaml
reality:
  personas:
    enabled: true
    ai_generation: true
    ai_config:
      provider: openai
      model: gpt-4
      generate_relationships: true
      generate_lifecycle_states: true
```

### Example 4: Generative Schema from Examples

```yaml
generative_schema:
  enabled: true
  examples:
    - path: ./examples/user.json
      entity_type: user
      routes:
        - GET /users
        - GET /users/{id}
        - POST /users
        - PUT /users/{id}
        - DELETE /users/{id}
    - path: ./examples/order.json
      entity_type: order
      relationships:
        - field: user_id
          links_to: user.id
```

## Troubleshooting

### AI Generation Not Working

Ensure AI features are enabled:

```yaml
ai:
  enabled: true
  provider: openai  # or anthropic, etc.
  api_key: ${OPENAI_API_KEY}
```

### Voice Interface Not Responding

Check your microphone permissions and voice configuration:

```yaml
voice:
  enabled: true
  provider: openai
  speech_to_text: true
```

### AI Contract Diff Not Analyzing

Verify your LLM provider configuration:

```yaml
ai_contract_diff:
  enabled: true
  llm_provider: openai
  api_key: ${OPENAI_API_KEY}
```

## Resources

- [Complete Pillars Documentation](../../docs/PILLARS.md)
- [AI Features Overview](../../docs/PILLARS.md#ai--llmvoice-flows-ai-diffassist-generative-behaviors)
- [MockAI Guide](../../docs/MOCKAI_USAGE.md)
- [API Reference](../../api/rust.md)
- [Examples Repository](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples)

---

**Ready to dive deeper?** Continue to the [MockAI Guide](../../docs/MOCKAI_USAGE.md) or explore [all AI features](../../docs/PILLARS.md#ai--llmvoice-flows-ai-diffassist-generative-behaviors).

