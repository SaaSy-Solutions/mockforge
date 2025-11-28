# Data Scenarios Marketplace

The MockForge Data Scenarios Marketplace allows you to discover, install, and use community-built realistic mock scenarios with one-click import functionality.

## Overview

Scenarios are complete mock system configurations that include:
- MockForge configuration files (`config.yaml`)
- OpenAPI specifications
- Protocol-specific fixtures
- Example data files
- Documentation

## Quick Start

### Install a Scenario

```bash
# Install from local path
mockforge scenario install ./examples/scenarios/ecommerce-store

# Install from URL
mockforge scenario install https://example.com/scenarios/ecommerce-store.zip

# Install from Git repository
mockforge scenario install https://github.com/user/scenarios#main:ecommerce-store

# Install from registry
mockforge scenario install ecommerce-store
```

### Apply Scenario to Workspace

```bash
# Apply installed scenario to current directory
mockforge scenario use ecommerce-store

# This copies:
# - config.yaml
# - openapi.json
# - fixtures/
# - examples/
```

### Start the Server

```bash
mockforge serve --config config.yaml
```

## Available Commands

### Install

Install a scenario from various sources:

```bash
mockforge scenario install <source> [--force] [--skip-validation] [--checksum <sha256>]
```

**Sources:**
- Local path: `./scenarios/my-scenario`
- URL: `https://example.com/scenario.zip`
- Git: `https://github.com/user/repo#main:scenarios/my-scenario`
- Registry: `ecommerce-store` or `ecommerce-store@1.0.0`

**Options:**
- `--force`: Force reinstall even if scenario exists
- `--skip-validation`: Skip package validation
- `--checksum`: Expected SHA-256 checksum (for URL sources)

### List

List all installed scenarios:

```bash
mockforge scenario list [--detailed]
```

### Info

Show detailed information about an installed scenario:

```bash
mockforge scenario info <name> [--version <version>]
```

### Use

Apply a scenario to the current workspace:

```bash
mockforge scenario use <name> [--version <version>]
```

This copies scenario files to the current directory, allowing you to start using the scenario immediately.

### Search

Search for scenarios in the registry:

```bash
mockforge scenario search <query> [--category <category>] [--limit <n>]
```

### Publish

Publish a scenario to the registry:

```bash
export MOCKFORGE_REGISTRY_TOKEN=your-token
mockforge scenario publish <path> [--registry <url>]
```

**Requirements:**
- Valid scenario package with `scenario.yaml` manifest
- `MOCKFORGE_REGISTRY_TOKEN` environment variable set
- Package must pass validation

### Update

Update installed scenarios:

```bash
# Update a specific scenario
mockforge scenario update <name>

# Update all scenarios
mockforge scenario update --all
```

### Uninstall

Remove an installed scenario:

```bash
mockforge scenario uninstall <name> [--version <version>]
```

## Scenario Format

Scenarios are packaged as directories with the following structure:

```
scenario-name/
├── scenario.yaml          # Scenario manifest (required)
├── README.md              # Documentation (recommended)
├── config.yaml            # MockForge configuration (recommended)
├── openapi.json           # OpenAPI spec (optional)
├── fixtures/              # Protocol-specific fixtures
│   ├── http/
│   ├── websocket/
│   └── grpc/
└── examples/              # Example data files
```

### Scenario Manifest

The `scenario.yaml` file defines the scenario metadata:

```yaml
manifest_version: "1.0"
name: ecommerce-store
version: "1.0.0"
title: E-commerce Store with Shopping Carts
description: Complete e-commerce API with shopping carts, products, and orders
author: community
category: ecommerce
tags:
  - ecommerce
  - shopping
  - cart
compatibility:
  min_version: "0.2.0"
  protocols:
    - http
    - websocket
files:
  - scenario.yaml
  - config.yaml
  - openapi.json
  - fixtures/
  - examples/
```

## Example Scenarios

### E-commerce Store

Complete e-commerce API with shopping carts, products, and orders:

```bash
mockforge scenario install ./examples/scenarios/ecommerce-store
mockforge scenario use ecommerce-store
```

**Features:**
- Product catalog with search
- Shopping cart operations
- Order processing and tracking
- User management
- Real-time updates via WebSocket

### Chat API

Real-time chat API with typing indicators:

```bash
mockforge scenario install ./examples/scenarios/chat-api
mockforge scenario use chat-api
```

**Features:**
- Message history
- Typing indicators
- User presence
- Real-time messaging via WebSocket

### Weather + Geolocation

Weather API with geolocation-based queries:

```bash
mockforge scenario install ./examples/scenarios/weather-geo
mockforge scenario use weather-geo
```

**Features:**
- Current weather data
- Forecast information
- Location-based queries
- Coordinate and city name support

## Creating Your Own Scenario

1. **Create the directory structure:**

```bash
mkdir my-scenario
cd my-scenario
```

2. **Create `scenario.yaml`:**

```yaml
manifest_version: "1.0"
name: my-scenario
version: "1.0.0"
title: My Scenario
description: Description of my scenario
author: your-name
category: other
tags:
  - tag1
  - tag2
compatibility:
  min_version: "0.2.0"
  protocols:
    - http
files:
  - scenario.yaml
  - config.yaml
  - README.md
```

3. **Add your configuration files:**
   - `config.yaml` - MockForge configuration
   - `openapi.json` - OpenAPI specification (optional)
   - `fixtures/` - Protocol fixtures
   - `examples/` - Example data

4. **Validate your scenario:**

```bash
mockforge scenario install ./my-scenario
```

5. **Publish to registry (optional):**

```bash
export MOCKFORGE_REGISTRY_TOKEN=your-token
mockforge scenario publish ./my-scenario
```

## Registry Integration

The scenarios marketplace integrates with the MockForge registry for:
- Scenario discovery
- Version management
- Automatic updates
- Community sharing

### Registry URLs

- Default: `https://registry.mockforge.dev`
- Custom: Set via `--registry` flag or environment variable

### Authentication

Publishing requires a registry token:

```bash
export MOCKFORGE_REGISTRY_TOKEN=your-token
```

## Best Practices

1. **Version your scenarios** using semantic versioning (e.g., `1.0.0`)
2. **Include comprehensive documentation** in `README.md`
3. **Test your scenario** before publishing
4. **Use descriptive tags** for better discoverability
5. **Keep scenarios focused** - one scenario per use case
6. **Update compatibility info** when using new MockForge features

## Troubleshooting

### Installation Fails

- Check that the scenario package is valid
- Verify all files listed in `scenario.yaml` exist
- Ensure compatibility requirements are met

### Update Fails

- Verify the scenario was installed from a registry
- Check network connectivity
- Ensure you have the latest MockForge version

### Publishing Fails

- Verify `MOCKFORGE_REGISTRY_TOKEN` is set
- Check that the scenario passes validation
- Ensure you have publishing permissions

## Enhanced Features

For information about advanced features including preview functionality, VBR integration, MockAI configuration, schema alignment, domain packs, and enhanced reviews, see:

- [Enhanced Features Guide](./SCENARIO_MARKETPLACE_ENHANCED_FEATURES.md) - Complete guide to advanced scenario marketplace features

## See Also

- [Configuration Guide](../CONFIG.md) - MockForge configuration options
- [OpenAPI Support](../docs/SCENARIOS.md) - Scenario switching
- [Plugin System](../docs/plugins/) - Extending MockForge with plugins
