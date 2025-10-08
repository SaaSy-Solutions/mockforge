# Chaos Experiment Orchestration

MockForge provides powerful orchestration capabilities to compose and chain multiple chaos experiments into complex, realistic test scenarios.

## Table of Contents

- [Overview](#overview)
- [Key Concepts](#key-concepts)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [CLI Usage](#cli-usage)
- [API Reference](#api-reference)
- [Orchestration Patterns](#orchestration-patterns)
- [Best Practices](#best-practices)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

## Overview

Chaos orchestration allows you to:

- **Chain Multiple Scenarios**: Execute chaos scenarios in sequence or parallel
- **Progressive Testing**: Gradually increase chaos intensity
- **Realistic Simulations**: Model complex real-world failure patterns
- **Looping**: Repeat orchestrations for endurance testing
- **Conditional Execution**: Continue or stop on failures
- **Import/Export**: Share orchestrations as JSON/YAML files

### Why Orchestration?

Single chaos scenarios are useful, but real-world systems face multiple simultaneous issues:

```
Real outage:
  1. Network degrades (latency spikes)
  2. Database slows down (compound effect)
  3. Rate limits kick in (protective measures)
  4. Circuit breakers open (cascading failure)

With orchestration, you can test this exact sequence!
```

## Key Concepts

### Orchestrated Scenario

A composition of multiple steps that execute chaos scenarios in a defined order.

```yaml
name: progressive_load_test
description: Gradually increase system stress
steps:
  - warmup (60s)
  - normal_load (120s)
  - peak_load (120s)
  - chaos_injection (60s)
```

### Scenario Step

An individual step in an orchestration with:
- **Scenario**: The chaos scenario to execute
- **Duration**: How long to run (seconds)
- **Delay**: Wait time before starting
- **Error Handling**: Continue or stop on failure

### Execution Modes

1. **Sequential**: Steps run one after another
2. **Parallel**: All steps run simultaneously
3. **Looping**: Repeat the orchestration N times

## Quick Start

### 1. Generate a Template

```bash
mockforge orchestrate template \
  --output orchestration.yaml \
  --format yaml
```

### 2. Customize the Orchestration

Edit `orchestration.yaml`:

```yaml
name: api_stress_test
description: API stress testing with progressive chaos
steps:
  - name: baseline
    scenario:
      name: network_degradation
      config:
        enabled: true
        latency:
          enabled: true
          fixed_delay_ms: 50
    duration_seconds: 30
    delay_before_seconds: 0
    continue_on_failure: false

  - name: moderate_stress
    scenario:
      name: service_instability
      config:
        enabled: true
        fault_injection:
          enabled: true
          http_error_probability: 0.1
    duration_seconds: 60
    delay_before_seconds: 5
    continue_on_failure: true

  - name: peak_chaos
    scenario:
      name: cascading_failure
      config:
        enabled: true
    duration_seconds: 30
    delay_before_seconds: 10
    continue_on_failure: true

parallel: false
loop_orchestration: false
max_iterations: 1
tags:
  - stress-test
  - api
```

### 3. Validate the Orchestration

```bash
mockforge orchestrate validate --file orchestration.yaml
```

### 4. Start the Orchestration

```bash
# Start MockForge server
mockforge serve --chaos &

# Run orchestration
mockforge orchestrate start \
  --file orchestration.yaml \
  --base-url http://localhost:3000
```

### 5. Monitor Progress

```bash
mockforge orchestrate status --base-url http://localhost:3000
```

## Configuration

### YAML Format

```yaml
# Orchestration metadata
name: string                    # Required: Orchestration name
description: string             # Optional: Description
tags: []                        # Optional: Tags for categorization

# Steps (required, at least one)
steps:
  - name: string               # Step name
    scenario:                  # Chaos scenario
      name: string             # Scenario name or inline config
      config: {}               # ChaosConfig object
    duration_seconds: number   # How long to run (0 = use scenario default)
    delay_before_seconds: number  # Wait before starting
    continue_on_failure: bool  # Continue if step fails

# Execution settings
parallel: bool                 # Run steps in parallel
loop_orchestration: bool       # Loop the orchestration
max_iterations: number         # Max loops (0 = infinite)
```

### JSON Format

```json
{
  "name": "example_orchestration",
  "description": "Example chaos orchestration",
  "steps": [
    {
      "name": "step1",
      "scenario": {
        "name": "network_degradation",
        "config": {
          "enabled": true,
          "latency": {
            "enabled": true,
            "fixed_delay_ms": 100
          }
        }
      },
      "duration_seconds": 60,
      "delay_before_seconds": 0,
      "continue_on_failure": false
    }
  ],
  "parallel": false,
  "loop_orchestration": false,
  "max_iterations": 1,
  "tags": ["example"]
}
```

## CLI Usage

### Commands

```bash
# Generate a template
mockforge orchestrate template \
  --output template.yaml \
  --format yaml

# Validate orchestration file
mockforge orchestrate validate \
  --file my-orchestration.yaml

# Start orchestration
mockforge orchestrate start \
  --file my-orchestration.yaml \
  --base-url http://localhost:3000

# Check status
mockforge orchestrate status \
  --base-url http://localhost:3000

# Stop orchestration
mockforge orchestrate stop \
  --base-url http://localhost:3000
```

### Complete Example

```bash
# 1. Start MockForge with chaos enabled
mockforge serve --chaos --http-port 3000 &

# 2. Generate template
mockforge orchestrate template \
  --output load-test.yaml

# 3. Edit the template (use your editor)
vim load-test.yaml

# 4. Validate
mockforge orchestrate validate --file load-test.yaml

# 5. Run orchestration
mockforge orchestrate start --file load-test.yaml

# 6. Monitor in another terminal
watch -n 1 "mockforge orchestrate status"

# 7. Stop if needed
mockforge orchestrate stop
```

## API Reference

### Import Orchestration

```http
POST /api/chaos/orchestration/import
Content-Type: application/json

{
  "content": "...",  # YAML or JSON string
  "format": "yaml"   # or "json"
}
```

Response:
```json
{
  "message": "Orchestration 'name' imported successfully (3 steps)"
}
```

### Start Orchestration

```http
POST /api/chaos/orchestration/start
Content-Type: application/json

{
  "name": "my_orchestration",
  "steps": [...],
  "parallel": false
}
```

Response:
```json
{
  "message": "Orchestration 'my_orchestration' started successfully"
}
```

### Get Status

```http
GET /api/chaos/orchestration/status
```

Response:
```json
{
  "is_running": true,
  "name": "my_orchestration",
  "progress": 0.45
}
```

### Stop Orchestration

```http
POST /api/chaos/orchestration/stop
```

Response:
```json
{
  "message": "Orchestration stopped"
}
```

## Orchestration Patterns

### Pattern 1: Progressive Load Testing

Gradually increase load to find breaking points:

```yaml
name: progressive_load
description: Find system breaking point
steps:
  - name: baseline
    scenario:
      name: network_degradation
      config:
        latency:
          fixed_delay_ms: 10
    duration_seconds: 60

  - name: light_load
    scenario:
      name: peak_traffic
      config:
        rate_limit:
          requests_per_second: 100
    duration_seconds: 120

  - name: medium_load
    scenario:
      config:
        rate_limit:
          requests_per_second: 500
    duration_seconds: 120

  - name: heavy_load
    scenario:
      config:
        rate_limit:
          requests_per_second: 1000
    duration_seconds: 120

parallel: false
```

### Pattern 2: Cascading Failure Simulation

Simulate failures that compound:

```yaml
name: cascading_failures
description: Test cascading failure handling
steps:
  - name: network_degradation
    scenario:
      name: network_degradation
    duration_seconds: 30
    continue_on_failure: true

  - name: add_errors
    scenario:
      name: service_instability
    duration_seconds: 30
    delay_before_seconds: 10
    continue_on_failure: true

  - name: full_chaos
    scenario:
      name: cascading_failure
    duration_seconds: 60
    delay_before_seconds: 10
```

### Pattern 3: Spike Testing

Sudden load increase:

```yaml
name: spike_test
description: Test sudden traffic spikes
steps:
  - name: normal_traffic
    scenario:
      config:
        rate_limit:
          requests_per_second: 100
    duration_seconds: 60

  - name: spike
    scenario:
      config:
        rate_limit:
          requests_per_second: 1000
    duration_seconds: 30
    delay_before_seconds: 0  # Immediate spike

  - name: recovery
    scenario:
      config:
        rate_limit:
          requests_per_second: 100
    duration_seconds: 60
```

### Pattern 4: Endurance Testing

Long-running loop for stability:

```yaml
name: endurance_test
description: 24-hour stability test
steps:
  - name: normal_operations
    scenario:
      config:
        latency:
          fixed_delay_ms: 50
        fault_injection:
          http_error_probability: 0.01
    duration_seconds: 3600  # 1 hour per iteration

parallel: false
loop_orchestration: true
max_iterations: 24  # Run for 24 hours
```

### Pattern 5: Multi-Protocol Chaos

Test all protocols simultaneously:

```yaml
name: multi_protocol_chaos
description: Chaos across all protocols
steps:
  - name: http_chaos
    scenario:
      config:
        fault_injection:
          http_errors: [500, 503]
          http_error_probability: 0.2

  - name: grpc_chaos
    scenario:
      config:
        # gRPC-specific chaos

  - name: websocket_chaos
    scenario:
      config:
        # WebSocket-specific chaos

parallel: true  # All protocols at once!
```

## Best Practices

### 1. Start Small

Begin with simple orchestrations:

```yaml
# Good: Simple, focused test
name: latency_test
steps:
  - name: low_latency
    duration_seconds: 30
  - name: high_latency
    duration_seconds: 30

# Avoid: Too complex initially
name: kitchen_sink
steps: [10 different scenarios...]
```

### 2. Use Meaningful Names

```yaml
# Good
steps:
  - name: warmup_phase
  - name: peak_load_simulation
  - name: recovery_period

# Bad
steps:
  - name: step1
  - name: step2
  - name: step3
```

### 3. Set Appropriate Durations

```yaml
# Consider your system's characteristics
steps:
  - name: cache_warmup
    duration_seconds: 60    # Allow caches to populate

  - name: steady_state
    duration_seconds: 300   # Long enough for meaningful data

  - name: chaos_injection
    duration_seconds: 120   # Enough to observe effects
```

### 4. Use continue_on_failure Wisely

```yaml
steps:
  - name: critical_setup
    continue_on_failure: false  # Must succeed

  - name: optional_chaos
    continue_on_failure: true   # Test resilience

  - name: cleanup
    continue_on_failure: true   # Always try to cleanup
```

### 5. Add Delays for Realistic Scenarios

```yaml
steps:
  - name: normal_operation
    duration_seconds: 60

  - name: introduce_latency
    delay_before_seconds: 10  # Give system time to stabilize
    duration_seconds: 60

  - name: add_errors
    delay_before_seconds: 30  # Compound after latency
    duration_seconds: 60
```

### 6. Tag Your Orchestrations

```yaml
tags:
  - environment:staging
  - type:load-test
  - severity:high
  - automated:ci-cd
```

### 7. Version Control

```bash
# Store orchestrations in git
git add orchestrations/*.yaml
git commit -m "Add load test orchestration"
```

### 8. Monitor and Document

```yaml
description: |
  Progressive load test for API endpoints.

  Expected behavior:
  - System should handle 100 RPS normally
  - At 500 RPS, response times increase but stay under 500ms
  - At 1000 RPS, circuit breakers should open
  - System should recover within 60s after load drops

  Success criteria:
  - No data loss
  - Graceful degradation
  - Full recovery post-chaos
```

## Examples

### Example 1: API Load Test

```yaml
name: api_load_test
description: Comprehensive API load testing
steps:
  # Warmup
  - name: warmup
    scenario:
      config:
        enabled: true
        latency:
          enabled: true
          fixed_delay_ms: 10
    duration_seconds: 30
    delay_before_seconds: 0
    continue_on_failure: false

  # Normal load
  - name: normal_load
    scenario:
      config:
        enabled: true
        rate_limit:
          enabled: true
          requests_per_second: 100
    duration_seconds: 120
    delay_before_seconds: 10
    continue_on_failure: false

  # Peak load
  - name: peak_load
    scenario:
      config:
        enabled: true
        rate_limit:
          enabled: true
          requests_per_second: 500
        latency:
          enabled: true
          fixed_delay_ms: 100
    duration_seconds: 120
    delay_before_seconds: 10
    continue_on_failure: true

  # Chaos
  - name: chaos_injection
    scenario:
      name: cascading_failure
    duration_seconds: 60
    delay_before_seconds: 10
    continue_on_failure: true

  # Recovery
  - name: recovery
    scenario:
      config:
        enabled: true
    duration_seconds: 60
    delay_before_seconds: 20
    continue_on_failure: true

parallel: false
loop_orchestration: false
max_iterations: 1
tags:
  - load-test
  - api
  - comprehensive
```

### Example 2: Network Partition Simulation

```yaml
name: network_partition
description: Simulate network partition and recovery
steps:
  - name: normal_operation
    scenario:
      config:
        enabled: false
    duration_seconds: 60

  - name: partition_starts
    scenario:
      config:
        enabled: true
        traffic_shaping:
          enabled: true
          packet_loss_percent: 50.0
    duration_seconds: 30
    delay_before_seconds: 5

  - name: full_partition
    scenario:
      config:
        enabled: true
        traffic_shaping:
          enabled: true
          packet_loss_percent: 100.0
    duration_seconds: 20
    delay_before_seconds: 0

  - name: partition_heals
    scenario:
      config:
        enabled: true
        traffic_shaping:
          enabled: true
          packet_loss_percent: 25.0
    duration_seconds: 30
    delay_before_seconds: 5

  - name: recovery
    scenario:
      config:
        enabled: false
    duration_seconds: 60
    delay_before_seconds: 10

parallel: false
```

### Example 3: Multi-Region Failure

```yaml
name: multi_region_failure
description: Simulate multi-region outage
steps:
  - name: region_a_degrades
    scenario:
      config:
        latency:
          fixed_delay_ms: 200
    duration_seconds: 60

  - name: region_b_fails
    scenario:
      config:
        fault_injection:
          http_errors: [503]
          http_error_probability: 0.9
    duration_seconds: 60
    continue_on_failure: true

  - name: both_regions_degraded
    scenario:
      name: cascading_failure
    duration_seconds: 120
    continue_on_failure: true

parallel: false
```

## Troubleshooting

### Issue: Orchestration Doesn't Start

**Symptoms**: API returns error when starting

**Solutions**:
1. Validate the file first:
   ```bash
   mockforge orchestrate validate --file my-orch.yaml
   ```

2. Check the API is accessible:
   ```bash
   curl http://localhost:3000/api/chaos/status
   ```

3. Verify chaos is enabled:
   ```bash
   mockforge serve --chaos
   ```

### Issue: Steps Not Executing

**Symptoms**: Orchestration starts but steps don't run

**Solutions**:
1. Check orchestration status:
   ```bash
   mockforge orchestrate status
   ```

2. Verify step configuration has valid scenarios

3. Check server logs for errors

### Issue: Orchestration Stuck

**Symptoms**: Progress doesn't advance

**Solutions**:
1. Stop the orchestration:
   ```bash
   mockforge orchestrate stop
   ```

2. Check for infinite loops:
   ```yaml
   loop_orchestration: true
   max_iterations: 0  # ← Infinite loop!
   ```

3. Verify duration_seconds is set for all steps

### Issue: Steps Fail Immediately

**Symptoms**: All steps fail without running

**Solutions**:
1. Set `continue_on_failure: true` for optional steps

2. Check if first step's scenario is valid

3. Verify ChaosConfig in scenarios

## Integration

### With CI/CD

```yaml
# .github/workflows/chaos-test.yml
name: Chaos Test
on: [push]
jobs:
  chaos:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Start MockForge
        run: |
          mockforge serve --chaos &
          sleep 5

      - name: Run orchestration
        run: |
          mockforge orchestrate start \
            --file .mockforge/load-test.yaml

      - name: Wait for completion
        run: |
          while mockforge orchestrate status | grep -q "is_running.*true"; do
            sleep 10
          done
```

### With Monitoring

```bash
# Send metrics to monitoring system
while true; do
  STATUS=$(curl -s http://localhost:3000/api/chaos/orchestration/status)
  PROGRESS=$(echo $STATUS | jq -r '.progress')

  # Send to monitoring
  echo "chaos.orchestration.progress $PROGRESS" | nc -w1 -u statsd 8125

  sleep 5
done
```

## See Also

- [Chaos Engineering Guide](./CHAOS_ENGINEERING.md) - Base chaos capabilities
- [Resilience Patterns](./RESILIENCE_PATTERNS.md) - Circuit breaker & bulkhead
- [Protocol Chaos](./PROTOCOL_CHAOS.md) - gRPC, WebSocket, GraphQL
- [Observability Guide](./OBSERVABILITY.md) - Metrics and monitoring

## Advanced Topics

### Custom Scenarios in Orchestrations

You can define custom scenarios inline:

```yaml
steps:
  - name: custom_scenario
    scenario:
      config:
        enabled: true
        latency:
          enabled: true
          random_delay_range_ms: [100, 500]
          jitter_percent: 20.0
        fault_injection:
          enabled: true
          http_errors: [500, 502]
          http_error_probability: 0.15
        circuit_breaker:
          enabled: true
          failure_threshold: 3
        bulkhead:
          enabled: true
          max_concurrent_requests: 50
```

### Programmatic Orchestration

```rust
use mockforge_chaos::{
    scenario_orchestrator::{OrchestratedScenario, ScenarioStep},
    scenarios::ChaosScenario,
};

let orchestration = OrchestratedScenario::new("my_test")
    .with_description("Programmatic orchestration")
    .add_step(
        ScenarioStep::new("step1", scenario1)
            .with_duration(60)
            .with_delay_before(5)
    )
    .add_step(
        ScenarioStep::new("step2", scenario2)
            .with_duration(120)
            .continue_on_failure()
    )
    .with_parallel_execution();

// Export to share
let yaml = orchestration.to_yaml()?;
std::fs::write("orchestration.yaml", yaml)?;
```

---

**Chaos Orchestration** provides the power to create complex, realistic chaos experiments that mirror real-world failure patterns!
