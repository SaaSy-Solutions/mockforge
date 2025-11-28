# Synthetic → Recorded Drift Learning

**Pillars:** [Reality]

The Drift Learning System allows mocks to gradually learn from recorded traffic and adapt their behavior. Instead of static synthetic data, mocks can evolve to match real-world patterns observed in traffic.

## Overview

Drift Learning extends the DataDriftEngine with learning capabilities that enable:

- **Traffic Pattern Learning**: Mocks learn from recorded request patterns
- **Persona Behavior Adaptation**: Personas adapt based on observed request patterns
- **Configurable Learning Rate**: Control how quickly mocks learn
- **Opt-in Per Endpoint/Persona**: Enable learning selectively
- **Pattern Decay**: Old patterns fade if upstream patterns reverse

## Key Concepts

### Learning Modes

Drift Learning supports three learning modes:

#### Behavioral Learning

Adapts to behavior patterns observed in traffic:

- **Request Sequences**: Learns common request sequences
- **User Flows**: Adapts to typical user workflows
- **Interaction Patterns**: Learns how users interact with the API

**Use Case:** When you want mocks to reflect real user behavior patterns.

#### Statistical Learning

Adapts to statistical patterns in traffic:

- **Latency Patterns**: Learns typical latency distributions
- **Error Rates**: Adapts to observed error rate patterns
- **Request Frequency**: Learns request frequency patterns

**Use Case:** When you want mocks to match statistical properties of real traffic.

#### Hybrid Learning

Combines behavioral and statistical learning:

- **Best of Both**: Uses both behavioral and statistical patterns
- **Balanced Adaptation**: Balances behavior and statistics
- **Comprehensive Learning**: Most comprehensive learning mode

**Use Case:** When you want the most realistic mocks possible.

### Learning Configuration

```yaml
drift_learning:
  enabled: true
  mode: hybrid  # behavioral, statistical, or hybrid
  sensitivity: 0.2  # How quickly mocks learn (0.0-1.0)
  decay: 0.05  # How quickly old patterns fade (0.0-1.0)
  min_samples: 10  # Minimum samples before learning starts
  update_interval: 60s  # How often to update learned patterns
  persona_adaptation: true  # Enable persona-specific learning
  traffic_mirroring: true  # Enable traffic pattern mirroring
```

## How It Works

### 1. Traffic Recording

The system records traffic patterns:

- **Request Sequences**: Tracks sequences of requests
- **Response Patterns**: Records response patterns
- **Timing Information**: Captures latency and timing data
- **Error Patterns**: Tracks error occurrences

### 2. Pattern Analysis

Recorded traffic is analyzed to identify patterns:

- **Behavioral Patterns**: User behavior sequences
- **Statistical Patterns**: Latency, error rate distributions
- **Persona Patterns**: Persona-specific behavior patterns

### 3. Learning Application

Learned patterns are applied to mocks:

- **Gradual Adaptation**: Mocks gradually adapt to learned patterns
- **Confidence Scoring**: Patterns are scored by confidence
- **Decay Handling**: Low-confidence patterns fade over time

### 4. Pattern Updates

Patterns are updated periodically:

- **Update Interval**: Configurable update frequency
- **Minimum Samples**: Requires minimum samples before learning
- **Confidence Threshold**: Only high-confidence patterns are applied

## Usage

### Enable Drift Learning

```yaml
# mockforge.yaml
drift_learning:
  enabled: true
  mode: hybrid
  sensitivity: 0.2
  decay: 0.05
  min_samples: 10
  update_interval: 60s
```

### Per-Endpoint Learning

Enable learning for specific endpoints:

```yaml
drift_learning:
  enabled: true
  endpoint_learning:
    "/api/users/*": true  # Enable for user endpoints
    "/api/orders/*": true  # Enable for order endpoints
    "/api/payments/*": false  # Disable for payment endpoints
```

### Per-Persona Learning

Enable learning for specific personas:

```yaml
drift_learning:
  enabled: true
  persona_adaptation: true
  persona_learning:
    "premium-customer": true  # Enable for premium customers
    "regular-customer": true  # Enable for regular customers
    "admin": false  # Disable for admins
```

### CLI Commands

```bash
# Enable drift learning
mockforge config set drift_learning.enabled true

# Set learning mode
mockforge config set drift_learning.mode hybrid

# View learned patterns
mockforge drift-learning patterns

# Reset learned patterns
mockforge drift-learning reset
```

## Learning Parameters

### Sensitivity

Controls how quickly mocks learn from patterns:

- **Low (0.1)**: Slow, conservative learning
- **Medium (0.2)**: Balanced learning rate (default)
- **High (0.5)**: Fast, aggressive learning

**Recommendation:** Start with 0.2 and adjust based on results.

### Decay

Controls how quickly old patterns fade:

- **Low (0.01)**: Patterns persist longer
- **Medium (0.05)**: Balanced decay (default)
- **High (0.1)**: Patterns fade quickly

**Recommendation:** Use 0.05 for most cases.

### Minimum Samples

Minimum number of samples required before learning starts:

- **Low (5)**: Learn from few samples (may be noisy)
- **Medium (10)**: Balanced threshold (default)
- **High (50)**: Require many samples (more stable)

**Recommendation:** Use 10 for most cases, increase if patterns are noisy.

## Real-World Examples

### Example 1: E-Commerce Checkout Flow

**Scenario:** Learn from real checkout patterns

```yaml
drift_learning:
  enabled: true
  mode: behavioral
  endpoint_learning:
    "/api/checkout/*": true
  persona_adaptation: true
```

**Result:** Mocks learn the typical checkout flow sequence and adapt responses accordingly.

### Example 2: API Latency Patterns

**Scenario:** Learn latency patterns from real traffic

```yaml
drift_learning:
  enabled: true
  mode: statistical
  endpoint_learning:
    "/api/*": true
  sensitivity: 0.3
```

**Result:** Mocks adapt their latency to match observed latency distributions.

### Example 3: Persona-Specific Behavior

**Scenario:** Learn persona-specific behavior patterns

```yaml
drift_learning:
  enabled: true
  mode: hybrid
  persona_adaptation: true
  persona_learning:
    "premium-customer": true
    "regular-customer": true
```

**Result:** Mocks learn different behavior patterns for different personas.

## Traffic Pattern Mirroring

When `traffic_mirroring` is enabled, mocks mirror observed traffic patterns:

- **Request Frequency**: Mirrors request frequency patterns
- **Request Sequences**: Mirrors common request sequences
- **Timing Patterns**: Mirrors timing patterns
- **Error Patterns**: Mirrors error occurrence patterns

### Configuration

```yaml
drift_learning:
  enabled: true
  traffic_mirroring: true
  mode: hybrid
```

## Persona Behavior Adaptation

When `persona_adaptation` is enabled, personas adapt based on observed behavior:

- **Behavior Patterns**: Personas learn behavior patterns
- **Request Patterns**: Personas learn request patterns
- **Response Patterns**: Personas learn response patterns

### Configuration

```yaml
drift_learning:
  enabled: true
  persona_adaptation: true
  persona_learning:
    "premium-customer": true
    "regular-customer": true
```

## Best Practices

1. **Start Conservative**: Begin with low sensitivity and high min_samples
2. **Monitor Patterns**: Regularly review learned patterns
3. **Selective Learning**: Enable learning only for endpoints/personas that need it
4. **Test Incrementally**: Enable learning for one endpoint at a time
5. **Review Confidence**: Only apply high-confidence patterns
6. **Reset When Needed**: Reset learned patterns if they become inaccurate

## Troubleshooting

### Mocks Not Learning

- **Check Enabled**: Verify `enabled: true`
- **Check Samples**: Ensure minimum samples are reached
- **Check Confidence**: Low-confidence patterns may not be applied
- **Check Endpoint/Persona**: Verify learning is enabled for the endpoint/persona

### Patterns Too Aggressive

- **Lower Sensitivity**: Reduce sensitivity value
- **Increase Min Samples**: Require more samples before learning
- **Increase Decay**: Make patterns fade faster

### Patterns Not Updating

- **Check Update Interval**: Verify update interval is reasonable
- **Check Samples**: Ensure new samples are being recorded
- **Check Confidence**: Low-confidence patterns may not update

## Related Documentation

- [Smart Personas](smart-personas.md) - Persona system
- [Reality Continuum](reality-continuum.md) - Reality levels
- [Behavioral Economics Engine](behavioral-economics.md) - Behavioral rules

