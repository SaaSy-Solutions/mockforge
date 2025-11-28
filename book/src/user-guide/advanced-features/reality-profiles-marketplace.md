# Reality Profiles Marketplace

**Pillars:** [Reality]

The Reality Profiles Marketplace provides pre-tuned "realism packs" that bundle personas, scenarios, chaos rules, latency curves, error distributions, data mutation behaviors, and protocol behaviors into ready-to-use packages. Think of them as MockForge's "Kubernetes Operators" moment—reusable ops-level behaviors.

## Overview

Reality profile packs are complete configurations that simulate real-world scenarios for specific domains. Instead of manually configuring latency curves, error distributions, and behavioral patterns, you can install a pack that provides all of this out of the box.

## Available Packs

### E-Commerce Peak Season Pack

Simulates high-load e-commerce scenarios with:
- **Increased latency** during peak hours (300ms mean, up to 2000ms)
- **Cart abandonment patterns** (cart value decreases over time)
- **Inventory depletion behaviors** (quantity decreases under load)
- **Seasonal purchase patterns**

**Install:**
```bash
mockforge reality-profile install ecommerce-peak-season
```

**Use Case:** Testing your frontend's resilience during Black Friday, Cyber Monday, or other high-traffic events.

### Fintech Fraud Pack

Simulates financial services scenarios with:
- **Fraud detection triggers** (suspicious transaction patterns)
- **Transaction declines** (card declined, insufficient funds)
- **Risk scoring** (high-risk vs low-risk customer segments)
- **Compliance behaviors** (KYC checks, AML flags)

**Install:**
```bash
mockforge reality-profile install fintech-fraud
```

**Use Case:** Testing fraud detection systems, payment flows, and compliance workflows.

### Healthcare HL7/Insurance Edge Cases Pack

Simulates healthcare scenarios with:
- **HL7 message patterns** (ADT, ORU, MDM message types)
- **Insurance edge cases** (pre-authorization failures, coverage gaps)
- **Patient data patterns** (HIPAA-compliant test data)
- **Medical device integration** (device disconnections, data bursts)

**Install:**
```bash
mockforge reality-profile install healthcare-hl7
```

**Use Case:** Testing HL7 integrations, insurance workflows, and medical device connectivity.

### IoT Device Fleet Chaos Pack

Simulates IoT scenarios with:
- **Device disconnections** (random disconnects, network failures)
- **Message bursts** (sudden spikes in telemetry data)
- **Protocol behaviors** (MQTT, CoAP, WebSocket patterns)
- **Edge computing patterns** (offline mode, sync conflicts)

**Install:**
```bash
mockforge reality-profile install iot-fleet-chaos
```

**Use Case:** Testing IoT platforms, device management systems, and edge computing scenarios.

## Installing Packs

### From Pre-built Packs

```bash
# List available packs
mockforge reality-profile list

# Install a pack
mockforge reality-profile install <pack-name>

# Examples
mockforge reality-profile install ecommerce-peak-season
mockforge reality-profile install fintech-fraud
mockforge reality-profile install healthcare-hl7
mockforge reality-profile install iot-fleet-chaos
```

### From Custom Paths

You can also install packs from local files or URLs:

```bash
# From local file
mockforge reality-profile install ./my-custom-pack.yaml

# From URL
mockforge reality-profile install https://example.com/packs/custom-pack.yaml
```

## Pack Structure

Each reality profile pack includes:

### 1. Personas
Pre-configured personas with traits matching the domain:
- E-commerce: `premium-customer`, `bargain-hunter`, `cart-abandoner`
- Fintech: `high-risk-user`, `vip-customer`, `fraud-suspect`
- Healthcare: `new-patient`, `chronic-care-patient`, `emergency-case`
- IoT: `smart-home-owner`, `industrial-operator`, `fleet-manager`

### 2. Scenarios
Ready-to-use scenarios for common workflows:
- E-commerce: `peak-season-checkout`, `cart-abandonment-flow`, `inventory-depletion`
- Fintech: `fraud-detection-flow`, `payment-decline-scenario`, `kyc-verification`
- Healthcare: `patient-admission`, `insurance-claim-processing`, `device-alert`
- IoT: `device-onboarding`, `telemetry-burst`, `firmware-update-failure`

### 3. Chaos Rules
Pre-configured chaos engineering rules:
- Latency spikes during peak hours
- Error rate increases under load
- Network partition simulations
- Resource exhaustion patterns

### 4. Latency Curves
Realistic latency distributions:
- Normal distributions for typical traffic
- Exponential distributions for burst scenarios
- Custom curves for specific endpoints

### 5. Error Distributions
Realistic error patterns:
- 5xx errors under load
- 4xx errors for invalid requests
- Rate limiting (429) during peak
- Timeout errors (504) for slow endpoints

### 6. Data Mutation Behaviors
Dynamic data changes:
- Inventory depletion over time
- Cart abandonment patterns
- Account balance changes
- Device state transitions

### 7. Protocol Behaviors
Protocol-specific behaviors:
- REST: HTTP method patterns, status code distributions
- WebSocket: Connection lifecycle, message patterns
- MQTT: Topic subscriptions, QoS levels
- gRPC: Streaming patterns, error codes

## Using Packs in Your Workspace

After installing a pack, it's available for use in your workspace configuration:

```yaml
# mockforge.yaml
reality:
  profiles:
    - name: ecommerce-peak-season
      enabled: true
      # Pack-specific configuration
      peak_hours:
        start: "09:00"
        end: "17:00"
      load_multiplier: 1.5  # 50% more load during peak
```

## Creating Custom Packs

You can create your own reality profile packs:

### 1. Create Pack Manifest

```yaml
# my-custom-pack.yaml
name: my-custom-pack
version: 1.0.0
title: My Custom Reality Pack
description: Custom reality profile for my domain
domain: custom
author: My Team

tags:
  - custom
  - internal

# Latency curves
latency_curves:
  - protocol: rest
    distribution: normal
    params:
      mean: 200.0
      std_dev: 50.0
    base_ms: 200
    endpoint_patterns:
      - "/api/custom/*"
    jitter_ms: 25
    min_ms: 100
    max_ms: 1000

# Error distributions
error_distributions:
  - endpoint_pattern: "/api/custom/*"
    error_codes: [500, 503]
    probabilities: [0.05, 0.02]
    pattern:
      type: random
      probability: 0.07

# Data mutation behaviors
data_mutation_behaviors:
  - field_pattern: "body.value"
    mutation_type: increment
    rate: 0.1
    params:
      increment_by: 1
      max_value: 100
```

### 2. Install Custom Pack

```bash
mockforge reality-profile install ./my-custom-pack.yaml
```

## Pack Configuration

Packs can be configured per-workspace:

```yaml
reality:
  profiles:
    - name: ecommerce-peak-season
      enabled: true
      config:
        # Override pack defaults
        peak_hours:
          start: "10:00"
          end: "18:00"
        load_multiplier: 2.0
        error_rate_multiplier: 1.5
```

## Integration with Other Features

Reality profile packs integrate seamlessly with:

- **Smart Personas**: Packs include personas that work with the pack's scenarios
- **Scenarios**: Pre-built scenarios use the pack's behavioral patterns
- **Chaos Lab**: Pack chaos rules integrate with your chaos experiments
- **Reality Continuum**: Packs work with blend ratios and reality levels
- **Time Travel**: Pack behaviors respect virtual time settings

## Best Practices

1. **Start with Pre-built Packs**: Use existing packs before creating custom ones
2. **Combine Packs**: You can enable multiple packs and combine their behaviors
3. **Customize Gradually**: Start with defaults, then customize as needed
4. **Version Control**: Keep pack configurations in version control
5. **Team Sharing**: Share custom packs via Git or internal registry

## Examples

### E-Commerce Peak Season Testing

```bash
# Install pack
mockforge reality-profile install ecommerce-peak-season

# Configure workspace
# mockforge.yaml
reality:
  profiles:
    - name: ecommerce-peak-season
      enabled: true
      config:
        peak_hours:
          start: "00:00"  # Black Friday - all day peak
          end: "23:59"
        load_multiplier: 3.0  # 3x normal load
```

### Fintech Fraud Testing

```bash
# Install pack
mockforge reality-profile install fintech-fraud

# Use in scenarios
scenarios:
  - name: fraud-detection-test
    persona: high-risk-user
    reality_profile: fintech-fraud
    steps:
      - endpoint: POST /api/transactions
        expected_fraud_score: "> 0.7"
```

## Troubleshooting

### Pack Not Found

If a pack isn't found, check:
- Pack name spelling
- Pack is installed: `mockforge reality-profile list`
- Pack version compatibility

### Conflicts Between Packs

If multiple packs conflict:
- Check endpoint pattern overlaps
- Adjust pack priorities
- Disable conflicting packs
- Create custom pack combining needed features

## Related Documentation

- [Smart Personas](smart-personas.md) - Persona system used by packs
- [Chaos Lab](chaos-lab.md) - Chaos rules in packs
- [Reality Continuum](reality-continuum.md) - Reality levels and blending
- [Scenarios](scenario-state-machines.md) - Scenario system

