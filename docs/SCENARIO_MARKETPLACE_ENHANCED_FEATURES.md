# Scenario Marketplace Enhanced Features

This document covers the enhanced features added to the Scenario Marketplace, including preview functionality, VBR integration, MockAI configuration, schema alignment, domain packs, and enhanced reviews.

## Table of Contents

1. [Preview Functionality](#preview-functionality)
2. [VBR Integration](#vbr-integration)
3. [MockAI Configuration](#mockai-configuration)
4. [Schema Alignment](#schema-alignment)
5. [Domain-Specific Packs](#domain-specific-packs)
6. [Enhanced Reviews](#enhanced-reviews)
7. [Example Scenarios](#example-scenarios)

## Preview Functionality

Preview scenarios before installing them to see what they contain, check compatibility, and estimate installation size.

### Basic Usage

```bash
# Preview a scenario from any source
mockforge scenario preview ./scenarios/my-scenario
mockforge scenario preview https://github.com/user/repo#main:scenarios/my-scenario
mockforge scenario preview ecommerce-store@1.0.0
```

### Preview Output

The preview command displays:

- **Manifest Information**: Name, version, description, author, category
- **Compatibility Check**: Whether the scenario is compatible with your MockForge version
- **File Tree**: Visual representation of the scenario's file structure
- **OpenAPI Endpoints**: List of all API endpoints defined in the scenario
- **Estimated Size**: Approximate installation size in bytes
- **Config Preview**: First 50 lines of the configuration file (if present)

### Example Output

```
👁️  Previewing scenario from: ./scenarios/ecommerce-store

═══════════════════════════════════════════════════════════════
Scenario: ecommerce-store@1.0.0
Title: E-commerce Store with Shopping Carts
Author: community
Category: ecommerce
═══════════════════════════════════════════════════════════════

Compatibility: ✅ Compatible
  Current Version: 0.2.7
  Required Version: >= 0.2.0

File Structure:
  scenario.yaml
  config.yaml
  openapi.json
  fixtures/
    http/
    websocket/
  examples/

OpenAPI Endpoints (5):
  GET    /products          - List products
  GET    /products/{id}     - Get product details
  POST   /cart              - Add to cart
  GET    /cart              - Get cart contents
  POST   /orders            - Create order

Estimated Size: 245 KB
```

## VBR Integration

Virtual Backend Reality (VBR) integration allows scenarios to include entity definitions that can be automatically applied to your VBR engine.

### Scenario with VBR Entities

Add VBR entity definitions to your scenario manifest:

```yaml
manifest_version: "1.0"
name: user-management
version: "1.0.0"
# ... other fields ...

vbr_entities:
  - name: User
    schema:
      type: object
      properties:
        id:
          type: string
        name:
          type: string
        email:
          type: string
        created_at:
          type: string
          format: date-time
    seed_data_path: fixtures/users.json
    state_machine:
      initial_state: active
      states:
        - active
        - suspended
        - deleted
      transitions:
        - from_state: active
          to_state: suspended
          trigger: suspend
        - from_state: suspended
          to_state: active
          trigger: reactivate
```

### Applying VBR Entities

When you install a scenario with VBR entities, they are stored with the scenario. To apply them:

```bash
# Install the scenario
mockforge scenario install user-management

# Apply VBR entities (requires VBR engine)
# This is typically done programmatically or via VBR CLI commands
```

### Retrieving VBR Entities Programmatically

```rust
use mockforge_scenarios::ScenarioInstaller;

let installer = ScenarioInstaller::new()?;
installer.init().await?;

// Get VBR entities from a scenario
if let Some(entities) = installer.get_vbr_entities("user-management", None)? {
    for entity in entities {
        println!("Entity: {}", entity.name);
        // Apply entity to VBR engine
    }
}
```

## MockAI Configuration

Scenarios can include MockAI configuration for intelligent behavior generation.

### Scenario with MockAI Config

Add MockAI configuration to your scenario manifest:

```yaml
manifest_version: "1.0"
name: intelligent-chat
version: "1.0.0"
# ... other fields ...

mockai_config:
  config:
    enabled: true
    behavior_model:
      llm_provider: ollama
      model: llama3.2
      temperature: 0.7
      max_tokens: 1000
    auto_learn: true
    mutation_detection: true
    ai_validation_errors: true
    intelligent_pagination: true
  behavior_rules_path: behavior-rules.json
  example_pairs_path: examples/chat-examples.json
```

### MockAI Config File Format

You can also include a separate `mockai.yaml` file in your scenario:

```yaml
enabled: true
behavior_model:
  llm_provider: ollama
  model: llama3.2
  temperature: 0.7
  max_tokens: 1000
auto_learn: true
mutation_detection: true
ai_validation_errors: true
intelligent_pagination: true
```

### Behavior Rules File

Create `behavior-rules.json` with custom behavior rules:

```json
{
  "rules": [
    {
      "pattern": "GET /users",
      "behavior": "return_paginated_list",
      "params": {
        "page_size": 20,
        "sort_by": "created_at"
      }
    },
    {
      "pattern": "POST /users",
      "behavior": "generate_realistic_user",
      "params": {
        "include_email": true,
        "include_avatar": true
      }
    }
  ]
}
```

### Example Pairs File

Create `examples/chat-examples.json` for learning:

```json
[
  {
    "request": {
      "method": "POST",
      "path": "/chat/messages",
      "body": {
        "user_id": "123",
        "message": "Hello"
      }
    },
    "response": {
      "status": 200,
      "body": {
        "message_id": "msg_456",
        "timestamp": "2024-01-01T12:00:00Z",
        "status": "sent"
      }
    }
  }
]
```

### Applying MockAI Config

When you apply a scenario with MockAI config, it will be merged with your existing `config.yaml`:

```bash
mockforge scenario use intelligent-chat
# MockAI config is automatically merged into config.yaml
```

## Schema Alignment

Automatically align and merge OpenAPI specifications when applying scenarios to workspaces with existing configurations.

### Merge Strategies

Choose how to handle conflicts when merging schemas:

#### Prefer Existing (Default)

Keeps existing schemas and only adds new paths from the scenario:

```bash
mockforge scenario use ecommerce-store \
  --auto-align \
  --merge-strategy prefer-existing
```

#### Prefer Scenario

Replaces existing schemas with scenario schemas:

```bash
mockforge scenario use ecommerce-store \
  --auto-align \
  --merge-strategy prefer-scenario
```

#### Intelligent

Intelligently merges operations, combining GET and POST on the same path:

```bash
mockforge scenario use ecommerce-store \
  --auto-align \
  --merge-strategy intelligent
```

#### Interactive

Prompts for each conflict (not yet implemented in CLI, available programmatically):

```rust
use mockforge_scenarios::{SchemaAlignmentConfig, MergeStrategy};

let config = SchemaAlignmentConfig {
    merge_strategy: MergeStrategy::Interactive,
    validate_merged: true,
    backup_existing: true,
};

installer.apply_to_workspace_with_alignment(
    "ecommerce-store",
    None,
    Some(config)
).await?;
```

### Example: Merging OpenAPI Specs

**Existing spec:**
```json
{
  "paths": {
    "/users": {
      "get": {
        "summary": "Get users"
      }
    }
  }
}
```

**Scenario spec:**
```json
{
  "paths": {
    "/users": {
      "post": {
        "summary": "Create user"
      }
    },
    "/products": {
      "get": {
        "summary": "Get products"
      }
    }
  }
}
```

**Merged (Intelligent strategy):**
```json
{
  "paths": {
    "/users": {
      "get": {
        "summary": "Get users"
      },
      "post": {
        "summary": "Create user"
      }
    },
    "/products": {
      "get": {
        "summary": "Get products"
      }
    }
  }
}
```

### Conflict Resolution

When conflicts occur, the alignment process:

1. **Detects conflicts**: Same path + method in both specs
2. **Applies strategy**: Based on selected merge strategy
3. **Reports warnings**: Logs all conflicts and resolutions
4. **Validates result**: Ensures merged spec is valid OpenAPI

## Domain-Specific Packs

Domain packs bundle multiple related scenarios together for specific use cases (e-commerce, fintech, IoT, etc.).

### Creating a Domain Pack

Create a `pack.yaml` file:

```yaml
manifest_version: "1.0"
name: ecommerce-pack
version: "1.0.0"
title: E-commerce Domain Pack
description: Complete e-commerce scenarios including storefront, payments, and inventory
domain: ecommerce
author: community

scenarios:
  - name: product-catalog
    version: "1.0.0"
    source: product-catalog@1.0.0
    required: true
    description: Product catalog with search and filtering

  - name: shopping-cart
    version: "1.0.0"
    source: shopping-cart@1.0.0
    required: true
    description: Shopping cart management

  - name: payment-processing
    version: "1.0.0"
    source: payment-processing@1.0.0
    required: false
    description: Payment processing integration

tags:
  - ecommerce
  - retail
  - payments
```

### Installing a Pack

```bash
# Install pack from manifest file
mockforge scenario pack install ./packs/ecommerce-pack.yaml

# List installed packs
mockforge scenario pack list

# Show pack information
mockforge scenario pack info ecommerce-pack
```

### Pack Structure

```
ecommerce-pack/
├── pack.yaml              # Pack manifest
├── README.md              # Pack documentation
└── scenarios/             # Optional: bundled scenarios
    ├── product-catalog/
    ├── shopping-cart/
    └── payment-processing/
```

### Installing Scenarios from a Pack

After installing a pack, install individual scenarios:

```bash
# Install all scenarios from a pack
mockforge scenario pack install ./packs/ecommerce-pack.yaml

# The output will show you how to install each scenario:
# To install scenarios from this pack, use:
#   mockforge scenario install product-catalog@1.0.0
#   mockforge scenario install shopping-cart@1.0.0
#   mockforge scenario install payment-processing@1.0.0
```

## Enhanced Reviews

Submit and view reviews for scenarios in the registry.

### Submitting a Review

```bash
mockforge scenario review submit \
  --scenario-name ecommerce-store \
  --scenario-version 1.0.0 \
  --rating 5 \
  --title "Excellent scenario!" \
  --comment "This scenario saved me hours of setup time. Highly recommended!" \
  --reviewer "john-doe" \
  --reviewer-email john@example.com \
  --verified
```

### Viewing Reviews

```bash
# List reviews for a scenario
mockforge scenario review list ecommerce-store

# With pagination
mockforge scenario review list ecommerce-store \
  --page 0 \
  --per-page 20
```

### Review Fields

- **Rating**: 1-5 stars
- **Title**: Optional review title
- **Comment**: Review text/comment
- **Reviewer**: Reviewer name/username
- **Reviewer Email**: Optional email (may be hidden)
- **Verified Purchase**: Whether reviewer actually used the scenario

### Review Display

```
📝 Reviews for scenario: ecommerce-store
  Found 3 reviews:

  - john-doe (5/5)
    Title: Excellent scenario!
    Comment: This scenario saved me hours of setup time. Highly recommended!
    Date: 2024-01-15T10:30:00Z
    ✓ Verified purchase
    👍 12 helpful

  - jane-smith (4/5)
    Title: Good starting point
    Comment: Works well but needed some customization for our use case.
    Date: 2024-01-14T15:20:00Z

  - dev-team (5/5)
    Title: Perfect for our needs
    Comment: Exactly what we needed for testing our e-commerce integration.
    Date: 2024-01-13T09:15:00Z
    ✓ Verified purchase
    👍 8 helpful
```

## Example Scenarios

### Complete Example: E-commerce with VBR and MockAI

Here's a complete example scenario that includes VBR entities and MockAI configuration:

**scenario.yaml:**
```yaml
manifest_version: "1.0"
name: advanced-ecommerce
version: "1.0.0"
title: Advanced E-commerce Store
description: Complete e-commerce API with VBR entities and MockAI
author: community
category: ecommerce
tags:
  - ecommerce
  - vbr
  - mockai
  - intelligent

compatibility:
  min_version: "0.2.7"
  protocols:
    - http
    - websocket

vbr_entities:
  - name: Product
    schema:
      type: object
      properties:
        id:
          type: string
        name:
          type: string
        price:
          type: number
        stock:
          type: integer
        category:
          type: string
    seed_data_path: fixtures/products.json
    state_machine:
      initial_state: available
      states:
        - available
        - out_of_stock
        - discontinued
      transitions:
        - from_state: available
          to_state: out_of_stock
          trigger: stock_depleted
        - from_state: out_of_stock
          to_state: available
          trigger: restocked

  - name: Order
    schema:
      type: object
      properties:
        id:
          type: string
        user_id:
          type: string
        items:
          type: array
          items:
            type: object
        total:
          type: number
        status:
          type: string
    seed_data_path: fixtures/orders.json

mockai_config:
  config:
    enabled: true
    behavior_model:
      llm_provider: ollama
      model: llama3.2
      temperature: 0.7
    auto_learn: true
    mutation_detection: true
    intelligent_pagination: true
  behavior_rules_path: behavior-rules.json
  example_pairs_path: examples/ecommerce-examples.json

files:
  - scenario.yaml
  - config.yaml
  - openapi.json
  - mockai.yaml
  - behavior-rules.json
  - fixtures/
  - examples/
```

**behavior-rules.json:**
```json
{
  "rules": [
    {
      "pattern": "GET /products",
      "behavior": "return_paginated_list",
      "params": {
        "page_size": 20,
        "sort_by": "name"
      }
    },
    {
      "pattern": "POST /orders",
      "behavior": "generate_order_id",
      "params": {
        "prefix": "ORD",
        "include_timestamp": true
      }
    }
  ]
}
```

### Installing and Using

```bash
# Preview the scenario first
mockforge scenario preview ./scenarios/advanced-ecommerce

# Install the scenario
mockforge scenario install ./scenarios/advanced-ecommerce

# Apply to workspace with intelligent schema alignment
mockforge scenario use advanced-ecommerce \
  --auto-align \
  --merge-strategy intelligent

# Start the server
mockforge serve --config config.yaml
```

## Best Practices

### Creating Scenarios with VBR

1. **Define clear entity schemas**: Use JSON Schema for type safety
2. **Include seed data**: Provide realistic example data
3. **Document state machines**: Explain state transitions
4. **Test entity creation**: Verify entities work with VBR engine

### Creating Scenarios with MockAI

1. **Start with simple rules**: Build complexity gradually
2. **Provide example pairs**: Help MockAI learn patterns
3. **Test behavior generation**: Verify intelligent responses
4. **Document expected behavior**: Explain what MockAI should do

### Schema Alignment

1. **Use intelligent merging**: Best for most cases
2. **Review conflicts**: Check warnings after merging
3. **Backup existing specs**: Enable backup before merging
4. **Validate merged specs**: Ensure OpenAPI validity

### Domain Packs

1. **Group related scenarios**: Keep packs focused
2. **Mark required scenarios**: Indicate dependencies
3. **Provide pack documentation**: Explain pack purpose
4. **Version packs carefully**: Update when scenarios change

### Reviews

1. **Be specific**: Explain what worked and what didn't
2. **Mark verified purchases**: Only if you actually used it
3. **Rate fairly**: Consider all aspects of the scenario
4. **Update reviews**: Revise if scenario improves

## Troubleshooting

### Preview Issues

**Preview fails to load:**
- Check scenario path is correct
- Verify scenario.yaml exists and is valid
- Ensure you have read permissions

**Compatibility check fails:**
- Update MockForge to required version
- Check compatibility requirements in manifest

### VBR Integration Issues

**Entities not applying:**
- Verify VBR engine is running
- Check entity schema is valid JSON Schema
- Ensure seed data file exists and is valid JSON

### MockAI Configuration Issues

**MockAI not working:**
- Check MockAI is enabled in config
- Verify LLM provider is configured
- Check behavior rules file is valid JSON
- Ensure example pairs are properly formatted

### Schema Alignment Issues

**Merging fails:**
- Check both specs are valid OpenAPI
- Review conflict warnings
- Try different merge strategy
- Validate merged spec manually

### Domain Pack Issues

**Pack installation fails:**
- Verify pack.yaml is valid YAML
- Check all referenced scenarios exist
- Ensure pack manifest version is correct

### Review Submission Issues

**Review submission fails:**
- Check registry connectivity
- Verify authentication token (if required)
- Ensure rating is 1-5
- Check review comment is not empty

## See Also

- [Scenario Marketplace Guide](../docs/SCENARIOS_MARKETPLACE.md) - Basic scenario marketplace usage
- [VBR Documentation](../docs/VBR_IMPLEMENTATION_SUMMARY.md) - Virtual Backend Reality
- [MockAI Guide](../docs/MOCKAI_USAGE.md) - MockAI intelligent behavior
- [OpenAPI Support](../docs/SCENARIOS.md) - OpenAPI specification support
