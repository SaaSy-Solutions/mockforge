# Reality-First Onboarding

**Pillars:** [Reality]

[Reality] - Everything that makes mocks feel like a real, evolving backend

## Start Here If...

You care about **realism**. You want mocks that feel indistinguishable from production backends, with realistic data, stateful behavior, and production-like network conditions.

Perfect for:
- Frontend teams needing realistic data that evolves over time
- Testing resilience and failure scenarios
- Simulating production-like network conditions
- Creating believable mock backends before real APIs exist

## Quick Start: 5 Minutes

Let's create a realistic mock API with Smart Personas and Reality Continuum:

```bash
# Install MockForge
cargo install mockforge-cli

# Create a simple config with reality features
cat > mockforge.yaml <<EOF
workspaces:
  - name: realistic-api
    reality:
      level: 3  # Moderate Realism
      personas:
        enabled: true
    endpoints:
      - path: /api/users/{id}
        method: GET
        response:
          body: |
            {
              "id": "{{persona.user.id}}",
              "name": "{{persona.user.name}}",
              "email": "{{persona.user.email}}"
            }
EOF

# Start the server
mockforge serve --config mockforge.yaml
```

Now test it:

```bash
# Request the same user ID multiple times - persona ensures consistency
curl http://localhost:3000/api/users/123
curl http://localhost:3000/api/users/123  # Same user data!
```

## Key Reality Features

### 1. Reality Continuum

Blend mock and real data seamlessly:

```yaml
reality:
  continuum:
    enabled: true
    blend_ratio: 0.5  # 50% mock, 50% real
    upstream_url: https://api.example.com
```

**Why it matters:** Develop against a backend that's still under construction by gradually transitioning from mock to real.

**Learn more:** [Reality Continuum Guide](../../docs/REALITY_CONTINUUM.md)

### 2. Smart Personas

Generate consistent, relationship-aware data across endpoints:

```yaml
reality:
  personas:
    enabled: true
    entities:
      - type: user
        fields:
          - name: id
            generator: uuid
          - name: email
            generator: email
      - type: order
        relationships:
          - field: user_id
            links_to: user.id
```

**Why it matters:** Maintain data consistency across your entire mock API. When you request a user and their orders, the relationships are preserved.

**Learn more:** [Smart Personas Guide](../../docs/PERSONAS.md)

### 3. Reality Slider

Adjust realism levels on the fly:

```yaml
reality:
  level: 3  # 1=Static, 2=Light, 3=Moderate, 4=High, 5=Production Chaos
```

**Why it matters:** Instantly transform your mock environment to match different testing scenarios without manual configuration.

**Learn more:** [Reality Slider Guide](../../docs/REALITY_SLIDER.md)

### 4. Chaos Lab

Simulate network conditions and failures:

```yaml
chaos:
  enabled: true
  latency:
    mean_ms: 200
    std_dev_ms: 50
  failures:
    error_rate: 0.05  # 5% of requests fail
    status_codes: [500, 502, 503]
```

**Why it matters:** Test how your application handles real-world network conditions and failures.

**Learn more:** [Chaos Lab Guide](../../docs/CHAOS_LAB.md)

### 5. Temporal Simulation

Time travel and time-based data mutations:

```yaml
time_travel:
  enabled: true
  virtual_clock: true
  scheduled_responses:
    - time: "2025-01-01T00:00:00Z"
      endpoint: /api/events
      response: {...}
```

**Why it matters:** Test time-dependent features, scheduled events, and data evolution over time.

**Learn more:** [Time Travel Guide](../../docs/TIME_TRAVEL.md)

## Next Steps

### Explore Reality Features

1. **Reality Continuum**: [Complete Guide](../../docs/REALITY_CONTINUUM.md)
   - Learn about blend ratios and merge strategies
   - Configure time-based progression
   - Set up fallback handling

2. **Smart Personas**: [Complete Guide](../../docs/PERSONAS.md)
   - Create persona graphs with relationships
   - Configure lifecycle states
   - Understand fidelity scores

3. **Reality Slider**: [Complete Guide](../../docs/REALITY_SLIDER.md)
   - Understand reality levels
   - Configure hot-reload
   - Coordinate chaos, latency, and AI

4. **Chaos Lab**: [Complete Guide](../../docs/CHAOS_LAB.md)
   - Configure latency profiles
   - Set up failure injection
   - Simulate network conditions

5. **Temporal Simulation**: [Complete Guide](../../docs/TIME_TRAVEL.md)
   - Use virtual clocks
   - Schedule time-based responses
   - Test time-dependent features

### Related Pillars

Once you've mastered Reality, explore these complementary pillars:

- **[Contracts]** - Add validation and drift detection to ensure your realistic mocks stay in sync with real APIs
  - [Contracts-First Onboarding](contracts-first.md)
  - [Drift Budgets Guide](../../docs/DRIFT_BUDGETS.md)

- **[DevX]** - Improve your workflow with SDKs, generators, and developer tools
  - [DevX Features](../../user-guide/http-mocking.md)
  - [CLI Reference](../../reference/cli.md)

- **[AI]** - Enhance realism with AI-powered data generation
  - [AI-First Onboarding](ai-first.md)
  - [MockAI Guide](../../docs/MOCKAI_USAGE.md)

## Examples

### Example 1: Realistic E-Commerce API

```yaml
workspaces:
  - name: ecommerce
    reality:
      level: 4  # High Realism
      personas:
        enabled: true
        entities:
          - type: user
          - type: product
          - type: order
            relationships:
              - field: user_id
                links_to: user.id
              - field: product_id
                links_to: product.id
    endpoints:
      - path: /api/users/{id}
        method: GET
        response:
          body: |
            {
              "id": "{{persona.user.id}}",
              "name": "{{persona.user.name}}",
              "orders": "{{persona.user.orders}}"
            }
```

### Example 2: Production-Like Testing

```yaml
reality:
  level: 5  # Production Chaos
chaos:
  enabled: true
  latency:
    distribution: normal
    mean_ms: 300
    std_dev_ms: 100
  failures:
    error_rate: 0.02
    status_codes: [500, 502, 503, 504]
```

## Troubleshooting

### Personas Not Working

Ensure personas are enabled in your config:

```yaml
reality:
  personas:
    enabled: true
```

### Reality Continuum Not Blending

Check your upstream URL and blend ratio:

```yaml
reality:
  continuum:
    enabled: true
    blend_ratio: 0.5
    upstream_url: https://api.example.com  # Must be accessible
```

### Chaos Not Injecting

Verify chaos is enabled and configured:

```yaml
chaos:
  enabled: true
  latency:
    mean_ms: 200
```

## Resources

- [Complete Pillars Documentation](../../docs/PILLARS.md)
- [Reality Features Overview](../../docs/REALITY_CONTINUUM.md)
- [API Reference](../../api/rust.md)
- [Examples Repository](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples)

---

**Ready to dive deeper?** Continue to the [Reality Continuum Guide](../../docs/REALITY_CONTINUUM.md) or explore [all Reality features](../../docs/PILLARS.md#reality--everything-that-makes-mocks-feel-like-a-real-evolving-backend).

