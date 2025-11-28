# Reality Slider Guide

**Pillars:** [Reality][DevX]

[Reality] - Makes mocks feel like real backends through configurable realism levels
[DevX] - Hot-reload capabilities improve developer experience

**Last Updated**: 2025-01-XX
**Status**: ✅ Fully Implemented

## Overview

The Reality Slider is a unified control mechanism that adjusts the realism of your mock environment from simple static stubs to full production-level chaos. It coordinates three key subsystems:

- **Chaos Engineering**: Error injection, delays, and timeouts
- **Latency Simulation**: Network delay patterns and jitter
- **MockAI**: Intelligent response generation and behavior

By adjusting a single slider from 1 to 5, you can instantly transform your mock environment to match different testing scenarios without manually configuring each subsystem.

## Reality Levels

### Level 1: Static Stubs
**Use Case**: Fast, predictable responses for basic functionality testing

- **Chaos**: Disabled
- **Latency**: 0ms (instant responses)
- **MockAI**: Disabled
- **Best For**: Unit tests, rapid prototyping, simple integration checks

### Level 2: Light Simulation
**Use Case**: Minimal realism with basic intelligence

- **Chaos**: Disabled
- **Latency**: 10-50ms (minimal network delay)
- **MockAI**: Basic AI (simple response generation)
- **Best For**: Frontend development, basic API testing, quick demos

### Level 3: Moderate Realism (Default)
**Use Case**: Balanced realism for most development scenarios

- **Chaos**: 5% error rate, 10% delay probability
- **Latency**: 50-200ms (moderate network conditions)
- **MockAI**: Full AI enabled (intelligent responses, relationship awareness)
- **Best For**: Integration testing, development environments, staging-like behavior

### Level 4: High Realism
**Use Case**: Production-like conditions with increased complexity

- **Chaos**: 10% error rate, 20% delay probability
- **Latency**: 100-500ms (realistic network conditions)
- **MockAI**: Full AI + session state management
- **Best For**: Pre-production testing, realistic user flows, stress testing preparation

### Level 5: Production Chaos
**Use Case**: Maximum realism for resilience testing

- **Chaos**: 15% error rate, 30% delay probability
- **Latency**: 200-2000ms (production-like network conditions)
- **MockAI**: Full AI + mutations + advanced features
- **Best For**: Chaos engineering, resilience testing, production simulation

## Usage

### UI Usage

#### Dashboard
The Reality Slider is available on the Dashboard page alongside the Time Travel widget:

1. Navigate to **Dashboard** in the admin UI
2. Find the **Environment Control** section
3. Use the slider to adjust the reality level (1-5)
4. Click level indicators for quick selection
5. View current configuration in the details panel

#### Configuration Page
For advanced control and preset management:

1. Navigate to **Configuration** → **Reality Slider**
2. Use the full-featured slider with visual feedback
3. Manage presets (export/import configurations)
4. View keyboard shortcuts reference

#### Reality Indicator Badge
A compact badge in page headers shows the current reality level:
- Hover for detailed configuration information
- Color-coded by level (gray → blue → green → orange → red)

### CLI Usage

#### Command Line Flag
```bash
# Set reality level at startup
mockforge serve --reality-level 5

# With OpenAPI spec
mockforge serve --spec api.yaml --reality-level 3
```

#### Environment Variable
```bash
# Set via environment variable
export MOCKFORGE_REALITY_LEVEL=4
mockforge serve

# Or inline
MOCKFORGE_REALITY_LEVEL=2 mockforge serve --spec api.yaml
```

**Precedence**: CLI flag > Environment variable > Config file > Default (Level 3)

### Configuration File

Add to your `mockforge.yaml`:

```yaml
reality:
  enabled: true
  level: 3  # 1-5
```

Or use per-profile configuration:

```yaml
profiles:
  development:
    reality:
      level: 2
  staging:
    reality:
      level: 4
  production:
    reality:
      level: 5
```

## Keyboard Shortcuts

Quick reality level changes from anywhere in the UI:

| Shortcut | Action |
|----------|--------|
| `Ctrl+Shift+1` | Set to Level 1 (Static Stubs) |
| `Ctrl+Shift+2` | Set to Level 2 (Light Simulation) |
| `Ctrl+Shift+3` | Set to Level 3 (Moderate Realism) |
| `Ctrl+Shift+4` | Set to Level 4 (High Realism) |
| `Ctrl+Shift+5` | Set to Level 5 (Production Chaos) |
| `Ctrl+Shift+R` | Reset to default (Level 3) |
| `Ctrl+Shift+P` | Open preset manager (Config page) |

**Note**: Shortcuts are disabled when typing in input fields to avoid conflicts.

## Presets

### Exporting Presets

Save your current reality configuration for reuse:

1. Navigate to **Configuration** → **Reality Slider**
2. Click **Export Current**
3. Enter a preset name (e.g., "production-chaos", "staging-realistic")
4. Optionally add a description
5. Click **Export Preset**

Presets are saved as JSON or YAML files in the workspace presets directory.

### Importing Presets

1. Navigate to **Configuration** → **Reality Slider**
2. Click **Import Preset**
3. Select a preset from the list
4. Click **Load** to apply

### Preset File Format

Presets are stored as JSON or YAML:

```json
{
  "metadata": {
    "name": "production-chaos",
    "description": "Maximum realism for resilience testing",
    "created_at": "2025-01-15T10:30:00Z",
    "version": "1.0"
  },
  "config": {
    "chaos": {
      "enabled": true,
      "error_rate": 0.15,
      "delay_rate": 0.30
    },
    "latency": {
      "base_ms": 200,
      "jitter_ms": 1800
    },
    "mockai": {
      "enabled": true
    }
  }
}
```

### Listing Presets

View all available presets via the API:

```bash
curl http://localhost:9080/__mockforge/reality/presets
```

## CI/CD Integration

### GitHub Actions

Set the reality level for your test environment:

```yaml
env:
  MOCKFORGE_REALITY_LEVEL: 3  # Moderate Realism for tests

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - name: Run tests with mock
        run: |
          mockforge serve --reality-level ${{ env.MOCKFORGE_REALITY_LEVEL }} &
          # Run your tests
```

### GitLab CI

```yaml
variables:
  MOCKFORGE_REALITY_LEVEL: "3"

test:integration:
  variables:
    MOCKFORGE_REALITY_LEVEL: "4"  # Override per job
  script:
    - mockforge serve --reality-level $MOCKFORGE_REALITY_LEVEL
    - npm test
```

### Docker Compose

```yaml
services:
  mockforge:
    environment:
      - MOCKFORGE_REALITY_LEVEL=${MOCKFORGE_REALITY_LEVEL:-3}
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mockforge
spec:
  template:
    spec:
      containers:
      - name: mockforge
        env:
        - name: MOCKFORGE_REALITY_LEVEL
          value: "4"
```

## API Reference

### Get Current Reality Level

```http
GET /__mockforge/reality/level
```

**Response**:
```json
{
  "level": 3,
  "level_name": "Moderate Realism",
  "description": "Some chaos, moderate latency, full intelligence",
  "chaos": {
    "enabled": true,
    "error_rate": 0.05,
    "delay_rate": 0.10
  },
  "latency": {
    "base_ms": 50,
    "jitter_ms": 150
  },
  "mockai": {
    "enabled": true
  }
}
```

### Set Reality Level

```http
PUT /__mockforge/reality/level
Content-Type: application/json

{
  "level": 5
}
```

**Response**: Same as GET, with updated configuration

### List Presets

```http
GET /__mockforge/reality/presets
```

**Response**:
```json
[
  {
    "id": "preset-1",
    "path": "/path/to/preset.json",
    "name": "production-chaos"
  }
]
```

### Import Preset

```http
POST /__mockforge/reality/presets/import
Content-Type: application/json

{
  "path": "/path/to/preset.json"
}
```

### Export Preset

```http
POST /__mockforge/reality/presets/export
Content-Type: application/json

{
  "name": "my-preset",
  "description": "Optional description"
}
```

## Use Cases

### Development Workflow

1. **Start Development**: Level 2 (Light Simulation)
   - Fast responses for rapid iteration
   - Basic AI for realistic data

2. **Integration Testing**: Level 3 (Moderate Realism)
   - Some chaos to catch error handling
   - Realistic latency for network-aware code

3. **Pre-Production**: Level 4 (High Realism)
   - Production-like conditions
   - Full feature set enabled

4. **Resilience Testing**: Level 5 (Production Chaos)
   - Maximum chaos for stress testing
   - Simulate worst-case scenarios

### Testing Scenarios

#### Unit Tests
```bash
# Fast, predictable responses
MOCKFORGE_REALITY_LEVEL=1 npm test
```

#### Integration Tests
```bash
# Moderate realism
MOCKFORGE_REALITY_LEVEL=3 npm test
```

#### E2E Tests
```bash
# High realism for production-like testing
MOCKFORGE_REALITY_LEVEL=4 npm test
```

#### Chaos Engineering
```bash
# Maximum chaos for resilience testing
MOCKFORGE_REALITY_LEVEL=5 npm test
```

## Best Practices

1. **Start Low, Increase Gradually**: Begin with Level 1-2 for development, increase as you approach production
2. **Use Presets**: Save common configurations for different environments
3. **CI/CD Integration**: Set appropriate levels for different test stages
4. **Monitor Impact**: Watch metrics as you change levels to understand the impact
5. **Document Your Levels**: Use preset descriptions to document when to use each configuration

## Troubleshooting

### Level Changes Not Applying

- Check that the reality slider is enabled in configuration
- Verify API endpoint is accessible: `curl http://localhost:9080/__mockforge/reality/level`
- Check server logs for errors

### Shortcuts Not Working

- Ensure you're not typing in an input field
- Check browser console for JavaScript errors
- Verify shortcuts are enabled (disabled in compact mode)

### Presets Not Loading

- Verify preset file format (JSON or YAML)
- Check file permissions
- Ensure preset path is correct
- Review server logs for import errors

## Related Documentation

- [Chaos Engineering Guide](./CHAOS_ENGINEERING.md)
- [Intelligent Behavior Guide](./INTELLIGENT_BEHAVIOR_GUIDE.md)
- [Network Profiles and Chaos](./NETWORK_PROFILES_AND_CHAOS.md)
- [Configuration Guide](./generate-configuration.md)

## Examples

### Example 1: Development Setup

```yaml
# config.dev.yaml
reality:
  enabled: true
  level: 2  # Light Simulation for fast development
```

```bash
mockforge serve --config config.dev.yaml
```

### Example 2: Staging Environment

```yaml
# config.staging.yaml
reality:
  enabled: true
  level: 4  # High Realism for staging
```

### Example 3: CI Pipeline

```yaml
# .github/workflows/test.yml
name: Test
on: [push]

jobs:
  test:
    runs-on: ubuntu-latest
    env:
      MOCKFORGE_REALITY_LEVEL: 3
    steps:
      - uses: actions/checkout@v3
      - name: Start MockForge
        run: |
          mockforge serve --reality-level $MOCKFORGE_REALITY_LEVEL &
          sleep 5
      - name: Run Tests
        run: npm test
```

### Example 4: Preset Export/Import

```bash
# Export current configuration
curl -X POST http://localhost:9080/__mockforge/reality/presets/export \
  -H "Content-Type: application/json" \
  -d '{"name": "my-test-preset", "description": "For integration tests"}'

# Import preset
curl -X POST http://localhost:9080/__mockforge/reality/presets/import \
  -H "Content-Type: application/json" \
  -d '{"path": "/path/to/my-test-preset.json"}'
```

## Summary

The Reality Slider provides a simple, unified interface for controlling mock environment realism. Whether you're developing locally, testing in CI, or simulating production conditions, the Reality Slider helps you match your mock environment to your testing needs with a single control.
