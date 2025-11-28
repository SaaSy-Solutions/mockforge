# Data Scenario Marketplace

The Data Scenario Marketplace allows you to discover, install, and use community-built realistic mock scenarios with one-click import functionality. Share your scenarios with the community or use pre-built scenarios for common use cases.

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
mockforge scenario search <query> [--category <category>] [--tags <tags>]
```

### Publish

Publish your scenario to the marketplace:

```bash
mockforge scenario publish \
  --name "my-scenario" \
  --version "1.0.0" \
  --description "My awesome scenario" \
  --category "ecommerce" \
  --tags "api,rest,mock"
```

## Scenario Structure

A scenario package must follow this structure:

```
my-scenario/
├── scenario.yaml          # Scenario metadata
├── config.yaml            # MockForge configuration
├── openapi.json           # OpenAPI specification
├── fixtures/              # Protocol-specific fixtures
│   ├── http/
│   ├── grpc/
│   └── websocket/
├── examples/              # Example data files
├── README.md              # Documentation
└── CHANGELOG.md           # Version history
```

### scenario.yaml

```yaml
name: ecommerce-store
version: 1.0.0
description: Complete e-commerce API mock
author: John Doe
category: ecommerce
tags:
  - api
  - rest
  - ecommerce
  - shopping
dependencies: []
```

## Marketplace Features

### Tags and Categories

Scenarios are organized by:

- **Categories**: ecommerce, fintech, healthcare, iot, etc.
- **Tags**: api, rest, grpc, websocket, etc.
- **Ratings**: Community ratings and reviews
- **Versioning**: Semantic versioning support

### Ratings and Reviews

Rate and review scenarios:

```bash
# Rate a scenario
mockforge scenario rate <name> --rating 5 --comment "Great scenario!"

# View ratings
mockforge scenario info <name> --show-ratings
```

### Versioning

Scenarios use semantic versioning:

```bash
# Install specific version
mockforge scenario install ecommerce-store@1.0.0

# Install latest version
mockforge scenario install ecommerce-store@latest

# Update to latest
mockforge scenario update ecommerce-store
```

## Domain-Specific Packs

### E-commerce

Complete e-commerce API scenarios:

```bash
mockforge scenario install ecommerce-store
```

Includes:
- Product catalog
- Shopping cart
- Order management
- Payment processing
- User accounts

### Fintech

Financial services scenarios:

```bash
mockforge scenario install fintech-banking
```

Includes:
- Account management
- Transactions
- Payments
- Cards
- Loans

### Healthcare

Healthcare API scenarios:

```bash
mockforge scenario install healthcare-api
```

Includes:
- Patient records
- Appointments
- Prescriptions
- Medical devices

### IoT

IoT device scenarios:

```bash
mockforge scenario install iot-devices
```

Includes:
- Device management
- Sensor data
- Commands
- Telemetry

## Integration with VBR and MockAI

Scenarios automatically integrate with VBR and MockAI:

### VBR Integration

Scenarios can include VBR entity definitions:

```yaml
# scenario.yaml
vbr_entities:
  - name: users
    schema: ./schemas/user.json
    seed_data: ./data/users.json
```

### MockAI Integration

Scenarios can include MockAI rules:

```yaml
# scenario.yaml
mockai_rules:
  - endpoint: "/users"
    rules: ./rules/users.json
```

## API Endpoints

### Marketplace API

```http
GET /api/scenarios/marketplace?category=ecommerce&tags=api
```

List scenarios from marketplace.

```http
GET /api/scenarios/marketplace/{name}
```

Get scenario details.

```http
POST /api/scenarios/marketplace/{name}/install
```

Install scenario from marketplace.

### Local Scenarios

```http
GET /api/scenarios/local
```

List installed scenarios.

```http
GET /api/scenarios/local/{name}
```

Get installed scenario details.

```http
POST /api/scenarios/local/{name}/use
```

Apply scenario to workspace.

## Use Cases

### Quick Prototyping

Start with a pre-built scenario:

```bash
# Install e-commerce scenario
mockforge scenario install ecommerce-store

# Apply to workspace
mockforge scenario use ecommerce-store

# Start server
mockforge serve --config config.yaml
```

### Team Sharing

Share scenarios within your team:

```bash
# Publish to internal registry
mockforge scenario publish \
  --name "internal-api" \
  --registry "https://internal-registry.example.com"
```

### Community Contribution

Contribute scenarios to the community:

```bash
# Publish to public marketplace
mockforge scenario publish \
  --name "my-awesome-scenario" \
  --public
```

## Best Practices

1. **Document Well**: Include comprehensive README and examples
2. **Version Properly**: Use semantic versioning
3. **Test Thoroughly**: Ensure scenarios work out of the box
4. **Tag Appropriately**: Use relevant tags and categories
5. **Keep Updated**: Maintain scenarios with bug fixes and improvements

## Troubleshooting

### Installation Fails

- Verify scenario structure is correct
- Check file permissions
- Review scenario.yaml for errors

### Scenario Not Working

- Check MockForge version compatibility
- Verify all dependencies are installed
- Review scenario documentation

### Marketplace Connection Issues

- Verify network connectivity
- Check marketplace URL is correct
- Review authentication credentials

## Related Documentation

- [Cloud Workspaces](cloud-workspaces.md) - Sharing scenarios with teams
- [VBR Engine](vbr-engine.md) - State management in scenarios
- [Configuration Guide](../configuration/files.md) - Complete configuration reference

