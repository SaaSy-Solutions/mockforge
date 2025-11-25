# Behavioral Economics Engine

**Pillars:** [Reality]

The Behavioral Economics Engine makes mocks react to real-world pressures like latency, load, pricing changes, fraud suspicion, and customer segments. This creates mocks that behave like real customer-driven systems, not just static endpoints.

## Overview

Traditional mocks return fixed responses. The Behavioral Economics Engine adds intelligence that makes mocks adapt their behavior based on system conditions:

- **Cart conversion drops** if latency exceeds 400ms
- **Bank declines transactions** if prior balance checks failed
- **User churn increases** after multiple 500 errors
- **Pricing changes** affect purchase behavior
- **Fraud suspicion** triggers additional verification

## Key Concepts

### Conditions

Conditions are triggers that evaluate system state:

- **Latency Threshold**: Endpoint latency exceeds a threshold
- **Load Pressure**: Requests per second exceeds a threshold
- **Pricing Change**: Product pricing changes by a percentage
- **Fraud Suspicion**: User's risk score exceeds a threshold
- **Customer Segment**: User belongs to a specific segment
- **Error Rate**: Error rate for an endpoint exceeds a threshold

### Actions

Actions are behaviors executed when conditions are met:

- **Modify Conversion Rate**: Adjust success probability
- **Change Response Behavior**: Modify response data
- **Trigger Chaos**: Activate chaos scenarios
- **Adjust Latency**: Increase/decrease response time
- **Change Error Rate**: Modify error probability

### Rules

Rules combine conditions and actions:

- **Declarative Rules**: Simple if-then logic (YAML/JSON)
- **Scriptable Rules**: Advanced logic (JavaScript/WASM)

## Quick Start

### Basic Configuration

```yaml
# mockforge.yaml
behavioral_economics:
  enabled: true
  rules:
    - name: latency-conversion-impact
      condition:
        type: latency_threshold
        endpoint: "/api/checkout/*"
        threshold_ms: 400
      action:
        type: modify_conversion_rate
        multiplier: 0.8  # 20% drop in conversion
      priority: 100
```

### Example: Cart Abandonment

```yaml
behavioral_economics:
  enabled: true
  rules:
    - name: cart-abandonment-on-latency
      condition:
        type: latency_threshold
        endpoint: "/api/checkout/*"
        threshold_ms: 500
      action:
        type: modify_response
        field: "body.cart_status"
        value: "abandoned"
      priority: 200
```

## Condition Types

### Latency Threshold

Triggers when endpoint latency exceeds a threshold:

```yaml
condition:
  type: latency_threshold
  endpoint: "/api/checkout/*"  # Pattern matching
  threshold_ms: 400
```

**Use Cases:**
- Cart abandonment on slow checkout
- User frustration on slow search
- Timeout handling

### Load Pressure

Triggers when requests per second exceed a threshold:

```yaml
condition:
  type: load_pressure
  threshold_rps: 100.0
```

**Use Cases:**
- Degraded service under load
- Rate limiting activation
- Circuit breaker patterns

### Pricing Change

Triggers when product pricing changes:

```yaml
condition:
  type: pricing_change
  product_id: "product-123"
  threshold: 0.1  # 10% change
```

**Use Cases:**
- Purchase behavior changes
- Cart abandonment on price increase
- Surge pricing effects

### Fraud Suspicion

Triggers when user's fraud risk score exceeds threshold:

```yaml
condition:
  type: fraud_suspicion
  user_id: "user-123"
  risk_score: 0.7  # 0.0 to 1.0
```

**Use Cases:**
- Additional verification required
- Transaction declines
- Account restrictions

### Customer Segment

Triggers when user belongs to a segment:

```yaml
condition:
  type: customer_segment
  segment: "premium"
```

**Use Cases:**
- Different service levels
- Priority handling
- Exclusive features

### Error Rate

Triggers when error rate exceeds threshold:

```yaml
condition:
  type: error_rate
  endpoint: "/api/payments/*"
  threshold: 0.05  # 5% error rate
```

**Use Cases:**
- User churn after errors
- Retry behavior changes
- Fallback activation

### Composite Conditions

Combine multiple conditions with logical operators:

```yaml
condition:
  type: composite
  operator: and  # or, not
  conditions:
    - type: latency_threshold
      endpoint: "/api/checkout/*"
      threshold_ms: 400
    - type: load_pressure
      threshold_rps: 100.0
```

## Action Types

### Modify Conversion Rate

Adjusts the probability of successful operations:

```yaml
action:
  type: modify_conversion_rate
  multiplier: 0.8  # 80% of normal conversion
```

**Use Cases:**
- Cart abandonment on slow checkout
- Purchase drop on high latency
- Signup reduction on errors

### Modify Response

Changes response data:

```yaml
action:
  type: modify_response
  field: "body.status"
  value: "pending"
```

**Use Cases:**
- Status changes based on conditions
- Data mutations
- State transitions

### Trigger Chaos

Activates chaos scenarios:

```yaml
action:
  type: trigger_chaos
  scenario: "high-error-rate"
  duration_seconds: 60
```

**Use Cases:**
- Simulating incidents
- Testing resilience
- Chaos engineering

### Adjust Latency

Modifies response latency:

```yaml
action:
  type: adjust_latency
  multiplier: 1.5  # 50% increase
  max_ms: 2000
```

**Use Cases:**
- Degraded performance simulation
- Network condition emulation
- Timeout testing

### Change Error Rate

Modifies error probability:

```yaml
action:
  type: change_error_rate
  multiplier: 2.0  # Double error rate
  error_codes: [500, 503]
```

**Use Cases:**
- Error spike simulation
- Failure testing
- Resilience validation

## Advanced: Scriptable Rules

For complex logic, use JavaScript/WASM scripts:

```yaml
behavioral_economics:
  enabled: true
  rules:
    - name: complex-fraud-detection
      rule_type: scriptable
      script: |
        function evaluate(context) {
          const latency = context.latency;
          const load = context.load_rps;
          const fraudScore = context.fraud_score;
          
          if (latency > 400 && load > 100 && fraudScore > 0.7) {
            return {
              action: "decline_transaction",
              reason: "High risk under load"
            };
          }
          return null;
        }
      priority: 1000
```

## Real-World Examples

### E-Commerce: Cart Conversion

```yaml
behavioral_economics:
  rules:
    - name: cart-conversion-drop
      condition:
        type: latency_threshold
        endpoint: "/api/checkout/*"
        threshold_ms: 400
      action:
        type: modify_conversion_rate
        multiplier: 0.7  # 30% drop
      priority: 100
```

**Result:** When checkout latency exceeds 400ms, only 70% of carts convert (vs normal 90%).

### Fintech: Transaction Declines

```yaml
behavioral_economics:
  rules:
    - name: decline-on-failed-balance-check
      condition:
        type: composite
        operator: and
        conditions:
          - type: error_rate
            endpoint: "/api/balance/*"
            threshold: 0.1
          - type: fraud_suspicion
            risk_score: 0.6
      action:
        type: modify_response
        field: "body.status"
        value: "declined"
        reason: "Balance check failed"
      priority: 200
```

**Result:** Transactions are declined if balance checks fail and fraud risk is high.

### SaaS: User Churn

```yaml
behavioral_economics:
  rules:
    - name: churn-after-multiple-errors
      condition:
        type: error_rate
        endpoint: "/api/*"
        threshold: 0.05  # 5% error rate
      action:
        type: modify_response
        field: "body.churn_risk"
        value: "high"
      priority: 150
```

**Result:** User churn risk increases after experiencing multiple errors.

## Integration with Other Features

The Behavioral Economics Engine integrates with:

- **Smart Personas**: Persona traits influence condition evaluation
- **Chaos Lab**: Actions can trigger chaos scenarios
- **Reality Continuum**: Behaviors respect reality levels
- **Scenarios**: Rules can be scenario-specific
- **Time Travel**: Conditions respect virtual time

## Configuration Reference

### Full Configuration

```yaml
behavioral_economics:
  enabled: true
  update_interval_ms: 1000  # How often to evaluate rules
  rules:
    - name: rule-name
      rule_type: declarative  # or scriptable
      condition:
        # Condition configuration
      action:
        # Action configuration
      priority: 100  # Higher = evaluated first
      enabled: true
```

### Rule Priority

Rules are evaluated in priority order (highest first). The first matching rule's action is executed.

## Best Practices

1. **Start Simple**: Use declarative rules for 80% of use cases
2. **Test Incrementally**: Add one rule at a time and test
3. **Monitor Impact**: Track how rules affect system behavior
4. **Document Rules**: Document why each rule exists
5. **Version Control**: Keep rules in version control

## Troubleshooting

### Rules Not Triggering

- Check condition thresholds (may be too high/low)
- Verify endpoint patterns match actual endpoints
- Check rule priority (higher priority rules may override)
- Enable debug logging: `behavioral_economics.debug: true`

### Unexpected Behavior

- Review rule priority order
- Check for conflicting rules
- Verify condition evaluation logic
- Test rules in isolation

## Related Documentation

- [Smart Personas](smart-personas.md) - Persona system
- [Chaos Lab](chaos-lab.md) - Chaos scenarios
- [Reality Continuum](reality-continuum.md) - Reality levels
- [Scenarios](scenario-state-machines.md) - Scenario system

