# AI Behavioral Simulation Engine

**Pillars:** [AI]

The AI Behavioral Simulation Engine models users as narrative agents that react to app state, form intentions, respond to errors, and trigger multi-step interactions automatically. MockForge becomes **an AI user simulator**—not just an API simulator.

## Overview

Traditional mocks return static responses. The Behavioral Simulation Engine creates **narrative agents** that:

- **React to app state** (e.g., "cart is empty" → intention: "browse products")
- **Form intentions** (shop, browse, buy, abandon, retry, navigate, search, compare, review)
- **Respond to errors** (rage clicking on 500 errors, retry logic, cart abandonment on payment failure)
- **Trigger multi-step interactions** automatically
- **Maintain session context** across interactions

## Key Concepts

### Narrative Agents

Agents are AI-powered personas that simulate user behavior:

```yaml
agent:
  agent_id: "agent-123"
  persona_id: "customer:premium-001"
  current_intention: "shop"
  behavioral_traits:
    patience: 0.7
    risk_tolerance: 0.5
    price_sensitivity: 0.3
  state_awareness:
    cart_empty: true
    last_error: null
    session_duration: 120
```

### Intentions

Agents form intentions based on state:

- **Browse**: Explore products/content
- **Shop**: Actively looking to purchase
- **Buy**: Ready to complete purchase
- **Abandon**: Leave due to frustration/error
- **Retry**: Retry after error
- **Navigate**: Move to different section
- **Search**: Search for something
- **Compare**: Compare options
- **Review**: Review/read content

### Behavior Policies

Policies define how agents behave:

- **bargain-hunter**: Price-sensitive, compares options
- **power-user**: Efficient, knows the system
- **churn-risk**: Frustrated, likely to abandon
- **new-user**: Exploring, needs guidance

## Usage

### Create Agent

```bash
# Create agent from persona
mockforge ai behavioral-sim create-agent \
  --persona customer:premium-001 \
  --policy power-user

# Or via API
POST /api/v1/ai-studio/simulate-behavior/create-agent
{
  "persona_id": "customer:premium-001",
  "behavior_policy": "power-user",
  "generate_persona": false
}
```

### Simulate Behavior

```bash
# Simulate agent behavior
mockforge ai behavioral-sim simulate \
  --agent agent-123 \
  --state '{"cart_empty": true}' \
  --trigger "user_landed_on_homepage"
```

### Via UI

1. Navigate to AI Studio
2. Select "Behavioral Simulation"
3. Create or select agent
4. Set app state
5. Trigger event
6. Observe agent behavior

## Example Workflows

### Example 1: Cart Abandonment

**State:**
```json
{
  "cart_empty": true,
  "last_action": "viewed_product",
  "session_duration": 300
}
```

**Trigger:** `user_viewed_product`

**Agent Behavior:**
- Intention: `browse`
- Action: `GET /api/products?category=electronics`
- Reasoning: "User is browsing, likely looking for products to add to cart"

### Example 2: Payment Failure Response

**State:**
```json
{
  "cart_total": 99.99,
  "payment_attempts": 1,
  "last_error": "payment_failed"
}
```

**Trigger:** `payment_failed`

**Agent Behavior:**
- Intention: `retry`
- Action: `POST /api/payments/retry`
- Reasoning: "User will retry payment once, then abandon if it fails again"

### Example 3: Error Rage Clicking

**State:**
```json
{
  "error_count": 3,
  "last_error": "500",
  "session_duration": 60
}
```

**Trigger:** `error_occurred`

**Agent Behavior:**
- Intention: `abandon`
- Action: `navigate_away`
- Reasoning: "Multiple errors frustrate user, likely to abandon"

## Behavior Policies

### Bargain Hunter

```yaml
behavior_policy: bargain-hunter
traits:
  price_sensitivity: 0.9
  comparison_tendency: 0.8
  patience: 0.6
behaviors:
  - always_compares_prices
  - waits_for_discounts
  - abandons_on_high_price
```

### Power User

```yaml
behavior_policy: power-user
traits:
  efficiency: 0.9
  system_knowledge: 0.8
  patience: 0.7
behaviors:
  - uses_shortcuts
  - expects_fast_responses
  - retries_on_errors
```

### Churn Risk

```yaml
behavior_policy: churn-risk
traits:
  frustration: 0.8
  patience: 0.3
  loyalty: 0.4
behaviors:
  - abandons_on_errors
  - sensitive_to_latency
  - likely_to_churn
```

## Integration with Personas

Agents can be attached to existing Smart Personas:

```yaml
persona:
  id: customer:premium-001
  traits:
    subscription_tier: premium
    spending_level: high
  behavioral_simulation:
    enabled: true
    policy: power-user
```

## Configuration

### Enable Behavioral Simulation

```yaml
# mockforge.yaml
ai_studio:
  behavioral_simulation:
    enabled: true
    default_policy: power-user
    llm:
      provider: openai
      model: gpt-4
      temperature: 0.7
```

## Best Practices

1. **Start with Policies**: Use pre-built policies before creating custom
2. **Test Scenarios**: Test agents with various app states
3. **Monitor Behavior**: Track agent behavior patterns
4. **Refine Policies**: Adjust policies based on observed behavior
5. **Combine with Personas**: Use Smart Personas for realistic agents

## Related Documentation

- [Smart Personas](smart-personas.md) - Persona system
- [AI Studio](llm-studio.md) - AI features overview
- [System Generation](system-generation.md) - NL to system generation

